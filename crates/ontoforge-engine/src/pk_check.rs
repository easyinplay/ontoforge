//! `--check-pk`: duplicate and NULL primary keys of bound object types (BIND-03).
//!
//! Both queries scan the whole table, so they only run when the operator asks for them
//! (ADR-0015). The duplicate query leaves NULL out; NULL keys are only counted by the second
//! query, so two NULL rows never show up as one duplicated value (E17).

use ontoforge_schema::ir::{BindingTarget, Ontology, ProfileName};

use crate::bind_check::BindingKey;
use crate::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use crate::printer::print_select;
use crate::{
    AnyEngine, Dialect, Engine, EngineError, PrintError, PrintedSql, QueryContext, TableRef, Value,
};

/// At most this many duplicated values are reported per binding.
pub const MAX_DUPLICATE_SAMPLES: usize = 20;

/// Longest value text in a message, in characters.
const MAX_VALUE_CHARS: usize = 64;

/// The primary key problems of one object binding.
#[derive(Debug, Clone, PartialEq)]
pub struct PkFinding {
    pub key: BindingKey,
    pub table: TableRef,
    /// Duplicated values with their row counts, most frequent first, at most
    /// [`MAX_DUPLICATE_SAMPLES`].
    pub duplicates: Vec<(Value, i64)>,
    /// There are more duplicated values than `duplicates` lists.
    pub more_duplicates: bool,
    /// Rows whose primary key is NULL.
    pub null_count: i64,
}

/// The two queries of the check.
#[derive(Debug, Clone)]
pub struct PkQueries {
    pub duplicates: PrintedSql,
    pub nulls: PrintedSql,
}

fn item(expr: Expr, alias: &str) -> SelectItem {
    SelectItem {
        expr,
        alias: Some(alias.to_owned()),
    }
}

/// The duplicate and NULL count queries for column `pk` of `table`.
pub fn pk_queries(table: &TableRef, pk: &str, dialect: Dialect) -> Result<PkQueries, PrintError> {
    let column = || Expr::Column(pk.to_owned());
    let duplicates = Select {
        projection: vec![item(column(), "pk"), item(Expr::CountStar, "n")],
        from: Some(table.clone()),
        filter: Some(Expr::IsNull {
            expr: Box::new(column()),
            negated: true,
        }),
        group_by: vec![column()],
        having: Some(Expr::Compare {
            left: Box::new(Expr::CountStar),
            op: CompareOp::Gt,
            right: Box::new(Expr::Literal(Literal::Long(1))),
        }),
        order_by: vec![
            OrderItem {
                expr: Expr::CountStar,
                descending: true,
                nulls_first: false,
            },
            OrderItem {
                expr: column(),
                descending: false,
                nulls_first: false,
            },
        ],
        limit: Some(MAX_DUPLICATE_SAMPLES as u64 + 1),
    };
    let nulls = Select {
        projection: vec![item(Expr::CountStar, "n")],
        from: Some(table.clone()),
        filter: Some(Expr::IsNull {
            expr: Box::new(column()),
            negated: false,
        }),
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    Ok(PkQueries {
        duplicates: print_select(&duplicates, dialect)?,
        nulls: print_select(&nulls, dialect)?,
    })
}

/// A row count from a `count(*)` cell.
fn count(v: Option<&Value>) -> Result<i64, EngineError> {
    match v {
        Some(Value::Long(n)) => Ok(*n),
        other => Err(EngineError::Protocol(format!(
            "expected a row count, got {other:?}"
        ))),
    }
}

/// Checks the primary keys of the object bindings among `keys`; link bindings and keys
/// without a binding in `profile` are skipped. Only bindings with a problem are returned.
pub async fn check_primary_keys(
    engine: &AnyEngine,
    ontology: &Ontology,
    profile: &ProfileName,
    keys: &[BindingKey],
    ctx: &QueryContext,
) -> Result<Vec<PkFinding>, EngineError> {
    let Some(bindings) = ontology.profile_bindings(profile) else {
        return Ok(Vec::new());
    };
    let mut sample_ctx = ctx.clone();
    sample_ctx.max_rows = MAX_DUPLICATE_SAMPLES + 1;
    let mut found = Vec::new();
    for key in keys {
        let BindingTarget::ObjectType(name) = &key.target else {
            continue;
        };
        let (Some(object), Some(binding)) =
            (ontology.object_types.get(name), bindings.objects.get(name))
        else {
            continue;
        };
        let table = TableRef {
            catalog: binding.source.catalog.clone(),
            schema: binding.source.schema.clone(),
            table: binding.source.table.clone(),
        };
        let pk = binding.column_for(object.primary_key.as_str());
        let queries = pk_queries(&table, pk, engine.dialect())
            .map_err(|e| EngineError::Config(e.to_string()))?;

        let out = engine.execute(&queries.duplicates, &sample_ctx).await?;
        let mut duplicates = out
            .rows
            .into_iter()
            .map(|row| {
                let n = count(row.get(1))?;
                let value = row.into_iter().next().unwrap_or(Value::Null);
                Ok((value, n))
            })
            .collect::<Result<Vec<_>, EngineError>>()?;
        let more_duplicates = duplicates.len() > MAX_DUPLICATE_SAMPLES;
        duplicates.truncate(MAX_DUPLICATE_SAMPLES);

        let out = engine.execute(&queries.nulls, ctx).await?;
        let null_count = count(out.rows.first().and_then(|row| row.first()))?;

        if !duplicates.is_empty() || null_count > 0 {
            found.push(PkFinding {
                key: key.clone(),
                table,
                duplicates,
                more_duplicates,
                null_count,
            });
        }
    }
    Ok(found)
}

/// Short text of `v` for a message: strings as they are, numbers in decimal, NULL as `NULL`,
/// cut to 64 characters followed by `…`.
#[must_use]
pub fn display_value(v: &Value) -> String {
    let text = match v {
        Value::Null => "NULL".to_owned(),
        Value::Boolean(b) => b.to_string(),
        Value::Long(n) => n.to_string(),
        Value::Double(f) => f.to_string(),
        Value::Decimal(s)
        | Value::String(s)
        | Value::Date(s)
        | Value::Timestamp(s)
        | Value::TimestampTz(s) => s.clone(),
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Unsupported(raw) => raw.to_string(),
    };
    if text.chars().count() > MAX_VALUE_CHARS {
        let mut cut: String = text.chars().take(MAX_VALUE_CHARS).collect();
        cut.push('…');
        cut
    } else {
        text
    }
}
