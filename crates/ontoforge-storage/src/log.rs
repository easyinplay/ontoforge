//! The edit log: one record per successful commit (D-10, D-11).
//!
//! A record holds who did what and when plus one entry per edit. An entry stores only the new
//! values and the kind of operation, never an old value and never a copy of a source value.

use crate::error::StoreError;
use crate::store::EditStore;

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

impl EditStore {
    /// Up to `limit` records with `seq > after_seq`, in `seq` order.
    pub async fn read_log(
        &self,
        _after_seq: i64,
        _limit: u32,
    ) -> Result<Vec<LogRecord>, StoreError> {
        Ok(Vec::new())
    }
}
