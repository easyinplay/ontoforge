//! Dialect specific printing.

use super::ast::{Expr, Literal, Select, SelectItem};
use super::escape::{quote_ident, quote_string};
use super::{Dialect, PrintedSql};
use crate::TableRef;
use crate::error::PrintError;

pub(super) fn print_select(select: &Select, dialect: Dialect) -> Result<PrintedSql, PrintError> {
    if select.projection.is_empty() {
        return Err(PrintError::EmptyProjection);
    }
    let items = select
        .projection
        .iter()
        .map(|item| select_item(item, dialect))
        .collect::<Result<Vec<_>, _>>()?;
    let mut sql = format!("SELECT {}", items.join(", "));
    if let Some(table) = &select.from {
        sql.push_str(" FROM ");
        sql.push_str(&table_ref(table, dialect)?);
    }
    if let Some(limit) = select.limit {
        sql.push_str(" LIMIT ");
        sql.push_str(&limit.to_string());
    }
    Ok(PrintedSql::new(sql))
}

fn select_item(item: &SelectItem, dialect: Dialect) -> Result<String, PrintError> {
    let mut out = expr(&item.expr, dialect)?;
    if let Some(alias) = &item.alias {
        out.push_str(" AS ");
        out.push_str(&quote_ident(alias)?);
    }
    Ok(out)
}

fn table_ref(table: &TableRef, dialect: Dialect) -> Result<String, PrintError> {
    match dialect {
        Dialect::Trino => Ok(format!(
            "{}.{}.{}",
            quote_ident(&table.catalog)?,
            quote_ident(&table.schema)?,
            quote_ident(&table.table)?
        )),
        Dialect::Sqlite => Ok(format!(
            "{}.{}",
            quote_ident(&table.sqlite_schema_name())?,
            quote_ident(&table.table)?
        )),
    }
}

fn expr(e: &Expr, _dialect: Dialect) -> Result<String, PrintError> {
    match e {
        Expr::Column(name) => quote_ident(name),
        Expr::Literal(Literal::Null) => Ok("NULL".to_owned()),
        Expr::Literal(Literal::Long(n)) => Ok(n.to_string()),
        Expr::Literal(Literal::String(s)) => quote_string(s),
        _ => Err(PrintError::Unsupported("expression")),
    }
}
