//! RED stub: replaced by the real loader in the GREEN commit.

use camino::Utf8Path;

use crate::diag::Diagnostic;
use crate::ir::Ontology;

pub mod sourcemap;

pub use sourcemap::SourceMap;

pub struct LoadOutput {
    pub ontology: Option<Ontology>,
    pub sources: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn load_dir(_root: &Utf8Path) -> LoadOutput {
    LoadOutput {
        ontology: None,
        sources: SourceMap::default(),
        diagnostics: Vec::new(),
    }
}

pub fn load_from_memory(_files: &[(&str, &str)]) -> LoadOutput {
    LoadOutput {
        ontology: None,
        sources: SourceMap::default(),
        diagnostics: Vec::new(),
    }
}
