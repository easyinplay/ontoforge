use crate::{ExecutionLimits, ListPage, QueryError, QueryService};
use crate::compile::ValidatedQuery;
use ontoforge_engine::Engine;

pub struct WorkBudget {
    limits: ExecutionLimits,
    backend_calls: usize,
    raw_candidates: usize,
    backfill_keys: usize,
    decoded_bytes: usize,
}

impl WorkBudget {
    pub fn new(limits: ExecutionLimits) -> Self {
        Self {
            limits,
            backend_calls: 0,
            raw_candidates: 0,
            backfill_keys: 0,
            decoded_bytes: 0,
        }
    }

    pub fn charge_backend_calls(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.backend_calls,
            count,
            self.limits.max_backend_calls,
        )
    }
    pub fn charge_raw_candidates(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.raw_candidates,
            count,
            self.limits.max_raw_candidates,
        )
    }
    pub fn charge_backfill_keys(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.backfill_keys,
            count,
            self.limits.max_backfill_keys,
        )
    }
    pub fn charge_decoded_bytes(&mut self, count: usize) -> Result<(), QueryError> {
        charge(
            &mut self.decoded_bytes,
            count,
            self.limits.max_decoded_bytes,
        )
    }
}

fn charge(current: &mut usize, count: usize, limit: usize) -> Result<(), QueryError> {
    let next = current
        .checked_add(count)
        .ok_or(QueryError::WorkBudgetExceeded)?;
    if next > limit {
        return Err(QueryError::WorkBudgetExceeded);
    }
    *current = next;
    Ok(())
}

impl<E: Engine> QueryService<'_, E> {
    pub async fn execute_list(&self, _query: &ValidatedQuery) -> Result<ListPage, QueryError> {
        Err(QueryError::InvalidQuery)
    }
}
