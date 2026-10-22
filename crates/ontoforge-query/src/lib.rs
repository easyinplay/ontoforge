//! Scoped L0 reads with a single Rust source/edit overlay (ADR-0020).

use std::collections::BTreeMap;
use std::sync::Arc;

use ontoforge_auth::Principal;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::Engine;
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ObjectSet {
    Pk { object_type: ObjectTypeName, pk: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct QueryRequest {
    pub set: ObjectSet,
    pub select: Vec<PropertyName>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRow {
    pub object_type: ObjectTypeName,
    pub pk: Value,
    pub properties: BTreeMap<PropertyName, Value>,
    #[serde(rename = "_version")]
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueryPage {
    pub rows: Vec<ObjectRow>,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("object not found")]
    NotFound,
    #[error("object key is ambiguous")]
    AmbiguousObject,
    #[error("invalid query")]
    InvalidQuery,
    #[error("query cost limit exceeded")]
    CostExceeded,
    #[error("source unavailable")]
    Source,
    #[error("edit store unavailable")]
    Store,
    #[error("invalid version token")]
    InvalidToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLimits {
    pub max_backend_calls: usize,
    pub max_raw_candidates: usize,
    pub max_backfill_keys: usize,
    pub max_decoded_bytes: usize,
}

impl ExecutionLimits {
    #[must_use]
    pub fn phase4_default() -> Self {
        Self { max_backend_calls: 1, max_raw_candidates: 1, max_backfill_keys: 1, max_decoded_bytes: 1 }
    }

    pub fn try_new(
        max_backend_calls: usize,
        max_raw_candidates: usize,
        max_backfill_keys: usize,
        max_decoded_bytes: usize,
    ) -> Result<Self, QueryError> {
        if [max_backend_calls, max_raw_candidates, max_backfill_keys, max_decoded_bytes]
            .contains(&0)
        {
            return Err(QueryError::InvalidQuery);
        }
        Ok(Self { max_backend_calls, max_raw_candidates, max_backfill_keys, max_decoded_bytes })
    }
}

pub struct TokenCodec {
    key: Vec<u8>,
}

impl TokenCodec {
    pub fn new(key: Vec<u8>) -> Result<Self, QueryError> {
        if key.is_empty() { return Err(QueryError::InvalidQuery); }
        Ok(Self { key })
    }
}

pub struct QueryService<'a, E: Engine> {
    pub principal: &'a Principal,
    pub scope: &'a PrincipalScope,
    pub active: Arc<ActiveDefinition>,
    pub engine: &'a E,
    pub store: &'a EditStore,
    pub limits: ExecutionLimits,
    pub tokens: &'a TokenCodec,
}

impl<E: Engine> QueryService<'_, E> {
    pub async fn execute(&self, _request: QueryRequest) -> Result<QueryPage, QueryError> {
        Err(QueryError::NotFound)
    }
}
