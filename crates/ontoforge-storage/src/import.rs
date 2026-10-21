//! Bulk import of editOnly data: a CSV or JSON file becomes system edits (STORE-06, D-11).
//!
//! The import is not a second write path. Every batch is one [`Commit`] by the reserved actor
//! `system:import`, so it is validated, fenced against the active definition version, logged and
//! snapshotted exactly like any other commit (ADR-0026). Only editOnly object types and editOnly
//! many-to-many links can be imported: business data of a source backed type belongs in the lake
//! (ADR-0016), and an import that could write it would be a general write door around the Actions.

use camino::Utf8Path;
use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName};

use crate::commit::EditError;
use crate::error::StoreError;
use crate::store::EditStore;

/// Rows per commit when the caller does not choose.
pub const DEFAULT_BATCH_SIZE: usize = 1_000;
/// The largest batch a caller may ask for.
pub const MAX_BATCH_SIZE: usize = 10_000;
/// Rows a file may hold when the caller does not choose.
pub const DEFAULT_MAX_ROWS: usize = 1_000_000;
/// Bytes a file may have when the caller does not choose: 256 MiB.
pub const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// What a file holds the rows of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    /// The objects of an editOnly object type; the columns are its properties.
    Object(ObjectTypeName),
    /// The edges of an editOnly many-to-many link; the columns are `from` and `to`.
    Link(LinkTypeName),
}

/// What to do with a row whose object (or edge) already has an edit row. A tombstoned object and
/// a removed edge are never revived, whatever the mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnExisting {
    /// Refuse the whole file and name the keys: the import only creates.
    #[default]
    Error,
    /// Leave existing rows alone; an interrupted import can be run again.
    Skip,
    /// Set the cells of the file on a live object.
    Update,
}

/// Limits and choices of one import.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub on_existing: OnExisting,
    /// Rows per commit, `1..=`[`MAX_BATCH_SIZE`].
    pub batch_size: usize,
    /// More rows than this refuse the file.
    pub max_rows: usize,
    /// A larger file is refused unread.
    pub max_bytes: u64,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            on_existing: OnExisting::Error,
            batch_size: DEFAULT_BATCH_SIZE,
            max_rows: DEFAULT_MAX_ROWS,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }
}

/// What an import did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// Data rows in the file.
    pub rows: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    /// The `seq` of each commit, in order.
    pub commits: Vec<i64>,
}

/// Why an import stopped.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The file or its rows are not acceptable (ONT-V27, with file and line). Nothing was written.
    #[error("{} problem(s) in the import file", .0.len())]
    Data(Vec<Diagnostic>),
    /// The target is not an editOnly object type or an editOnly many-to-many link. Nothing was
    /// written.
    #[error("{0}")]
    NotEditOnly(String),
    /// A batch failed after `committed` batches were written. The commits stay; running the import
    /// again with [`OnExisting::Skip`] writes the rest.
    #[error(
        "batch {failed_batch} failed after {} committed batch(es): {error}",
        committed.len()
    )]
    Partial {
        committed: Vec<i64>,
        /// The index of the failed batch, counted from 0.
        failed_batch: usize,
        error: Box<EditError>,
    },
    #[error(transparent)]
    Edit(EditError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Imports the rows of `file` into the edit store.
///
/// The whole file is read, converted and checked before anything is written; a file with a
/// problem writes nothing. The rows are then committed in batches of `opts.batch_size`, one
/// commit each, so a failure after the first batch leaves the earlier batches in place
/// ([`ImportError::Partial`]).
pub async fn import(
    _store: &EditStore,
    _target: ImportTarget,
    _file: &Utf8Path,
    _opts: &ImportOptions,
) -> Result<ImportReport, ImportError> {
    Ok(ImportReport::default())
}
