//! Definition reload (D-15, STORE-02).
//!
//! `reload` is a library function: the `ontoforge reload` command and the server process both call
//! it. It derives the physical schema of the new definition, compares it with the active one and
//! either applies the additive DDL and records a new definition version in one transaction, or
//! refuses with `ONT-V25` when the change is destructive. The two-step switch (pending, Trino
//! visibility probe, active) and the reload lock come with plan 03-18.

use std::sync::Arc;

use chrono::{SecondsFormat, Utc};
use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::Ontology;

use crate::ddl::{self, DdlStep};
use crate::diff::{DestructiveChange, diff};
use crate::error::StoreError;
use crate::physical::PhysicalSchema;
use crate::sql_exit;
use crate::store::{ActiveDefinition, EditStore, WriteTx};

#[derive(Debug, Clone, Copy, Default)]
pub struct ReloadOptions {
    /// Return the DDL that would run and change nothing.
    pub dry_run: bool,
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
    Store(#[from] StoreError),
}

/// Brings the edit tables in line with `ontology`.
///
/// The call is idempotent: the same definition again changes nothing. Additive changes run as
/// DDL and add a definition version in one transaction; a destructive change is refused before
/// any statement runs.
pub async fn reload(
    store: &EditStore,
    ontology: &Ontology,
    opts: &ReloadOptions,
) -> Result<ReloadOutcome, ReloadError> {
    let physical = PhysicalSchema::derive(ontology).map_err(ReloadError::Definition)?;
    let active = store.active_definition().await;
    let previous_def_version = active.as_ref().map(|a| a.def_version);

    if let Some(active) = &active
        && active.physical == physical
        && same_ontology(&active.ontology, ontology)?
    {
        return Ok(ReloadOutcome {
            previous_def_version,
            def_version: previous_def_version,
            changed: false,
            ddl: Vec::new(),
        });
    }

    let plan = diff(active.as_ref().map(|a| &a.physical), &physical);
    if !plan.destructive.is_empty() {
        return Err(ReloadError::Definition(
            plan.destructive
                .iter()
                .map(DestructiveChange::diagnostic)
                .collect(),
        ));
    }

    let ddl: Vec<String> = plan
        .steps
        .iter()
        .flat_map(|step| ddl::render(step, store.backend(), store.schema_edit()))
        .collect();
    if opts.dry_run {
        return Ok(ReloadOutcome {
            previous_def_version,
            def_version: None,
            changed: true,
            ddl,
        });
    }

    let ontology_json = to_json(ontology)?;
    let physical_json = to_json(&physical)?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    let mut tx = store.begin_write().await?;
    for step in &plan.steps {
        apply_step(&mut tx, step).await?;
    }
    let def_version = sql_exit::next_def_version(&mut tx).await?;
    sql_exit::supersede_active(&mut tx).await?;
    sql_exit::activate_definition(&mut tx, def_version, &ontology_json, &physical_json, &now)
        .await?;
    sql_exit::commit(tx).await?;

    *store.active.write().await = Some(Arc::new(ActiveDefinition {
        def_version,
        ontology: ontology.clone(),
        physical,
    }));
    Ok(ReloadOutcome {
        previous_def_version,
        def_version: Some(def_version),
        changed: true,
        ddl,
    })
}

/// Runs one step so that it can be repeated: a table is created only if missing, and columns are
/// added only if the table lacks them. A table left behind by an interrupted run is completed.
async fn apply_step(tx: &mut WriteTx<'_>, step: &DdlStep) -> Result<(), StoreError> {
    if let Some(create) = ddl::create_sql(step) {
        sql_exit::execute_ddl(tx, &create).await?;
    }
    let (table, columns) = ddl::step_columns(step);
    let existing = sql_exit::table_columns(tx, None, table).await?;
    for col in columns.iter().filter(|c| !existing.contains(&c.name)) {
        if col.key {
            return Err(StoreError::Corrupt(format!(
                "table `{table}` exists without its key column `{}`; it cannot be repaired by reload",
                col.name
            )));
        }
        sql_exit::execute_ddl(tx, &ddl::add_column_sql(table, col)).await?;
    }
    Ok(())
}

/// Whether two ontologies serialize to the same document, ignoring the order of mapping keys.
fn same_ontology(a: &Ontology, b: &Ontology) -> Result<bool, StoreError> {
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
