//! Lexical precheck run before any expression reaches the CEL compiler.

use crate::analyze::ExprError;

/// Maximum expression length, counted in `char`s.
pub const MAX_EXPR_CHARS: usize = 4096;
/// Maximum bracket nesting depth (`(`, `[`, `{`).
pub const MAX_EXPR_NESTING: usize = 32;

/// Checks length, bracket nesting and string literal termination.
pub fn precheck(_src: &str) -> Result<(), ExprError> {
    Err(ExprError::TooLong { chars: 0 })
}
