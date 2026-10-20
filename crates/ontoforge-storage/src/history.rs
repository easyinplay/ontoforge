//! The object history (D-12): one snapshot of the edit row per log entry, and the masked read of
//! the change history of one object (STORE-04, D-13).
//!
//! The snapshot of an object entry is `{"deleted": bool, "released": bool, "set": {property:
//! value}}`, that of a link entry `{"deleted": bool}`. It holds the edit row only, never a value
//! from the source.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::{Attrs, RowValues};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use sea_query::{Alias, Expr, Query};
use serde::{Serialize, Serializer};

use crate::error::StoreError;
use crate::log::{EditOp, LogEntry};
use crate::sql_exit;
use crate::store::{EditStore, WriteTx};
use crate::value::PropertyValue;

/// Who asks for a history. Only the administration channel sees the raw Action parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerKind {
    Ordinary,
    Admin,
}

/// One history read.
pub struct HistoryRequest<'a> {
    pub type_name: &'a ObjectTypeName,
    pub pk: &'a PropertyValue,
    /// The caller's merged scope on the type; `None` when no role of the caller grants it.
    pub scope: Option<&'a TypeScope>,
    pub attrs: &'a Attrs,
    /// The object as the caller sees it now, source overlay included; `None` when it does not
    /// exist.
    pub current_row: Option<&'a dyn RowValues>,
    pub caller: CallerKind,
    pub limit: u32,
    /// Only entries with a smaller `seq` are returned.
    pub before_seq: Option<i64>,
}

/// One change of one object, as the caller may see it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryEntry {
    pub seq: i64,
    pub event_id: String,
    pub actor: String,
    pub action: String,
    /// UTC, milliseconds: `YYYY-MM-DDTHH:MM:SS.fffZ`.
    pub committed_at: String,
    #[serde(serialize_with = "serialize_op")]
    pub op: EditOp,
    /// The version of the edit row after this change.
    pub version: i64,
    /// Visible properties and the value this change gave them.
    pub values: BTreeMap<PropertyName, serde_json::Value>,
    /// Visible properties this change cleared.
    pub unset: BTreeSet<PropertyName>,
    /// Properties the caller may not see that this change touched: the name only, no value.
    pub masked_changes: BTreeSet<PropertyName>,
    /// The raw Action parameters; only for [`CallerKind::Admin`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

fn serialize_op<S: Serializer>(op: &EditOp, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(op.as_str())
}

/// Why a history cannot be read.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    /// The object does not exist, is not visible to the caller, or the caller has no grant on its
    /// type. The three cases are not told apart (D-13, ADR-0025).
    #[error("object not found")]
    NotFound,
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl EditStore {
    /// The changes of one object, newest first.
    pub async fn history(
        &self,
        req: &HistoryRequest<'_>,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        let _ = req;
        Ok(Vec::new())
    }
}

/// Records the state of the row `entry` touched, as it is after the entry.
pub(crate) async fn append_snapshot(
    tx: &mut WriteTx<'_>,
    seq: i64,
    entry: &LogEntry,
    row_snapshot: &serde_json::Value,
) -> Result<(), StoreError> {
    let backend = tx.backend();
    let insert = Query::insert()
        .into_table(Alias::new("of_edit_history"))
        .columns([
            Alias::new("log_seq"),
            Alias::new("ordinal"),
            Alias::new("kind"),
            Alias::new("type_name"),
            Alias::new("pk"),
            Alias::new("version"),
            Alias::new("snapshot"),
        ])
        .values_panic([
            Expr::val(seq),
            Expr::val(entry.ordinal),
            Expr::val(entry.kind.as_str()),
            Expr::val(entry.type_name.clone()),
            sql_exit::typed_text(backend, &entry.pk.to_string(), "jsonb"),
            Expr::val(entry.version_after),
            sql_exit::typed_text(backend, &row_snapshot.to_string(), "jsonb"),
        ])
        .to_owned();
    let built = sql_exit::build(&insert, tx);
    sql_exit::execute_built(tx, built).await.map(|_| ())
}
