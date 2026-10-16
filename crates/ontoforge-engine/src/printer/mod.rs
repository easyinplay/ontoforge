//! The typed SQL printer: the only producer of [`PrintedSql`].
//!
//! `PrintedSql::new` has no visibility modifier, so only this module and its children can
//! call it. The compiler therefore guarantees that SQL text reaching an engine was produced
//! here (ENGINE-03).

pub mod ast;
mod dialect;
mod escape;
pub(crate) mod sqlite_ddl;

use std::fmt;

use crate::error::PrintError;

/// SQL text produced by the printer. The field is private and the constructor is private to
/// this module tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedSql(String);

impl PrintedSql {
    fn new(sql: String) -> Self {
        Self(sql)
    }

    /// The SQL text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PrintedSql {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// SQL dialect to print for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Trino,
    Sqlite,
}

/// Prints `select` for `dialect`.
pub fn print_select(select: &ast::Select, dialect: Dialect) -> Result<PrintedSql, PrintError> {
    dialect::print_select(select, dialect)
}
