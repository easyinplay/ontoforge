//! Orchestration shared by the CLI and later by the server.

use camino::Utf8Path;
use ontoforge_schema::LoadOutput;
use ontoforge_schema::diag::{
    Diagnostic, Severity, codes, has_errors, parse_custom_de_error, sort_diagnostics,
};
use ontoforge_schema::ir::{Ontology, ProfileName};
use ontoforge_schema::loader::MANIFEST;
use ontoforge_schema::validate::{ProfileOrigin, check_profile};

/// The result of [`check_with_profile`].
#[derive(Debug)]
pub struct Checked {
    pub out: LoadOutput,
    /// The profile that was checked against; `None` when the load failed or no
    /// profile is selected.
    pub profile: Option<ProfileName>,
}

/// Load and validate the ontology under `root` for the profile chosen by `profile`
/// (`--profile` or `ONTOFORGE_PROFILE`, read by the caller, never here) or else by
/// the manifest's `defaultProfile`: structural checks first, then binding
/// completeness, then the CEL checks of submission criteria (`ONT-V13`, hard errors)
/// once the IR exists.
pub fn check_with_profile(root: &Utf8Path, profile: Option<&str>) -> Checked {
    let mut out = ontoforge_schema::load_dir(root);
    let mut chosen = None;
    if let Some(ontology) = &out.ontology {
        match select_profile(ontology, profile) {
            Ok(selection) => {
                let found = check_profile(
                    ontology,
                    &out.sources,
                    selection.as_ref().map(|(p, origin)| (p, *origin)),
                );
                out.diagnostics.extend(found);
                chosen = selection.map(|(p, _)| p);
            }
            Err(d) => out.diagnostics.push(d),
        }
        out.diagnostics
            .extend(ontoforge_expr::check_ontology(ontology, &out.sources));
        sort_diagnostics(&mut out.diagnostics);
    }
    if has_errors(&out.diagnostics) {
        // `ontology` is `Some` only when no error was found.
        out.ontology = None;
    }
    Checked {
        out,
        profile: chosen,
    }
}

/// [`check_with_profile`] without an explicit profile.
pub fn check(root: &Utf8Path) -> LoadOutput {
    check_with_profile(root, None).out
}

/// `requested` (flag or environment) wins over the manifest's `defaultProfile`. A
/// requested name must be a valid profile name; it is only ever looked up in the
/// loaded bindings, never turned into a path.
fn select_profile(
    ontology: &Ontology,
    requested: Option<&str>,
) -> Result<Option<(ProfileName, ProfileOrigin)>, Diagnostic> {
    match requested {
        Some(raw) => match ProfileName::try_from(raw.to_owned()) {
            Ok(profile) => Ok(Some((profile, ProfileOrigin::Flag))),
            Err(text) => {
                let (message, help) =
                    parse_custom_de_error(&text).map_or((text, None), |c| (c.message, c.help));
                let d =
                    Diagnostic::without_location(codes::V16, Severity::Error, message, MANIFEST);
                Err(match help {
                    Some(h) => d.with_help(h),
                    None => d,
                })
            }
        },
        None => Ok(ontology
            .default_profile
            .clone()
            .map(|profile| (profile, ProfileOrigin::Manifest))),
    }
}
