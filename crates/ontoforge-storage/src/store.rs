//! The edit store: settings, connection and the active definition (ADR-0016, ADR-0024).
//!
//! One `EditStore` serves one backend, SQLite or PostgreSQL. On PostgreSQL the typed edit tables
//! live in `schema_edit` and everything else (definitions, migration bookkeeping) in `schema_sys`;
//! every connection has `schema_sys` as its `search_path`, so system tables are addressed by
//! bare name and edit tables always by qualified name.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_engine::config::{StoreConfig, StoreKind, resolve_secret};
use ontoforge_engine::sqlite::{
    DECIMAL_COLLATION, TIMESTAMP_COLLATION, compare_decimal, compare_timestamp,
};
use ontoforge_schema::ir::Ontology;
use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{PgPool, Postgres, Sqlite, SqlitePool};
use tokio::sync::RwLock;

use crate::error::StoreError;
use crate::naming;
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

/// The PostgreSQL advisory lock key that `reload`, `migrate` and `scope apply` of the edit store
/// whose edit tables live in `schema_edit` take, so that they never run at the same time.
///
/// Advisory locks belong to the database, not to a schema. The key therefore depends on the
/// schema: two deployments in one database, or two test runs in one server, do not wait for each
/// other's definition changes.
#[must_use]
pub fn definition_lock_key(schema_edit: &str) -> i64 {
    let hash =
        u64::from_str_radix(&naming::fnv1a64_hex(schema_edit.as_bytes()), 16).unwrap_or_default();
    i64::from_ne_bytes(hash.to_ne_bytes()) ^ sql_exit::DEFINITION_LOCK_BASE
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
pub(crate) enum Db {
    Sqlite(SqlitePool),
    Postgres { pool: PgPool, schema_edit: String },
}

/// A write transaction. SQLite takes the write lock at `BEGIN`.
pub(crate) enum WriteTx<'c> {
    Sqlite(sqlx::Transaction<'c, Sqlite>),
    Postgres {
        tx: sqlx::Transaction<'c, Postgres>,
        schema_edit: String,
    },
}

impl WriteTx<'_> {
    pub(crate) fn backend(&self) -> Backend {
        match self {
            Self::Sqlite(_) => Backend::Sqlite,
            Self::Postgres { .. } => Backend::Postgres,
        }
    }

    /// The PostgreSQL schema of the edit tables, `None` on SQLite.
    pub(crate) fn schema_edit(&self) -> Option<&str> {
        match self {
            Self::Sqlite(_) => None,
            Self::Postgres { schema_edit, .. } => Some(schema_edit),
        }
    }
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
                    .map_err(|e| connect_error(&e))?;
                sqlx::migrate!("./migrations/sqlite")
                    .run(&pool)
                    .await
                    .map_err(|e| StoreError::Migrate(e.to_string()))?;
                Db::Sqlite(pool)
            }
            BackendSettings::Postgres {
                url,
                schema_edit,
                schema_sys,
            } => open_postgres(url, schema_edit, schema_sys).await?,
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

    /// Keeps `def` as the active definition unless a newer one is held already. A switch and the
    /// commit fence both call this, in whatever order their tasks run.
    pub(crate) async fn install_if_newer(&self, def: &Arc<ActiveDefinition>) {
        let mut slot = self.active.write().await;
        if slot
            .as_ref()
            .is_none_or(|current| current.def_version < def.def_version)
        {
            *slot = Some(Arc::clone(def));
        }
    }

    /// Starts a write transaction. SQLite: `BEGIN IMMEDIATE`.
    pub(crate) async fn begin_write(&self) -> Result<WriteTx<'_>, StoreError> {
        sql_exit::begin_write(&self.db).await
    }
}

/// Connection options of the PostgreSQL edit store: the connection string `url` with
/// `schema_sys` as the `search_path` of every connection, and the session settings that the text
/// read-back depends on pinned:
///
/// - `DateStyle = ISO, MDY`: `date` and `date[]` come back as `YYYY-MM-DD`,
/// - `extra_float_digits = 3`: `double precision` and `real` arrays come back with the shortest
///   text that round-trips (3 does that on PostgreSQL 12 and later, and is exact for `float4`
///   on older servers),
/// - `TimeZone = UTC`: a fallback, timestamp reads already say `AT TIME ZONE 'UTC'`.
///
/// They are sent in the startup packet, which wins over `ALTER ROLE` and `ALTER DATABASE`
/// defaults. sqlx 0.9.0 already sends `DateStyle`, `TimeZone` and `extra_float_digits = 2`; they
/// are set here so the store does not depend on sqlx's defaults. They go both through `options`
/// (`-c`) and through the dedicated startup parameter, so whichever PostgreSQL applies last
/// carries the same value.
///
/// The connection string is a secret: it is parsed here and never appears in an error.
pub fn pg_connect_options(
    url: &SecretString,
    schema_sys: &str,
) -> Result<PgConnectOptions, StoreError> {
    Ok(PgConnectOptions::from_str(url.expose_secret())
        .map_err(|_| StoreError::Config("the PostgreSQL connection string is invalid".to_owned()))?
        .options([
            ("search_path", schema_sys),
            ("DateStyle", "ISO, MDY"),
            ("TimeZone", "UTC"),
            ("extra_float_digits", "3"),
        ])
        .extra_float_digits(3))
}

/// Connects to PostgreSQL, creates the two schemas and runs the system migrations.
///
/// The connection string is a secret: it is parsed here and never appears in an error.
async fn open_postgres(
    url: &SecretString,
    schema_edit: &str,
    schema_sys: &str,
) -> Result<Db, StoreError> {
    let options = pg_connect_options(url, schema_sys)?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| connect_error(&e))?;
    for schema in [schema_sys, schema_edit] {
        sql_exit::create_schema(&pool, schema).await?;
    }
    sqlx::migrate!("./migrations/postgres")
        .run(&pool)
        .await
        .map_err(|e| StoreError::Migrate(e.to_string()))?;
    Ok(Db::Postgres {
        pool,
        schema_edit: schema_edit.to_owned(),
    })
}

/// A connect failure without anything that could echo the connection string.
fn connect_error(error: &sqlx::Error) -> StoreError {
    match error {
        sqlx::Error::Configuration(_) => {
            StoreError::Connect("invalid connection settings".to_owned())
        }
        other => StoreError::Connect(other.to_string()),
    }
}
