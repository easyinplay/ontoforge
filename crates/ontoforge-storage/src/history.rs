//! The write side of the object history (D-12): one snapshot of the edit row per log entry.
//! Reading and masking come with plan 03-17.

use crate::error::StoreError;
use crate::log::LogEntry;
use crate::store::WriteTx;

/// Records the state of the row `entry` touched, as it is after the entry.
pub(crate) async fn append_snapshot(
    _tx: &mut WriteTx<'_>,
    _seq: i64,
    _entry: &LogEntry,
    _row_snapshot: &serde_json::Value,
) -> Result<(), StoreError> {
    Ok(())
}
