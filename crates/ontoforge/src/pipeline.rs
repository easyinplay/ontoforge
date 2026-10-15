//! Orchestration shared by the CLI and later by the server.

use camino::Utf8Path;
use ontoforge_schema::LoadOutput;

/// Load and validate the ontology under `root`.
pub fn check(root: &Utf8Path) -> LoadOutput {
    ontoforge_schema::load_dir(root)
}
