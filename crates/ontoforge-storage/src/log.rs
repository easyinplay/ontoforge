//! The edit log: one record per successful commit (D-10, D-11).
//!
//! A record holds who did what and when plus one entry per edit. An entry stores only the new
//! values and the kind of operation, never an old value and never a copy of a source value.

use sea_query::{Alias, Expr, ExprTrait, Order, Query};

use crate::commit::EditError;
use crate::error::StoreError;
use crate::sql_exit::{self, Built};
use crate::store::{Backend, EditStore, WriteTx};

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
#[allow(
    clippy::unused_async,
    reason = "stub of the RED commit; the real body awaits"
)]
pub async fn replay(records: &[LogRecord], target: &EditStore) -> Result<(), ReplayError> {
    let _ = (records, target);
    Err(StoreError::Config("replay is not implemented".to_owned()).into())
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
