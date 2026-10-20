//! The write side of the object history (D-12): one snapshot of the edit row per log entry.
//! Reading and masking come with plan 03-17.
//!
//! The snapshot of an object entry is `{"deleted": bool, "released": bool, "set": {property:
//! value}}`, that of a link entry `{"deleted": bool}`. It holds the edit row only, never a value
//! from the source.

use sea_query::{Alias, Expr, Query};

use crate::error::StoreError;
use crate::log::LogEntry;
use crate::sql_exit;
use crate::store::WriteTx;

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
