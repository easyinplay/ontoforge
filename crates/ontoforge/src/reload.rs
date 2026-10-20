//! `ontoforge reload`: bring the edit store's tables in line with the definitions (D-15).
//!
//! The command is thin: load, validate and lint the definitions, open the edit store named by
//! the server configuration, call `ontoforge_storage::reload::reload`, print the outcome. A
//! definition with an error never reaches the store, so no DDL runs for it.
//!
//! Exit codes: 0 success, 1 diagnostics about the definitions or the configuration, 3 the edit
//! store could not be used (ONT-V26). Neither messages nor output carry a connection string:
//! the store configuration only names an environment variable or a file for it.

use camino::Utf8Path;
use ontoforge_auth::scope::attr_types;
use ontoforge_engine::config::{AuthConfig, ServerConfig, load_server_config};
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{Ontology, ProfileName};
use ontoforge_schema::loader::SourceMap;
use ontoforge_storage::error::StoreError;
use ontoforge_storage::reload::{ReloadError, ReloadOptions, ReloadOutcome, RolesCheck};
use ontoforge_storage::store::{EditStore, StoreSettings};

use crate::cli::ReloadArgs;
use crate::render::{self, Format};
use crate::{lint_cmd, online, print_rendered};

/// Exit code when the edit store could not be used.
pub(crate) const EXIT_STORE: u8 = 3;

/// Runs `ontoforge reload` and returns the process exit code.
pub fn run(args: &ReloadArgs) -> u8 {
    let out = lint_cmd::lint(&args.dir, None, args.profile.profile.as_deref());
    let failed = render::exit_code(&out.diagnostics, args.output.deny_warnings) != 0;
    let Some(ontology) = out.ontology.as_ref().filter(|_| !failed) else {
        finish(&out.diagnostics, &out.sources, None, args);
        if args.output.format == Format::Human {
            eprintln!(
                "{} problem(s) found, the store was not touched",
                out.diagnostics.len()
            );
        }
        return 1;
    };

    let mut diagnostics = out.diagnostics.clone();
    let (settings, auth) = match server_settings(&args.server_config) {
        Ok(found) => found,
        Err(found) => {
            diagnostics.extend(found);
            return fail(&diagnostics, &out.sources, 1, args);
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            diagnostics.push(store_diagnostic(
                &format!("could not start the async runtime: {e}"),
                &args.server_config,
            ));
            return fail(&diagnostics, &out.sources, EXIT_STORE, args);
        }
    };

    // A name that is not valid was rejected when the ontology was loaded (ONT-V16).
    let profile = args
        .profile
        .profile
        .clone()
        .and_then(|name| ProfileName::try_from(name).ok());
    let opts = ReloadOptions {
        dry_run: args.dry_run,
        roles: Some(RolesCheck {
            attrs: attr_types(auth.as_ref()),
            profile,
        }),
        probe: None,
    };
    match runtime.block_on(open_and_reload(settings, ontology, &opts)) {
        Ok(outcome) => {
            finish(&diagnostics, &out.sources, Some(&outcome), args);
            if args.output.format == Format::Human {
                eprintln!("{}", summary(&outcome, args.dry_run));
            }
            0
        }
        Err(ReloadError::Definition(found)) => {
            diagnostics.extend(found);
            fail(&diagnostics, &out.sources, 1, args)
        }
        Err(ReloadError::Store(e)) => {
            let (code, exit) = store_failure(&e);
            diagnostics.push(Diagnostic::without_location(
                code,
                Severity::Error,
                e.to_string(),
                args.server_config.as_str(),
            ));
            fail(&diagnostics, &out.sources, exit, args)
        }
        Err(e @ ReloadError::LockTimeout) => {
            diagnostics.push(lock_timeout_diagnostic(&e, &args.server_config));
            fail(&diagnostics, &out.sources, EXIT_STORE, args)
        }
        Err(e @ ReloadError::NotVisible { .. }) => {
            diagnostics.push(not_visible_diagnostic(&e, &args.server_config));
            finish(&diagnostics, &out.sources, None, args);
            if args.output.format == Format::Human {
                eprintln!(
                    "reload not finished: the new tables are in the database, the previous definition is still active"
                );
            }
            EXIT_STORE
        }
    }
}

/// ONT-V26 for new tables or columns that Trino does not show in time. The DDL is committed and
/// only adds, so running the command again is safe.
pub(crate) fn not_visible_diagnostic(
    error: &dyn std::fmt::Display,
    config: &Utf8Path,
) -> Diagnostic {
    store_diagnostic(&error.to_string(), config).with_help(
        "check that the Trino catalog in `[store.trino]` maps the edit schema (see docs/edit-store.md), that array columns are mapped (`postgresql.array-mapping=AS_ARRAY`) and that the metadata cache can be flushed, then run the command again; raise `probe_timeout_ms` in `[store]` if Trino is slow",
    )
}

/// ONT-V26 for a lock that was not free in time. The previous definition is still active.
pub(crate) fn lock_timeout_diagnostic(
    error: &dyn std::fmt::Display,
    config: &Utf8Path,
) -> Diagnostic {
    store_diagnostic(&error.to_string(), config).with_help(
        "another reload, migration or scope apply, or a long write, holds a lock; retry, or raise `lock_timeout_ms` in `[store]`",
    )
}

async fn open_and_reload(
    settings: StoreSettings,
    ontology: &Ontology,
    opts: &ReloadOptions<'_>,
) -> Result<ReloadOutcome, ReloadError> {
    let store = EditStore::open(settings).await?;
    ontoforge_storage::reload::reload(&store, ontology, opts).await
}

/// The `[store]` settings of the server configuration at `path`.
pub(crate) fn store_settings(path: &Utf8Path) -> Result<StoreSettings, Vec<Diagnostic>> {
    server_settings(path).map(|(settings, _)| settings)
}

/// The `[store]` settings and the `[auth]` section of the server configuration at `path`.
pub(crate) fn server_settings(
    path: &Utf8Path,
) -> Result<(StoreSettings, Option<AuthConfig>), Vec<Diagnostic>> {
    let file = path.as_str();
    let (config, _text): (ServerConfig, String) =
        load_server_config(path).map_err(|(errors, text)| {
            // The text only turns spans into line and column and is never printed (CR-C01).
            errors
                .iter()
                .map(|e| online::config_diagnostic(e, file, text.as_deref()))
                .collect::<Vec<_>>()
        })?;
    let Some(store) = &config.store else {
        return Err(vec![
            Diagnostic::without_location(
                codes::V20,
                Severity::Error,
                "the server configuration has no [store] section",
                file,
            )
            .with_help("add a [store] section, for example `kind = \"sqlite\"` with a `path`"),
        ]);
    };
    let config_dir = path
        .parent()
        .filter(|p| !p.as_str().is_empty())
        .unwrap_or(Utf8Path::new("."));
    let settings = StoreSettings::from_config(store, config_dir, &|name| std::env::var(name).ok())
        .map_err(|e| vec![store_error_diagnostic(&e, file)])?;
    Ok((settings, config.auth))
}

fn store_error_diagnostic(e: &StoreError, file: &str) -> Diagnostic {
    let (code, _) = store_failure(e);
    Diagnostic::without_location(code, Severity::Error, e.to_string(), file)
}

pub(crate) fn store_diagnostic(message: &str, file: &Utf8Path) -> Diagnostic {
    Diagnostic::without_location(codes::V26, Severity::Error, message, file.as_str())
}

/// The diagnostic code and exit code of a store failure: a configuration the build cannot serve
/// is a configuration error, everything else is an unusable store.
pub(crate) fn store_failure(e: &StoreError) -> (&'static str, u8) {
    match e {
        StoreError::Config(_) => (codes::V20, 1),
        _ => (codes::V26, EXIT_STORE),
    }
}

/// Prints the failure and returns `exit`.
fn fail(diagnostics: &[Diagnostic], sources: &SourceMap, exit: u8, args: &ReloadArgs) -> u8 {
    finish(diagnostics, sources, None, args);
    if args.output.format == Format::Human {
        eprintln!("reload failed, the edit store was not changed");
    }
    exit
}

/// Prints diagnostics and the result on stdout.
fn finish(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
    outcome: Option<&ReloadOutcome>,
    args: &ReloadArgs,
) {
    match args.output.format {
        Format::Json => {
            let result = outcome.map(|o| {
                serde_json::json!({
                    "defVersion": o.def_version,
                    "changed": o.changed,
                    "dryRun": args.dry_run,
                    "ddl": o.ddl,
                })
            });
            let document = serde_json::json!({
                "diagnostics": diagnostics,
                "result": result,
            });
            let text = serde_json::to_string_pretty(&document)
                .unwrap_or_else(|_| "{\"diagnostics\":[],\"result\":null}".to_owned());
            println!("{text}");
        }
        Format::Human => {
            print_rendered(diagnostics, sources, &args.output);
            if let Some(outcome) = outcome.filter(|_| args.dry_run) {
                for statement in &outcome.ddl {
                    println!("{statement};");
                }
            }
        }
    }
}

/// The stderr line of a successful human-format run.
fn summary(outcome: &ReloadOutcome, dry_run: bool) -> String {
    let statements = format!("{} DDL statement(s)", outcome.ddl.len());
    if !outcome.changed {
        let version = outcome
            .def_version
            .map_or_else(|| "none".to_owned(), |v| v.to_string());
        return format!("ok: definitions unchanged (def_version {version})");
    }
    match (dry_run, outcome.def_version) {
        (false, Some(version)) => format!("ok: def_version {version} applied ({statements})"),
        _ => format!("dry run: {statements}, nothing applied"),
    }
}
