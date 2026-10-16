//! `ontoforge validate --online`: check the bindings of the selected profile against the
//! engine (BIND-03).
//!
//! Without `--server-config` the engine is the SQLite stand-in, loaded from the fixtures the
//! bindings name (ADR-0021). Every physical table is described once per catalog and compared
//! with its binding by `ontoforge_engine::bind_check::compare_binding`; each difference
//! becomes an `ONT-V17` pointing into the binding file.

use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::bind_check::{
    BindingKey, DescribedBinding, compare_binding, describe_bindings,
};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::{FixtureError, plan_fixture_tables};
use ontoforge_engine::{AnyEngine, EngineError, QueryContext};
use ontoforge_schema::LoadOutput;
use ontoforge_schema::diag::{Diagnostic, Severity, codes, has_errors, sort_diagnostics};
use ontoforge_schema::ir::{BindingTarget, Ontology, ProfileName, SourceBinding};
use ontoforge_schema::loader::MANIFEST;
use ontoforge_schema::loader::SourceMap;
use ontoforge_schema::loader::sourcemap::{DefKey, DefKind, Part, Seg};

use crate::cli::ValidateArgs;
use crate::{pipeline, render};

/// Exit code when the engine could not answer for at least one binding.
const EXIT_ENGINE: u8 = 3;

/// The outcome of an online check.
#[derive(Debug)]
pub struct OnlineRun {
    pub out: LoadOutput,
    pub code: u8,
    /// The profile that was checked; `None` when no profile is selected.
    pub profile: Option<ProfileName>,
    /// How many bindings were compared with their physical table.
    pub bindings: usize,
}

/// Runs the online check and reports what the summary line needs.
pub fn run_online(args: &ValidateArgs) -> OnlineRun {
    let pipeline::Checked { mut out, profile } =
        pipeline::check_with_profile(&args.dir, args.profile.profile.as_deref());
    let deny_warnings = args.output.deny_warnings;
    // `ontology` is `Some` only when the offline checks found no error.
    let (Some(ontology), Some(profile)) = (out.ontology.take(), profile.clone()) else {
        let code = render::exit_code(&out.diagnostics, deny_warnings);
        return OnlineRun {
            out,
            code,
            profile,
            bindings: 0,
        };
    };

    let (found, bindings) =
        check_online(&args.dir, args.timeout, &ontology, &out.sources, &profile);
    out.diagnostics.extend(found);
    sort_diagnostics(&mut out.diagnostics);
    if !has_errors(&out.diagnostics) {
        out.ontology = Some(ontology);
    }
    let code = if out.diagnostics.iter().any(|d| d.code == codes::V19) {
        EXIT_ENGINE
    } else {
        render::exit_code(&out.diagnostics, deny_warnings)
    };
    OnlineRun {
        out,
        code,
        profile: Some(profile),
        bindings,
    }
}

/// [`run_online`] without the summary details.
pub fn validate_online(args: &ValidateArgs) -> (LoadOutput, u8) {
    let run = run_online(args);
    (run.out, run.code)
}

/// ONT-V19 for an engine failure that stops the whole round (not tied to a binding).
pub fn engine_error_diagnostic(e: &EngineError, file: &str) -> Diagnostic {
    Diagnostic::without_location(
        codes::V19,
        Severity::Error,
        format!("the online check could not complete: {e}"),
        file,
    )
    .with_help(format!("engine error `{}`", e.code()))
}

/// The context of the online check's queries.
pub(crate) fn query_context(timeout_ms: Option<u64>) -> QueryContext {
    let mut ctx = QueryContext::new(
        format!("ontoforge-cli-{}", ulid::Ulid::generate()),
        vec!["ontoforge".to_owned(), "validate-online".to_owned()],
    );
    if let Some(ms) = timeout_ms {
        ctx.timeout = Duration::from_millis(ms);
    }
    ctx
}

/// The findings of the online check and how many bindings were compared.
fn check_online(
    root: &Utf8Path,
    timeout_ms: Option<u64>,
    ontology: &Ontology,
    sources: &SourceMap,
    profile: &ProfileName,
) -> (Vec<Diagnostic>, usize) {
    let plan = match plan_fixture_tables(ontology, sources, profile) {
        Ok(plan) => plan,
        Err(errors) => return (fixture_diagnostics(&errors), 0),
    };
    if !plan.without_fixture.is_empty() {
        // Skipping a binding silently would report a profile as checked that was not.
        let found = plan
            .without_fixture
            .iter()
            .map(|key| missing_fixture_diagnostic(key, ontology, sources))
            .collect();
        return (found, 0);
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            let diagnostic = Diagnostic::without_location(
                codes::V19,
                Severity::Error,
                format!("the online check could not start its async runtime: {e}"),
                MANIFEST,
            );
            return (vec![diagnostic], 0);
        }
    };
    runtime.block_on(async {
        let engine = match SqliteEngine::from_fixtures(root, &plan.tables).await {
            Ok(engine) => AnyEngine::Sqlite(engine),
            Err(errors) => return (fixture_diagnostics(&errors), 0),
        };
        let ctx = query_context(timeout_ms);
        let described = match describe_bindings(&engine, ontology, profile, &ctx).await {
            Ok(described) => described,
            Err(e) => return (vec![engine_error_diagnostic(&e, MANIFEST)], 0),
        };
        let found = described
            .iter()
            .flat_map(|d| binding_diagnostics(d, ontology, sources))
            .collect();
        (found, described.len())
    })
}

/// ONT-V16 for fixture data that cannot be loaded.
fn fixture_diagnostics(errors: &[FixtureError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|e| {
            let message = match e.line {
                Some(line) => format!("{} (line {line})", e.message),
                None => e.message.clone(),
            };
            let file = if e.file.is_empty() { MANIFEST } else { &e.file };
            Diagnostic::without_location(codes::V16, Severity::Error, message, file)
        })
        .collect()
}

fn source_key(key: &BindingKey) -> DefKey {
    DefKey {
        kind: DefKind::SourceBinding,
        api_name: key.def_name(),
    }
}

fn target_name(key: &BindingKey) -> &str {
    match &key.target {
        BindingTarget::ObjectType(name) => name.as_str(),
        BindingTarget::LinkType(name) => name.as_str(),
    }
}

/// ONT-V20: a binding without a fixture cannot be checked without `--server-config`.
fn missing_fixture_diagnostic(
    key: &BindingKey,
    ontology: &Ontology,
    sources: &SourceMap,
) -> Diagnostic {
    let _ = ontology;
    let at = sources.locate(
        &source_key(key),
        &[Seg::Key("source".to_owned())],
        Part::Value,
    );
    Diagnostic::error(
        codes::V20,
        format!(
            "the binding of `{}` has no fixture, so there is no table to check it against",
            target_name(key)
        ),
        &at,
    )
    .with_help("pass --server-config to check this profile against Trino")
}

fn binding_of<'a>(ontology: &'a Ontology, key: &BindingKey) -> Option<&'a SourceBinding> {
    let bindings = ontology.profile_bindings(&key.profile)?;
    match &key.target {
        BindingTarget::ObjectType(name) => bindings.objects.get(name),
        BindingTarget::LinkType(name) => bindings.links.get(name),
    }
}

/// The diagnostics of one described binding: ONT-V19 when the engine failed for it, otherwise
/// one ONT-V17 per mismatch.
fn binding_diagnostics(
    described: &DescribedBinding,
    ontology: &Ontology,
    sources: &SourceMap,
) -> Vec<Diagnostic> {
    let key = &described.key;
    let physical = match &described.physical {
        Ok(physical) => physical,
        Err(e) => {
            let at = sources.locate(
                &source_key(key),
                &[Seg::Key("source".to_owned())],
                Part::Value,
            );
            return vec![
                Diagnostic::error(
                    codes::V19,
                    format!(
                        "the online check of `{}` could not complete: {e}",
                        target_name(key)
                    ),
                    &at,
                )
                .with_help(format!("engine error `{}`", e.code())),
            ];
        }
    };
    let Some(binding) = binding_of(ontology, key) else {
        return Vec::new();
    };
    compare_binding(ontology, binding, physical.as_deref())
        .into_iter()
        .map(|mismatch| {
            let at = sources.locate(&source_key(key), &mismatch.path, Part::Value);
            let diagnostic = Diagnostic::new(codes::V17, mismatch.severity, mismatch.message, &at);
            match mismatch.help {
                Some(help) => diagnostic.with_help(help),
                None => diagnostic,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ontoforge_engine::{DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS};

    use super::query_context;

    #[test]
    fn query_context_uses_defaults_and_timeout_flag() {
        let ctx = query_context(None);
        assert_eq!(ctx.timeout, Duration::from_millis(DEFAULT_QUERY_TIMEOUT_MS));
        assert_eq!(ctx.max_rows, DEFAULT_MAX_ROWS);
        assert_eq!(ctx.max_bytes, DEFAULT_MAX_BYTES);
        assert!(ctx.trace_token.starts_with("ontoforge-cli-"));

        let ctx = query_context(Some(1500));
        assert_eq!(ctx.timeout, Duration::from_millis(1500));
        assert_eq!(ctx.max_rows, DEFAULT_MAX_ROWS);
    }
}
