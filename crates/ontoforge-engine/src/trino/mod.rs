//! Trino client: the statement protocol over `reqwest` (ENGINE-01, ADR-0017).

use std::collections::BTreeMap;
use std::time::Duration;

use secrecy::SecretString;

use crate::{
    Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext,
    QueryOutput, TableRef,
};

/// How a service identity authenticates.
pub enum Auth {
    None,
    Basic { password: SecretString },
    Bearer { token: SecretString },
}

/// Everything needed to talk to one Trino coordinator as one service identity.
pub struct TrinoConnection {
    pub url: reqwest::Url,
    pub user: String,
    pub auth: Auth,
    pub ca_pem: Option<Vec<u8>>,
    pub connect_timeout: Duration,
    pub max_page_bytes: usize,
    pub extra_origins: Vec<reqwest::Url>,
}

/// Trino implementation of [`Engine`].
pub struct TrinoEngine {
    _conn: TrinoConnection,
}

impl TrinoEngine {
    pub fn new(conn: TrinoConnection) -> Result<Self, EngineError> {
        Ok(Self { _conn: conn })
    }
}

impl Engine for TrinoEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Trino
    }

    async fn execute(
        &self,
        _sql: &PrintedSql,
        _ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        Err(EngineError::Cancelled)
    }

    async fn describe_tables(
        &self,
        _tables: &[TableRef],
        _ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        Err(EngineError::Config("not implemented".to_owned()))
    }

    async fn status(&self) -> EngineStatus {
        EngineStatus::Unavailable {
            reason: "not implemented".to_owned(),
        }
    }
}
