//! Procedure calls the service makes besides queries.
//!
//! The only variable part of a call is a catalog name, and it goes through the same identifier
//! quoting as every other name, so a catalog name cannot end a statement or start another one.

use super::PrintedSql;
use super::escape::quote_ident;
use crate::error::PrintError;

/// `CALL "<catalog>".system.flush_metadata_cache()`: makes a Trino connector forget the table
/// metadata it cached, so tables and columns created since become visible.
pub(super) fn flush_metadata_cache(catalog: &str) -> Result<PrintedSql, PrintError> {
    Ok(PrintedSql::new(format!(
        "CALL {}.system.flush_metadata_cache()",
        quote_ident(catalog)?
    )))
}
