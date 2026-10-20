//! Criteria column coverage (ONT-V23, SCOPE-06).
//!
//! RED placeholder (plan 03-14 Task 2): reports nothing.

#![allow(dead_code)]

use ontoforge_schema::diag::{Diagnostic, SourceLocation};
use ontoforge_schema::ir::{ActionTypeName, Ontology, Role};

pub(crate) fn criteria_coverage(
    role: &Role,
    ontology: &Ontology,
    at: &dyn Fn(&ActionTypeName) -> SourceLocation,
) -> Vec<Diagnostic> {
    let _ = (role, ontology, at);
    Vec::new()
}
