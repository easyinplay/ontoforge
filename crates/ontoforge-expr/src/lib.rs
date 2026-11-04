//! CEL expression wrapper (ADR-0011). Implemented in Phase 1 (plans 01-07 and 01-12).

pub mod analyze;
pub mod check;
pub mod infer;
pub mod limits;
pub mod rowpred;
pub mod walk;

pub use analyze::{ExprError, analyze};
pub use check::check_ontology;
pub use limits::{MAX_EXPR_CHARS, MAX_EXPR_NESTING};
pub use rowpred::{
    AttrValue, AttrValueError, Attrs, CmpOp, ListSource, MatchKind, Operand, Pred, PredContext,
    PredIssue, RowValues, Scalar, Tri, compile_row_filter, eval,
};
pub use walk::{MemberPath, member_paths};

/// Cargo package name of this crate.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }
}
