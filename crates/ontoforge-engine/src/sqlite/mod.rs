//! SQLite stand-in engine: one connection, in-memory.

use std::collections::BTreeMap;

use crate::{
    Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext,
    QueryOutput, TableRef,
};

/// SQLite implementation of [`Engine`].
pub struct SqliteEngine {
    tables: BTreeMap<TableRef, Vec<PhysicalColumn>>,
}

impl SqliteEngine {
    /// Opens an empty in-memory engine.
    pub async fn in_memory() -> Result<Self, EngineError> {
        Ok(Self {
            tables: BTreeMap::new(),
        })
    }
}

impl Engine for SqliteEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    async fn execute(
        &self,
        _sql: &PrintedSql,
        _ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        Ok(QueryOutput {
            columns: Vec::new(),
            rows: Vec::new(),
            truncated: false,
        })
    }

    async fn describe_tables(
        &self,
        _tables: &[TableRef],
        _ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        Ok(self.tables.clone())
    }

    async fn status(&self) -> EngineStatus {
        EngineStatus::Unavailable {
            reason: String::new(),
        }
    }
}
