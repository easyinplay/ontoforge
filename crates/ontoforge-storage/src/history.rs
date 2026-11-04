//! The object history (D-12): one snapshot of the edit row per log entry, and the masked read of
//! the change history of one object (STORE-04, D-13).
//!
//! The snapshot of an object entry is `{"deleted": bool, "released": bool, "set": {property:
//! value}}`, that of a link entry `{"deleted": bool}`. It holds the edit row only, never a value
//! from the source.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::rowpred::Scalar;
use ontoforge_expr::{Attrs, RowValues};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use sea_query::{Alias, Expr, ExprTrait, Order, Query};
use serde::{Serialize, Serializer};

use crate::error::StoreError;
use crate::log::{EditOp, LogEntry};
use crate::sql_exit::{self, Built};
use crate::store::{Backend, EditStore, WriteTx};
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
    pub current_row: Option<&'a (dyn RowValues + Sync)>,
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

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature serde's `serialize_with` calls"
)]
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

/// The most entries one read returns.
const MAX_LIMIT: u32 = 500;

/// `(log_seq, ordinal)` of an entry.
type EntryKey = (i64, i32);
/// `log_seq, ordinal, op, payload, version_after`.
type EntryRow = (i64, i32, String, Option<String>, i64);
/// `seq, event_id, actor, action, committed_at`.
type HeaderRow = (i64, String, String, String, String);

impl EditStore {
    /// The changes of one object, newest first (STORE-04, D-13).
    ///
    /// The caller must see the object now: no scope, no object and an object the scope hides give
    /// the same [`HistoryError::NotFound`], and that is decided before the log is touched. Values
    /// are shown for the columns the scope lets the caller see in the current row, whatever the
    /// scope was when the change was made; the other changed columns are only named. The raw
    /// Action parameters are read and returned for [`CallerKind::Admin`] only.
    pub async fn history(
        &self,
        req: &HistoryRequest<'_>,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        // Preserve the existing short-circuit: invisible requests must not touch storage.
        visible_columns(req)?;
        let active = self.active_definition().await.ok_or_else(|| {
            StoreError::Config("no definition is loaded; run reload first".to_owned())
        })?;
        self.history_with_snapshot(req, &active).await
    }

    /// Reads history using the definition captured when the request was admitted.
    pub async fn history_with_snapshot(
        &self,
        req: &HistoryRequest<'_>,
        active: &crate::store::ActiveDefinition,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        let visible = visible_columns(req)?;
        let ty = active
            .ontology
            .object_types
            .get(req.type_name)
            .ok_or(HistoryError::NotFound)?;
        let pk_def = ty.properties.get(&ty.primary_key).ok_or_else(|| {
            StoreError::Corrupt(format!(
                "type `{}` has no primary key property `{}`",
                ty.api_name, ty.primary_key
            ))
        })?;
        let pk = req
            .pk
            .canonical(pk_def)
            .map_err(|_| HistoryError::NotFound)?;
        let object = ObjectRef {
            type_name: req.type_name.as_str(),
            pk: pk.to_json().to_string(),
        };
        let backend = self.backend();

        let limit = effective_limit(req.limit);
        let mut located = sql_exit::fetch_all_as::<EntryKey>(
            &self.db,
            locate_query(backend, &object, req.before_seq, limit),
        )
        .await?;
        // `before_seq` pages by record, so a page never ends inside the edits one commit made to
        // this object: the rest of the last record is added, and the page may exceed the limit by
        // that.
        if let Some(&(seq, ordinal)) = located.last()
            && located.len() >= usize::try_from(limit).unwrap_or(usize::MAX)
        {
            located.extend(
                sql_exit::fetch_all_as::<EntryKey>(
                    &self.db,
                    rest_of_record_query(backend, &object, seq, ordinal),
                )
                .await?,
            );
        }
        if located.is_empty() {
            return Ok(Vec::new());
        }
        let seqs: Vec<i64> = located.iter().map(|(seq, _)| *seq).collect();

        let entry_rows =
            sql_exit::fetch_all_as::<EntryRow>(&self.db, entry_query(backend, &object, &seqs))
                .await?;
        let entry_rows: BTreeMap<EntryKey, EntryRow> = entry_rows
            .into_iter()
            .map(|row| ((row.0, row.1), row))
            .collect();
        let headers =
            sql_exit::fetch_all_as::<HeaderRow>(&self.db, header_query(backend, &seqs)).await?;
        let headers: BTreeMap<i64, HeaderRow> =
            headers.into_iter().map(|row| (row.0, row)).collect();
        let mut params: BTreeMap<i64, serde_json::Value> = BTreeMap::new();
        if req.caller == CallerKind::Admin {
            let rows =
                sql_exit::fetch_all_as::<(i64, String)>(&self.db, params_query(backend, &seqs))
                    .await?;
            for (seq, text) in rows {
                let value = serde_json::from_str(&text).map_err(|e| {
                    StoreError::Corrupt(format!("edit log record {seq}: params are not JSON: {e}"))
                })?;
                params.insert(seq, value);
            }
        }

        located
            .iter()
            .map(|key| {
                let missing = |what: &str| {
                    HistoryError::Store(StoreError::Corrupt(format!(
                        "edit log entry {}/{} has no {what}",
                        key.0, key.1
                    )))
                };
                let entry = entry_rows.get(key).ok_or_else(|| missing("entry"))?;
                let header = headers.get(&key.0).ok_or_else(|| missing("record"))?;
                build_entry(header, entry, &visible, params.get(&key.0).cloned())
            })
            .collect()
    }
}

/// A limit of 0 reads as 1; anything above [`MAX_LIMIT`] reads as the maximum.
fn effective_limit(limit: u32) -> u32 {
    limit.clamp(1, MAX_LIMIT)
}

/// What the caller may see of the object now, or [`HistoryError::NotFound`]. No scope, no object
/// and a hidden object are one case; nothing is read from the store to decide it.
fn visible_columns(req: &HistoryRequest<'_>) -> Result<BTreeSet<PropertyName>, HistoryError> {
    let (Some(scope), Some(row)) = (req.scope, req.current_row) else {
        return Err(HistoryError::NotFound);
    };
    let seen = scope.evaluate(&DynRow(row), req.attrs);
    if seen.visible {
        Ok(seen.columns)
    } else {
        Err(HistoryError::NotFound)
    }
}

/// `evaluate` takes a sized row; this lets it take the caller's `dyn RowValues`.
struct DynRow<'a>(&'a dyn RowValues);

impl RowValues for DynRow<'_> {
    fn value(&self, property: &PropertyName) -> Scalar {
        self.0.value(property)
    }
}

/// The object a history is about: its type and primary key as canonical JSON text.
struct ObjectRef<'a> {
    type_name: &'a str,
    pk: String,
}

/// One entry of the caller's history. `visible` is the only gate between a stored value and the
/// response: a property outside it leaves this function as a name in `masked_changes`.
fn build_entry(
    header: &HeaderRow,
    entry: &EntryRow,
    visible: &BTreeSet<PropertyName>,
    params: Option<serde_json::Value>,
) -> Result<HistoryEntry, HistoryError> {
    let (seq, ordinal, op, payload, version) = entry;
    let corrupt = |what: &str| {
        HistoryError::Store(StoreError::Corrupt(format!(
            "edit log entry {seq}/{ordinal}: bad {what}"
        )))
    };
    let op = EditOp::parse(op).ok_or_else(|| corrupt("op"))?;
    let payload = payload
        .as_deref()
        .map(serde_json::from_str::<serde_json::Value>)
        .transpose()
        .map_err(|_| corrupt("payload"))?;
    let property =
        |name: &str| PropertyName::try_from(name.to_owned()).map_err(|_| corrupt("property name"));

    let mut values = BTreeMap::new();
    let mut unset = BTreeSet::new();
    let mut masked_changes = BTreeSet::new();
    match (op, payload) {
        (EditOp::Set, Some(serde_json::Value::Object(set))) => {
            for (name, value) in set {
                let name = property(&name)?;
                if visible.contains(&name) {
                    values.insert(name, value);
                } else {
                    masked_changes.insert(name);
                }
            }
        }
        (EditOp::Unset, Some(serde_json::Value::Array(names))) => {
            for name in &names {
                let name = property(name.as_str().ok_or_else(|| corrupt("property name"))?)?;
                if visible.contains(&name) {
                    unset.insert(name);
                } else {
                    masked_changes.insert(name);
                }
            }
        }
        (EditOp::Tombstone | EditOp::Release, None) => {}
        _ => return Err(corrupt("operation for an object")),
    }
    let (_, event_id, actor, action, committed_at) = header;
    Ok(HistoryEntry {
        seq: *seq,
        event_id: event_id.clone(),
        actor: actor.clone(),
        action: action.clone(),
        committed_at: committed_at.clone(),
        op,
        version: *version,
        values,
        unset,
        masked_changes,
        params,
    })
}

/// `(log_seq, ordinal)` of the object entries of `object`, newest first.
fn locate_query(
    backend: Backend,
    object: &ObjectRef<'_>,
    before_seq: Option<i64>,
    limit: u32,
) -> Built {
    let mut select = Query::select()
        .column(Alias::new("log_seq"))
        .column(Alias::new("ordinal"))
        .from(Alias::new("of_edit_history"))
        .and_where(Expr::col(Alias::new("kind")).eq("object"))
        .and_where(Expr::col(Alias::new("type_name")).eq(object.type_name))
        .and_where(
            Expr::col(Alias::new("pk")).eq(sql_exit::typed_text(backend, &object.pk, "jsonb")),
        )
        .order_by(Alias::new("log_seq"), Order::Desc)
        .order_by(Alias::new("ordinal"), Order::Desc)
        .limit(u64::from(limit))
        .to_owned();
    if let Some(before) = before_seq {
        select.and_where(Expr::col(Alias::new("log_seq")).lt(before));
    }
    sql_exit::build_for(&select, backend)
}

/// `(log_seq, ordinal)` of the entries of `object` in record `seq` below `ordinal`, newest first.
fn rest_of_record_query(backend: Backend, object: &ObjectRef<'_>, seq: i64, ordinal: i32) -> Built {
    let select = Query::select()
        .column(Alias::new("log_seq"))
        .column(Alias::new("ordinal"))
        .from(Alias::new("of_edit_history"))
        .and_where(Expr::col(Alias::new("kind")).eq("object"))
        .and_where(Expr::col(Alias::new("type_name")).eq(object.type_name))
        .and_where(
            Expr::col(Alias::new("pk")).eq(sql_exit::typed_text(backend, &object.pk, "jsonb")),
        )
        .and_where(Expr::col(Alias::new("log_seq")).eq(seq))
        .and_where(Expr::col(Alias::new("ordinal")).lt(ordinal))
        .order_by(Alias::new("ordinal"), Order::Desc)
        .to_owned();
    sql_exit::build_for(&select, backend)
}

/// The log entries of `object` inside the records `seqs`.
fn entry_query(backend: Backend, object: &ObjectRef<'_>, seqs: &[i64]) -> Built {
    let select = Query::select()
        .column(Alias::new("log_seq"))
        .column(Alias::new("ordinal"))
        .column(Alias::new("op"))
        .expr(sql_exit::column_text(backend, "payload"))
        .column(Alias::new("version_after"))
        .from(Alias::new("of_edit_log_entry"))
        .and_where(Expr::col(Alias::new("kind")).eq("object"))
        .and_where(Expr::col(Alias::new("type_name")).eq(object.type_name))
        .and_where(
            Expr::col(Alias::new("pk")).eq(sql_exit::typed_text(backend, &object.pk, "jsonb")),
        )
        .and_where(Expr::col(Alias::new("log_seq")).is_in(seqs.iter().copied()))
        .to_owned();
    sql_exit::build_for(&select, backend)
}

/// The record headers `seqs`, without the Action parameters.
fn header_query(backend: Backend, seqs: &[i64]) -> Built {
    let select = Query::select()
        .column(Alias::new("seq"))
        .column(Alias::new("event_id"))
        .column(Alias::new("actor"))
        .column(Alias::new("action"))
        .expr(sql_exit::timestamp_text(backend, "committed_at"))
        .from(Alias::new("of_edit_log"))
        .and_where(Expr::col(Alias::new("seq")).is_in(seqs.iter().copied()))
        .to_owned();
    sql_exit::build_for(&select, backend)
}

/// The Action parameters of the records `seqs`. Built only for the administration channel.
fn params_query(backend: Backend, seqs: &[i64]) -> Built {
    let select = Query::select()
        .column(Alias::new("seq"))
        .expr(sql_exit::column_text(backend, "params"))
        .from(Alias::new("of_edit_log"))
        .and_where(Expr::col(Alias::new("seq")).is_in(seqs.iter().copied()))
        .to_owned();
    sql_exit::build_for(&select, backend)
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

#[cfg(test)]
mod tests {
    use super::{MAX_LIMIT, effective_limit};

    #[test]
    fn limit_is_between_one_and_the_maximum() {
        assert_eq!(effective_limit(0), 1);
        assert_eq!(effective_limit(1), 1);
        assert_eq!(effective_limit(MAX_LIMIT), MAX_LIMIT);
        assert_eq!(effective_limit(10_000), 500);
        assert_eq!(effective_limit(u32::MAX), 500);
    }
}
