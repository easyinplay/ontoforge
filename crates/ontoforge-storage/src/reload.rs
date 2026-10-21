//! Definition reload (D-15, STORE-02).
//!
//! `reload` is a library function: the `ontoforge reload` command and the server process both call
//! it. It derives the physical schema of the new definition, compares it with the active one and
//! either applies the additive DDL and records a new definition version, or refuses with
//! `ONT-V25` when the change is destructive.
//!
//! How a definition becomes active:
//!
//! 1. Under the definition lock (advisory lock on PostgreSQL, `BEGIN IMMEDIATE` on SQLite; the same
//!    lock `migrate` and `scope apply` take) and with `lock_timeout` set, everything is decided
//!    again from the database: the active definition, the difference, the stored roles (D-18).
//! 2. Without a probe, or when the change adds nothing Trino has to show, the DDL, the new
//!    version and its activation are one transaction.
//! 3. With a probe, the DDL and the version, `pending`, commit first. The old definition is still
//!    active and serves; the reload flushes Trino's metadata cache and looks until the new tables
//!    and columns are visible (`wait_until_visible`). Only then a second short transaction makes
//!    the pending version active. Writes validated against the old version are refused from that
//!    commit on (the fence in [`crate::commit`]).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use futures_util::future::BoxFuture;
use ontoforge_auth::scope::{ScopeInputs, compile_scope};
use ontoforge_engine::printer::print_flush_metadata_cache;
use ontoforge_engine::{AnyEngine, Engine, EngineError, PhysicalColumn, QueryContext, TableRef};
use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::{
    AttrType, Cardinality, ObjectTypeName, Ontology, ProfileName, PropertyName,
};

use crate::ddl::{self, DdlStep};
use crate::diff::{DestructiveChange, diff};
use crate::error::StoreError;
use crate::index;
use crate::migrate::{self, MigrationStep};
use crate::naming;
use crate::physical::{ColumnType, IndexKind, PhysicalSchema};
use crate::scope_store;
use crate::sql_exit;
use crate::store::{ActiveDefinition, Backend, EditStore, WriteTx};

#[derive(Debug, Clone, Default)]
pub struct ReloadOptions<'a> {
    /// Return the DDL that would run and change nothing.
    pub dry_run: bool,
    /// Check the roles in the store against the new definition first (D-18, SCOPE-06).
    pub roles: Option<RolesCheck>,
    /// Wait until Trino sees the new tables and columns before the definition becomes active.
    pub probe: Option<ProbeContext<'a>>,
}

/// What a reload asks Trino. An `Engine` cannot be a trait object (its methods return
/// `impl Future`), so the two calls a probe makes are boxed here; `AnyEngine` implements it, and a
/// test can put a script in its place.
pub trait ProbeTarget: Send + Sync {
    /// `CALL <catalog>.system.flush_metadata_cache()`.
    fn flush_metadata<'a>(&'a self, catalog: &'a str) -> BoxFuture<'a, Result<(), EngineError>>;
    /// The columns Trino shows for `tables`; a table that is absent from the map is not visible.
    fn describe<'a>(
        &'a self,
        tables: &'a [TableRef],
    ) -> BoxFuture<'a, Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError>>;
}

/// Where and how long to look for the new tables and columns.
#[derive(Clone)]
pub struct ProbeContext<'a> {
    pub target: &'a dyn ProbeTarget,
    /// The Trino catalog that maps the edit store's PostgreSQL schema (`[store.trino] catalog`).
    pub catalog: String,
    /// How long to wait for visibility (`[store] probe_timeout_ms`).
    pub timeout: Duration,
}

impl std::fmt::Debug for ProbeContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProbeContext")
            .field("catalog", &self.catalog)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

/// What the role check of a reload needs besides the new definition: the declared principal
/// attributes (`[auth.principal_attrs]`) and the selected profile, the inputs `compile_scope`
/// takes.
#[derive(Debug, Clone, Default)]
pub struct RolesCheck {
    pub attrs: BTreeMap<String, AttrType>,
    /// `None` falls back to the manifest's default profile.
    pub profile: Option<ProfileName>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadOutcome {
    /// The definition version that was active before this call.
    pub previous_def_version: Option<i64>,
    /// The definition version that is active afterwards. `None` for a dry run that would change
    /// something: nothing is recorded.
    pub def_version: Option<i64>,
    /// Whether the definition differs from the active one.
    pub changed: bool,
    /// The DDL that ran, or for a dry run the DDL that would run.
    pub ddl: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    /// The definition cannot be applied; the diagnostics say why.
    #[error("the definition cannot be applied")]
    Definition(Vec<Diagnostic>),
    #[error(transparent)]
    Store(StoreError),
    /// A lock was not free within `[store] lock_timeout_ms`; the previous definition stays.
    #[error("gave up waiting for a database lock (lock_timeout); the previous definition stays")]
    LockTimeout,
    /// Trino did not show the new tables and columns within `[store] probe_timeout_ms`. The DDL
    /// is committed (it only adds) and the previous definition stays active.
    #[error("Trino does not show {} yet; the previous definition stays", .missing.join(", "))]
    NotVisible { missing: Vec<String> },
}

/// Brings the edit tables in line with `ontology`.
///
/// The call is idempotent: the same definition again changes nothing. Additive changes run as
/// DDL and add a definition version in one transaction; a destructive change is refused before
/// any statement runs.
///
/// Everything that decides happens under the definition lock (the advisory lock `migrate` and
/// `scope apply` take as well, on PostgreSQL; `BEGIN IMMEDIATE` on SQLite): the active definition
/// is read again, the difference is computed again and the stored roles are checked again. Two
/// reloads of one new definition therefore make one version: the second one finds it active and
/// does nothing.
///
/// With `opts.roles` the roles in the store must still compile against the new definition (D-18):
/// a role that no longer does stops the reload before any DDL runs. The row filter indexes the
/// roles need are created if they are missing, which is not a new definition version.
pub async fn reload(
    store: &EditStore,
    ontology: &Ontology,
    opts: &ReloadOptions<'_>,
) -> Result<ReloadOutcome, ReloadError> {
    let physical = PhysicalSchema::derive(ontology).map_err(ReloadError::Definition)?;
    if opts.dry_run {
        return dry_run(store, ontology, physical, opts).await;
    }
    let mut tx = begin_definition_tx(store).await?;
    match locked_reload(&mut tx, ontology, physical, opts).await {
        Ok(done) => {
            if done.write {
                sql_exit::commit(tx).await?;
            } else {
                sql_exit::rollback(tx).await;
            }
            if let Some(active) = done.active {
                store.install_if_newer(&Arc::new(active)).await;
            }
            let (Some(pending), Some(probe)) = (done.pending, &opts.probe) else {
                return Ok(done.outcome);
            };
            // The DDL is committed and only adds; the old definition keeps serving until
            // Trino shows what the new one needs.
            let active = publish(store, probe, pending).await?;
            let def_version = active.def_version;
            store.install_if_newer(&Arc::new(active)).await;
            Ok(ReloadOutcome {
                def_version: Some(def_version),
                ..done.outcome
            })
        }
        Err(error) => {
            sql_exit::rollback(tx).await;
            Err(error)
        }
    }
}

/// Starts the write transaction of a definition change: `SET LOCAL lock_timeout` from
/// `[store] lock_timeout_ms` first, so that it bounds the wait for the advisory lock as well, then
/// the advisory lock that `reload`, `migrate` and `scope apply` share. A wait that runs out is a
/// [`ReloadError::LockTimeout`] (PostgreSQL SQLSTATE 55P03) once converted.
pub(crate) async fn begin_definition_tx(store: &EditStore) -> Result<WriteTx<'_>, StoreError> {
    let mut tx = store.begin_write().await?;
    let millis = u64::try_from(store.settings.lock_timeout.as_millis()).unwrap_or(u64::MAX);
    let locked = async {
        sql_exit::set_local_lock_timeout(&mut tx, millis).await?;
        sql_exit::advisory_xact_lock(&mut tx).await
    }
    .await;
    match locked {
        Ok(()) => Ok(tx),
        Err(error) => {
            sql_exit::rollback(tx).await;
            Err(error)
        }
    }
}

/// Whether `error` is PostgreSQL's "lock not available": `lock_timeout` ran out.
pub(crate) fn is_lock_timeout(error: &StoreError) -> bool {
    matches!(error, StoreError::Query { sqlstate: Some(code), .. } if code == "55P03")
}

impl From<StoreError> for ReloadError {
    fn from(error: StoreError) -> Self {
        if is_lock_timeout(&error) {
            Self::LockTimeout
        } else {
            Self::Store(error)
        }
    }
}

/// What a reload decided, and what to do with its transaction.
struct Done {
    outcome: ReloadOutcome,
    /// Commit the transaction; otherwise roll it back, nothing was written.
    write: bool,
    /// The definition the in-memory copy of the store should follow after the commit.
    active: Option<ActiveDefinition>,
    /// A definition that is committed as `pending` and waits for Trino.
    pending: Option<Pending>,
}

/// The decisions of one reload, made in a transaction.
struct Prepared {
    /// The active definition as the transaction sees it.
    active: Option<ActiveDefinition>,
    physical: PhysicalSchema,
    /// Whether the definition differs from the active one.
    changed: bool,
    /// The steps that bring the tables in line with `physical`.
    steps: Vec<DdlStep>,
    /// The row filter indexes that do not exist yet.
    role_steps: Vec<DdlStep>,
}

impl Prepared {
    fn previous(&self) -> Option<i64> {
        self.active.as_ref().map(|a| a.def_version)
    }

    /// The statements of `steps` and `role_steps`.
    fn ddl(&self, backend: Backend, schema: Option<&str>) -> Vec<String> {
        self.steps
            .iter()
            .chain(&self.role_steps)
            .flat_map(|step| ddl::render(step, backend, schema))
            .collect()
    }
}

/// Reads the active definition in `tx`, compares it with `ontology`, checks the stored roles and
/// lists what is missing. Writes nothing.
async fn prepare(
    tx: &mut WriteTx<'_>,
    ontology: &Ontology,
    mut physical: PhysicalSchema,
    opts: &ReloadOptions<'_>,
) -> Result<Prepared, ReloadError> {
    let active = sql_exit::read_active_definition_in(tx).await?;
    if let Some(active) = &active {
        physical.keep_plain_indexes_of(&active.physical);
        physical.keep_archived_of(&active.physical);
    }

    let unchanged = match &active {
        Some(active) => active.physical == physical && same_ontology(&active.ontology, ontology)?,
        None => false,
    };
    let mut steps = Vec::new();
    if !unchanged {
        let plan = diff(active.as_ref().map(|a| &a.physical), &physical);
        if !plan.destructive.is_empty() {
            return Err(ReloadError::Definition(
                plan.destructive
                    .iter()
                    .map(DestructiveChange::diagnostic)
                    .collect(),
            ));
        }
        steps = plan.steps;
    }

    // D-18: the roles must still compile against the definition that is about to become active.
    let mut role_steps = Vec::new();
    if let Some(check) = &opts.roles {
        let scope = scope_store::load_scope_in(tx).await?;
        let compiled = compile_scope(&ScopeInputs {
            ontology,
            profile: check.profile.as_ref(),
            attrs: &check.attrs,
            scope: &scope,
            sources: None,
        })
        .map_err(ReloadError::Definition)?;
        role_steps =
            missing_role_indexes(tx, &physical, &compiled.row_filter_properties(), &steps).await?;
    }
    Ok(Prepared {
        active,
        physical,
        changed: !unchanged,
        steps,
        role_steps,
    })
}

/// The row filter indexes of `props` that neither exist nor are created by `steps`.
async fn missing_role_indexes(
    tx: &mut WriteTx<'_>,
    physical: &PhysicalSchema,
    props: &BTreeMap<ObjectTypeName, BTreeSet<PropertyName>>,
    steps: &[DdlStep],
) -> Result<Vec<DdlStep>, StoreError> {
    let wanted = index::role_index_steps(physical, props);
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let existing = sql_exit::existing_index_names(tx).await?;
    let planned = |name: &str| {
        steps
            .iter()
            .any(|step| matches!(step, DdlStep::CreateIndex { index, .. } if index.name == name))
    };
    Ok(wanted
        .into_iter()
        .filter(|step| match step {
            DdlStep::CreateIndex { index, .. } => {
                !existing.contains(&index.name) && !planned(&index.name)
            }
            _ => true,
        })
        .collect())
}

/// A reload that only looks: what would change, in a transaction that is rolled back.
async fn dry_run(
    store: &EditStore,
    ontology: &Ontology,
    physical: PhysicalSchema,
    opts: &ReloadOptions<'_>,
) -> Result<ReloadOutcome, ReloadError> {
    let mut tx = store.begin_write().await?;
    let prepared = prepare(&mut tx, ontology, physical, opts).await;
    sql_exit::rollback(tx).await;
    let prepared = prepared?;
    let previous = prepared.previous();
    Ok(ReloadOutcome {
        previous_def_version: previous,
        def_version: if prepared.changed { None } else { previous },
        changed: prepared.changed,
        ddl: prepared.ddl(store.backend(), store.schema_edit()),
    })
}

/// The decisions and the writes of a reload, under the definition lock of `tx`.
async fn locked_reload(
    tx: &mut WriteTx<'_>,
    ontology: &Ontology,
    physical: PhysicalSchema,
    opts: &ReloadOptions<'_>,
) -> Result<Done, ReloadError> {
    // Before any DDL: see `sql_exit::lock_def_state`.
    sql_exit::lock_def_state(tx).await?;
    let prepared = prepare(tx, ontology, physical, opts).await?;
    let previous = prepared.previous();
    let schema = tx.schema_edit().map(str::to_owned);
    let ddl = prepared.ddl(tx.backend(), schema.as_deref());

    if !prepared.changed && prepared.role_steps.is_empty() {
        return Ok(Done {
            outcome: ReloadOutcome {
                previous_def_version: previous,
                def_version: previous,
                changed: false,
                ddl,
            },
            write: false,
            active: prepared.active,
            pending: None,
        });
    }

    let Prepared {
        active,
        physical,
        changed,
        steps,
        role_steps,
    } = prepared;
    let steps: Vec<MigrationStep> = steps
        .into_iter()
        .chain(role_steps)
        .map(MigrationStep::Ddl)
        .collect();
    execute_steps(tx, &steps, ontology).await?;

    if !changed {
        // Indexes were completed; the definition is the one that is active.
        return Ok(Done {
            outcome: ReloadOutcome {
                previous_def_version: previous,
                def_version: previous,
                changed: false,
                ddl,
            },
            write: true,
            active,
            pending: None,
        });
    }
    let expected = if opts.probe.is_some() {
        expected_visible(&steps)
    } else {
        Vec::new()
    };
    if !expected.is_empty() {
        // Two steps: the DDL and the pending version commit now, the version becomes active when
        // Trino shows the new tables and columns (`publish`).
        let def_version = insert_pending(tx, ontology, &physical, None).await?;
        return Ok(Done {
            outcome: ReloadOutcome {
                previous_def_version: previous,
                def_version: None,
                changed: true,
                ddl,
            },
            write: true,
            active: None,
            pending: Some(Pending {
                def_version,
                from_version: previous.unwrap_or(0),
                ontology: ontology.clone(),
                physical,
                expected,
            }),
        });
    }
    let def_version = switch_definition(tx, ontology, &physical, None).await?;
    Ok(Done {
        outcome: ReloadOutcome {
            previous_def_version: previous,
            def_version: Some(def_version),
            changed: true,
            ddl,
        },
        write: true,
        active: Some(ActiveDefinition {
            def_version,
            ontology: ontology.clone(),
            physical,
        }),
        pending: None,
    })
}

/// Runs `steps` in order in the transaction `tx`. `reload` and `migrate` share this path, so
/// whatever guards the DDL of a reload (plan 03-18: lock timeout, advisory lock, fence) guards a
/// migration as well.
pub(crate) async fn execute_steps(
    tx: &mut WriteTx<'_>,
    steps: &[MigrationStep],
    ontology: &Ontology,
) -> Result<(), StoreError> {
    for step in steps {
        match step {
            MigrationStep::Ddl(step) => apply_step(tx, step, ontology).await?,
            other => {
                let schema = tx.schema_edit().map(str::to_owned);
                for sql in migrate::statements(other, tx.backend(), schema.as_deref()) {
                    sql_exit::execute_ddl(tx, &sql).await?;
                }
            }
        }
    }
    Ok(())
}

/// Records `ontology` and `physical` as the next definition version, `pending`, in the
/// transaction `tx`. `of_def_state` still points at the previous version. `migration_json` holds
/// the steps a migration ran (ADR-0026: replay of an edit log across versions reads them); a
/// reload passes `None`.
///
/// A pending version left behind by an earlier reload of the very same definition (its probe ran
/// out) is reused instead of numbering a new one; every other pending version is superseded, so
/// there is at most one.
pub(crate) async fn insert_pending(
    tx: &mut WriteTx<'_>,
    ontology: &Ontology,
    physical: &PhysicalSchema,
    migration_json: Option<&serde_json::Value>,
) -> Result<i64, StoreError> {
    let mut reusable = None;
    if migration_json.is_none() {
        for found in sql_exit::pending_definitions(tx).await? {
            if found.physical == *physical && same_ontology(&found.ontology, ontology)? {
                reusable = Some(found.def_version);
                break;
            }
        }
    }
    sql_exit::supersede_pending_except(tx, reusable).await?;
    if let Some(def_version) = reusable {
        return Ok(def_version);
    }
    let ontology_json = to_json(ontology)?;
    let physical_json = to_json(physical)?;
    let migration_json = migration_json.map(to_json).transpose()?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let def_version = sql_exit::next_def_version(tx).await?;
    let record = sql_exit::DefinitionRecord {
        ontology_json: &ontology_json,
        physical_json: &physical_json,
        migration_json: migration_json.as_deref(),
        now: &now,
    };
    sql_exit::insert_pending_definition(tx, def_version, &record).await?;
    Ok(def_version)
}

/// Makes the pending `def_version` the active one, in the transaction `tx`: from the commit of
/// this transaction on, a write validated against an older version is refused (the fence).
pub(crate) async fn activate(tx: &mut WriteTx<'_>, def_version: i64) -> Result<(), StoreError> {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    sql_exit::activate_definition(tx, def_version, &now).await
}

/// [`insert_pending`] and [`activate`] in the one transaction `tx`.
pub(crate) async fn switch_definition(
    tx: &mut WriteTx<'_>,
    ontology: &Ontology,
    physical: &PhysicalSchema,
    migration_json: Option<&serde_json::Value>,
) -> Result<i64, StoreError> {
    let def_version = insert_pending(tx, ontology, physical, migration_json).await?;
    activate(tx, def_version).await?;
    Ok(def_version)
}

// ---------------------------------------------------------------------------------------------
// The Trino visibility probe (STORE-02: "Trino metadata visible before the switch")
// ---------------------------------------------------------------------------------------------

impl ProbeTarget for AnyEngine {
    fn flush_metadata<'a>(&'a self, catalog: &'a str) -> BoxFuture<'a, Result<(), EngineError>> {
        Box::pin(async move {
            let sql = print_flush_metadata_cache(catalog)?;
            self.execute(&sql, &probe_query_context()).await.map(|_| ())
        })
    }

    fn describe<'a>(
        &'a self,
        tables: &'a [TableRef],
    ) -> BoxFuture<'a, Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError>> {
        Box::pin(async move { self.describe_tables(tables, &probe_query_context()).await })
    }
}

/// The service's own queries of a probe carry no user: a trace token and the `ontoforge` tag.
fn probe_query_context() -> QueryContext {
    QueryContext::new(
        "ontoforge-reload-probe".to_owned(),
        vec!["ontoforge".to_owned()],
    )
}

/// What Trino has to show before a definition may become active: a table, and the value columns
/// a step added to it. Array columns are left out: Trino's PostgreSQL connector skips them unless
/// `postgresql.array-mapping` says otherwise (RESEARCH Pitfall 7), so waiting for one could wait
/// forever. A column's `__set` flag is added in the same statement as the column and not asked
/// for separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Visible {
    table: String,
    /// The table itself has to be visible, not only some columns of it.
    new_table: bool,
    columns: BTreeSet<String>,
}

/// The tables and columns that `steps` create or move to a new name.
pub(crate) fn expected_visible(steps: &[MigrationStep]) -> Vec<Visible> {
    let mut by_table: BTreeMap<String, Visible> = BTreeMap::new();
    let mut want = |table: &str, new_table: bool, columns: Vec<String>| {
        let entry = by_table.entry(table.to_owned()).or_insert_with(|| Visible {
            table: table.to_owned(),
            new_table: false,
            columns: BTreeSet::new(),
        });
        entry.new_table |= new_table;
        entry.columns.extend(columns);
    };
    let is_array = |ty: &ColumnType| matches!(ty, ColumnType::Array(_));
    for step in steps {
        match step {
            MigrationStep::Ddl(DdlStep::CreateObjectTable(table)) => want(
                &table.table,
                true,
                std::iter::once(table.pk.column.clone())
                    .chain(
                        table
                            .properties
                            .values()
                            .filter(|column| !is_array(&column.ty))
                            .map(|column| column.column.clone()),
                    )
                    .collect(),
            ),
            MigrationStep::Ddl(DdlStep::AddPropertyColumn { table, column }) => want(
                table,
                false,
                if is_array(&column.ty) {
                    Vec::new()
                } else {
                    vec![column.column.clone()]
                },
            ),
            MigrationStep::Ddl(DdlStep::CreateLinkTable(table)) => want(
                &table.table,
                true,
                vec![table.a.column.clone(), table.b.column.clone()],
            ),
            MigrationStep::ArchiveColumn {
                table, archived, ..
            } => want(table, false, vec![archived.clone()]),
            MigrationStep::RenameColumn {
                table, to_column, ..
            } => want(table, false, vec![to_column.clone()]),
            MigrationStep::RenameTable { to, .. } => want(to, true, Vec::new()),
            MigrationStep::ArchiveTable { archived, .. } => want(archived, true, Vec::new()),
            MigrationStep::Ddl(DdlStep::CreateIndex { .. } | DdlStep::DropIndex { .. })
            | MigrationStep::DropArchivedColumn { .. }
            | MigrationStep::DropArchivedTable { .. } => {}
        }
    }
    by_table
        .into_values()
        .filter(|visible| visible.new_table || !visible.columns.is_empty())
        .collect()
}

/// What is missing from `shown` of `expected`, as `table` or `table.column`.
fn missing_from(
    expected: &[Visible],
    tables: &[TableRef],
    shown: &BTreeMap<TableRef, Vec<PhysicalColumn>>,
) -> Vec<String> {
    let mut missing = Vec::new();
    for (visible, table) in expected.iter().zip(tables) {
        let Some(columns) = shown.get(table) else {
            missing.push(visible.table.clone());
            continue;
        };
        for column in &visible.columns {
            if !columns
                .iter()
                .any(|shown| shown.name.eq_ignore_ascii_case(column))
            {
                missing.push(format!("{}.{column}", visible.table));
            }
        }
    }
    missing
}

/// Flushes the metadata cache of the catalog, then looks for the `expected` tables and columns
/// until they are all there or `probe.timeout` has passed: the first look at once, then after 200
/// ms, doubling up to 2 s. A flush that fails does not stop the wait (the cache expires by
/// itself); a Trino that cannot be asked counts as "nothing visible yet".
pub(crate) async fn wait_until_visible(
    store: &EditStore,
    probe: &ProbeContext<'_>,
    expected: &[Visible],
) -> Result<(), ReloadError> {
    if expected.is_empty() {
        return Ok(());
    }
    let schema = store.schema_edit().unwrap_or("main");
    let tables: Vec<TableRef> = expected
        .iter()
        .map(|visible| TableRef {
            catalog: probe.catalog.clone(),
            schema: schema.to_owned(),
            table: visible.table.clone(),
        })
        .collect();
    let flush_error = probe.target.flush_metadata(&probe.catalog).await.err();
    let started = tokio::time::Instant::now();
    let mut delay = Duration::from_millis(200);
    loop {
        let mut missing = match probe.target.describe(&tables).await {
            Ok(shown) => {
                let missing = missing_from(expected, &tables, &shown);
                if missing.is_empty() {
                    return Ok(());
                }
                missing
            }
            Err(error) => {
                let mut all: Vec<String> = expected
                    .iter()
                    .flat_map(|visible| {
                        if visible.columns.is_empty() {
                            vec![visible.table.clone()]
                        } else {
                            visible
                                .columns
                                .iter()
                                .map(|column| format!("{}.{column}", visible.table))
                                .collect()
                        }
                    })
                    .collect();
                all.push(format!("(Trino could not be asked: {error})"));
                all
            }
        };
        let Some(left) = probe.timeout.checked_sub(started.elapsed()) else {
            if let Some(error) = &flush_error {
                missing.push(format!("(flush_metadata_cache failed: {error})"));
            }
            return Err(ReloadError::NotVisible { missing });
        };
        tokio::time::sleep(delay.min(left)).await;
        delay = (delay * 2).min(Duration::from_secs(2));
    }
}

/// A definition whose DDL is committed and whose version waits, `pending`, for Trino.
struct Pending {
    def_version: i64,
    /// The version that was active when the DDL ran, 0 before the first definition.
    from_version: i64,
    ontology: Ontology,
    physical: PhysicalSchema,
    expected: Vec<Visible>,
}

/// Second half of a reload with a probe: waits for Trino, then makes the pending version active
/// in a transaction of its own, under the definition lock again. If the active version is no
/// longer the one the DDL ran against, or the pending version was replaced, someone else changed
/// the definition meanwhile and the reload is to be run again.
async fn publish(
    store: &EditStore,
    probe: &ProbeContext<'_>,
    pending: Pending,
) -> Result<ActiveDefinition, ReloadError> {
    wait_until_visible(store, probe, &pending.expected).await?;
    let mut tx = begin_definition_tx(store).await?;
    match activate_pending(&mut tx, &pending).await {
        Ok(()) => sql_exit::commit(tx).await?,
        Err(error) => {
            sql_exit::rollback(tx).await;
            return Err(error);
        }
    }
    Ok(ActiveDefinition {
        def_version: pending.def_version,
        ontology: pending.ontology,
        physical: pending.physical,
    })
}

async fn activate_pending(tx: &mut WriteTx<'_>, pending: &Pending) -> Result<(), ReloadError> {
    let active = sql_exit::lock_def_state(tx).await?;
    let still_pending = sql_exit::pending_definitions(tx)
        .await?
        .iter()
        .any(|found| found.def_version == pending.def_version);
    if active != pending.from_version || !still_pending {
        return Err(ReloadError::Store(StoreError::Conflict(format!(
            "definition version {} was prepared on top of version {}, but version {active} is active now; run the reload again",
            pending.def_version, pending.from_version
        ))));
    }
    activate(tx, pending.def_version).await?;
    Ok(())
}

/// Runs one step so that it can be repeated: a table is created only if missing, and columns are
/// added only if the table lacks them. A table left behind by an interrupted run is completed.
async fn apply_step(
    tx: &mut WriteTx<'_>,
    step: &DdlStep,
    ontology: &Ontology,
) -> Result<(), StoreError> {
    let backend = tx.backend();
    let schema = tx.schema_edit().map(str::to_owned);
    let schema = schema.as_deref();
    if let Some(sql) = ddl::index_sql(step, backend, schema) {
        return sql_exit::execute_ddl(tx, &sql)
            .await
            .map_err(|e| explain_unique_violation(e, step, ontology));
    }
    if let Some(create) = ddl::create_sql(step, backend, schema) {
        sql_exit::execute_ddl(tx, &create).await?;
    }
    let (table, columns) = ddl::step_columns(step, backend);
    let existing = sql_exit::table_columns(tx, schema, table).await?;
    for col in columns.iter().filter(|c| !existing.contains(&c.name)) {
        if col.key {
            return Err(StoreError::Corrupt(format!(
                "table `{table}` exists without its key column `{}`; it cannot be repaired by reload",
                col.name
            )));
        }
        sql_exit::execute_ddl(tx, &ddl::add_column_sql(table, col, backend, schema)).await?;
    }
    Ok(())
}

/// Tightening a link to `oneToOne` fails when existing edits already put two live objects on one
/// key. The database says "unique constraint"; this says which link and why.
fn explain_unique_violation(error: StoreError, step: &DdlStep, ontology: &Ontology) -> StoreError {
    let (DdlStep::CreateIndex { table, index }, StoreError::Query { sqlstate, message }) =
        (step, &error)
    else {
        return error;
    };
    let violation = sqlstate
        .as_deref()
        .is_some_and(|code| matches!(code, "23505" | "2067"))
        || message.contains("UNIQUE constraint failed");
    if index.kind != IndexKind::UniqueWhenSet || !violation {
        return error;
    }
    let link = ontology
        .link_types
        .iter()
        .find(|(_, link)| {
            link.cardinality == Cardinality::OneToOne
                && link.to.foreign_key.as_ref().is_some_and(|fk| {
                    naming::property_column(fk) == index.column
                        && naming::object_table(&link.to.object_type) == *table
                })
        })
        .map_or_else(|| index.name.clone(), |(name, _)| name.to_string());
    StoreError::Query {
        sqlstate: sqlstate.clone(),
        message: format!(
            "link type `{link}`: existing edits violate the oneToOne cardinality, so the unique index `{}` cannot be created: {message}",
            index.name
        ),
    }
}

/// Whether two ontologies serialize to the same document, ignoring the order of mapping keys.
pub(crate) fn same_ontology(a: &Ontology, b: &Ontology) -> Result<bool, StoreError> {
    let value = |o: &Ontology| {
        serde_json::to_value(o)
            .map_err(|e| StoreError::Corrupt(format!("cannot serialize the ontology: {e}")))
    };
    Ok(value(a)? == value(b)?)
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value)
        .map_err(|e| StoreError::Corrupt(format!("cannot serialize the definition: {e}")))
}
