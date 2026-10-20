//! The edit log: one record per successful commit (D-10, D-11).
//!
//! A record holds who did what and when plus one entry per edit. An entry stores only the new
//! values and the kind of operation, never an old value and never a copy of a source value.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_schema::ir::{LinkTypeName, ObjectType, ObjectTypeName, PropertyName};
use sea_query::{Alias, Expr, ExprTrait, Order, Query};
use serde_json::Value;

use crate::commit::{self, EditError, LinkOp, ObjectOp, Planned, Target};
use crate::error::StoreError;
use crate::sql_exit::{self, Built};
use crate::store::{ActiveDefinition, Backend, EditStore, WriteTx};
use crate::value::PropertyValue;

/// What an entry did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOp {
    Set,
    Unset,
    Tombstone,
    Release,
    LinkAdd,
    LinkRemove,
}

impl EditOp {
    /// The text stored in `of_edit_log_entry.op`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Unset => "unset",
            Self::Tombstone => "tombstone",
            Self::Release => "release",
            Self::LinkAdd => "link_add",
            Self::LinkRemove => "link_remove",
        }
    }

    /// Reads the stored text back.
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "set" => Self::Set,
            "unset" => Self::Unset,
            "tombstone" => Self::Tombstone,
            "release" => Self::Release,
            "link_add" => Self::LinkAdd,
            "link_remove" => Self::LinkRemove,
            _ => return None,
        })
    }
}

/// Whether an entry is about an object or a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Object,
    Link,
}

impl EntryKind {
    /// The text stored in `of_edit_log_entry.kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Link => "link",
        }
    }

    /// Reads the stored text back.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "object" => Some(Self::Object),
            "link" => Some(Self::Link),
            _ => None,
        }
    }
}

/// One edit of a commit.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub ordinal: i32,
    pub kind: EntryKind,
    pub type_name: String,
    /// The primary key as canonical JSON; for a link `[a, b]`.
    pub pk: serde_json::Value,
    pub op: EditOp,
    /// `set`: property to new value; `unset`: the list of property names; otherwise `None`.
    pub payload: Option<serde_json::Value>,
    pub version_after: i64,
}

/// One successful commit.
#[derive(Debug, Clone, PartialEq)]
pub struct LogRecord {
    pub seq: i64,
    pub event_id: String,
    pub actor: String,
    pub action: String,
    /// UTC, milliseconds: `YYYY-MM-DDTHH:MM:SS.fffZ`.
    pub committed_at: String,
    pub def_version: i64,
    pub params: serde_json::Value,
    pub entries: Vec<LogEntry>,
}

/// Inserts one entry of the record `seq`.
pub(crate) async fn append_entry(
    tx: &mut WriteTx<'_>,
    seq: i64,
    entry: &LogEntry,
) -> Result<(), StoreError> {
    let backend = tx.backend();
    let payload = match &entry.payload {
        Some(payload) => sql_exit::typed_text(backend, &payload.to_string(), "jsonb"),
        None => Expr::null(),
    };
    let insert = Query::insert()
        .into_table(Alias::new("of_edit_log_entry"))
        .columns([
            Alias::new("log_seq"),
            Alias::new("ordinal"),
            Alias::new("kind"),
            Alias::new("type_name"),
            Alias::new("pk"),
            Alias::new("op"),
            Alias::new("payload"),
            Alias::new("version_after"),
        ])
        .values_panic([
            Expr::val(seq),
            Expr::val(entry.ordinal),
            Expr::val(entry.kind.as_str()),
            Expr::val(entry.type_name.clone()),
            sql_exit::typed_text(backend, &entry.pk.to_string(), "jsonb"),
            Expr::val(entry.op.as_str()),
            payload,
            Expr::val(entry.version_after),
        ])
        .to_owned();
    let built = sql_exit::build(&insert, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}

/// Why a replay stopped. Nothing is written to the target in any of these cases.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    /// Applying the entry to the rebuilt row gave another version than the log recorded.
    #[error(
        "replaying log record {seq}, entry {ordinal}, gave a version other than the logged one"
    )]
    Diverged { seq: i64, ordinal: i32 },
    #[error(transparent)]
    Edit(#[from] EditError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Rebuilds the edit tables of `target` from `records` (D-10).
///
/// The target must hold the definition the records were written under, loaded and empty. Records
/// are applied in `seq` order and entries in `ordinal` order, with the same `next_state` and
/// `next_link_state` a commit uses. After each entry the version the transition gave must equal
/// the logged `version_after`, otherwise the replay stops with [`ReplayError::Diverged`]. Each row
/// keeps the original `seq` in `of_log_seq` and the logged definition version in
/// `of_def_version`. The log and the history of the target are not written.
///
/// All records are applied in one transaction: a failed replay leaves the target as it was.
pub async fn replay(records: &[LogRecord], target: &EditStore) -> Result<(), ReplayError> {
    let active = target.active_definition().await.ok_or_else(|| {
        StoreError::Config("no definition is loaded; run reload first".to_owned())
    })?;
    let mut ordered: Vec<&LogRecord> = records.iter().collect();
    ordered.sort_by_key(|record| record.seq);
    let mut tx = target.begin_write().await?;
    match apply_records(&mut tx, &active, &ordered).await {
        Ok(()) => {
            sql_exit::commit(tx).await?;
            Ok(())
        }
        Err(error) => {
            sql_exit::rollback(tx).await;
            Err(error)
        }
    }
}

async fn apply_records(
    tx: &mut WriteTx<'_>,
    def: &ActiveDefinition,
    records: &[&LogRecord],
) -> Result<(), ReplayError> {
    let schema = tx.schema_edit().map(str::to_owned);
    let schema = schema.as_deref();
    for record in records {
        if record.def_version != def.def_version {
            return Err(EditError::DefinitionStale {
                expected: record.def_version,
                active: def.def_version,
            }
            .into());
        }
        // A record is replayed the way it was committed: rows locked and advanced in lock order,
        // then each row written once in the state the record leaves it.
        let mut entries: Vec<&LogEntry> = record.entries.iter().collect();
        entries.sort_by_key(|entry| entry.ordinal);
        let mut plan = Vec::with_capacity(entries.len());
        for entry in &entries {
            plan.push(replay_target(def, record.seq, entry)?);
        }
        commit::sort_plan(&mut plan);
        let advanced = commit::lock_and_advance(tx, schema, &plan).await?;
        for (planned, state) in plan.iter().zip(&advanced.after) {
            let logged = entries
                .iter()
                .find(|entry| usize::try_from(entry.ordinal).is_ok_and(|o| o == planned.ordinal));
            if logged.is_none_or(|entry| entry.version_after != state.version()) {
                return Err(ReplayError::Diverged {
                    seq: record.seq,
                    ordinal: i32::try_from(planned.ordinal).unwrap_or(i32::MAX),
                });
            }
        }
        commit::write_rows(
            tx,
            schema,
            def,
            record.def_version,
            record.seq,
            &plan,
            &advanced,
        )
        .await?;
    }
    Ok(())
}

/// The operation of an object entry. A malformed entry is `StoreError::Corrupt` with the reason.
fn replay_object_op(ty: &ObjectType, entry: &LogEntry) -> Result<ObjectOp, ReplayError> {
    let corrupt = |what: &str| ReplayError::Store(StoreError::Corrupt(what.to_owned()));
    Ok(match (entry.op, &entry.payload) {
        (EditOp::Set, Some(Value::Object(values))) => {
            let mut set = BTreeMap::new();
            for (property, json) in values {
                let name = PropertyName::try_from(property.clone())
                    .map_err(|_| corrupt("bad property name"))?;
                let property_def =
                    ty.properties
                        .get(&name)
                        .ok_or_else(|| EditError::UnknownProperty {
                            type_name: entry.type_name.clone(),
                            property: property.clone(),
                        })?;
                let value = PropertyValue::from_json(property_def, json)
                    .map_err(|e| corrupt(&format!("bad value of `{property}`: {e}")))?;
                set.insert(name, value);
            }
            ObjectOp::Set(set)
        }
        (EditOp::Unset, Some(Value::Array(names))) => {
            let mut unset = BTreeSet::new();
            for name in names {
                let name = name
                    .as_str()
                    .and_then(|text| PropertyName::try_from(text.to_owned()).ok())
                    .ok_or_else(|| corrupt("bad property name"))?;
                unset.insert(name);
            }
            ObjectOp::Unset(unset)
        }
        (EditOp::Tombstone, None) => ObjectOp::Tombstone,
        (EditOp::Release, None) => ObjectOp::Release,
        _ => return Err(corrupt("the operation does not fit an object entry")),
    })
}

/// The edit an entry stands for, resolved against the definition.
fn replay_target<'a>(
    def: &'a ActiveDefinition,
    seq: i64,
    entry: &LogEntry,
) -> Result<Planned<'a>, ReplayError> {
    let ordinal = usize::try_from(entry.ordinal).map_err(|_| {
        ReplayError::Store(StoreError::Corrupt(format!(
            "edit log entry {seq}/{}: negative ordinal",
            entry.ordinal
        )))
    })?;
    let corrupt = |what: &str| {
        ReplayError::Store(StoreError::Corrupt(format!(
            "edit log entry {seq}/{}: {what}",
            entry.ordinal
        )))
    };
    let unknown = || ReplayError::Edit(EditError::UnknownType(entry.type_name.clone()));
    match entry.kind {
        EntryKind::Object => {
            let type_name =
                ObjectTypeName::try_from(entry.type_name.clone()).map_err(|_| unknown())?;
            let (name, ty) = def
                .ontology
                .object_types
                .get_key_value(&type_name)
                .ok_or_else(unknown)?;
            let table = def.physical.object_table(name).ok_or_else(unknown)?;
            let pk = PropertyValue::from_json(commit::primary_key_def(ty)?, &entry.pk)
                .map_err(|e| corrupt(&format!("bad primary key: {e}")))?;
            let op = replay_object_op(ty, entry).map_err(|e| match e {
                ReplayError::Store(StoreError::Corrupt(what)) => corrupt(&what),
                other => other,
            })?;
            Ok(Planned::new(
                ordinal,
                name.as_str(),
                Target::Object { ty, table, pk, op },
            ))
        }
        EntryKind::Link => {
            let link_name =
                LinkTypeName::try_from(entry.type_name.clone()).map_err(|_| unknown())?;
            let (name, link_type) = def
                .ontology
                .link_types
                .get_key_value(&link_name)
                .ok_or_else(unknown)?;
            let table = def.physical.link_table(name).ok_or_else(unknown)?;
            let [a, b] = entry.pk.as_array().map_or(&[][..], Vec::as_slice) else {
                return Err(corrupt("a link key is `[a, b]`"));
            };
            let end = |object: &ObjectTypeName, json: &Value| {
                let ty = def.ontology.object_types.get(object).ok_or_else(|| {
                    corrupt(&format!("the link refers to unknown type `{object}`"))
                })?;
                PropertyValue::from_json(commit::primary_key_def(ty)?, json)
                    .map_err(|e| corrupt(&format!("bad link key: {e}")))
            };
            let op = match entry.op {
                EditOp::LinkAdd => LinkOp::Add,
                EditOp::LinkRemove => LinkOp::Remove,
                _ => return Err(corrupt("the operation does not fit a link entry")),
            };
            Ok(Planned::new(
                ordinal,
                name.as_str(),
                Target::Link {
                    table,
                    backing: commit::link_backing(link_type.backing),
                    a: end(&link_type.from.object_type, a)?,
                    b: end(&link_type.to.object_type, b)?,
                    op,
                },
            ))
        }
    }
}

type HeaderRow = (i64, String, String, String, String, i64, String);
type EntryRow = (
    i64,
    i32,
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
);

impl EditStore {
    /// Up to `limit` records with `seq > after_seq`, in `seq` order.
    pub async fn read_log(&self, after_seq: i64, limit: u32) -> Result<Vec<LogRecord>, StoreError> {
        let backend = self.backend();
        let headers =
            sql_exit::fetch_all_as::<HeaderRow>(&self.db, header_query(backend, after_seq, limit))
                .await?;
        let Some(last_seq) = headers.last().map(|row| row.0) else {
            return Ok(Vec::new());
        };
        let entries =
            sql_exit::fetch_all_as::<EntryRow>(&self.db, entry_query(backend, after_seq, last_seq))
                .await?;

        let mut records = headers
            .into_iter()
            .map(
                |(seq, event_id, actor, action, committed_at, def_version, params)| {
                    Ok(LogRecord {
                        seq,
                        event_id,
                        actor,
                        action,
                        committed_at,
                        def_version,
                        params: parse_json(&params, "params")?,
                        entries: Vec::new(),
                    })
                },
            )
            .collect::<Result<Vec<_>, StoreError>>()?;
        for (log_seq, ordinal, kind, type_name, pk, op, payload, version_after) in entries {
            let corrupt = |what: &str| {
                StoreError::Corrupt(format!("edit log entry {log_seq}/{ordinal}: bad {what}"))
            };
            let Some(record) = records.iter_mut().find(|record| record.seq == log_seq) else {
                continue;
            };
            record.entries.push(LogEntry {
                ordinal,
                kind: EntryKind::parse(&kind).ok_or_else(|| corrupt("kind"))?,
                type_name,
                pk: parse_json(&pk, "pk")?,
                op: EditOp::parse(&op).ok_or_else(|| corrupt("op"))?,
                payload: payload
                    .as_deref()
                    .map(|text| parse_json(text, "payload"))
                    .transpose()?,
                version_after,
            });
        }
        Ok(records)
    }
}

fn header_query(backend: Backend, after_seq: i64, limit: u32) -> Built {
    let select = Query::select()
        .column(Alias::new("seq"))
        .column(Alias::new("event_id"))
        .column(Alias::new("actor"))
        .column(Alias::new("action"))
        .expr(sql_exit::timestamp_text(backend, "committed_at"))
        .column(Alias::new("def_version"))
        .expr(sql_exit::column_text(backend, "params"))
        .from(Alias::new("of_edit_log"))
        .and_where(Expr::col(Alias::new("seq")).gt(after_seq))
        .order_by(Alias::new("seq"), Order::Asc)
        .limit(u64::from(limit))
        .to_owned();
    sql_exit::build_for(&select, backend)
}

fn entry_query(backend: Backend, after_seq: i64, last_seq: i64) -> Built {
    let select = Query::select()
        .column(Alias::new("log_seq"))
        .column(Alias::new("ordinal"))
        .column(Alias::new("kind"))
        .column(Alias::new("type_name"))
        .expr(sql_exit::column_text(backend, "pk"))
        .column(Alias::new("op"))
        .expr(sql_exit::column_text(backend, "payload"))
        .column(Alias::new("version_after"))
        .from(Alias::new("of_edit_log_entry"))
        .and_where(Expr::col(Alias::new("log_seq")).gt(after_seq))
        .and_where(Expr::col(Alias::new("log_seq")).lte(last_seq))
        .order_by(Alias::new("log_seq"), Order::Asc)
        .order_by(Alias::new("ordinal"), Order::Asc)
        .to_owned();
    sql_exit::build_for(&select, backend)
}

fn parse_json(text: &str, what: &str) -> Result<serde_json::Value, StoreError> {
    serde_json::from_str(text)
        .map_err(|e| StoreError::Corrupt(format!("edit log {what} is not JSON: {e}")))
}
