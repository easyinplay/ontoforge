use crate::{ExecutionLimits, QueryError};

pub struct WorkBudget {
    limits: ExecutionLimits,
}

impl WorkBudget {
    pub fn new(limits: ExecutionLimits) -> Self {
        Self { limits }
    }

    pub fn charge_backend_calls(&mut self, _count: usize) -> Result<(), QueryError> { Ok(()) }
    pub fn charge_raw_candidates(&mut self, _count: usize) -> Result<(), QueryError> { Ok(()) }
    pub fn charge_backfill_keys(&mut self, _count: usize) -> Result<(), QueryError> { Ok(()) }
    pub fn charge_decoded_bytes(&mut self, _count: usize) -> Result<(), QueryError> { Ok(()) }
}
