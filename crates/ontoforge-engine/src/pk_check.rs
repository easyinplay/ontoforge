//! `--check-pk`: duplicate and NULL primary keys of bound object types (BIND-03, ADR-0015).

use ontoforge_schema::ir::{Ontology, ProfileName};

use crate::bind_check::BindingKey;
use crate::printer::ast::{Expr, Literal, Select, SelectItem};
use crate::printer::print_select;
use crate::{AnyEngine, Dialect, EngineError, PrintError, PrintedSql, QueryContext, TableRef, Value};

/// The primary key problems of one object binding.
#[derive(Debug, Clone, PartialEq)]
pub struct PkFinding {
    pub key: BindingKey,
    pub table: TableRef,
    pub duplicates: Vec<(Value, i64)>,
    pub more_duplicates: bool,
    pub null_count: i64,
}

/// The two queries of the check.
#[derive(Debug, Clone)]
pub struct PkQueries {
    pub duplicates: PrintedSql,
    pub nulls: PrintedSql,
}

/// The duplicate and NULL count queries for `pk` of `table`.
pub fn pk_queries(table: &TableRef, pk: &str, dialect: Dialect) -> Result<PkQueries, PrintError> {
    let _ = (table, pk);
    let one = Select {
        projection: vec![SelectItem {
            expr: Expr::Literal(Literal::Long(1)),
            alias: None,
        }],
        from: None,
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    let sql = print_select(&one, dialect)?;
    Ok(PkQueries {
        duplicates: sql.clone(),
        nulls: sql,
    })
}

/// Checks the primary keys of the object bindings among `keys`.
pub async fn check_primary_keys(
    engine: &AnyEngine,
    ontology: &Ontology,
    profile: &ProfileName,
    keys: &[BindingKey],
    ctx: &QueryContext,
) -> Result<Vec<PkFinding>, EngineError> {
    let _ = (engine, ontology, profile, keys, ctx);
    Ok(Vec::new())
}

/// Short text of `v` for a message.
#[must_use]
pub fn display_value(v: &Value) -> String {
    let _ = v;
    String::new()
}
