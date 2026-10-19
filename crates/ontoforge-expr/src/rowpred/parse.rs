//! CEL text to [`Pred`]: parse with `cel`, lower to the own IR, same-row and type checks.

use std::collections::BTreeMap;

use ontoforge_schema::ir::{AttrType, ObjectType};

use super::ir::Pred;

/// What a row filter may refer to.
pub struct PredContext<'a> {
    /// The object type whose rows the filter selects.
    pub object_type: &'a ObjectType,
    /// The declared principal attributes (`[auth.principal_attrs]`, D-05).
    pub attrs: &'a BTreeMap<String, AttrType>,
}

/// A row filter problem, positioned inside the expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredIssue {
    pub message: String,
    /// 1-based `(line, column)` inside the expression.
    pub pos: Option<(usize, usize)>,
}

/// Compiles `src` into a [`Pred`].
///
/// # Errors
///
/// Every problem found, positioned inside the expression.
pub fn compile_row_filter(_src: &str, _cx: &PredContext<'_>) -> Result<Pred, Vec<PredIssue>> {
    Err(vec![PredIssue {
        message: "not implemented".to_owned(),
        pos: None,
    }])
}
