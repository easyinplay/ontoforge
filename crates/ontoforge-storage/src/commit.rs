//! The write path of the edit store: one commit, one transaction (STORE-03).
//!
//! A [`Commit`] is validated without a transaction; then one transaction locks or creates the
//! edit rows, computes their new state, appends one log record with its entries and the history
//! snapshots, and writes the rows. Any failure rolls everything back.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::{SecondsFormat, Utc};
use ontoforge_schema::ir::{
    Backing, Cardinality, LinkTypeName, ObjectType, ObjectTypeName, PropertyDef, PropertyName,
};
use sea_query::{Alias, Expr, ExprTrait, LockType, OnConflict, Query};
use serde_json::{Map, Value, json};

use crate::error::StoreError;
use crate::history;
use crate::log::{self, EditOp, EntryKind, LogEntry};
use crate::naming;
use crate::physical::{ColumnType, IndexKind, LinkTable, ObjectTable};
use crate::sql_exit::{self, Cell, CellKind};
use crate::state::{EditRow, LinkRow, next_link_state, next_state};
use crate::store::{ActiveDefinition, Backend, EditStore, WriteTx};
use crate::value::PropertyValue;

/// Who commits. The reserved system actors are stored as `system:<name>`; a principal id is
/// `{iss}|{sub}` and never starts with `system:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    /// `{iss}|{sub}`.
    Principal(String),
    System(SystemActor),
}

/// The reserved actors that are not a principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemActor {
    /// Stored as `system:import`.
    Import,
}

impl Actor {
    /// The text stored in `of_edit_log.actor`.
    pub fn as_text(&self) -> &str {
        match self {
            Self::Principal(id) => id,
            Self::System(SystemActor::Import) => "system:import",
        }
    }
}

/// One atomic change set.
#[derive(Debug, Clone)]
pub struct Commit {
    pub actor: Actor,
    pub action: String,
    /// The definition version the caller validated against.
    pub def_version: i64,
    /// The action parameters, stored as they are (D-11).
    pub params: serde_json::Value,
    pub edits: Vec<Edit>,
}

/// One change of an object or a link.
#[derive(Debug, Clone)]
pub enum Edit {
    Object {
        type_name: ObjectTypeName,
        pk: PropertyValue,
        op: ObjectOp,
    },
    Link {
        link: LinkTypeName,
        a: PropertyValue,
        b: PropertyValue,
        op: LinkOp,
    },
}

#[derive(Debug, Clone)]
pub enum ObjectOp {
    Set(BTreeMap<PropertyName, PropertyValue>),
    Unset(BTreeSet<PropertyName>),
    Tombstone,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOp {
    Add,
    Remove,
}

/// What a successful commit returns.
#[derive(Debug, Clone)]
pub struct Receipt {
    pub seq: i64,
    pub event_id: ulid::Ulid,
    /// The version of each edited row after its edit, in the order of `Commit::edits`.
    pub versions: Vec<i64>,
}

/// Why a commit was refused. Nothing is written in any of these cases.
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("unknown type `{0}`")]
    UnknownType(String),
    #[error("type `{type_name}` has no property `{property}`")]
    UnknownProperty { type_name: String, property: String },
    #[error("invalid value for `{property}`: {reason}")]
    Value { property: String, reason: String },
    #[error("the primary key cannot be set")]
    PrimaryKeyNotSettable,
    #[error("required property `{property}` is missing")]
    MissingRequired { property: String },
    #[error("the object does not exist")]
    NotFound,
    #[error("the object is deleted")]
    Tombstoned,
    #[error("cannot {op} an object that is {from}")]
    InvalidTransition {
        from: &'static str,
        op: &'static str,
    },
    #[error("only a source backed object can be released")]
    ReleaseOnEditOnly,
    #[error("link `{link}` would violate its cardinality")]
    CardinalityViolation { link: String },
    #[error("the commit was validated against definition {expected}, but {active} is active")]
    DefinitionStale { expected: i64, active: i64 },
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl EditStore {
    /// Applies `commit` in one transaction: the definition version is checked against the
    /// database first (the fence), then every row is locked or created, then the log record gets
    /// its `seq`, then the rows, the entries and the history snapshots are written. Because `seq`
    /// is allocated after all row locks, the `seq` order of two commits that touch the same row
    /// is their commit order. Any failure rolls the transaction back.
    pub async fn commit(&self, commit: Commit) -> Result<Receipt, EditError> {
        // A commit that is malformed whatever the definition says never takes the write lock.
        check_shape(&commit)?;
        let mut tx = self.begin_write().await?;
        let result = match self.fence(&mut tx, commit.def_version).await {
            Ok(def) => match validate(&def, &commit) {
                Ok(plan) => write(&mut tx, &def, &commit, plan).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        match result {
            Ok(receipt) => {
                sql_exit::commit(tx).await?;
                Ok(receipt)
            }
            Err(error) => {
                sql_exit::rollback(tx).await;
                Err(error)
            }
        }
    }

    /// The `def_version` fence (ADR-0016 / D20): the first statement of the write transaction
    /// reads the active definition version from the database, `FOR SHARE` on PostgreSQL, so a
    /// switch (which updates that row) waits for this commit to end and every commit that starts
    /// after the switch sees the new version. A commit validated against any other version is
    /// refused as [`EditError::DefinitionStale`]; nothing has been written yet.
    ///
    /// Returns the definition the commit is validated against. When the in-memory definition is
    /// not the one the database says is active (another process switched), the active one is
    /// read from the database and kept, also when the commit is refused, so the caller's next
    /// attempt can use it.
    async fn fence(
        &self,
        tx: &mut WriteTx<'_>,
        expected: i64,
    ) -> Result<Arc<ActiveDefinition>, EditError> {
        let select = Query::select()
            .column(col("active_def_version"))
            .from(col(sql_exit::STATE_TABLE))
            .lock(LockType::Share)
            .to_owned();
        let built = sql_exit::build(&select, tx);
        let active = sql_exit::fetch_scalar_i64(tx, built).await?.unwrap_or(0);
        if active == 0 {
            return Err(
                StoreError::Config("no definition is loaded; run reload first".to_owned()).into(),
            );
        }
        let def = match self.active_definition().await {
            Some(def) if def.def_version == active => def,
            _ => {
                let fresh = Arc::new(sql_exit::read_definition_in(tx, active).await?);
                self.install_if_newer(&fresh).await;
                fresh
            }
        };
        if expected != active {
            return Err(EditError::DefinitionStale { expected, active });
        }
        Ok(def)
    }
}

fn value_error(property: &str, reason: &(impl std::fmt::Display + ?Sized)) -> EditError {
    EditError::Value {
        property: property.to_owned(),
        reason: reason.to_string(),
    }
}

// ---------------------------------------------------------------------------------------------
// Validation (no transaction)
// ---------------------------------------------------------------------------------------------

/// Which row an edit touches. The derived order is the order in which rows are locked, the same
/// in every commit, so two commits cannot wait on each other.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RowKey {
    /// `0` object, `1` link.
    kind: u8,
    name: String,
    /// The primary key as canonical JSON text; for a link `[a, b]`.
    pk: String,
}

/// A validated edit with its values in canonical form.
pub(crate) struct Planned<'a> {
    pub(crate) ordinal: usize,
    key: RowKey,
    pub(crate) target: Target<'a>,
}

impl<'a> Planned<'a> {
    /// The edit `target` on the object type or link type `name`.
    pub(crate) fn new(ordinal: usize, name: &str, target: Target<'a>) -> Self {
        let (kind, pk) = match &target {
            Target::Object { pk, .. } => (0, pk.to_json().to_string()),
            Target::Link { a, b, .. } => (1, json!([a.to_json(), b.to_json()]).to_string()),
        };
        Self {
            ordinal,
            key: RowKey {
                kind,
                name: name.to_owned(),
                pk,
            },
            target,
        }
    }

    pub(crate) fn type_name(&self) -> &str {
        &self.key.name
    }
}

/// Puts the edits in lock order. The sort is stable, so edits of one row keep their order.
pub(crate) fn sort_plan(plan: &mut [Planned<'_>]) {
    plan.sort_by(|x, y| x.key.cmp(&y.key));
}

pub(crate) enum Target<'a> {
    Object {
        ty: &'a ObjectType,
        table: &'a ObjectTable,
        pk: PropertyValue,
        op: ObjectOp,
    },
    Link {
        table: &'a LinkTable,
        backing: Backing,
        a: PropertyValue,
        b: PropertyValue,
        op: LinkOp,
    },
}

/// The checks that need no definition.
fn check_shape(commit: &Commit) -> Result<(), EditError> {
    if commit.edits.is_empty() {
        return Err(value_error("edits", "a commit needs at least one edit"));
    }
    if i32::try_from(commit.edits.len()).is_err() {
        return Err(value_error("edits", "a commit has too many edits"));
    }
    if commit.action.trim().is_empty() {
        return Err(value_error("action", "the action name is empty"));
    }
    if let Actor::Principal(id) = &commit.actor
        && !id
            .split_once('|')
            .is_some_and(|(issuer, subject)| !issuer.is_empty() && !subject.is_empty())
    {
        return Err(value_error(
            "actor",
            "a principal id is `{issuer}|{subject}`; other actors are the reserved system actors",
        ));
    }
    // PostgreSQL text and jsonb cannot hold U+0000; refuse it here so both backends agree.
    if commit.action.contains('\0') {
        return Err(value_error(
            "action",
            "the action name cannot contain U+0000",
        ));
    }
    if let Actor::Principal(id) = &commit.actor
        && id.contains('\0')
    {
        return Err(value_error("actor", "a principal id cannot contain U+0000"));
    }
    if json_contains_nul(&commit.params) {
        return Err(value_error(
            "params",
            "an action parameter string or key cannot contain U+0000; PostgreSQL jsonb cannot store it",
        ));
    }
    Ok(())
}

/// Whether a string or an object key anywhere in `value` contains U+0000. Iterative, so a deeply
/// nested parameter document cannot exhaust the stack.
fn json_contains_nul(value: &serde_json::Value) -> bool {
    let mut stack = vec![value];
    while let Some(next) = stack.pop() {
        match next {
            serde_json::Value::String(s) if s.contains('\0') => return true,
            serde_json::Value::Array(items) => stack.extend(items),
            serde_json::Value::Object(map) => {
                for (key, item) in map {
                    if key.contains('\0') {
                        return true;
                    }
                    stack.push(item);
                }
            }
            _ => {}
        }
    }
    false
}

/// The checks against the definition `def`, which the fence has shown to be the active one.
fn validate<'a>(def: &'a ActiveDefinition, commit: &Commit) -> Result<Vec<Planned<'a>>, EditError> {
    if commit.def_version != def.def_version {
        return Err(EditError::DefinitionStale {
            expected: commit.def_version,
            active: def.def_version,
        });
    }
    commit
        .edits
        .iter()
        .enumerate()
        .map(|(ordinal, edit)| plan_edit(def, ordinal, edit))
        .collect()
}

fn plan_edit<'a>(
    def: &'a ActiveDefinition,
    ordinal: usize,
    edit: &Edit,
) -> Result<Planned<'a>, EditError> {
    match edit {
        Edit::Object { type_name, pk, op } => {
            let unknown = || EditError::UnknownType(type_name.as_str().to_owned());
            let (name, ty) = def
                .ontology
                .object_types
                .get_key_value(type_name)
                .ok_or_else(unknown)?;
            let table = def.physical.object_table(name).ok_or_else(unknown)?;
            let pk = pk
                .canonical(primary_key_def(ty)?)
                .map_err(|e| value_error(ty.primary_key.as_str(), &e))?;
            let op = plan_object_op(ty, op)?;
            Ok(Planned::new(
                ordinal,
                name.as_str(),
                Target::Object { ty, table, pk, op },
            ))
        }
        Edit::Link { link, a, b, op } => {
            let (name, link_type) = def
                .ontology
                .link_types
                .get_key_value(link)
                .ok_or_else(|| EditError::UnknownType(link.as_str().to_owned()))?;
            let table = def.physical.link_table(name).ok_or_else(|| {
                EditError::UnknownType(format!(
                    "{} (only many-to-many links have rows to edit)",
                    link.as_str()
                ))
            })?;
            let end = |object: &ObjectTypeName, value: &PropertyValue, which: &str| {
                let ty = def.ontology.object_types.get(object).ok_or_else(|| {
                    StoreError::Corrupt(format!("link `{link}` refers to unknown type `{object}`"))
                })?;
                value
                    .canonical(primary_key_def(ty)?)
                    .map_err(|e| value_error(which, &e))
            };
            let a = end(&link_type.from.object_type, a, "a")?;
            let b = end(&link_type.to.object_type, b, "b")?;
            Ok(Planned::new(
                ordinal,
                name.as_str(),
                Target::Link {
                    table,
                    backing: link_backing(link_type.backing),
                    a,
                    b,
                    op: *op,
                },
            ))
        }
    }
}

/// A many-to-many link always declares its backing; `None` cannot reach a link table and reads
/// as the stricter editOnly rules.
pub(crate) fn link_backing(declared: Option<Backing>) -> Backing {
    declared.unwrap_or(Backing::EditOnly)
}

pub(crate) fn primary_key_def(ty: &ObjectType) -> Result<&PropertyDef, EditError> {
    ty.properties.get(&ty.primary_key).ok_or_else(|| {
        StoreError::Corrupt(format!(
            "type `{}` has no primary key property `{}`",
            ty.api_name, ty.primary_key
        ))
        .into()
    })
}

fn plan_object_op(ty: &ObjectType, op: &ObjectOp) -> Result<ObjectOp, EditError> {
    let property = |name: &PropertyName| -> Result<&PropertyDef, EditError> {
        if *name == ty.primary_key {
            return Err(EditError::PrimaryKeyNotSettable);
        }
        ty.properties
            .get(name)
            .ok_or_else(|| EditError::UnknownProperty {
                type_name: ty.api_name.as_str().to_owned(),
                property: name.as_str().to_owned(),
            })
    };
    match op {
        ObjectOp::Set(values) => {
            if values.is_empty() {
                return Err(value_error("set", "a set needs at least one property"));
            }
            values
                .iter()
                .map(|(name, value)| {
                    let canonical = value
                        .canonical(property(name)?)
                        .map_err(|e| value_error(name.as_str(), &e))?;
                    Ok((name.clone(), canonical))
                })
                .collect::<Result<BTreeMap<_, _>, EditError>>()
                .map(ObjectOp::Set)
        }
        ObjectOp::Unset(names) => {
            if names.is_empty() {
                return Err(value_error("unset", "an unset needs at least one property"));
            }
            for name in names {
                property(name)?;
            }
            Ok(op.clone())
        }
        ObjectOp::Tombstone | ObjectOp::Release => Ok(op.clone()),
    }
}

// ---------------------------------------------------------------------------------------------
// The transaction
// ---------------------------------------------------------------------------------------------

/// The state of a row as the commit sees it.
#[derive(Debug, Clone)]
pub(crate) enum RowState {
    Object(EditRow),
    Link(LinkRow),
}

impl RowState {
    pub(crate) fn version(&self) -> i64 {
        match self {
            Self::Object(row) => row.version,
            Self::Link(row) => row.version,
        }
    }

    /// The history snapshot of the row.
    fn snapshot(&self) -> Value {
        match self {
            Self::Object(row) => {
                let set: Map<String, Value> = row
                    .set
                    .iter()
                    .map(|(name, value)| (name.as_str().to_owned(), value.to_json()))
                    .collect();
                json!({ "deleted": row.deleted, "released": row.released, "set": set })
            }
            Self::Link(row) => json!({ "deleted": row.deleted }),
        }
    }
}

/// The rows of a plan after their edits.
pub(crate) struct Advanced {
    /// The state after each planned edit, in plan order.
    pub(crate) after: Vec<RowState>,
    /// Each row as it was when locked and as the commit leaves it.
    rows: BTreeMap<RowKey, (RowState, RowState)>,
}

/// Locks or creates every row of `plan` and computes its new state, in plan order. A second edit
/// of the same row sees the state the first one left. Nothing but the empty version 0 rows is
/// written.
pub(crate) async fn lock_and_advance(
    tx: &mut WriteTx<'_>,
    schema: Option<&str>,
    plan: &[Planned<'_>],
) -> Result<Advanced, EditError> {
    let mut rows: BTreeMap<RowKey, (RowState, RowState)> = BTreeMap::new();
    let mut after: Vec<RowState> = Vec::with_capacity(plan.len());
    for planned in plan {
        if !rows.contains_key(&planned.key) {
            let loaded = load_row(tx, schema, &planned.target).await?;
            rows.insert(planned.key.clone(), (loaded.clone(), loaded));
        }
        let (_, current) = rows.get_mut(&planned.key).ok_or_else(|| {
            StoreError::Corrupt("a locked row is missing from the commit".to_owned())
        })?;
        let next = advance(current, &planned.target)?;
        current.clone_from(&next);
        after.push(next);
    }
    Ok(Advanced { after, rows })
}

/// Writes each row of `plan` once, in lock order, in the state the plan leaves it. A unique
/// violation of a one-to-one foreign key is a [`EditError::CardinalityViolation`]. The check is
/// per written row, so a plan that moves a key from one object to another must list the object
/// that gives it up first in lock order.
pub(crate) async fn write_rows(
    tx: &mut WriteTx<'_>,
    schema: Option<&str>,
    def: &ActiveDefinition,
    def_version: i64,
    seq: i64,
    plan: &[Planned<'_>],
    advanced: &Advanced,
) -> Result<(), EditError> {
    let mut written: Option<&RowKey> = None;
    for planned in plan {
        if written == Some(&planned.key) {
            continue;
        }
        written = Some(&planned.key);
        let (before, after) = advanced.rows.get(&planned.key).ok_or_else(|| {
            StoreError::Corrupt("a locked row is missing from the commit".to_owned())
        })?;
        update_row(tx, schema, def_version, seq, &planned.target, before, after)
            .await
            .map_err(|error| cardinality_error(def, &planned.target, error))?;
    }
    Ok(())
}

/// A unique violation of an object row is the one-to-one link whose foreign key it is.
fn cardinality_error(def: &ActiveDefinition, target: &Target<'_>, error: EditError) -> EditError {
    let (Target::Object { table, .. }, EditError::Store(StoreError::Query { sqlstate, message })) =
        (target, &error)
    else {
        return error;
    };
    let violation = matches!(sqlstate.as_deref(), Some("23505" | "2067"))
        || message.contains("UNIQUE constraint failed");
    if !violation {
        return error;
    }
    let unique: Vec<_> = table
        .indexes
        .iter()
        .filter(|index| index.kind == IndexKind::UniqueWhenSet)
        .collect();
    let named = unique.iter().find(|index| {
        message.contains(index.name.as_str())
            || message.contains(&format!("{}.{}", table.table, index.column))
    });
    let index = match (named, unique.as_slice()) {
        (Some(index), _) | (None, [index]) => *index,
        _ => return error,
    };
    def.ontology
        .link_types
        .iter()
        .find(|(_, link)| {
            link.cardinality == Cardinality::OneToOne
                && link.to.foreign_key.as_ref().is_some_and(|fk| {
                    naming::property_column(fk) == index.column
                        && naming::object_table(&link.to.object_type) == table.table
                })
        })
        .map_or(error, |(name, _)| EditError::CardinalityViolation {
            link: name.as_str().to_owned(),
        })
}

async fn write(
    tx: &mut WriteTx<'_>,
    def: &ActiveDefinition,
    commit: &Commit,
    mut plan: Vec<Planned<'_>>,
) -> Result<Receipt, EditError> {
    sort_plan(&mut plan);
    let schema = tx.schema_edit().map(str::to_owned);
    let schema = schema.as_deref();

    let advanced = lock_and_advance(tx, schema, &plan).await?;
    let after = &advanced.after;

    let event_id = ulid::Ulid::generate();
    let event_text = event_id.to_string();
    let committed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let seq = append_header(
        tx,
        &NewRecord {
            event_id: &event_text,
            actor: commit.actor.as_text(),
            action: &commit.action,
            committed_at: &committed_at,
            def_version: commit.def_version,
            params: &commit.params,
        },
    )
    .await?;

    write_rows(tx, schema, def, commit.def_version, seq, &plan, &advanced).await?;

    let mut entries: Vec<(LogEntry, Value)> = plan
        .iter()
        .zip(after)
        .map(|(planned, state)| (log_entry(planned, state), state.snapshot()))
        .collect();
    entries.sort_by_key(|(entry, _)| entry.ordinal);
    for (entry, snapshot) in &entries {
        log::append_entry(tx, seq, entry).await?;
        history::append_snapshot(tx, seq, entry, snapshot).await?;
    }
    Ok(Receipt {
        seq,
        event_id,
        versions: entries
            .iter()
            .map(|(entry, _)| entry.version_after)
            .collect(),
    })
}

/// The header row of a record, as written.
struct NewRecord<'a> {
    event_id: &'a str,
    actor: &'a str,
    action: &'a str,
    /// `YYYY-MM-DDTHH:MM:SS.fffZ`.
    committed_at: &'a str,
    def_version: i64,
    params: &'a serde_json::Value,
}

/// Inserts the header row of a record and returns its `seq`.
async fn append_header(tx: &mut WriteTx<'_>, record: &NewRecord<'_>) -> Result<i64, StoreError> {
    let backend = tx.backend();
    let insert = Query::insert()
        .into_table(Alias::new("of_edit_log"))
        .columns([
            Alias::new("event_id"),
            Alias::new("actor"),
            Alias::new("action"),
            Alias::new("committed_at"),
            Alias::new("def_version"),
            Alias::new("params"),
        ])
        .values_panic([
            Expr::val(record.event_id.to_owned()),
            Expr::val(record.actor.to_owned()),
            Expr::val(record.action.to_owned()),
            sql_exit::typed_text(backend, record.committed_at, "timestamptz"),
            Expr::val(record.def_version),
            sql_exit::typed_text(backend, &record.params.to_string(), "jsonb"),
        ])
        .returning_col(Alias::new("seq"))
        .to_owned();
    let built = sql_exit::build(&insert, tx);
    sql_exit::fetch_scalar_i64(tx, built)
        .await?
        .ok_or_else(|| StoreError::Corrupt("the edit log insert returned no seq".to_owned()))
}

pub(crate) fn advance(current: &RowState, target: &Target<'_>) -> Result<RowState, EditError> {
    match (current, target) {
        (RowState::Object(row), Target::Object { ty, op, .. }) => {
            next_state(Some(row), op, ty).map(RowState::Object)
        }
        (RowState::Link(row), Target::Link { op, backing, .. }) => {
            next_link_state(Some(row), *op, *backing).map(RowState::Link)
        }
        _ => Err(StoreError::Corrupt("a row changed its kind".to_owned()).into()),
    }
}

fn log_entry(planned: &Planned<'_>, state: &RowState) -> LogEntry {
    let (kind, op, payload, pk) = match &planned.target {
        Target::Object { op, pk, .. } => {
            let (edit_op, payload) = match op {
                ObjectOp::Set(values) => (
                    EditOp::Set,
                    Some(Value::Object(
                        values
                            .iter()
                            .map(|(name, value)| (name.as_str().to_owned(), value.to_json()))
                            .collect(),
                    )),
                ),
                ObjectOp::Unset(names) => (
                    EditOp::Unset,
                    Some(Value::Array(
                        names
                            .iter()
                            .map(|name| Value::String(name.as_str().to_owned()))
                            .collect(),
                    )),
                ),
                ObjectOp::Tombstone => (EditOp::Tombstone, None),
                ObjectOp::Release => (EditOp::Release, None),
            };
            (EntryKind::Object, edit_op, payload, pk.to_json())
        }
        Target::Link { a, b, op, .. } => (
            EntryKind::Link,
            match op {
                LinkOp::Add => EditOp::LinkAdd,
                LinkOp::Remove => EditOp::LinkRemove,
            },
            None,
            json!([a.to_json(), b.to_json()]),
        ),
    };
    LogEntry {
        ordinal: i32::try_from(planned.ordinal).unwrap_or(i32::MAX),
        kind,
        type_name: planned.type_name().to_owned(),
        pk,
        op,
        payload,
        version_after: state.version(),
    }
}

fn col(name: &str) -> Alias {
    Alias::new(name)
}

/// Creates the version 0 row of the target when it is missing, then reads it under a row lock.
pub(crate) async fn load_row(
    tx: &mut WriteTx<'_>,
    schema: Option<&str>,
    target: &Target<'_>,
) -> Result<RowState, EditError> {
    let backend = tx.backend();
    match target {
        Target::Object { ty, table, pk, .. } => load_object(tx, schema, backend, ty, table, pk)
            .await
            .map(RowState::Object),
        Target::Link { table, a, b, .. } => load_link(tx, schema, backend, table, a, b)
            .await
            .map(RowState::Link),
    }
}

async fn load_object(
    tx: &mut WriteTx<'_>,
    schema: Option<&str>,
    backend: Backend,
    ty: &ObjectType,
    table: &ObjectTable,
    pk: &PropertyValue,
) -> Result<EditRow, EditError> {
    let table_ref = sql_exit::edit_table(schema, &table.table);
    let pk_expr = bind(pk, &table.pk.ty, backend);

    let skeleton = Query::insert()
        .into_table(table_ref.clone())
        .columns([col(&table.pk.column)])
        .values_panic([pk_expr.clone()])
        .on_conflict(
            OnConflict::column(col(&table.pk.column))
                .do_nothing()
                .to_owned(),
        )
        .to_owned();
    let built = sql_exit::build(&skeleton, tx);
    sql_exit::execute_built(tx, built).await?;

    let mut select = Query::select();
    let mut kinds = Vec::new();
    for column in table.properties.values() {
        select.column(col(&column.set_column));
        kinds.push(CellKind::Bool);
        let (expr, kind) = read_expr(&column.column, &column.ty, backend);
        select.expr(expr);
        kinds.push(kind);
    }
    select
        .column(col("of_version"))
        .column(col("of_deleted"))
        .column(col("of_released"))
        .from(table_ref)
        .and_where(Expr::col(col(&table.pk.column)).eq(pk_expr))
        .lock(LockType::Update);
    kinds.extend([CellKind::Int64, CellKind::Bool, CellKind::Bool]);
    let built = sql_exit::build(&select, tx);
    let cells = sql_exit::fetch_cells(tx, built, &kinds)
        .await?
        .ok_or_else(|| StoreError::Corrupt("the row created for the lock is missing".to_owned()))?;
    decode_object(ty, table, cells).map_err(EditError::from)
}

pub(crate) fn decode_object(
    ty: &ObjectType,
    table: &ObjectTable,
    cells: Vec<Cell>,
) -> Result<EditRow, StoreError> {
    let mut cells = cells.into_iter();
    let mut set = BTreeMap::new();
    for (name, column) in &table.properties {
        let (flag, value) = (cells.next(), cells.next());
        let (Some(Cell::Bool(true)), Some(value)) = (flag, value) else {
            continue;
        };
        let def = ty.properties.get(name).ok_or_else(|| {
            StoreError::Corrupt(format!("property `{name}` is not in the definition"))
        })?;
        let json = cell_json(value, &column.ty)?;
        let value = PropertyValue::from_json(def, &json).map_err(|e| {
            StoreError::Corrupt(format!("the stored value of `{name}` is invalid: {e}"))
        })?;
        set.insert(name.clone(), value);
    }
    match (cells.next(), cells.next(), cells.next()) {
        (Some(Cell::Int(version)), Some(Cell::Bool(deleted)), Some(Cell::Bool(released))) => {
            Ok(EditRow {
                set,
                deleted,
                released,
                version,
            })
        }
        _ => Err(StoreError::Corrupt(
            "an edit row has no version or flags".to_owned(),
        )),
    }
}

async fn load_link(
    tx: &mut WriteTx<'_>,
    schema: Option<&str>,
    backend: Backend,
    table: &LinkTable,
    a: &PropertyValue,
    b: &PropertyValue,
) -> Result<LinkRow, EditError> {
    let table_ref = sql_exit::edit_table(schema, &table.table);
    let a_expr = bind(a, &table.a.ty, backend);
    let b_expr = bind(b, &table.b.ty, backend);

    let skeleton = Query::insert()
        .into_table(table_ref.clone())
        .columns([col(&table.a.column), col(&table.b.column)])
        .values_panic([a_expr.clone(), b_expr.clone()])
        .on_conflict(
            OnConflict::columns([col(&table.a.column), col(&table.b.column)])
                .do_nothing()
                .to_owned(),
        )
        .to_owned();
    let built = sql_exit::build(&skeleton, tx);
    sql_exit::execute_built(tx, built).await?;

    let select = Query::select()
        .column(col("of_version"))
        .column(col("of_deleted"))
        .from(table_ref)
        .and_where(Expr::col(col(&table.a.column)).eq(a_expr))
        .and_where(Expr::col(col(&table.b.column)).eq(b_expr))
        .lock(LockType::Update)
        .to_owned();
    let built = sql_exit::build(&select, tx);
    let cells = sql_exit::fetch_cells(tx, built, &[CellKind::Int64, CellKind::Bool])
        .await?
        .ok_or_else(|| StoreError::Corrupt("the row created for the lock is missing".to_owned()))?;
    match cells.as_slice() {
        [Cell::Int(version), Cell::Bool(deleted)] => Ok(LinkRow {
            deleted: *deleted,
            version: *version,
        }),
        _ => Err(StoreError::Corrupt("a link row has no version or flag".to_owned()).into()),
    }
}

/// Writes the state a row has after its edits, with the system columns of the commit. Only the
/// property columns whose value differs from `before` are written: a property that is no longer
/// set gets its value cleared and its `__set` flag lowered.
pub(crate) async fn update_row(
    tx: &mut WriteTx<'_>,
    schema: Option<&str>,
    def_version: i64,
    seq: i64,
    target: &Target<'_>,
    before: &RowState,
    state: &RowState,
) -> Result<(), EditError> {
    let backend = tx.backend();
    let mut update = Query::update();
    match (target, before, state) {
        (Target::Object { table, pk, .. }, RowState::Object(old), RowState::Object(row)) => {
            update.table(sql_exit::edit_table(schema, &table.table));
            for (name, column) in &table.properties {
                let (was, now) = (old.set.get(name), row.set.get(name));
                if was == now {
                    continue;
                }
                match now {
                    Some(value) => update
                        .value(col(&column.column), bind(value, &column.ty, backend))
                        .value(col(&column.set_column), true),
                    None => update
                        .value(col(&column.column), Expr::null())
                        .value(col(&column.set_column), false),
                };
            }
            update
                .value(col("of_version"), row.version)
                .value(col("of_deleted"), row.deleted)
                .value(col("of_released"), row.released)
                .and_where(Expr::col(col(&table.pk.column)).eq(bind(pk, &table.pk.ty, backend)));
        }
        (Target::Link { table, a, b, .. }, _, RowState::Link(row)) => {
            update
                .table(sql_exit::edit_table(schema, &table.table))
                .value(col("of_version"), row.version)
                .value(col("of_deleted"), row.deleted)
                .and_where(Expr::col(col(&table.a.column)).eq(bind(a, &table.a.ty, backend)))
                .and_where(Expr::col(col(&table.b.column)).eq(bind(b, &table.b.ty, backend)));
        }
        _ => return Err(StoreError::Corrupt("a row changed its kind".to_owned()).into()),
    }
    update
        .value(col("of_def_version"), def_version)
        .value(col("of_log_seq"), seq);
    let built = sql_exit::build(&update, tx);
    sql_exit::execute_built(tx, built).await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Values in and out of the typed columns (ADR-0024 point 7)
// ---------------------------------------------------------------------------------------------

/// The PostgreSQL type of a column, for casts. The same names as the DDL gives the columns.
fn pg_type(ty: &ColumnType) -> String {
    match ty {
        ColumnType::Byte | ColumnType::Short => "smallint".to_owned(),
        ColumnType::Integer => "integer".to_owned(),
        ColumnType::Long => "bigint".to_owned(),
        ColumnType::Float => "real".to_owned(),
        ColumnType::Double => "double precision".to_owned(),
        ColumnType::Boolean => "boolean".to_owned(),
        ColumnType::String => "text".to_owned(),
        ColumnType::Decimal { precision, scale } => format!("numeric({precision}, {scale})"),
        ColumnType::Date => "date".to_owned(),
        ColumnType::Timestamp => "timestamptz".to_owned(),
        ColumnType::Array(element) => format!("{}[]", pg_type(element)),
        ColumnType::Geopoint => "jsonb".to_owned(),
    }
}

/// `value` as the literal a column of type `ty` takes.
///
/// SQLite: integers, reals and text as they are; a timestamp is `YYYY-MM-DD HH:MM:SS.fffZ` (the
/// form the `ontoforge_timestamp` collation parses); arrays and geopoints are JSON text, because
/// the SQLite binder cannot bind arrays.
/// PostgreSQL: text-borne types are cast (`numeric(p, s)`, `date`, `timestamptz`, `jsonb`); an
/// array is bound as `text[]` and cast to its element type, because the driver here has no native
/// decimal, date or timestamp types.
pub(crate) fn bind(value: &PropertyValue, ty: &ColumnType, backend: Backend) -> Expr {
    let postgres = backend == Backend::Postgres;
    match value {
        PropertyValue::Null => Expr::null(),
        PropertyValue::Bool(b) => Expr::val(*b),
        PropertyValue::Int(i) => Expr::val(*i),
        PropertyValue::Double(d) => Expr::val(*d),
        PropertyValue::String(s) => Expr::val(s.clone()),
        PropertyValue::Decimal(s) | PropertyValue::Date(s) => {
            sql_exit::typed_text(backend, s, &pg_type(ty))
        }
        PropertyValue::Timestamp(s) if postgres => sql_exit::typed_text(backend, s, "timestamptz"),
        PropertyValue::Timestamp(s) => Expr::val(s.replacen('T', " ", 1)),
        PropertyValue::Geopoint { .. } => {
            sql_exit::typed_text(backend, &value.to_json().to_string(), "jsonb")
        }
        PropertyValue::Array(items) if postgres => {
            let texts: Vec<String> = items.iter().map(element_text).collect();
            Expr::val(texts).cast_as(Alias::new(pg_type(ty)))
        }
        PropertyValue::Array(_) => Expr::val(value.to_json().to_string()),
    }
}

/// An array element as the text PostgreSQL casts to the element type.
fn element_text(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Decimal(s)
        | PropertyValue::Date(s)
        | PropertyValue::Timestamp(s)
        | PropertyValue::String(s) => s.clone(),
        PropertyValue::Bool(b) => b.to_string(),
        PropertyValue::Int(i) => i.to_string(),
        other => other.to_json().to_string(),
    }
}

/// The select expression and cell kind that read a value column back.
pub(crate) fn read_expr(column: &str, ty: &ColumnType, backend: Backend) -> (Expr, CellKind) {
    if backend == Backend::Sqlite {
        let kind = match ty {
            ColumnType::Byte | ColumnType::Short | ColumnType::Integer | ColumnType::Long => {
                CellKind::Int64
            }
            ColumnType::Float | ColumnType::Double => CellKind::Float64,
            ColumnType::Boolean => CellKind::Bool,
            _ => CellKind::Text,
        };
        return (Expr::col(col(column)), kind);
    }
    match ty {
        ColumnType::Byte | ColumnType::Short => (Expr::col(col(column)), CellKind::Int16),
        ColumnType::Integer => (Expr::col(col(column)), CellKind::Int32),
        ColumnType::Long => (Expr::col(col(column)), CellKind::Int64),
        ColumnType::Float => (Expr::col(col(column)), CellKind::Float32),
        ColumnType::Double => (Expr::col(col(column)), CellKind::Float64),
        ColumnType::Boolean => (Expr::col(col(column)), CellKind::Bool),
        ColumnType::String => (Expr::col(col(column)), CellKind::Text),
        ColumnType::Decimal { .. } | ColumnType::Date | ColumnType::Geopoint => {
            (sql_exit::column_text(backend, column), CellKind::Text)
        }
        ColumnType::Timestamp => (sql_exit::timestamp_text(backend, column), CellKind::Text),
        ColumnType::Array(element) => (
            sql_exit::array_text(column, **element == ColumnType::Timestamp),
            CellKind::TextArray,
        ),
    }
}

/// The JSON form of a stored cell, ready for `PropertyValue::from_json`.
pub(crate) fn cell_json(cell: Cell, ty: &ColumnType) -> Result<Value, StoreError> {
    let corrupt = |what: &str| StoreError::Corrupt(format!("a stored value is not {what}"));
    Ok(match cell {
        Cell::Null => Value::Null,
        Cell::Bool(b) => Value::Bool(b),
        Cell::Int(i) => Value::from(i),
        Cell::Float(f) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        Cell::Text(text) => match ty {
            // SQLite holds arrays and geopoints as JSON text.
            ColumnType::Array(_) | ColumnType::Geopoint => {
                serde_json::from_str(&text).map_err(|_| corrupt("JSON"))?
            }
            _ => Value::String(text),
        },
        Cell::TextArray(items) => {
            let ColumnType::Array(element) = ty else {
                return Err(corrupt("an array"));
            };
            Value::Array(
                items
                    .iter()
                    .map(|item| element_json(item, element))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
    })
}

/// One element of a PostgreSQL `text[]` as JSON of its element type.
fn element_json(text: &str, ty: &ColumnType) -> Result<Value, StoreError> {
    let corrupt = |what: &str| StoreError::Corrupt(format!("an array element is not {what}"));
    Ok(match ty {
        ColumnType::Byte | ColumnType::Short | ColumnType::Integer | ColumnType::Long => {
            Value::from(text.parse::<i64>().map_err(|_| corrupt("an integer"))?)
        }
        ColumnType::Float | ColumnType::Double => {
            let number = text.parse::<f64>().map_err(|_| corrupt("a number"))?;
            serde_json::Number::from_f64(number).map_or(Value::Null, Value::Number)
        }
        ColumnType::Boolean => Value::Bool(matches!(text, "t" | "true")),
        ColumnType::Geopoint => serde_json::from_str(text).map_err(|_| corrupt("JSON"))?,
        _ => Value::String(text.to_owned()),
    })
}
