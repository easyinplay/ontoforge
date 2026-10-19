//! The only exits for values and identifiers into SQL text.
//!
//! Every string literal and every identifier in generated SQL goes through [`quote_string`]
//! or [`quote_ident`]. Neither Trino nor SQLite treats a backslash specially inside a
//! string literal, so doubling the quote character is the complete escaping rule.

use crate::error::PrintError;

/// Quotes `s` as a SQL string literal. Rejects NUL: SQLite's C API truncates SQL text there.
pub(super) fn quote_string(s: &str) -> Result<String, PrintError> {
    if s.contains('\0') {
        return Err(PrintError::NulInString);
    }
    Ok(format!("'{}'", s.replace('\'', "''")))
}

/// Quotes `s` as a delimited identifier. Rejects the empty string and NUL.
pub(super) fn quote_ident(s: &str) -> Result<String, PrintError> {
    if s.is_empty() {
        return Err(PrintError::EmptyIdentifier);
    }
    if s.contains('\0') {
        return Err(PrintError::NulInIdentifier);
    }
    Ok(format!("\"{}\"", s.replace('"', "\"\"")))
}
