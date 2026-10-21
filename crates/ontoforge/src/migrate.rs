//! `ontoforge migrate`: review and run a destructive definition change (STORE-02, D-14, D-16).
//!
//! `reload` refuses a change that could lose or reinterpret data and points here. `--plan` loads
//! the definitions, compares them with the active definition in the edit store and prints the
//! steps of the migration and their hash. `--apply --plan-hash <HASH>` makes the plan again
//! inside the store's write transaction and runs it only if the hash is the one that was
//! reviewed. Nothing is deleted by a definition change: old columns and tables are archived.
//! `--apply` also compiles the roles stored by `scope apply` against the new definition; a role
//! that no longer holds ends the run with exit code 1 and changes nothing, so change the scope
//! first and migrate afterwards (D-18, SCOPE-06).
//! `--drop-archived` plans and runs the explicit removal of the archived data.
//!
//! Exit codes as for `reload`: 0 success, 1 diagnostics about the definitions or the
//! configuration or a plan that changed, 3 the edit store could not be used (ONT-V26).

use ontoforge_auth::scope::attr_types;
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{Ontology, ProfileName};
use ontoforge_schema::loader::SourceMap;
use ontoforge_storage::migrate::{
    MigrateError, MigrationPlan, apply_drop_archived, apply_migration_probed, plan_drop_archived,
    plan_migration,
};
use ontoforge_storage::reload::{ProbeContext, RolesCheck};
use ontoforge_storage::store::{EditStore, StoreSettings};

use crate::cli::MigrateArgs;
use crate::reload::{
    EXIT_STORE, ProbeEngine, load_server, lock_timeout_diagnostic, not_visible_diagnostic,
    probe_engine, store_diagnostic, store_failure,
};
use crate::render::{self, Format};
use crate::{lint_cmd, print_rendered};

/// What a successful run has to report.
enum Outcome {
    Plan(MigrationPlan),
    Applied { def_version: i64 },
}

/// Runs `ontoforge migrate` and returns the process exit code.
pub fn run(args: &MigrateArgs) -> u8 {
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
    let server = match load_server(&args.server_config) {
        Ok(server) => server,
        Err(found) => {
            diagnostics.extend(found);
            return fail(&diagnostics, &out.sources, 1, args);
        }
    };
    let probe = match probe_engine(&server, ontology, args.profile.profile.as_deref()) {
        Ok(probe) => probe,
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
    let roles = RolesCheck {
        attrs: attr_types(server.config.auth.as_ref()),
        profile,
    };
    let context = probe.as_ref().map(ProbeEngine::context);
    match runtime.block_on(execute(
        server.settings,
        ontology,
        args,
        context.as_ref(),
        &roles,
    )) {
        Ok(outcome) => {
            finish(&diagnostics, &out.sources, Some(&outcome), args);
            if args.output.format == Format::Human {
                eprintln!("{}", summary(&outcome));
            }
            0
        }
        Err(MigrateError::Definition(found)) => {
            diagnostics.extend(found);
            fail(&diagnostics, &out.sources, 1, args)
        }
        Err(e @ MigrateError::PlanChanged { .. }) => {
            if args.output.format == Format::Human {
                eprintln!("{e}");
                eprintln!("run `ontoforge migrate --plan` again and review the new plan");
            } else {
                diagnostics.push(
                    Diagnostic::without_location(
                        codes::V25,
                        Severity::Error,
                        e.to_string(),
                        args.dir.as_str(),
                    )
                    .with_help("run `ontoforge migrate --plan` again and review the new plan"),
                );
            }
            fail(&diagnostics, &out.sources, 1, args)
        }
        Err(MigrateError::Store(e)) => {
            let (code, exit) = store_failure(&e);
            diagnostics.push(Diagnostic::without_location(
                code,
                Severity::Error,
                e.to_string(),
                args.server_config.as_str(),
            ));
            fail(&diagnostics, &out.sources, exit, args)
        }
        Err(e @ MigrateError::LockTimeout) => {
            diagnostics.push(lock_timeout_diagnostic(&e, &args.server_config));
            fail(&diagnostics, &out.sources, EXIT_STORE, args)
        }
        Err(e @ MigrateError::NotVisible { .. }) => {
            // The migration is applied and active; only Trino lags behind.
            diagnostics.push(not_visible_diagnostic(&e, &args.server_config));
            finish(&diagnostics, &out.sources, None, args);
            if args.output.format == Format::Human {
                eprintln!("migration applied, but Trino does not show the new names yet");
            }
            EXIT_STORE
        }
    }
}

async fn execute(
    settings: StoreSettings,
    ontology: &Ontology,
    args: &MigrateArgs,
    probe: Option<&ProbeContext<'_>>,
    roles: &RolesCheck,
) -> Result<Outcome, MigrateError> {
    let store = EditStore::open(settings).await?;
    if args.apply {
        let hash = args.plan_hash.as_deref().unwrap_or_default();
        let def_version = if args.drop_archived {
            apply_drop_archived(&store, hash).await?
        } else {
            apply_migration_probed(&store, ontology, &args.rename, hash, probe, Some(roles)).await?
        };
        return Ok(Outcome::Applied { def_version });
    }
    let active = store.active_definition().await.ok_or_else(|| {
        MigrateError::Definition(vec![Diagnostic::without_location(
            codes::V25,
            Severity::Error,
            "no definition is active yet; run `ontoforge reload` before `ontoforge migrate`",
            args.server_config.as_str(),
        )])
    })?;
    let plan = if args.drop_archived {
        plan_drop_archived(&active)
    } else {
        plan_migration(&active, ontology, &args.rename).map_err(MigrateError::Definition)?
    };
    Ok(Outcome::Plan(plan))
}

/// Prints the failure and returns `exit`.
fn fail(diagnostics: &[Diagnostic], sources: &SourceMap, exit: u8, args: &MigrateArgs) -> u8 {
    finish(diagnostics, sources, None, args);
    if args.output.format == Format::Human {
        eprintln!("migrate failed, the edit store was not changed");
    }
    exit
}

/// Prints diagnostics and the result on stdout.
fn finish(
    diagnostics: &[Diagnostic],
    sources: &SourceMap,
    outcome: Option<&Outcome>,
    args: &MigrateArgs,
) {
    match args.output.format {
        Format::Json => {
            let result = outcome.map(|outcome| match outcome {
                Outcome::Plan(plan) => serde_json::json!({ "plan": plan }),
                Outcome::Applied { def_version } => {
                    serde_json::json!({ "defVersion": def_version })
                }
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
            if let Some(Outcome::Plan(plan)) = outcome {
                print_plan(plan);
            }
        }
    }
}

fn print_plan(plan: &MigrationPlan) {
    if plan.steps.is_empty() {
        println!(
            "no migration steps needed from def_version {}",
            plan.from_def_version
        );
    } else {
        println!(
            "migration plan from def_version {} ({} step(s)):",
            plan.from_def_version,
            plan.steps.len()
        );
        for (i, step) in plan.steps.iter().enumerate() {
            println!("  {}. {}", i + 1, step.describe());
        }
    }
    println!("plan hash: {}", plan.hash);
}

/// The stderr line of a successful human-format run.
fn summary(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Plan(plan) => format!(
            "plan only, nothing changed; run `ontoforge migrate --apply --plan-hash {}` to apply it",
            plan.hash
        ),
        Outcome::Applied { def_version } => format!("ok: def_version {def_version} applied"),
    }
}
