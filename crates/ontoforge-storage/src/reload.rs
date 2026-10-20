//! Definition reload (D-15, STORE-02).
//!
//! `reload` is a library function: the `ontoforge reload` command and the server process both call
//! it. It derives the physical schema of the new definition, compares it with the active one and
//! either applies the additive DDL and records a new definition version in one transaction, or
//! refuses with `ONT-V25` when the change is destructive. The two-step switch (pending, Trino
//! visibility probe, active) and the reload lock come with plan 03-18.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::{SecondsFormat, Utc};
use ontoforge_auth::scope::{ScopeInputs, compile_scope};
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
use crate::physical::{IndexKind, PhysicalSchema};
use crate::scope_store;
use crate::sql_exit;
use crate::store::{ActiveDefinition, Backend, EditStore, WriteTx};

#[derive(Debug, Clone, Default)]
pub struct ReloadOptions {
    /// Return the DDL that would run and change nothing.
    pub dry_run: bool,
    /// Check the roles in the store against the new definition first (D-18, SCOPE-06).
    pub roles: Option<RolesCheck>,
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
    opts: &ReloadOptions,
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
            Ok(done.outcome)
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
    opts: &ReloadOptions,
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
    opts: &ReloadOptions,
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
    opts: &ReloadOptions,
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
pub(crate) async fn insert_pending(
    tx: &mut WriteTx<'_>,
    ontology: &Ontology,
    physical: &PhysicalSchema,
    migration_json: Option<&serde_json::Value>,
) -> Result<i64, StoreError> {
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
