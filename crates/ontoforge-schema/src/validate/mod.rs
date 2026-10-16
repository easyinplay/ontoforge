//! Cross-definition validation (hard `ONT-V` errors). Grows with plans 01-04..06.

use crate::diag::Diagnostic;
use crate::ir::Ontology;
use crate::loader::SourceMap;

mod actions;
mod bindings;
mod links;
mod objects;

pub(crate) use bindings::check_fixture_files;
pub use bindings::{ProfileOrigin, check_profile};

/// All structural checks on a fully merged ontology.
pub fn validate(ontology: &Ontology, sources: &SourceMap) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    objects::check(ontology, sources, &mut out);
    links::check(ontology, sources, &mut out);
    actions::check(ontology, sources, &mut out);
    bindings::check(ontology, sources, &mut out);
    out
}
