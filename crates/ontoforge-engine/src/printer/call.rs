//! Procedure calls the service makes besides queries.

use super::PrintedSql;
use crate::error::PrintError;

/// `CALL "<catalog>".system.flush_metadata_cache()`.
pub(super) fn flush_metadata_cache(_catalog: &str) -> Result<PrintedSql, PrintError> {
    Err(PrintError::EmptyProjection)
}
