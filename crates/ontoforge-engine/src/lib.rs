//! Query engine abstraction: the [`Engine`] trait, the typed SQL printer and the SQLite
//! stand-in (ADR-0021).
//!
//! SQL text can only reach an engine as a [`PrintedSql`], and only the [`printer`] module can
//! make one.

use std::collections::BTreeMap;
use std::time::Duration;

pub mod error;
pub mod printer;
pub mod sqlite;
pub mod value;

pub use error::{EngineError, PrintError};
pub use printer::{Dialect, PrintedSql};
pub use value::{Column, PhysicalColumn, PhysicalType, Value};

/// Default per query timeout in milliseconds. The single definition of this default; the CLI
/// and the server configuration both take it from here.
pub const DEFAULT_QUERY_TIMEOUT_MS: u64 = 30_000;
/// Default maximum number of rows a query may return. See [`DEFAULT_QUERY_TIMEOUT_MS`].
pub const DEFAULT_MAX_ROWS: usize = 10_000;
/// Default maximum number of result bytes a query may return. See
/// [`DEFAULT_QUERY_TIMEOUT_MS`].
pub const DEFAULT_MAX_BYTES: usize = 16 * 1024 * 1024;

/// A physical table: `catalog.schema.table`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableRef {
    pub catalog: String,
    pub schema: String,
    pub table: String,
}

impl TableRef {
    /// Name of the SQLite database a table lives in: `{catalog}__{schema}`.
    #[must_use]
    pub fn sqlite_schema_name(&self) -> String {
        format!("{}__{}", self.catalog, self.schema)
    }
}

/// Per request settings.
#[derive(Debug, Clone)]
pub struct QueryContext {
    pub trace_token: String,
    pub client_tags: Vec<String>,
    pub timeout: Duration,
    pub max_rows: usize,
    pub max_bytes: usize,
}

impl QueryContext {
    /// A context with the `DEFAULT_*` limits.
    #[must_use]
    pub fn new(trace_token: String, client_tags: Vec<String>) -> Self {
        Self {
            trace_token,
            client_tags,
            timeout: Duration::ZERO,
            max_rows: 0,
            max_bytes: 0,
        }
    }
}

/// The result of a query.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryOutput {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
    /// The engine stopped reading because a row or byte limit was reached.
    pub truncated: bool,
}

/// Result of an availability probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    Available,
    Unavailable { reason: String },
}

/// A query engine.
pub trait Engine {
    fn dialect(&self) -> Dialect;

    fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> impl Future<Output = Result<QueryOutput, EngineError>> + Send;

    /// Describes several tables of one catalog at once.
    ///
    /// A table missing from the returned map does not exist. `tables` spanning more than one
    /// catalog is an [`EngineError::Config`]; an empty slice gives an empty map.
    fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> impl Future<Output = Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError>> + Send;

    fn status(&self) -> impl Future<Output = EngineStatus> + Send;
}

/// Runtime selected engine.
pub enum AnyEngine {
    Sqlite(sqlite::SqliteEngine),
}

impl Engine for AnyEngine {
    fn dialect(&self) -> Dialect {
        match self {
            Self::Sqlite(e) => e.dialect(),
        }
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        match self {
            Self::Sqlite(e) => e.execute(sql, ctx).await,
        }
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        match self {
            Self::Sqlite(e) => e.describe_tables(tables, ctx).await,
        }
    }

    async fn status(&self) -> EngineStatus {
        match self {
            Self::Sqlite(e) => e.status().await,
        }
    }
}
