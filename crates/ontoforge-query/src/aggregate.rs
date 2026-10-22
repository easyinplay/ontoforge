use serde::Serialize;
use serde_json::Value;

use crate::QueryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateLimits {
    pub max_groups: usize,
    pub max_buckets: usize,
    pub max_distinct_values: usize,
    pub max_distinct_memory_bytes: usize,
}

impl AggregateLimits {
    #[must_use]
    pub fn phase4_default() -> Self {
        Self {
            max_groups: 1_024,
            max_buckets: 4_096,
            max_distinct_values: 10_000,
            max_distinct_memory_bytes: 16 * 1024 * 1024,
        }
    }

    pub fn try_new(
        max_groups: usize,
        max_buckets: usize,
        max_distinct_values: usize,
        max_distinct_memory_bytes: usize,
    ) -> Result<Self, QueryError> {
        if [max_groups, max_buckets, max_distinct_values, max_distinct_memory_bytes]
            .iter()
            .any(|&value| value == 0 || i64::try_from(value).is_err())
        {
            return Err(QueryError::InvalidQuery);
        }
        Ok(Self { max_groups, max_buckets, max_distinct_values, max_distinct_memory_bytes })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateCapabilities {
    pub count: bool,
    pub sum_integer: bool,
    pub sum_double: bool,
    pub min_max: bool,
    pub distinct: bool,
    pub group_by: bool,
    pub case_when: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateBucket {
    pub group: Vec<Value>,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateResult {
    pub buckets: Vec<AggregateBucket>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum AggregateError {
    #[error("invalid aggregate query")]
    InvalidQuery,
    #[error("aggregate operation is unsupported")]
    Unsupported,
    #[error("aggregate cost limit exceeded")]
    AggregateCostExceeded,
    #[error("aggregate source unavailable")]
    Source,
    #[error("aggregate edit store unavailable")]
    Store,
    #[error("unsupported numeric value")]
    UnsupportedNumericValue,
}
