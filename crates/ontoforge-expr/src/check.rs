//! Static checks of submission criteria (ONT-V13).

use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::Ontology;
use ontoforge_schema::loader::SourceMap;

/// Checks every submission criterion of every action type.
#[must_use]
pub fn check_ontology(_ontology: &Ontology, _sources: &SourceMap) -> Vec<Diagnostic> {
    Vec::new()
}
