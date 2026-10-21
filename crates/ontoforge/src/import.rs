//! `ontoforge import`: load a CSV or JSON file into an editOnly object type or link (STORE-06).
//!
//! The command is thin, like `reload`: it checks the definitions, opens the edit store named by
//! the server configuration and calls `ontoforge_storage::import::import`. The edit store's
//! active definition is what the file is checked against; the ontology directory only has to be
//! valid, and a warning says when it differs from the active definition.
//!
//! Exit codes: 0 success, 1 diagnostics about the definitions, the configuration, the target or
//! the file (nothing written), or a batch that failed after others were written, 3 the edit store
//! could not be used (ONT-V26).

use camino::Utf8Path;
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, Ontology};
use ontoforge_schema::loader::SourceMap;
use ontoforge_storage::commit::EditError;
use ontoforge_storage::error::StoreError;
use ontoforge_storage::import::{
    ImportError, ImportOptions, ImportReport, ImportTarget, OnExisting, import,
};
use ontoforge_storage::store::{EditStore, StoreSettings};

use crate::cli::{ImportArgs, OnExistingArg};
use crate::reload::{EXIT_STORE, load_server, store_diagnostic, store_failure};
use crate::render::{self, Format};
use crate::{lint_cmd, print_rendered};

/// Runs `ontoforge import` and returns the process exit code.
pub fn run(args: &ImportArgs) -> u8 {
    let out = lint_cmd::lint(&args.dir, None, args.profile.profile.as_deref());
    let failed = render::exit_code(&out.diagnostics, args.output.deny_warnings) != 0;
    let Some(ontology) = out.ontology.as_ref().filter(|_| !failed) else {
        finish(&out.diagnostics, &out.sources, None, args);
        if args.output.format == Format::Human {
            eprintln!(
                "{} problem(s) found, nothing was imported",
                out.diagnostics.len()
            );
        }
        return 1;
    };

    let mut diagnostics = out.diagnostics.clone();
    let target = match target_of(args, ontology) {
        Ok(target) => target,
        Err(found) => {
            diagnostics.push(found);
            return fail(&diagnostics, &out.sources, 1, args);
        }
    };
    let server = match load_server(&args.server_config) {
        Ok(server) => server,
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

    let opts = ImportOptions {
        on_existing: match args.on_existing {
            OnExistingArg::Error => OnExisting::Error,
            OnExistingArg::Skip => OnExisting::Skip,
            OnExistingArg::Update => OnExisting::Update,
        },
        batch_size: usize::try_from(args.batch_size).unwrap_or(usize::MAX),
        max_rows: usize::try_from(args.max_rows).unwrap_or(usize::MAX),
        ..ImportOptions::default()
    };
    let result = runtime.block_on(open_and_import(
        server.settings,
        ontology,
        target.clone(),
        &args.file,
        &opts,
    ));
    match result {
        Ok((report, differs)) => {
            if differs {
                diagnostics.push(
                    Diagnostic::without_location(
                        codes::V26,
                        Severity::Warning,
                        "the definitions in the ontology directory differ from the ones the edit \
                         store follows; the file was imported against the edit store's",
                        args.dir.as_str(),
                    )
                    .with_help(
                        "run `ontoforge reload` to make the directory the active definition",
                    ),
                );
            }
            finish(
                &diagnostics,
                &out.sources,
                Some(&Done::Imported(&report)),
                args,
            );
            if args.output.format == Format::Human {
                eprintln!("{}", summary(&report, &target));
            }
            0
        }
        Err(failure) => report_failure(failure, diagnostics, &out.sources, args),
    }
}

/// What can stop an import before or while it writes.
enum Failure {
    /// The edit store has no definition yet.
    NoDefinition,
    Store(StoreError),
    Import(ImportError),
}

async fn open_and_import(
    settings: StoreSettings,
    ontology: &Ontology,
    target: ImportTarget,
    file: &Utf8Path,
    opts: &ImportOptions,
) -> Result<(ImportReport, bool), Failure> {
    let store = EditStore::open(settings).await.map_err(Failure::Store)?;
    let Some(active) = store.active_definition().await else {
        return Err(Failure::NoDefinition);
    };
    let differs = active.ontology != *ontology;
    import(&store, target, file, opts)
        .await
        .map(|report| (report, differs))
        .map_err(Failure::Import)
}

/// The type or link the file is for, checked against the ontology directory.
fn target_of(args: &ImportArgs, ontology: &Ontology) -> Result<ImportTarget, Diagnostic> {
    let missing = |what: &str, name: &str| {
        Diagnostic::without_location(
            codes::V09,
            Severity::Error,
            format!("the ontology has no {what} `{name}`"),
            args.dir.as_str(),
        )
    };
    match (&args.object_type, &args.link) {
        (Some(name), None) => {
            let parsed = ObjectTypeName::try_from(name.clone())
                .ok()
                .filter(|parsed| ontology.object_types.contains_key(parsed));
            parsed
                .map(ImportTarget::Object)
                .ok_or_else(|| missing("object type", name))
        }
        (None, Some(name)) => {
            let parsed = LinkTypeName::try_from(name.clone())
                .ok()
                .filter(|parsed| ontology.link_types.contains_key(parsed));
            parsed
                .map(ImportTarget::Link)
                .ok_or_else(|| missing("link type", name))
        }
        // clap allows exactly one of the two.
        _ => Err(Diagnostic::without_location(
            codes::V09,
            Severity::Error,
            "name either --type or --link",
            args.dir.as_str(),
        )),
    }
}

/// Turns a failure into diagnostics, prints them and returns the exit code.
fn report_failure(
    failure: Failure,
    mut diagnostics: Vec<Diagnostic>,
    sources: &SourceMap,
    args: &ImportArgs,
) -> u8 {
    let config = args.server_config.as_str();
    let file = args.file.as_str();
    let store_diag = |error: &StoreError| {
        let (code, exit) = store_failure(error);
        (
            Diagnostic::without_location(code, Severity::Error, error.to_string(), config),
            exit,
        )
    };
    match failure {
        Failure::NoDefinition => {
            diagnostics.push(
                Diagnostic::without_location(
                    codes::V26,
                    Severity::Error,
                    "the edit store holds no definition yet",
                    config,
                )
                .with_help("run `ontoforge reload` for the ontology directory first"),
            );
            fail(&diagnostics, sources, 1, args)
        }
        Failure::Store(error) | Failure::Import(ImportError::Store(error)) => {
            let (found, exit) = store_diag(&error);
            diagnostics.push(found);
            fail(&diagnostics, sources, exit, args)
        }
        Failure::Import(ImportError::Data(found)) => {
            diagnostics.extend(found);
            fail(&diagnostics, sources, 1, args)
        }
        Failure::Import(ImportError::NotEditOnly(message)) => {
            diagnostics.push(Diagnostic::without_location(
                codes::V27,
                Severity::Error,
                message,
                file,
            ));
            fail(&diagnostics, sources, 1, args)
        }
        Failure::Import(ImportError::Edit(error)) => {
            let code = match error {
                EditError::UnknownType(_) => codes::V09,
                _ => codes::V27,
            };
            diagnostics.push(Diagnostic::without_location(
                code,
                Severity::Error,
                error.to_string(),
                file,
            ));
            fail(&diagnostics, sources, 1, args)
        }
        Failure::Import(ImportError::Partial {
            committed,
            failed_batch,
            error,
        }) => {
            let exit = match error.as_ref() {
                EditError::Store(inner) => store_failure(inner).1,
                _ => 1,
            };
            let mut found = Diagnostic::without_location(
                codes::V27,
                Severity::Error,
                format!(
                    "batch {} failed after {} committed batch(es): {error}",
                    failed_batch + 1,
                    committed.len()
                ),
                file,
            );
            if matches!(*error, EditError::DefinitionStale { .. }) {
                found = found.with_help(
                    "the definition changed while the import ran; the batches written so far stay",
                );
            }
            diagnostics.push(found);
            finish(
                &diagnostics,
                sources,
                Some(&Done::Partial {
                    committed: &committed,
                    failed_batch,
                }),
                args,
            );
            if args.output.format == Format::Human {
                eprintln!("{}", partial_summary(&committed, failed_batch));
                eprintln!(
                    "rerun with --on-existing skip to resume: the rows of the committed batches \
                     already exist and are skipped"
                );
            }
            exit
        }
    }
}

/// The stderr line of a successful human-format run.
fn summary(report: &ImportReport, target: &ImportTarget) -> String {
    let what = match target {
        ImportTarget::Object(_) => "object(s)",
        ImportTarget::Link(_) => "link(s)",
    };
    format!(
        "imported {} {what} (created {}, updated {}, skipped {}) in {} commit(s)",
        report.created + report.updated,
        report.created,
        report.updated,
        report.skipped,
        report.commits.len()
    )
}

/// Which batches are in the edit store after a failure, and which one failed.
fn partial_summary(committed: &[i64], failed_batch: usize) -> String {
    let written = match (committed.first(), committed.last()) {
        (Some(first), Some(last)) if first == last => format!("edit log record {first}"),
        (Some(first), Some(last)) => format!("edit log records {first} to {last}"),
        _ => "nothing".to_owned(),
    };
    format!(
        "committed {} batch(es) ({written}); batch {} failed and was not written",
        committed.len(),
        failed_batch + 1
    )
}

/// What the command did, for the JSON result.
enum Done<'a> {
    Imported(&'a ImportReport),
    Partial {
        committed: &'a [i64],
        failed_batch: usize,
    },
}

/// Prints the failure and returns `exit`.
fn fail(diagnostics: &[Diagnostic], sources: &SourceMap, exit: u8, args: &ImportArgs) -> u8 {
    finish(diagnostics, sources, None, args);
    if args.output.format == Format::Human {
        eprintln!("import failed, nothing was written");
    }
    exit
}

/// Prints diagnostics (and the result as JSON) on stdout.
fn finish(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
    done: Option<&Done<'_>>,
    args: &ImportArgs,
) {
    match args.output.format {
        Format::Json => {
            let result = done.map(|done| match done {
                Done::Imported(report) => serde_json::json!({
                    "rows": report.rows,
                    "created": report.created,
                    "updated": report.updated,
                    "skipped": report.skipped,
                    "commits": report.commits,
                }),
                Done::Partial {
                    committed,
                    failed_batch,
                } => serde_json::json!({
                    "committed": committed,
                    "failedBatch": failed_batch,
                }),
            });
            let document = serde_json::json!({
                "diagnostics": diagnostics,
                "result": result,
            });
            let text = serde_json::to_string_pretty(&document)
                .unwrap_or_else(|_| "{\"diagnostics\":[],\"result\":null}".to_owned());
            println!("{text}");
        }
        Format::Human => print_rendered(diagnostics, sources, &args.output),
    }
}
