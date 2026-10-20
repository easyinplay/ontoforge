//! Destructive definition changes through `ontoforge migrate` (STORE-02, D-14, D-16).
//!
//! `reload` refuses a change that could lose or reinterpret data (ONT-V25). `migrate` is the way
//! out: `plan_migration` turns the difference into a fixed list of steps with a hash, and
//! `apply_migration` runs exactly that list, in one transaction, only if the hash still matches.
//! Nothing is deleted: an old column or table is renamed (archived) and stays in the database
//! until `plan_drop_archived` / `apply_drop_archived` remove it on explicit request.
//!
//! How a plan is made:
//!
//! 1. The deployer's rename declarations are applied to a copy of the active physical schema, as
//!    if the renames had happened (`RenameTable`, `RenameColumn` steps, with the indexes of the
//!    renamed table or column rebuilt under their new names).
//! 2. The new physical schema is derived and compared with that copy by the same `diff` that
//!    `reload` uses. What `diff` calls additive becomes `Ddl` steps. What it calls destructive
//!    becomes an archive step, plus the new empty column or table where the change replaces one.
//! 3. The hash covers the version, the declarations, the steps and the new definition.
//!
//! Archived columns and tables are recorded in the physical schema (`archived_columns`,
//! `archived_tables`), so `reload` keeps seeing the database as the schema describes it.

use std::collections::BTreeSet;
use std::sync::Arc;

use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{ObjectTypeName, Ontology, PropertyName};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ddl::{self, DdlStep};
use crate::diff::{DestructiveChange, Plan, diff};
use crate::error::StoreError;
use crate::naming;
use crate::physical::{
    ArchivedColumn, ArchivedTable, IndexDef, IndexKind, ObjectTable, PhysicalSchema, PropertyColumn,
};
use crate::reload::{execute_steps, same_ontology, switch_definition};
use crate::sql_exit;
use crate::store::{ActiveDefinition, Backend, EditStore, WriteTx};

/// A rename the deployer declares, because a rename shows up in the diff as a removal plus an
/// addition (D-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenameDecl {
    /// `Type.old=new`: a property of the object type `type_name` (its name in the new definition).
    Property {
        type_name: String,
        from: String,
        to: String,
    },
    /// `OldType=NewType`.
    ObjectType { from: String, to: String },
}

impl std::str::FromStr for RenameDecl {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let malformed = || {
            format!(
                "`{text}` is not a rename declaration: write `Type.old=new` for a property or `OldType=NewType` for an object type"
            )
        };
        let (left, right) = text.split_once('=').ok_or_else(malformed)?;
        let object = |name: &str| {
            ObjectTypeName::try_from(name.to_owned())
                .map(|n| n.as_str().to_owned())
                .map_err(|e| format!("rename declaration `{text}`: {e}"))
        };
        let property = |name: &str| {
            PropertyName::try_from(name.to_owned())
                .map(|n| n.as_str().to_owned())
                .map_err(|e| format!("rename declaration `{text}`: {e}"))
        };
        match left.split_once('.') {
            Some((type_name, from)) => Ok(Self::Property {
                type_name: object(type_name)?,
                from: property(from)?,
                to: property(right)?,
            }),
            None => Ok(Self::ObjectType {
                from: object(left)?,
                to: object(right)?,
            }),
        }
    }
}

impl std::fmt::Display for RenameDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Property {
                type_name,
                from,
                to,
            } => write!(f, "{type_name}.{from}={to}"),
            Self::ObjectType { from, to } => write!(f, "{from}={to}"),
        }
    }
}

/// One step of a migration. A step does one thing; the SQL of a step may be several statements
/// (SQLite allows one operation per `ALTER TABLE`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MigrationStep {
    /// A step `reload` could also run: new tables, new columns, indexes.
    Ddl(DdlStep),
    /// The value column and its `__set` column get the archive names; the data stays.
    ArchiveColumn {
        type_name: String,
        property: String,
        table: String,
        column: String,
        archived: String,
        set_archived: String,
    },
    /// The value column and its `__set` column move to the new property name.
    RenameColumn {
        type_name: String,
        table: String,
        /// Property names.
        from: String,
        to: String,
        from_column: String,
        to_column: String,
    },
    /// The table moves to the name of the new object type. Its indexes are dropped before and
    /// created again after, because their names carry the table name.
    RenameTable {
        from_type: String,
        to_type: String,
        from: String,
        to: String,
    },
    /// A whole table gets the archive name; `type_name` is an object type or a link type.
    ArchiveTable {
        type_name: String,
        table: String,
        archived: String,
    },
    DropArchivedColumn {
        table: String,
        column: String,
    },
    DropArchivedTable {
        table: String,
    },
}

impl MigrationStep {
    /// One line for a person who reviews the plan.
    pub fn describe(&self) -> String {
        match self {
            Self::Ddl(step) => describe_ddl(step),
            Self::ArchiveColumn {
                type_name,
                property,
                column,
                archived,
                ..
            } => format!("archive column {type_name}.{property}: {column} -> {archived}"),
            Self::RenameColumn {
                type_name,
                from,
                to,
                ..
            } => format!("rename column {type_name}.{from} -> {to} (data moves with it)"),
            Self::RenameTable { from, to, .. } => {
                format!("rename table {from} -> {to} (data moves with it)")
            }
            Self::ArchiveTable {
                type_name,
                table,
                archived,
            } => format!("archive table of {type_name}: {table} -> {archived}"),
            Self::DropArchivedColumn { table, column } => {
                format!("drop archived column {table}.{column}")
            }
            Self::DropArchivedTable { table } => format!("drop archived table {table}"),
        }
    }
}

fn describe_ddl(step: &DdlStep) -> String {
    match step {
        DdlStep::CreateObjectTable(table) => format!("create table {}", table.table),
        DdlStep::AddPropertyColumn { table, column } => {
            format!("add column {table}.{}", column.column)
        }
        DdlStep::CreateLinkTable(table) => format!("create link table {}", table.table),
        DdlStep::CreateIndex { table, index } => {
            format!("create index {} on {table}.{}", index.name, index.column)
        }
        DdlStep::DropIndex { table, name } => format!("drop index {name} of {table}"),
    }
}

/// The statements of `step`.
pub(crate) fn statements(
    step: &MigrationStep,
    backend: Backend,
    schema_edit: Option<&str>,
) -> Vec<String> {
    match step {
        MigrationStep::Ddl(step) => ddl::render(step, backend, schema_edit),
        MigrationStep::ArchiveColumn {
            table,
            column,
            archived,
            set_archived,
            ..
        } => vec![
            ddl::rename_column_sql(table, column, archived, backend, schema_edit),
            ddl::rename_column_sql(
                table,
                &naming::set_column(column),
                set_archived,
                backend,
                schema_edit,
            ),
        ],
        MigrationStep::RenameColumn {
            table,
            from_column,
            to_column,
            ..
        } => vec![
            ddl::rename_column_sql(table, from_column, to_column, backend, schema_edit),
            ddl::rename_column_sql(
                table,
                &naming::set_column(from_column),
                &naming::set_column(to_column),
                backend,
                schema_edit,
            ),
        ],
        MigrationStep::RenameTable { from, to, .. } => {
            vec![ddl::rename_table_sql(from, to, backend, schema_edit)]
        }
        MigrationStep::ArchiveTable {
            table, archived, ..
        } => vec![ddl::rename_table_sql(table, archived, backend, schema_edit)],
        MigrationStep::DropArchivedColumn { table, column } => {
            vec![ddl::drop_column_sql(table, column, backend, schema_edit)]
        }
        MigrationStep::DropArchivedTable { table } => {
            vec![ddl::drop_table_sql(table, backend, schema_edit)]
        }
    }
}

/// The steps of a migration and the hash that identifies them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub from_def_version: i64,
    pub renames: Vec<RenameDecl>,
    pub steps: Vec<MigrationStep>,
    /// FNV-1a 64 bit, 16 lower case hex digits, over the other fields and the new definition.
    pub hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    /// The migration cannot be planned; the diagnostics say why.
    #[error("the migration cannot be planned")]
    Definition(Vec<Diagnostic>),
    /// The plan that `apply` computed is not the plan that was reviewed.
    #[error("the migration plan changed: expected {expected}, now {actual}")]
    PlanChanged { expected: String, actual: String },
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// A plan together with the physical schema that results from it.
struct Computed {
    plan: MigrationPlan,
    physical: PhysicalSchema,
}

/// Plans the migration from the `active` definition to `new`.
///
/// The plan is deterministic: the same definitions and declarations give the same steps and the
/// same hash.
pub fn plan_migration(
    active: &ActiveDefinition,
    new: &Ontology,
    renames: &[RenameDecl],
) -> Result<MigrationPlan, Vec<Diagnostic>> {
    compute(active, new, renames).map(|computed| computed.plan)
}

/// Runs the migration to `new` if its plan still has the hash `expected_hash`.
///
/// The plan is made again inside the write transaction, from the definition that is active in
/// the database at that moment. A different hash (the definition files or the database changed
/// since the review, or the plan was applied already) refuses the call before any DDL runs
/// (T-03-46). Returns the new definition version.
pub async fn apply_migration(
    store: &EditStore,
    new: &Ontology,
    renames: &[RenameDecl],
    expected_hash: &str,
) -> Result<i64, MigrateError> {
    let mut tx = store.begin_write().await?;
    match apply_in(&mut tx, new, renames, expected_hash).await {
        Ok(active) => {
            sql_exit::commit(tx).await?;
            let version = active.def_version;
            *store.active.write().await = Some(Arc::new(active));
            Ok(version)
        }
        Err(error) => {
            sql_exit::rollback(tx).await;
            Err(error)
        }
    }
}

async fn apply_in(
    tx: &mut WriteTx<'_>,
    new: &Ontology,
    renames: &[RenameDecl],
    expected_hash: &str,
) -> Result<ActiveDefinition, MigrateError> {
    let active = active_in(tx).await?;
    let computed = compute(&active, new, renames).map_err(MigrateError::Definition)?;
    check_hash(expected_hash, &computed.plan.hash)?;
    if computed.plan.steps.is_empty()
        && computed.physical == active.physical
        && same_ontology(&active.ontology, new)?
    {
        return Ok(active);
    }
    run(tx, new, computed).await
}

/// Runs the steps of `computed` and switches to `ontology`, in the transaction `tx`.
async fn run(
    tx: &mut WriteTx<'_>,
    ontology: &Ontology,
    computed: Computed,
) -> Result<ActiveDefinition, MigrateError> {
    execute_steps(tx, &computed.plan.steps, ontology).await?;
    let migration_json = serde_json::to_value(&computed.plan)
        .map_err(|e| StoreError::Corrupt(format!("cannot serialize the migration plan: {e}")))?;
    let def_version =
        switch_definition(tx, ontology, &computed.physical, Some(&migration_json)).await?;
    Ok(ActiveDefinition {
        def_version,
        ontology: ontology.clone(),
        physical: computed.physical,
    })
}

async fn active_in(tx: &mut WriteTx<'_>) -> Result<ActiveDefinition, MigrateError> {
    sql_exit::read_active_definition_in(tx)
        .await?
        .ok_or_else(|| {
            MigrateError::Definition(vec![error(
                "no definition is active yet; run `ontoforge reload` before `ontoforge migrate`",
            )])
        })
}

fn check_hash(expected: &str, actual: &str) -> Result<(), MigrateError> {
    if expected == actual {
        Ok(())
    } else {
        Err(MigrateError::PlanChanged {
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        })
    }
}

/// The plan that drops every archived column and table of the `active` definition (D-16).
///
/// This is the only way archived data is ever deleted.
pub fn plan_drop_archived(active: &ActiveDefinition) -> MigrationPlan {
    let mut steps = Vec::new();
    for table in active.physical.objects.values() {
        for archived in &table.archived_columns {
            for column in [&archived.column, &archived.set_column] {
                steps.push(MigrationStep::DropArchivedColumn {
                    table: table.table.clone(),
                    column: column.clone(),
                });
            }
        }
    }
    for archived in &active.physical.archived_tables {
        steps.push(MigrationStep::DropArchivedTable {
            table: archived.table.clone(),
        });
    }
    let hash = plan_hash(active.def_version, &[], &steps, &active.ontology).unwrap_or_default();
    MigrationPlan {
        from_def_version: active.def_version,
        renames: Vec::new(),
        steps,
        hash,
    }
}

/// Runs [`plan_drop_archived`] if its plan still has the hash `expected_hash`. Returns the new
/// definition version.
pub async fn apply_drop_archived(
    store: &EditStore,
    expected_hash: &str,
) -> Result<i64, MigrateError> {
    let mut tx = store.begin_write().await?;
    match drop_archived_in(&mut tx, expected_hash).await {
        Ok(active) => {
            sql_exit::commit(tx).await?;
            let version = active.def_version;
            *store.active.write().await = Some(Arc::new(active));
            Ok(version)
        }
        Err(error) => {
            sql_exit::rollback(tx).await;
            Err(error)
        }
    }
}

async fn drop_archived_in(
    tx: &mut WriteTx<'_>,
    expected_hash: &str,
) -> Result<ActiveDefinition, MigrateError> {
    let active = active_in(tx).await?;
    let plan = plan_drop_archived(&active);
    check_hash(expected_hash, &plan.hash)?;
    if plan.steps.is_empty() {
        return Ok(active);
    }
    let mut physical = active.physical.clone();
    for table in physical.objects.values_mut() {
        table.archived_columns.clear();
    }
    physical.archived_tables.clear();
    let ontology = active.ontology.clone();
    run(tx, &ontology, Computed { plan, physical }).await
}

// ---------------------------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------------------------

fn error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::without_location(codes::V25, Severity::Error, message, "ontology")
}

fn compute(
    active: &ActiveDefinition,
    new: &Ontology,
    renames: &[RenameDecl],
) -> Result<Computed, Vec<Diagnostic>> {
    let version = active.def_version + 1;
    let mut physical = PhysicalSchema::derive(new)?;

    // 1. The renames, applied to a copy of the active schema as if they had happened.
    let mut old = active.physical.clone();
    let mut steps = Vec::new();
    let mut errors = Vec::new();
    rename_object_types(&mut old, &physical, renames, &mut steps, &mut errors);
    rename_properties(&mut old, &physical, renames, &mut steps, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    // 2. What is left to do, by the same comparison `reload` makes.
    physical.keep_plain_indexes_of(&old);
    physical.keep_archived_of(&old);
    let plan = diff(Some(&old), &physical);
    let mut builder = Archiver {
        old: &old,
        physical: &mut physical,
        version,
        steps: &mut steps,
    };
    builder.destructive(&plan);
    builder.additive(&plan);

    let plan = MigrationPlan {
        from_def_version: active.def_version,
        renames: renames.to_vec(),
        hash: plan_hash(active.def_version, renames, &steps, new).map_err(|e| {
            vec![error(format!(
                "cannot compute the hash of the migration plan: {e}"
            ))]
        })?,
        steps,
    };
    Ok(Computed { plan, physical })
}

/// The hash of the reviewed content: the version, the declarations, the steps and the new
/// definition, as JSON with sorted keys.
fn plan_hash(
    from_def_version: i64,
    renames: &[RenameDecl],
    steps: &[MigrationStep],
    ontology: &Ontology,
) -> Result<String, serde_json::Error> {
    let document = canonical(serde_json::json!({
        "fromDefVersion": from_def_version,
        "renames": serde_json::to_value(renames)?,
        "steps": serde_json::to_value(steps)?,
        "ontology": serde_json::to_value(ontology)?,
    }));
    let text = serde_json::to_string(&document)?;
    Ok(naming::fnv1a64_hex(text.as_bytes()))
}

/// `value` with the keys of every object in sorted order, so the text does not depend on the
/// order of the files or keys the definition was loaded from.
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, canonical(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonical).collect()),
        other => other,
    }
}

/// `index` moved to `table` and `column`, with the name those give it.
fn rebuilt_index(table: &str, column: &str, index: &IndexDef) -> IndexDef {
    let name = match index.kind {
        IndexKind::Plain => naming::index_name(table, column),
        IndexKind::UniqueWhenSet => naming::unique_index_name(table, column),
    };
    IndexDef {
        name,
        column: column.to_owned(),
        kind: index.kind,
    }
}

fn drop_index(table: &str, index: &IndexDef) -> MigrationStep {
    MigrationStep::Ddl(DdlStep::DropIndex {
        table: table.to_owned(),
        name: index.name.clone(),
    })
}

fn create_index(table: &str, index: &IndexDef) -> MigrationStep {
    MigrationStep::Ddl(DdlStep::CreateIndex {
        table: table.to_owned(),
        index: index.clone(),
    })
}

/// `OldType=NewType` declarations. A declaration whose old type is gone and whose new type is
/// there is taken as applied already and skipped, so a repeated call plans nothing.
fn rename_object_types(
    old: &mut PhysicalSchema,
    new: &PhysicalSchema,
    renames: &[RenameDecl],
    steps: &mut Vec<MigrationStep>,
    errors: &mut Vec<Diagnostic>,
) {
    for decl in renames {
        let RenameDecl::ObjectType { from, to } = decl else {
            continue;
        };
        let applied =
            !old.objects.contains_key(from.as_str()) && old.objects.contains_key(to.as_str());
        if applied {
            continue;
        }
        let Some(table) = old.objects.get(from.as_str()) else {
            errors.push(error(format!(
                "rename `{decl}`: object type `{from}` does not exist in the active definition"
            )));
            continue;
        };
        if old.objects.contains_key(to.as_str()) {
            errors.push(error(format!(
                "rename `{decl}`: object type `{to}` already exists in the active definition"
            )));
            continue;
        }
        let Some(target) = new.objects.get(to.as_str()) else {
            errors.push(error(format!(
                "rename `{decl}`: object type `{to}` is not in the new definition"
            )));
            continue;
        };
        if new.objects.contains_key(from.as_str()) {
            errors.push(error(format!(
                "rename `{decl}`: object type `{from}` is still in the new definition; a rename removes the old name"
            )));
            continue;
        }
        if table.pk.property != target.pk.property || table.pk.ty != target.pk.ty {
            errors.push(error(format!(
                "rename `{decl}`: the primary key differs (`{}` of type {:?} against `{}` of type {:?}); rename first, then change the primary key in a later migration",
                table.pk.property, table.pk.ty, target.pk.property, target.pk.ty
            )));
            continue;
        }

        let Some((from_key, mut table)) = old.objects.remove_entry(from.as_str()) else {
            continue;
        };
        let to_table = naming::object_table(&target_name(new, to));
        let previous = std::mem::take(&mut table.indexes);
        steps.extend(previous.iter().map(|ix| drop_index(&table.table, ix)));
        steps.push(MigrationStep::RenameTable {
            from_type: from.clone(),
            to_type: to.clone(),
            from: table.table.clone(),
            to: to_table.clone(),
        });
        table.table = to_table;
        table.indexes = previous
            .iter()
            .map(|ix| rebuilt_index(&table.table, &ix.column, ix))
            .collect();
        steps.extend(
            table
                .indexes
                .iter()
                .map(|ix| create_index(&table.table, ix)),
        );
        for link in old.links.values_mut() {
            if link.from_type == from_key {
                link.from_type = target_name(new, to);
            }
            if link.to_type == from_key {
                link.to_type = target_name(new, to);
            }
        }
        old.objects.insert(target_name(new, to), table);
    }
}

/// The key of `name` in the objects of `new`, which the caller has checked exists.
fn target_name(new: &PhysicalSchema, name: &str) -> ObjectTypeName {
    new.objects
        .keys()
        .find(|key| key.as_str() == name)
        .cloned()
        .unwrap_or_else(|| unreachable!("`{name}` was checked to be in the new definition"))
}

/// `Type.old=new` declarations. The two properties must have the same type, or the data would be
/// reinterpreted (T-03-48).
fn rename_properties(
    old: &mut PhysicalSchema,
    new: &PhysicalSchema,
    renames: &[RenameDecl],
    steps: &mut Vec<MigrationStep>,
    errors: &mut Vec<Diagnostic>,
) {
    for decl in renames {
        let RenameDecl::Property { type_name, .. } = decl else {
            continue;
        };
        let Some(table) = old.objects.get_mut(type_name.as_str()) else {
            errors.push(error(format!(
                "rename `{decl}`: object type `{type_name}` does not exist in the active definition"
            )));
            continue;
        };
        match check_property_rename(decl, table, new.objects.get(type_name.as_str())) {
            Ok(Some(rename)) => move_property(table, type_name, &rename, steps),
            Ok(None) => {}
            Err(diagnostic) => errors.push(diagnostic),
        }
    }
}

/// A property rename that passed its checks.
struct PropertyRename {
    from: String,
    to: String,
    from_column: PropertyColumn,
    to_key: PropertyName,
    to_column: PropertyColumn,
}

/// Checks a `Type.old=new` declaration against the old table and the new one. `Ok(None)` means
/// the rename has happened already.
fn check_property_rename(
    decl: &RenameDecl,
    table: &ObjectTable,
    target: Option<&ObjectTable>,
) -> Result<Option<PropertyRename>, Diagnostic> {
    let RenameDecl::Property {
        type_name,
        from,
        to,
    } = decl
    else {
        return Ok(None);
    };
    let fail = |why: String| Err(error(format!("rename `{decl}`: {why}")));
    let has_to = table.properties.contains_key(to.as_str());
    let Some(from_column) = table.properties.get(from.as_str()) else {
        if has_to {
            return Ok(None);
        }
        return fail(if table.pk.property.as_str() == from.as_str() {
            format!(
                "property `{type_name}.{from}` is the primary key; renaming it changes the primary key"
            )
        } else {
            format!("property `{type_name}.{from}` does not exist in the active definition")
        });
    };
    if has_to {
        return fail(format!(
            "property `{type_name}.{to}` already exists in the active definition"
        ));
    }
    let Some(target) = target else {
        return fail(format!(
            "object type `{type_name}` is not in the new definition"
        ));
    };
    let Some((to_key, to_column)) = target.properties.get_key_value(to.as_str()) else {
        return fail(format!(
            "property `{type_name}.{to}` is not a property of the new definition"
        ));
    };
    if target.properties.contains_key(from.as_str()) {
        return fail(format!(
            "property `{type_name}.{from}` is still in the new definition; a rename removes the old name"
        ));
    }
    if from_column.ty != to_column.ty {
        return fail(format!(
            "the type changes from {:?} to {:?}; rename first, then change the type in a later migration",
            from_column.ty, to_column.ty
        ));
    }
    if to_column.required && !from_column.required {
        return fail(format!(
            "`{type_name}.{to}` is required but `{from}` is not, so existing objects could lack a value; make it required in a later migration"
        ));
    }
    Ok(Some(PropertyRename {
        from: from.clone(),
        to: to.clone(),
        from_column: from_column.clone(),
        to_key: to_key.clone(),
        to_column: to_column.clone(),
    }))
}

/// Moves the property in the old table to its new name, with the steps that do it: the indexes
/// on the column are dropped, the column renamed, the indexes created again under the new name.
fn move_property(
    table: &mut ObjectTable,
    type_name: &str,
    rename: &PropertyRename,
    steps: &mut Vec<MigrationStep>,
) {
    let moved: Vec<IndexDef> = table
        .indexes
        .iter()
        .filter(|ix| ix.column == rename.from_column.column)
        .cloned()
        .collect();
    steps.extend(moved.iter().map(|ix| drop_index(&table.table, ix)));
    if rename.from_column.column != rename.to_column.column {
        steps.push(MigrationStep::RenameColumn {
            type_name: type_name.to_owned(),
            table: table.table.clone(),
            from: rename.from.clone(),
            to: rename.to.clone(),
            from_column: rename.from_column.column.clone(),
            to_column: rename.to_column.column.clone(),
        });
    }
    let rebuilt: Vec<IndexDef> = moved
        .iter()
        .map(|ix| rebuilt_index(&table.table, &rename.to_column.column, ix))
        .collect();
    steps.extend(rebuilt.iter().map(|ix| create_index(&table.table, ix)));
    table
        .indexes
        .retain(|ix| ix.column != rename.from_column.column);
    table.indexes.extend(rebuilt);
    table.indexes.sort_by(|a, b| a.name.cmp(&b.name));
    table.properties.remove(rename.from.as_str());
    table
        .properties
        .insert(rename.to_key.clone(), rename.to_column.clone());
}

/// Turns the result of `diff` into steps and records what was archived in the new schema.
struct Archiver<'a> {
    old: &'a PhysicalSchema,
    physical: &'a mut PhysicalSchema,
    version: i64,
    steps: &'a mut Vec<MigrationStep>,
}

impl Archiver<'_> {
    fn destructive(&mut self, plan: &Plan) {
        // A table that is archived and created again leaves no room for finer changes of the
        // old table.
        let rebuilt: BTreeSet<&str> = plan
            .destructive
            .iter()
            .filter_map(|change| match change {
                DestructiveChange::PrimaryKeyChanged { type_name } => Some(type_name.as_str()),
                _ => None,
            })
            .collect();
        for change in &plan.destructive {
            match change {
                DestructiveChange::PropertyRemoved {
                    type_name,
                    property,
                } if !rebuilt.contains(type_name.as_str()) => {
                    self.archive_column(type_name, property);
                }
                DestructiveChange::PropertyTypeChanged {
                    type_name,
                    property,
                } if !rebuilt.contains(type_name.as_str()) => {
                    self.archive_column(type_name, property);
                    self.add_column(type_name, property);
                }
                DestructiveChange::RequiredPropertyAdded {
                    type_name,
                    property,
                } if !rebuilt.contains(type_name.as_str()) => {
                    self.add_column(type_name, property);
                }
                DestructiveChange::ObjectTypeRemoved { type_name } => {
                    self.archive_object_table(type_name);
                }
                DestructiveChange::PrimaryKeyChanged { type_name } => {
                    self.archive_object_table(type_name);
                    self.create_object_table(type_name);
                }
                DestructiveChange::LinkTypeRemoved { link } => self.archive_link_table(link),
                DestructiveChange::LinkEndsChanged { link } => {
                    self.archive_link_table(link);
                    self.create_link_table(link);
                }
                _ => {}
            }
        }
        // A link table keeps the key columns of its ends; when the key of an end changed, the
        // table is rebuilt, whether or not the ends themselves changed.
        let flagged: BTreeSet<&str> = plan
            .destructive
            .iter()
            .filter_map(|change| match change {
                DestructiveChange::LinkEndsChanged { link }
                | DestructiveChange::LinkTypeRemoved { link } => Some(link.as_str()),
                _ => None,
            })
            .collect();
        let keys_changed: Vec<String> = self
            .physical
            .links
            .iter()
            .filter(|(name, new)| {
                !flagged.contains(name.as_str())
                    && self
                        .old
                        .links
                        .get(name.as_str())
                        .is_some_and(|old| old.a != new.a || old.b != new.b)
            })
            .map(|(name, _)| name.to_string())
            .collect();
        for link in keys_changed {
            self.archive_link_table(&link);
            self.create_link_table(&link);
        }
    }

    /// The additive steps of `diff`, minus those on a table that is archived and created again.
    fn additive(&mut self, plan: &Plan) {
        let archived: BTreeSet<&str> = plan
            .destructive
            .iter()
            .filter_map(|change| match change {
                DestructiveChange::PrimaryKeyChanged { type_name } => self
                    .old
                    .objects
                    .get(type_name.as_str())
                    .map(|table| table.table.as_str()),
                _ => None,
            })
            .collect();
        for step in &plan.steps {
            let table = match step {
                DdlStep::AddPropertyColumn { table, .. }
                | DdlStep::CreateIndex { table, .. }
                | DdlStep::DropIndex { table, .. } => Some(table.as_str()),
                DdlStep::CreateObjectTable(_) | DdlStep::CreateLinkTable(_) => None,
            };
            if table.is_some_and(|table| archived.contains(table)) {
                continue;
            }
            self.steps.push(MigrationStep::Ddl(step.clone()));
        }
    }

    /// The column and its `__set` column get the archive names. The indexes on the column are
    /// dropped first: an archived column is never read, and an index would keep a name that a
    /// new column of the same name needs.
    fn archive_column(&mut self, type_name: &str, property: &str) {
        let Some(table) = self.old.objects.get(type_name) else {
            return;
        };
        let Some((key, column)) = table.properties.get_key_value(property) else {
            return;
        };
        self.steps.extend(
            table
                .indexes
                .iter()
                .filter(|ix| ix.column == column.column)
                .map(|ix| drop_index(&table.table, ix)),
        );
        let archived = naming::archived_column(&column.column, self.version);
        let set_archived = naming::archived_set_column(&column.column, self.version);
        self.steps.push(MigrationStep::ArchiveColumn {
            type_name: type_name.to_owned(),
            property: property.to_owned(),
            table: table.table.clone(),
            column: column.column.clone(),
            archived: archived.clone(),
            set_archived: set_archived.clone(),
        });
        if let Some(new_table) = self.physical.objects.get_mut(type_name) {
            new_table.archived_columns.push(ArchivedColumn {
                property: key.clone(),
                column: archived,
                set_column: set_archived,
                ty: column.ty.clone(),
                def_version: self.version,
            });
        }
    }

    /// A new empty column for the property, with its indexes.
    fn add_column(&mut self, type_name: &str, property: &str) {
        let Some(table) = self.physical.objects.get(type_name) else {
            return;
        };
        let Some(column) = table.properties.get(property) else {
            return;
        };
        self.steps
            .push(MigrationStep::Ddl(DdlStep::AddPropertyColumn {
                table: table.table.clone(),
                column: column.clone(),
            }));
        self.steps.extend(
            table
                .indexes
                .iter()
                .filter(|ix| ix.column == column.column)
                .map(|ix| create_index(&table.table, ix)),
        );
    }

    fn archive_object_table(&mut self, type_name: &str) {
        let Some(table) = self.old.objects.get(type_name) else {
            return;
        };
        self.steps
            .extend(table.indexes.iter().map(|ix| drop_index(&table.table, ix)));
        self.archive_table(type_name, &table.table);
    }

    fn archive_link_table(&mut self, link: &str) {
        let Some(table) = self.old.links.get(link) else {
            return;
        };
        self.archive_table(link, &table.table);
    }

    fn archive_table(&mut self, type_name: &str, table: &str) {
        let archived = naming::archived_table(table, self.version);
        self.steps.push(MigrationStep::ArchiveTable {
            type_name: type_name.to_owned(),
            table: table.to_owned(),
            archived: archived.clone(),
        });
        self.physical.archived_tables.push(ArchivedTable {
            type_name: type_name.to_owned(),
            table: archived,
            def_version: self.version,
        });
    }

    /// The table of the object type as the new definition has it, with its indexes. It starts
    /// empty and without archived columns.
    fn create_object_table(&mut self, type_name: &str) {
        let Some(table) = self.physical.objects.get_mut(type_name) else {
            return;
        };
        table.archived_columns.clear();
        let table = &*table;
        self.steps
            .push(MigrationStep::Ddl(DdlStep::CreateObjectTable(
                table.clone(),
            )));
        self.steps.extend(
            table
                .indexes
                .iter()
                .map(|ix| create_index(&table.table, ix)),
        );
    }

    fn create_link_table(&mut self, link: &str) {
        let Some(table) = self.physical.links.get(link) else {
            return;
        };
        self.steps
            .push(MigrationStep::Ddl(DdlStep::CreateLinkTable(table.clone())));
    }
}
