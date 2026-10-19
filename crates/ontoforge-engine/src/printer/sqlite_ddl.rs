//! SQL text the SQLite stand-in needs besides queries: attaching databases, creating tables,
//! inserting rows. Values never appear in this text; `insert` only prints placeholders.

use super::dialect::table_ref;
use super::escape::{quote_ident, quote_string};
use super::{Dialect, PrintedSql};
use crate::TableRef;
use crate::error::PrintError;

/// Collation the stand-in registers for decimal text: orders by value, so `1.5` equals
/// `1.50` and `-10` sorts before `9`, as `DECIMAL` does in Trino (WR-E03).
pub(crate) const DECIMAL_COLLATION: &str = "ontoforge_decimal";
/// Collation the stand-in registers for timestamp text: orders by instant, so a literal
/// without milliseconds equals the stored `.000` form and offsets are taken into account
/// (a value without an offset counts as UTC, Trino's session zone) (WR-E03).
pub(crate) const TIMESTAMP_COLLATION: &str = "ontoforge_timestamp";

/// SQLite storage class of a stand-in column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SqliteStorage {
    Text,
    Integer,
    Real,
    /// TEXT compared with [`DECIMAL_COLLATION`].
    Decimal,
    /// TEXT compared with [`TIMESTAMP_COLLATION`].
    Timestamp,
}

impl SqliteStorage {
    /// The storage keyword and the collation, if any.
    fn keyword(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Text => ("TEXT", None),
            Self::Integer => ("INTEGER", None),
            Self::Real => ("REAL", None),
            Self::Decimal => ("TEXT", Some(DECIMAL_COLLATION)),
            Self::Timestamp => ("TEXT", Some(TIMESTAMP_COLLATION)),
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

/// `CREATE TABLE "<schema>"."<table>" ("col" TYPE [COLLATE "<name>"], ...)`.
pub(crate) fn create_table(
    table: &TableRef,
    columns: &[(String, SqliteStorage)],
) -> Result<PrintedSql, PrintError> {
    if columns.is_empty() {
        return Err(PrintError::EmptyProjection);
    }
    let defs = columns
        .iter()
        .map(|(name, storage)| {
            let (keyword, collation) = storage.keyword();
            let mut def = format!("{} {keyword}", quote_ident(name)?);
            if let Some(collation) = collation {
                def.push_str(" COLLATE ");
                def.push_str(&quote_ident(collation)?);
            }
            Ok(def)
        })
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
