//! `ontoforge lint` orchestration: validate, then configuration, then rules (D-06).
//!
//! In the library, not the binary, so a later server can lint the same way.

use camino::Utf8Path;
use ontoforge_schema::LoadOutput;
use ontoforge_schema::diag::{has_errors, sort_diagnostics};
use ontoforge_schema::lint;

use crate::pipeline;

/// Check the ontology under `root` for `profile` (see [`pipeline::check_with_profile`]);
/// when it has no errors, load the lint
/// configuration (`config` overrides `<root>/ontoforge.toml`) and run the rules.
///
/// Structural errors (`ONT-V*`) and configuration errors (`ONT-V15`) stop the run
/// before any rule executes; they are returned as the only error diagnostics.
pub fn lint(root: &Utf8Path, config: Option<&Utf8Path>, profile: Option<&str>) -> LoadOutput {
    let mut out = pipeline::check_with_profile(root, profile).out;
    if has_errors(&out.diagnostics) {
        return out;
    }
    let Some(ontology) = out.ontology.as_ref() else {
        return out;
    };
    let (config, config_diags) = lint::load_config(root, config, &mut out.sources);
    let config_failed = has_errors(&config_diags);
    out.diagnostics.extend(config_diags);
    if !config_failed {
        out.diagnostics
            .extend(lint::run(ontology, &out.sources, &config));
    }
    sort_diagnostics(&mut out.diagnostics);
    out
}
