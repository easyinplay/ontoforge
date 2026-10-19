//! Connection options of the SQLite edit store.

use camino::Utf8Path;
use sqlx::sqlite::SqliteConnectOptions;

/// Connection options of the SQLite edit store at `path`.
pub fn sqlite_connect_options(path: &Utf8Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path.as_std_path())
        .create_if_missing(true)
}
