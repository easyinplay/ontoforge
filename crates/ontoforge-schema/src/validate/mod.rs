//! Cross-definition validation (hard `ONT-V` errors). Grows with plans 01-04..06.

use crate::diag::Diagnostic;
use crate::ir::Ontology;
use crate::loader::SourceMap;

mod links;
mod objects;

/// All structural checks on a fully merged ontology.
pub fn validate(ontology: &Ontology, sources: &SourceMap) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    objects::check(ontology, sources, &mut out);
    links::check(ontology, sources, &mut out);
    out
}
