//! Explicit source OneToOne checks. This checks FK uniqueness, not referential integrity.

use ontoforge_schema::ir::{
    Backing, BindingTarget, Cardinality, LinkTypeName, Ontology, ProfileName,
};

use crate::bind_check::BindingKey;
use crate::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use crate::printer::print_select;
use crate::{Dialect, Engine, EngineError, PrintError, PrintedSql, QueryContext, TableRef, Value};

pub const MAX_CARDINALITY_SAMPLES: usize = 20;

/// One source-backed to-object binding eligible after metadata validation.
#[derive(Debug, Clone, PartialEq)]
pub struct CardinalityTarget {
    pub link: LinkTypeName,
    pub key: BindingKey,
    pub table: TableRef,
    pub foreign_key: String,
    pub primary_key: String,
}

/// The bounded result of an executed check, including clean results with empty samples.
#[derive(Debug, Clone, PartialEq)]
pub struct CardinalityFinding {
    pub target: CardinalityTarget,
    pub samples: Vec<(Value, i64)>,
    pub more: bool,
}

/// Select source-backed OneToOne to-objects with a usable binding in this profile.
#[must_use]
pub fn cardinality_targets(
    ontology: &Ontology,
    profile: &ProfileName,
    usable: &[BindingKey],
) -> Vec<CardinalityTarget> {
    let Some(bindings) = ontology.profile_bindings(profile) else {
        return Vec::new();
    };
    ontology
        .link_types
        .values()
        .filter_map(|link| {
            if link.cardinality != Cardinality::OneToOne {
                return None;
            }
            let object = ontology.object_types.get(&link.to.object_type)?;
            if object.backing != Backing::Source {
                return None;
            }
            let key = BindingKey {
                profile: profile.clone(),
                target: BindingTarget::ObjectType(link.to.object_type.clone()),
            };
            if !usable.contains(&key) {
                return None;
            }
            let binding = bindings.objects.get(&link.to.object_type)?;
            let fk = link.to.foreign_key.as_ref()?;
            Some(CardinalityTarget {
                link: link.api_name.clone(),
                key,
                table: TableRef {
                    catalog: binding.source.catalog.clone(),
                    schema: binding.source.schema.clone(),
                    table: binding.source.table.clone(),
                },
                foreign_key: binding.column_for(fk.as_str()).to_owned(),
                primary_key: binding.column_for(object.primary_key.as_str()).to_owned(),
            })
        })
        .collect()
}

/// Count distinct logical objects, excluding NULL foreign keys and NULL primary keys.
pub fn cardinality_query(
    target: &CardinalityTarget,
    dialect: Dialect,
) -> Result<PrintedSql, PrintError> {
    let fk = Expr::Column(target.foreign_key.clone());
    let count = Expr::CountDistinct(Box::new(Expr::Column(target.primary_key.clone())));
    print_select(
        &Select {
            projection: vec![
                SelectItem {
                    expr: fk.clone(),
                    alias: Some("fk".to_owned()),
                },
                SelectItem {
                    expr: count.clone(),
                    alias: Some("n".to_owned()),
                },
            ],
            from: Some(target.table.clone()),
            filter: Some(Expr::IsNull {
                expr: Box::new(fk.clone()),
                negated: true,
            }),
            group_by: vec![fk.clone()],
            having: Some(Expr::Compare {
                left: Box::new(count.clone()),
                op: CompareOp::Gt,
                right: Box::new(Expr::Literal(Literal::Long(1))),
            }),
            order_by: vec![
                OrderItem {
                    expr: Expr::Column("n".to_owned()),
                    descending: true,
                    nulls_first: false,
                },
                OrderItem {
                    expr: Expr::Column("fk".to_owned()),
                    descending: false,
                    nulls_first: false,
                },
            ],
            limit: Some(MAX_CARDINALITY_SAMPLES as u64 + 1),
        },
        dialect,
    )
}

/// Scan one target without widening the caller's budgets. A truncated transport is not a
/// complete result, even if it returned the SQL sentinel row.
pub async fn check_cardinality(
    engine: &impl Engine,
    target: &CardinalityTarget,
    ctx: &QueryContext,
) -> Result<CardinalityFinding, EngineError> {
    let query = cardinality_query(target, engine.dialect())?;
    let out = engine.execute(&query, ctx).await?;
    if out.truncated {
        return Err(EngineError::Query {
            error_name: "CARDINALITY_CHECK_INCOMPLETE".to_owned(),
            message: "cardinality query result was truncated by a row or byte limit".to_owned(),
        });
    }
    let mut samples = out
        .rows
        .into_iter()
        .map(|row| match row.as_slice() {
            [fk, Value::Long(n)] if *fk != Value::Null && *n > 1 => Ok((fk.clone(), *n)),
            _ => Err(EngineError::Protocol(
                "expected a foreign key and distinct object count".to_owned(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let more = samples.len() > MAX_CARDINALITY_SAMPLES;
    samples.truncate(MAX_CARDINALITY_SAMPLES);
    Ok(CardinalityFinding {
        target: target.clone(),
        samples,
        more,
    })
}
