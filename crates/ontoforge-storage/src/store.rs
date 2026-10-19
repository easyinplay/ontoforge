//! The edit store: settings, connection and the active definition (ADR-0016, ADR-0024).
//!
//! One `EditStore` serves one backend, SQLite or PostgreSQL. The PostgreSQL backend is added
//! in plan 03-04; until then it is refused at `open` with a clear error.

use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_engine::config::{StoreConfig, StoreKind, resolve_secret};
use ontoforge_engine::sqlite::{
    DECIMAL_COLLATION, TIMESTAMP_COLLATION, compare_decimal, compare_timestamp,
};
use ontoforge_schema::ir::Ontology;
use secrecy::SecretString;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{PgPool, Postgres, Sqlite, SqlitePool};
use tokio::sync::RwLock;

use crate::error::StoreError;
use crate::physical::PhysicalSchema;
use crate::sql_exit;

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

/// The database an edit store runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Sqlite,
    Postgres,
}

/// Where and how the edit store connects.
#[derive(Debug)]
pub struct StoreSettings {
    pub backend: BackendSettings,
    pub lock_timeout: Duration,
    pub probe_timeout: Duration,
    pub trino_catalog: Option<String>,
}

/// Backend specific connection settings.
#[derive(Debug)]
pub enum BackendSettings {
    /// A database file of its own. The edit store is never attached to another database.
    Sqlite { path: Utf8PathBuf },
    /// A PostgreSQL database with one schema for edit tables and one for system tables.
    Postgres {
        url: SecretString,
        schema_edit: String,
        schema_sys: String,
    },
}

impl StoreSettings {
    /// Builds the settings of a `[store]` section.
    ///
    /// A relative SQLite `path` is resolved against `config_dir`. The PostgreSQL connection
    /// string is read through `url_env` or `url_file` and never appears in an error.
    pub fn from_config(
        cfg: &StoreConfig,
        config_dir: &Utf8Path,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self, StoreError> {
        let backend = match cfg.kind {
            StoreKind::Sqlite => {
                let path = cfg
                    .path
                    .as_deref()
                    .filter(|p| !p.is_empty())
                    .ok_or_else(|| StoreError::Config("a sqlite store needs `path`".to_owned()))?;
                BackendSettings::Sqlite {
                    path: config_dir.join(path),
                }
            }
            StoreKind::Postgres => BackendSettings::Postgres {
                url: resolve_secret(
                    cfg.url_env.as_deref(),
                    cfg.url_file.as_deref(),
                    config_dir,
                    env,
                )
                .map_err(|e| StoreError::Config(e.message))?,
                schema_edit: cfg.schema_edit.clone(),
                schema_sys: cfg.schema_sys.clone(),
            },
        };
        Ok(Self {
            backend,
            lock_timeout: Duration::from_millis(cfg.lock_timeout_ms),
            probe_timeout: Duration::from_millis(cfg.probe_timeout_ms),
            trino_catalog: cfg.trino.as_ref().map(|t| t.catalog.clone()),
        })
    }
}

/// The connection of an open store.
#[allow(
    dead_code,
    reason = "the PostgreSQL variants are constructed by plan 03-04"
)]
pub(crate) enum Db {
    Sqlite(SqlitePool),
    Postgres {
        pool: PgPool,
        schema_edit: String,
        schema_sys: String,
    },
}

/// A write transaction. SQLite takes the write lock at `BEGIN`.
#[allow(
    dead_code,
    reason = "the PostgreSQL variant is constructed by plan 03-04"
)]
pub(crate) enum WriteTx<'c> {
    Sqlite(sqlx::Transaction<'c, Sqlite>),
    Postgres(sqlx::Transaction<'c, Postgres>),
}

/// The definition the edit tables currently follow.
#[derive(Debug, Clone)]
pub struct ActiveDefinition {
    pub def_version: i64,
    pub ontology: Ontology,
    pub physical: PhysicalSchema,
}

/// The edit store.
pub struct EditStore {
    pub(crate) db: Db,
    #[allow(dead_code, reason = "read by the PostgreSQL backend and by plan 03-18")]
    pub(crate) settings: StoreSettings,
    pub(crate) active: RwLock<Option<Arc<ActiveDefinition>>>,
}

impl EditStore {
    /// Opens the store: connects, runs the system migrations and loads the active definition.
    pub async fn open(settings: StoreSettings) -> Result<Self, StoreError> {
        let db = match &settings.backend {
            BackendSettings::Sqlite { path } => {
                let pool = SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(sqlite_connect_options(path))
                    .await
                    .map_err(|e| StoreError::Connect(e.to_string()))?;
                sqlx::migrate!("./migrations/sqlite")
                    .run(&pool)
                    .await
                    .map_err(|e| StoreError::Migrate(e.to_string()))?;
                Db::Sqlite(pool)
            }
            BackendSettings::Postgres { .. } => return Err(StoreError::postgres_unsupported()),
        };
        let active = sql_exit::read_active_definition(&db).await?;
        Ok(Self {
            db,
            settings,
            active: RwLock::new(active.map(Arc::new)),
        })
    }

    pub fn backend(&self) -> Backend {
        match self.db {
            Db::Sqlite(_) => Backend::Sqlite,
            Db::Postgres { .. } => Backend::Postgres,
        }
    }

    /// The PostgreSQL schema of the edit tables, `None` on SQLite.
    pub(crate) fn schema_edit(&self) -> Option<&str> {
        match &self.db {
            Db::Sqlite(_) => None,
            Db::Postgres { schema_edit, .. } => Some(schema_edit),
        }
    }

    /// The definition the edit tables currently follow, `None` before the first reload.
    pub async fn active_definition(&self) -> Option<Arc<ActiveDefinition>> {
        self.active.read().await.clone()
    }

    /// Starts a write transaction. SQLite: `BEGIN IMMEDIATE`.
    pub(crate) async fn begin_write(&self) -> Result<WriteTx<'_>, StoreError> {
        sql_exit::begin_write(&self.db).await
    }
}
