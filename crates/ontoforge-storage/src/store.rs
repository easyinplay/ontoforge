//! Connection options of the SQLite edit store.

use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::sqlite::{
    DECIMAL_COLLATION, TIMESTAMP_COLLATION, compare_decimal, compare_timestamp,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};

/// Connection options of the SQLite edit store at `path`.
///
/// Every connection of a pool built from these options:
/// - enforces foreign keys and waits up to 5 s on a locked database,
/// - runs in WAL mode,
/// - has `recursive_triggers` on, so `INSERT OR REPLACE` fires the delete trigger that keeps the
///   edit log append only,
/// - knows the engine's `ontoforge_decimal` and `ontoforge_timestamp` collations, the same
///   implementation the SQLite stand-in uses.
pub fn sqlite_connect_options(path: &Utf8Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path.as_std_path())
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .journal_mode(SqliteJournalMode::Wal)
        .pragma("recursive_triggers", "ON")
        .collation(DECIMAL_COLLATION, compare_decimal)
        .collation(TIMESTAMP_COLLATION, compare_timestamp)
}
