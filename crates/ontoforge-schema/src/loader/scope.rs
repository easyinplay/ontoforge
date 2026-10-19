//! Loading a scope directory: roles and role bindings (D-01).

use camino::Utf8Path;

use super::SourceMap;
use crate::diag::Diagnostic;
use crate::ir::ScopeSet;

#[derive(Debug)]
pub struct ScopeLoadOutput {
    /// `Some` only when no error was found.
    pub scope: Option<ScopeSet>,
    pub sources: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn load_scope_dir(_root: &Utf8Path) -> ScopeLoadOutput {
    ScopeLoadOutput {
        scope: None,
        sources: SourceMap::default(),
        diagnostics: Vec::new(),
    }
}

pub fn load_scope_from_memory(_files: &[(&str, &str)]) -> ScopeLoadOutput {
    ScopeLoadOutput {
        scope: None,
        sources: SourceMap::default(),
        diagnostics: Vec::new(),
    }
}
