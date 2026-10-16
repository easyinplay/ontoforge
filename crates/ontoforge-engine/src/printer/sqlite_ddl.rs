//! SQL text the SQLite stand-in needs besides queries: attaching databases, creating tables,
//! inserting rows. Values never appear in this text; `insert` only prints placeholders.

#![allow(dead_code, reason = "consumed by fixture loading in plan 02-05")]

use super::dialect::table_ref;
use super::escape::{quote_ident, quote_string};
use super::{Dialect, PrintedSql};
use crate::TableRef;
use crate::error::PrintError;

/// SQLite storage class of a stand-in column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqliteStorage {
    Text,
    Integer,
    Real,
}

impl SqliteStorage {
    fn keyword(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::Integer => "INTEGER",
            Self::Real => "REAL",
        }
    }
}

/// `ATTACH DATABASE ':memory:' AS "<schema_name>"`.
pub(crate) fn attach_memory(schema_name: &str) -> Result<PrintedSql, PrintError> {
    Ok(PrintedSql::new(format!(
        "ATTACH DATABASE {} AS {}",
        quote_string(":memory:")?,
        quote_ident(schema_name)?
    )))
}

/// `CREATE TABLE "<schema>"."<table>" ("col" TYPE, ...)`.
pub(crate) fn create_table(
    table: &TableRef,
    columns: &[(String, SqliteStorage)],
) -> Result<PrintedSql, PrintError> {
    if columns.is_empty() {
        return Err(PrintError::EmptyProjection);
    }
    let defs = columns
        .iter()
        .map(|(name, storage)| Ok(format!("{} {}", quote_ident(name)?, storage.keyword())))
        .collect::<Result<Vec<_>, PrintError>>()?;
    Ok(PrintedSql::new(format!(
        "CREATE TABLE {} ({})",
        table_ref(table, Dialect::Sqlite)?,
        defs.join(", ")
    )))
}

/// `INSERT INTO "<schema>"."<table>" ("col", ...) VALUES (?1, ...)`.
pub(crate) fn insert(table: &TableRef, columns: &[String]) -> Result<PrintedSql, PrintError> {
    if columns.is_empty() {
        return Err(PrintError::EmptyProjection);
    }
    let names = columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Result<Vec<_>, _>>()?;
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
    Ok(PrintedSql::new(format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table_ref(table, Dialect::Sqlite)?,
        names.join(", "),
        placeholders.join(", ")
    )))
}

/// `PRAGMA query_only = ON`.
pub(crate) fn query_only() -> PrintedSql {
    PrintedSql::new("PRAGMA query_only = ON".to_owned())
}
