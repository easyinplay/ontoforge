//! `ontoforge validate --online`: check the bindings of the selected profile against the
//! engine (BIND-03).
//!
//! Without `--server-config` the engine is the SQLite stand-in, loaded from the fixtures the
//! bindings name (ADR-0021). With it, the profile's `engine` entry decides: `sqlite` is the
//! same stand-in, `trino` is the deployment's Trino, reached with the ontology service
//! identity. Every physical table is described once per catalog and compared with its
//! binding by `ontoforge_engine::bind_check::compare_binding`; each difference becomes an
//! `ONT-V17` pointing into the binding file. An engine that cannot answer is `ONT-V19` and
//! exit code 3, never a pass (ENGINE-02).

use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::bind_check::{
    BindingKey, DescribedBinding, compare_binding, describe_bindings,
};
use ontoforge_engine::config::{
    ConfigError, EngineKind, ServerConfig, TrinoConfig, build_trino_engines, load_server_config,
};
use ontoforge_engine::pk_check::{
    MAX_DUPLICATE_SAMPLES, PkFinding, check_primary_keys, display_value,
};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::{FixtureError, plan_fixture_tables};
use ontoforge_engine::{AnyEngine, EngineError, QueryContext};
use ontoforge_schema::LoadOutput;
use ontoforge_schema::diag::{
    Diagnostic, Severity, SourceLocation, codes, has_errors, offset_to_line_col, sort_diagnostics,
};
use ontoforge_schema::ir::{BindingTarget, Ontology, ProfileName, SourceBinding};
use ontoforge_schema::loader::sourcemap::{DefKey, DefKind, Part, Seg};
use ontoforge_schema::loader::{MANIFEST, SourceMap};

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
    let offline_only = |out: LoadOutput, profile: Option<ProfileName>| {
        let code = render::exit_code(&out.diagnostics, deny_warnings);
        OnlineRun {
            out,
            code,
            profile,
            bindings: 0,
        }
    };
    // Without a profile the ontology stays in `out` for the summary (WR-C01).
    let Some(profile) = profile else {
        return offline_only(out, None);
    };
    // `ontology` is `Some` only when the offline checks found no error.
    let Some(ontology) = out.ontology.take() else {
        return offline_only(out, Some(profile));
    };

    let (found, bindings) = check_online(args, &ontology, &out.sources, &profile);
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

/// What an engine failure means to the operator.
fn engine_failure(e: &EngineError) -> String {
    match e {
        EngineError::Unavailable { reason, .. } => format!("Trino is unavailable: {reason}"),
        EngineError::Unauthorized { reason } => format!("Trino authentication failed: {reason}"),
        EngineError::Timeout { after_ms } => {
            format!("the query timed out after {after_ms} ms")
        }
        EngineError::Protocol(message) => format!("Trino protocol error: {message}"),
        EngineError::Query {
            error_name,
            message,
        } => format!("query failed ({error_name}): {message}"),
        other => other.to_string(),
    }
}

/// ONT-V19 for an engine failure that stops the whole round (not tied to a binding).
pub fn engine_error_diagnostic(e: &EngineError, file: &str) -> Diagnostic {
    Diagnostic::without_location(codes::V19, Severity::Error, engine_failure(e), file)
        .with_help(format!("engine error `{}`", e.code()))
}

/// The context of the online check's queries: the limits of `trino` when the check runs
/// against it, and `--timeout` over its query timeout.
pub(crate) fn query_context(trino: Option<&TrinoConfig>, timeout_ms: Option<u64>) -> QueryContext {
    let token = format!("ontoforge-cli-{}", ulid::Ulid::generate());
    let tags = vec!["ontoforge".to_owned(), "validate-online".to_owned()];
    let mut ctx = match trino {
        Some(trino) => trino.query_context(token, tags),
        None => QueryContext::new(token, tags),
    };
    if let Some(ms) = timeout_ms {
        ctx.timeout = Duration::from_millis(ms);
    }
    ctx
}

/// The engine the check runs against.
enum Backend {
    /// The SQLite stand-in loaded from the fixtures of the bindings.
    Fixtures,
    /// The deployment's Trino.
    Trino(Box<TrinoBackend>),
}

struct TrinoBackend {
    engine: AnyEngine,
    ctx: QueryContext,
}

/// The findings of the online check and how many bindings were compared.
fn check_online(
    args: &ValidateArgs,
    ontology: &Ontology,
    sources: &SourceMap,
    profile: &ProfileName,
) -> (Vec<Diagnostic>, usize) {
    let backend = match select_backend(args, profile) {
        Ok(backend) => backend,
        Err(found) => return (found, 0),
    };
    let fixtures = match &backend {
        Backend::Fixtures => match plan_fixture_tables(ontology, sources, profile) {
            Ok(plan) if plan.without_fixture.is_empty() => Some(plan),
            Ok(plan) => {
                // Skipping a binding silently would report a profile as checked that was not.
                let found = plan
                    .without_fixture
                    .iter()
                    .map(|key| missing_fixture_diagnostic(key, sources))
                    .collect();
                return (found, 0);
            }
            Err(errors) => return (fixture_diagnostics(&errors), 0),
        },
        Backend::Trino(_) => None,
    };
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
    let check = Check {
        ontology,
        sources,
        profile,
    };
    runtime.block_on(async {
        match (backend, fixtures) {
            (Backend::Trino(trino), _) => {
                match compare_all(&trino.engine, &trino.ctx, check, args.check_pk).await {
                    Ok((compared, count)) => {
                        let mut found = ignored_fixtures(ontology, sources, profile);
                        found.extend(compared);
                        (found, count)
                    }
                    // A round that never reached Trino reports nothing but the failure.
                    Err(failure) => (vec![failure], 0),
                }
            }
            (Backend::Fixtures, Some(plan)) => {
                let engine = match SqliteEngine::from_fixtures(&args.dir, &plan.tables).await {
                    Ok(engine) => AnyEngine::Sqlite(engine),
                    Err(errors) => return (fixture_diagnostics(&errors), 0),
                };
                let ctx = query_context(None, args.timeout);
                compare_all(&engine, &ctx, check, args.check_pk)
                    .await
                    .unwrap_or_else(|failure| (vec![failure], 0))
            }
            (Backend::Fixtures, None) => (Vec::new(), 0),
        }
    })
}

/// Describes and compares every binding of `profile` on `engine`, then checks the primary
/// keys when `check_pk`; `Err` is the ONT-V19 of a failure that stopped the whole round.
async fn compare_all(
    engine: &AnyEngine,
    ctx: &QueryContext,
    check: Check<'_>,
    check_pk: bool,
) -> Result<(Vec<Diagnostic>, usize), Diagnostic> {
    let described = describe_bindings(engine, check.ontology, check.profile, ctx)
        .await
        .map_err(|e| engine_error_diagnostic(&e, MANIFEST))?;
    let mut found = Vec::new();
    let mut pk_keys = Vec::new();
    for d in &described {
        let diagnostics = binding_diagnostics(d, check.ontology, check.sources);
        // A table that does not match its binding, or could not be described, is not scanned.
        let usable = !diagnostics.iter().any(|d| d.severity == Severity::Error);
        if usable && matches!(d.key.target, BindingTarget::ObjectType(_)) {
            pk_keys.push(d.key.clone());
        }
        found.extend(diagnostics);
    }
    if check_pk {
        // The round has reached the engine: a failure of the key scan joins what was found
        // so far instead of replacing it (WR-C02).
        found.extend(check_keys(engine, ctx, check, &pk_keys).await);
    }
    Ok((found, described.len()))
}

/// What the online check runs over.
#[derive(Clone, Copy)]
struct Check<'a> {
    ontology: &'a Ontology,
    sources: &'a SourceMap,
    profile: &'a ProfileName,
}

/// `--check-pk`: one ONT-V18 per binding with duplicate keys and one per binding with NULL
/// keys. Each binding is checked on its own so that an error of one catalog only costs the
/// bindings of that catalog. An engine-wide failure ends the scan with its ONT-V19 after the
/// findings of the bindings scanned before it.
async fn check_keys(
    engine: &AnyEngine,
    ctx: &QueryContext,
    check: Check<'_>,
    keys: &[BindingKey],
) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for key in keys {
        match check_primary_keys(
            engine,
            check.ontology,
            check.profile,
            std::slice::from_ref(key),
            ctx,
        )
        .await
        {
            Ok(findings) => {
                for finding in &findings {
                    found.extend(pk_diagnostics(finding, check.sources));
                }
            }
            Err(e) if e.is_engine_wide() => {
                found.push(engine_error_diagnostic(&e, MANIFEST));
                break;
            }
            Err(e) => found.push(binding_failure(key, &e, check.sources)),
        }
    }
    found
}

/// The ONT-V18 diagnostics of one finding, located at the binding's `source.table`.
fn pk_diagnostics(finding: &PkFinding, sources: &SourceMap) -> Vec<Diagnostic> {
    let at = sources.locate(
        &source_key(&finding.key),
        &[Seg::Key("source".to_owned()), Seg::Key("table".to_owned())],
        Part::Value,
    );
    let name = target_name(&finding.key);
    let table = format!(
        "{}.{}.{}",
        finding.table.catalog, finding.table.schema, finding.table.table
    );
    let mut found = Vec::new();
    if !finding.duplicates.is_empty() {
        let values = finding
            .duplicates
            .iter()
            .map(|(value, n)| format!("`{}` ×{n}", display_value(value)))
            .collect::<Vec<_>>()
            .join(", ");
        let more = if finding.more_duplicates {
            format!(", and more (showing up to {MAX_DUPLICATE_SAMPLES})")
        } else {
            String::new()
        };
        found.push(
            Diagnostic::error(
                codes::V18,
                format!(
                    "`{name}` has {} duplicate primary key value(s) in `{table}`: {values}{more}",
                    finding.duplicates.len()
                ),
                &at,
            )
            .with_help("a primary key value must identify exactly one row"),
        );
    }
    if finding.null_count > 0 {
        found.push(
            Diagnostic::error(
                codes::V18,
                format!(
                    "`{name}` has {} row(s) with a NULL primary key in `{table}`",
                    finding.null_count
                ),
                &at,
            )
            .with_help("every row needs a primary key value"),
        );
    }
    found
}

/// Picks the engine: the SQLite stand-in unless `--server-config` maps the profile to Trino.
fn select_backend(args: &ValidateArgs, profile: &ProfileName) -> Result<Backend, Vec<Diagnostic>> {
    let Some(path) = &args.server_config else {
        return Ok(Backend::Fixtures);
    };
    let file = path.as_str();
    let (config, _text) = load_server_config(path).map_err(|(errors, text)| {
        // The text only turns spans into line and column. It is never registered as a source:
        // a rejected line may hold a plaintext secret (a `password`, a `token`, the userinfo of
        // `url`), and the human renderer prints the lines around a span (CR-C01).
        errors
            .iter()
            .map(|e| config_diagnostic(e, file, text.as_deref()))
            .collect::<Vec<_>>()
    })?;
    match engine_of(&config, profile, file)? {
        EngineKind::Sqlite => Ok(Backend::Fixtures),
        EngineKind::Trino => {
            let Some(trino) = &config.trino else {
                let error = ConfigError {
                    message: format!(
                        "profile `{profile}` uses engine \"trino\" but there is no [trino] section"
                    ),
                    help: Some("add a [trino] section, or use engine = \"sqlite\"".to_owned()),
                    span: None,
                };
                return Err(vec![config_diagnostic(&error, file, None)]);
            };
            let config_dir = path
                .parent()
                .filter(|p| !p.as_str().is_empty())
                .unwrap_or(Utf8Path::new("."));
            let engines = build_trino_engines(trino, config_dir, &|name| std::env::var(name).ok())
                .map_err(|errors| {
                    errors
                        .iter()
                        .map(|e| config_diagnostic(e, file, None))
                        .collect::<Vec<_>>()
                })?;
            Ok(Backend::Trino(Box::new(TrinoBackend {
                engine: AnyEngine::Trino(engines.ontology),
                ctx: query_context(Some(trino), args.timeout),
            })))
        }
    }
}

/// The engine kind `config` maps `profile` to.
fn engine_of(
    config: &ServerConfig,
    profile: &ProfileName,
    file: &str,
) -> Result<EngineKind, Vec<Diagnostic>> {
    match config.profiles.get(profile) {
        Some(entry) => Ok(entry.engine),
        None => Err(vec![
            Diagnostic::without_location(
                codes::V20,
                Severity::Error,
                format!("profile `{profile}` has no engine in the server configuration"),
                file,
            )
            .with_help(format!(
                "add a `[profiles.{profile}]` section with `engine = \"trino\"` or `engine = \"sqlite\"`"
            )),
        ]),
    }
}

/// ONT-V20 for a problem of the server configuration, located when the error has a span.
fn config_diagnostic(e: &ConfigError, file: &str, text: Option<&str>) -> Diagnostic {
    let diagnostic = match (&e.span, text) {
        (Some(span), Some(text)) => {
            let (line, column) = offset_to_line_col(text, span.start);
            let at = SourceLocation {
                file: file.to_owned(),
                line,
                column,
                offset: span.start,
                length: span.len(),
            };
            Diagnostic::error(codes::V20, e.message.clone(), &at)
        }
        _ => Diagnostic::without_location(codes::V20, Severity::Error, e.message.clone(), file),
    };
    match &e.help {
        Some(help) => diagnostic.with_help(help.clone()),
        None => diagnostic,
    }
}

/// One ONT-V17 warning per binding that names a fixture the Trino check does not use.
fn ignored_fixtures(
    ontology: &Ontology,
    sources: &SourceMap,
    profile: &ProfileName,
) -> Vec<Diagnostic> {
    let Some(bindings) = ontology.profile_bindings(profile) else {
        return Vec::new();
    };
    bindings
        .objects
        .values()
        .chain(bindings.links.values())
        .filter(|binding| binding.fixture.is_some())
        .map(|binding| {
            let key = BindingKey {
                profile: profile.clone(),
                target: binding.target.clone(),
            };
            let at = sources.locate(
                &source_key(&key),
                &[Seg::Key("fixture".to_owned()), Seg::Key("file".to_owned())],
                Part::Value,
            );
            Diagnostic::new(
                codes::V17,
                Severity::Warning,
                format!(
                    "the fixture of `{}` is ignored because profile `{profile}` is checked against Trino",
                    target_name(&key)
                ),
                &at,
            )
        })
        .collect()
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
fn missing_fixture_diagnostic(key: &BindingKey, sources: &SourceMap) -> Diagnostic {
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

/// ONT-V19 for an engine failure that only concerns the binding `key`.
fn binding_failure(key: &BindingKey, e: &EngineError, sources: &SourceMap) -> Diagnostic {
    let at = sources.locate(
        &source_key(key),
        &[Seg::Key("source".to_owned())],
        Part::Value,
    );
    Diagnostic::error(
        codes::V19,
        format!(
            "the online check of `{}` could not complete: {}",
            target_name(key),
            engine_failure(e)
        ),
        &at,
    )
    .with_help(format!("engine error `{}`", e.code()))
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
        Err(e) => return vec![binding_failure(key, e, sources)],
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

    use ontoforge_engine::config::parse_server_config;
    use ontoforge_engine::{DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS};

    use super::query_context;

    #[test]
    fn query_context_uses_defaults_and_timeout_flag() {
        let ctx = query_context(None, None);
        assert_eq!(ctx.timeout, Duration::from_millis(DEFAULT_QUERY_TIMEOUT_MS));
        assert_eq!(ctx.max_rows, DEFAULT_MAX_ROWS);
        assert_eq!(ctx.max_bytes, DEFAULT_MAX_BYTES);
        assert!(ctx.trace_token.starts_with("ontoforge-cli-"));
        assert_eq!(ctx.client_tags, ["ontoforge", "validate-online"]);

        let ctx = query_context(None, Some(1500));
        assert_eq!(ctx.timeout, Duration::from_millis(1500));
        assert_eq!(ctx.max_rows, DEFAULT_MAX_ROWS);
    }

    #[test]
    fn trino_context_takes_the_configured_limits() {
        let config = parse_server_config(
            "[trino]\nurl = \"http://127.0.0.1:8080\"\nquery_timeout_ms = 12000\nmax_rows = 50\n\
             [trino.identities.ontology]\nuser = \"u\"\n",
        )
        .expect("valid");
        let trino = config.trino.expect("trino section");

        let ctx = query_context(Some(&trino), None);
        assert_eq!(ctx.timeout, Duration::from_secs(12));
        assert_eq!(ctx.max_rows, 50);
        assert!(ctx.trace_token.starts_with("ontoforge-cli-"));

        let ctx = query_context(Some(&trino), Some(2_000));
        assert_eq!(ctx.timeout, Duration::from_secs(2));
        assert_eq!(ctx.max_rows, 50);
        assert_eq!(ctx.client_tags, ["ontoforge", "validate-online"]);
    }

    #[test]
    fn trino_context_sends_the_configured_client_tags() {
        // WR-E02: `[trino].client_tags` reaches the queries.
        let config = parse_server_config(
            "[trino]\nurl = \"http://127.0.0.1:8080\"\nclient_tags = [\"team-a\"]\n\
             [trino.identities.ontology]\nuser = \"u\"\n",
        )
        .expect("valid");
        let trino = config.trino.expect("trino section");
        let ctx = query_context(Some(&trino), None);
        assert_eq!(ctx.client_tags, ["team-a", "ontoforge", "validate-online"]);
    }
}
