//! The typed SQL printer: the only producer of [`PrintedSql`].
//!
//! `PrintedSql::new` has no visibility modifier, so only this module and its children can
//! call it. The compiler therefore guarantees that SQL text reaching an engine was produced
//! here (ENGINE-03).

pub mod ast;
mod call;
mod dialect;
mod escape;
pub mod l2;
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

pub fn print_explain_distributed(
    select: &ast::Select,
    dialect: Dialect,
) -> Result<PrintedSql, PrintError> {
    if dialect != Dialect::Trino {
        return Err(PrintError::UnsupportedDialect);
    }
    print_explain_distributed_sql(&print_select(select, dialect)?, dialect)
}

/// Wraps the exact printer output passed to an Engine, for plan evidence of that execution.
pub fn print_explain_distributed_sql(
    sql: &PrintedSql,
    dialect: Dialect,
) -> Result<PrintedSql, PrintError> {
    if dialect != Dialect::Trino {
        return Err(PrintError::UnsupportedDialect);
    }
    Ok(PrintedSql::new(format!(
        "EXPLAIN (TYPE DISTRIBUTED) {}",
        sql.as_str()
    )))
}

/// Prints the Trino call that makes a catalog forget the table metadata it cached:
/// `CALL "<catalog>".system.flush_metadata_cache()`. The catalog name is the only variable part
/// and goes through the identifier quoting.
pub fn print_flush_metadata_cache(catalog: &str) -> Result<PrintedSql, PrintError> {
    call::flush_metadata_cache(catalog)
}

#[cfg(test)]
mod tests;
