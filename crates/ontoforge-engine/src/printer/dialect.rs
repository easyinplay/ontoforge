//! Dialect specific printing.

use super::ast::Select;
use super::{Dialect, PrintedSql};
use crate::error::PrintError;

pub(super) fn print_select(_select: &Select, _dialect: Dialect) -> Result<PrintedSql, PrintError> {
    Ok(PrintedSql::new(String::new()))
}
