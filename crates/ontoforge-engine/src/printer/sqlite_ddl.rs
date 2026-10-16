//! SQL text the SQLite stand-in needs besides queries: attaching databases, creating tables,
//! inserting rows. Values never appear in this text; `insert` only prints placeholders.

#![allow(dead_code, reason = "consumed by fixture loading in plan 02-05")]

use super::PrintedSql;
use crate::TableRef;
use crate::error::PrintError;

/// SQLite storage class of a stand-in column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqliteStorage {
    Text,
    Integer,
    Real,
}

/// `ATTACH DATABASE ':memory:' AS "<schema_name>"`.
pub(crate) fn attach_memory(_schema_name: &str) -> Result<PrintedSql, PrintError> {
    Ok(PrintedSql::new(String::new()))
}

/// `CREATE TABLE "<schema>"."<table>" ("col" TYPE, ...)`.
pub(crate) fn create_table(
    _table: &TableRef,
    _columns: &[(String, SqliteStorage)],
) -> Result<PrintedSql, PrintError> {
    Ok(PrintedSql::new(String::new()))
}

/// `INSERT INTO "<schema>"."<table>" ("col", ...) VALUES (?1, ...)`.
pub(crate) fn insert(_table: &TableRef, _columns: &[String]) -> Result<PrintedSql, PrintError> {
    Ok(PrintedSql::new(String::new()))
}

/// `PRAGMA query_only = ON`.
pub(crate) fn query_only() -> PrintedSql {
    PrintedSql::new(String::new())
}
