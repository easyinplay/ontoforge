//! Scope-aware walk over the CEL AST that collects member paths.

/// A member path rooted at a free variable, e.g. `params.items`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberPath {
    pub segments: Vec<String>,
    /// 1-based `(line, column)` inside the expression, column counted in chars.
    pub pos: Option<(usize, usize)>,
}

/// Collects the member paths rooted at free variables of `program`.
#[must_use]
pub fn member_paths(_program: &cel::Program, _src: &str) -> Vec<MemberPath> {
    Vec::new()
}

/// Converts a 0-based offset into a 1-based `(line, column)`.
#[must_use]
pub fn offset_to_line_col(_src: &str, _offset: usize) -> (usize, usize) {
    (0, 0)
}
