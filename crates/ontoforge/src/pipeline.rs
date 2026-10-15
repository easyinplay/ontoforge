//! Orchestration shared by the CLI and later by the server.

use camino::Utf8Path;
use ontoforge_schema::LoadOutput;
use ontoforge_schema::diag::{has_errors, sort_diagnostics};

/// Load and validate the ontology under `root`: structural checks first, then the
/// CEL checks of submission criteria (`ONT-V13`, hard errors) once the IR exists.
pub fn check(root: &Utf8Path) -> LoadOutput {
    let mut out = ontoforge_schema::load_dir(root);
    if let Some(ontology) = &out.ontology {
        let cel = ontoforge_expr::check_ontology(ontology, &out.sources);
        if !cel.is_empty() {
            out.diagnostics.extend(cel);
            sort_diagnostics(&mut out.diagnostics);
        }
    }
    if has_errors(&out.diagnostics) {
        // `ontology` is `Some` only when no error was found.
        out.ontology = None;
    }
    out
}
