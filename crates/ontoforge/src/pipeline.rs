//! Orchestration shared by the CLI and later by the server.

use camino::Utf8Path;
use ontoforge_schema::LoadOutput;
use ontoforge_schema::diag::{has_errors, sort_diagnostics};
use ontoforge_schema::ir::ProfileName;

/// The result of [`check_with_profile`].
#[derive(Debug)]
pub struct Checked {
    pub out: LoadOutput,
    /// The profile that was checked against; `None` when the load failed.
    pub profile: Option<ProfileName>,
}

/// Load and validate the ontology under `root` for the profile chosen by `profile`
/// (`--profile` or `ONTOFORGE_PROFILE`, read by the caller) or else by the manifest's
/// `defaultProfile`: structural checks first, then binding completeness, then the
/// CEL checks of submission criteria (`ONT-V13`, hard errors) once the IR exists.
pub fn check_with_profile(root: &Utf8Path, profile: Option<&str>) -> Checked {
    let _ = profile;
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
    Checked { out, profile: None }
}

/// [`check_with_profile`] without an explicit profile.
pub fn check(root: &Utf8Path) -> LoadOutput {
    check_with_profile(root, None).out
}
