//! `ontoforge scope plan|apply`: roles-as-code administration (D-01, SCOPE-01, SCOPE-07).
//!
//! A scope directory (`Role` and `RoleBinding` documents) goes into the edit store in two steps.
//! `plan` shows what would change, including every role and binding that would be removed (the
//! store is made equal to the directory, D-02), and the hash of that plan. `apply --plan-hash`
//! makes the plan again inside the store's write transaction and writes only if the hash is the
//! one that was reviewed.
//!
//! Before the store is opened, the scope is checked offline against the ontology, the selected
//! profile and the declared principal attributes with `ontoforge_auth::scope::compile_scope`:
//! row predicates, references and criteria column coverage (ONT-V21..V23). Any diagnostic stops
//! the command with exit code 1 and the store is not touched (D-01). The ontology must also be
//! the one the store follows, otherwise the check would be against a definition that does not
//! exist in the store; `reload` first.
//!
//! Exit codes as for `reload`: 0 success, 1 diagnostics about the scope, the definitions or the
//! configuration or a plan that changed, 3 the edit store could not be used (ONT-V26).

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_auth::scope::{CompiledScope, ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{ObjectTypeName, Ontology, ProfileName, PropertyName, ScopeSet};
use ontoforge_schema::load_scope_dir;
use ontoforge_schema::loader::SourceMap;
use ontoforge_storage::scope_store::{BindingRow, ScopeApplyError, ScopePlan, SubjectKind};
use ontoforge_storage::store::{EditStore, StoreSettings};

use crate::cli::{ScopeArgs, ScopeCommand, ScopeSourceArgs};
use crate::reload::{EXIT_STORE, server_settings, store_diagnostic, store_failure};
use crate::render::{self, Format};
use crate::{lint_cmd, print_rendered};

/// What a run of `scope plan` or `scope apply` produced.
struct Outcome {
    plan: ScopePlan,
    /// The audit sequence number of an apply, `None` for a plan.
    audit_seq: Option<i64>,
}

/// Why the store was not used, with the exit code to report.
enum Failure {
    Diagnostics(Vec<Diagnostic>),
    /// A diagnostic printed without a source position, and the exit code.
    Store(Diagnostic, u8),
}

/// Diagnostics with the source map their positions refer to, in the order they were found.
#[derive(Default)]
struct Report {
    groups: Vec<(Vec<Diagnostic>, SourceMap)>,
}

impl Report {
    /// Adds a group and returns its index.
    fn push(&mut self, diagnostics: Vec<Diagnostic>, sources: SourceMap) -> usize {
        self.groups.push((diagnostics, sources));
        self.groups.len() - 1
    }

    /// Adds diagnostics to a group that exists.
    fn extend(&mut self, group: usize, diagnostics: Vec<Diagnostic>) {
        self.groups[group].0.extend(diagnostics);
    }

    fn problems(&self) -> usize {
        self.groups.iter().map(|(found, _)| found.len()).sum()
    }
}

/// Runs `ontoforge scope` and returns the process exit code.
pub fn run(args: &ScopeArgs) -> u8 {
    match &args.command {
        ScopeCommand::Plan(plan) => execute(&plan.source, None),
        ScopeCommand::Apply(apply) => execute(&apply.source, Some(&apply.plan_hash)),
    }
}

fn execute(args: &ScopeSourceArgs, apply_hash: Option<&str>) -> u8 {
    let mut report = Report::default();
    match prepare_and_run(args, apply_hash, &mut report) {
        Ok(outcome) => {
            finish(args, &report, &outcome);
            0
        }
        Err(exit) => {
            emit(args, &report, None, &[]);
            if args.output.format == Format::Human {
                let verb = if apply_hash.is_some() {
                    "apply"
                } else {
                    "plan"
                };
                eprintln!(
                    "scope {verb} failed ({} problem(s)), the edit store was not changed",
                    report.problems()
                );
            }
            exit
        }
    }
}

/// Every step of a run. A failure leaves its diagnostics in `report` and returns the exit code.
fn prepare_and_run(
    args: &ScopeSourceArgs,
    apply_hash: Option<&str>,
    report: &mut Report,
) -> Result<Outcome, u8> {
    // 1. The ontology the scope refers to.
    let mut ontology_out = lint_cmd::lint(&args.ontology, None, args.profile.profile.as_deref());
    let failed = render::exit_code(&ontology_out.diagnostics, args.output.deny_warnings) != 0;
    let ontology = ontology_out.ontology.take().filter(|_| !failed);
    report.push(ontology_out.diagnostics, ontology_out.sources);
    let Some(ontology) = ontology else {
        return Err(1);
    };

    // 2. The scope documents.
    let scope_out = load_scope_dir(&args.dir);
    let broken = scope_out
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    let scope_group = report.push(scope_out.diagnostics, scope_out.sources);
    let Some(scope) = scope_out.scope.filter(|_| !broken) else {
        return Err(1);
    };

    // 3. The server configuration: the store and the declared principal attributes.
    let (settings, auth) = server_settings(&args.server_config).map_err(|found| {
        report.push(found, SourceMap::default());
        1
    })?;

    // 4. The offline check (D-01). The store is not opened for a scope that does not compile.
    let checked = check_scope(
        args,
        &ontology,
        &scope,
        &report.groups[scope_group].1,
        auth.as_ref(),
    );
    let compiled = checked.map_err(|found| {
        report.extend(scope_group, found);
        1
    })?;

    // 5. The store.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| {
            let message = format!("could not start the async runtime: {e}");
            report.push(
                vec![store_diagnostic(&message, &args.server_config)],
                SourceMap::default(),
            );
            EXIT_STORE
        })?;
    let row_filter_properties = compiled.row_filter_properties();
    runtime
        .block_on(run_store(
            settings,
            &ontology,
            &scope,
            &row_filter_properties,
            apply_hash,
            args,
        ))
        .map_err(|failure| match failure {
            Failure::Diagnostics(found) => {
                report.push(found, SourceMap::default());
                1
            }
            Failure::Store(diagnostic, exit) => {
                report.push(vec![diagnostic], SourceMap::default());
                exit
            }
        })
}

/// `compile_scope` for the scope directory against the loaded ontology.
fn check_scope(
    args: &ScopeSourceArgs,
    ontology: &Ontology,
    scope: &ScopeSet,
    sources: &SourceMap,
    auth: Option<&AuthConfig>,
) -> Result<CompiledScope, Vec<Diagnostic>> {
    // A name that is not valid was rejected when the ontology was loaded (ONT-V16).
    let profile = args
        .profile
        .profile
        .clone()
        .and_then(|name| ProfileName::try_from(name).ok());
    compile_scope(&ScopeInputs {
        ontology,
        profile: profile.as_ref(),
        attrs: &attr_types(auth),
        scope,
        sources: Some(sources),
    })
}

/// Opens the store, checks that it follows `ontology`, then plans or applies.
async fn run_store(
    settings: StoreSettings,
    ontology: &Ontology,
    scope: &ScopeSet,
    row_filter_properties: &BTreeMap<ObjectTypeName, BTreeSet<PropertyName>>,
    apply_hash: Option<&str>,
    args: &ScopeSourceArgs,
) -> Result<Outcome, Failure> {
    let config = args.server_config.as_str();
    let store_failure_of = |e: &ontoforge_storage::error::StoreError| {
        let (code, exit) = store_failure(e);
        Failure::Store(
            Diagnostic::without_location(code, Severity::Error, e.to_string(), config),
            exit,
        )
    };
    let store = EditStore::open(settings)
        .await
        .map_err(|e| store_failure_of(&e))?;
    check_active_definition(&store, ontology, config).await?;

    match apply_hash {
        None => {
            let plan = store
                .scope_plan(scope, row_filter_properties)
                .await
                .map_err(|e| store_failure_of(&e))?;
            Ok(Outcome {
                plan,
                audit_seq: None,
            })
        }
        Some(hash) => {
            let actor = actor();
            match store
                .scope_apply(scope, row_filter_properties, hash, &actor)
                .await
            {
                Ok(applied) => Ok(Outcome {
                    plan: applied.plan,
                    audit_seq: Some(applied.audit_seq),
                }),
                Err(e @ ScopeApplyError::PlanChanged { .. }) => Err(Failure::Diagnostics(vec![
                    Diagnostic::without_location(
                        codes::V25,
                        Severity::Error,
                        e.to_string(),
                        args.dir.as_str(),
                    )
                    .with_help("run `ontoforge scope plan` again and review the new plan"),
                ])),
                Err(ScopeApplyError::NoActiveDefinition) => {
                    Err(Failure::Diagnostics(vec![no_active_definition(config)]))
                }
                Err(ScopeApplyError::Store(e)) => Err(store_failure_of(&e)),
            }
        }
    }
}

fn no_active_definition(config: &str) -> Diagnostic {
    Diagnostic::without_location(
        codes::V25,
        Severity::Error,
        "no definition is active yet; run `ontoforge reload` before `ontoforge scope`",
        config,
    )
}

/// The ontology on disk must be the definition the store follows (a role is checked against the
/// definition its predicates will run on). Otherwise the command asks for a `reload` first.
async fn check_active_definition(
    store: &EditStore,
    ontology: &Ontology,
    config: &str,
) -> Result<(), Failure> {
    let Some(active) = store.active_definition().await else {
        return Err(Failure::Diagnostics(vec![no_active_definition(config)]));
    };
    let same = match (
        serde_json::to_value(&active.ontology),
        serde_json::to_value(ontology),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same {
        return Ok(());
    }
    Err(Failure::Diagnostics(vec![Diagnostic::without_location(
        codes::V25,
        Severity::Error,
        format!(
            "ontology differs from the active definition (def_version {}); run `ontoforge reload` first",
            active.def_version
        ),
        config,
    )]))
}

/// `<user>@<host>` of the operator, from the environment (`unknown` when it has neither).
fn actor() -> String {
    let first = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|v| !v.trim().is_empty()))
            .map_or_else(|| "unknown".to_owned(), |v| v.trim().to_owned())
    };
    format!(
        "{}@{}",
        first(&["USERNAME", "USER"]),
        first(&["COMPUTERNAME", "HOSTNAME"])
    )
}

/// Prints the diagnostics and the result on stdout.
fn emit(args: &ScopeSourceArgs, report: &Report, result: Option<&Outcome>, lines: &[String]) {
    match args.output.format {
        Format::Json => {
            let diagnostics: Vec<&Diagnostic> = report
                .groups
                .iter()
                .flat_map(|(found, _)| found.iter())
                .collect();
            let result = result.map(|outcome| match outcome.audit_seq {
                Some(seq) => serde_json::json!({ "plan": outcome.plan, "auditSeq": seq }),
                None => serde_json::json!({ "plan": outcome.plan }),
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
            for (found, sources) in &report.groups {
                print_rendered(found, sources, &args.output);
            }
            for line in lines {
                println!("{line}");
            }
        }
    }
}

fn finish(args: &ScopeSourceArgs, report: &Report, outcome: &Outcome) {
    let lines = plan_lines(&outcome.plan);
    emit(args, report, Some(outcome), &lines);
    if args.output.format == Format::Human {
        match outcome.audit_seq {
            Some(seq) => eprintln!("ok: scope applied (audit record {seq})"),
            None if outcome.plan.is_empty() => eprintln!("ok: the store already holds this scope"),
            None => eprintln!(
                "plan only, nothing changed; run `ontoforge scope apply --plan-hash {}` to apply it",
                outcome.plan.hash
            ),
        }
    }
}

/// One line per change, then the hash.
fn plan_lines(plan: &ScopePlan) -> Vec<String> {
    let mut lines = Vec::new();
    lines.extend(plan.roles_added.iter().map(|r| format!("add role {r}")));
    lines.extend(
        plan.roles_changed
            .iter()
            .map(|r| format!("change role {r}")),
    );
    lines.extend(
        plan.roles_removed
            .iter()
            .map(|r| format!("remove role {r}")),
    );
    let binding = |verb: &str, row: &BindingRow| {
        let kind = match row.kind {
            SubjectKind::User => "user",
            SubjectKind::Group => "group",
        };
        format!(
            "{verb} binding {} {kind} {}",
            row.role,
            render::sanitize(&row.subject)
        )
    };
    lines.extend(plan.bindings_added.iter().map(|b| binding("add", b)));
    lines.extend(plan.bindings_removed.iter().map(|b| binding("remove", b)));
    lines.extend(
        plan.index_ddl
            .iter()
            .map(|sql| format!("create index {}", index_name(sql))),
    );
    if plan.is_empty() {
        lines.push("no changes".to_owned());
    }
    lines.push(format!("plan hash: {}", plan.hash));
    lines
}

/// The quoted name in `CREATE INDEX IF NOT EXISTS "<name>" ON ...`, else the whole statement.
fn index_name(sql: &str) -> &str {
    sql.split('"').nth(1).unwrap_or(sql)
}
