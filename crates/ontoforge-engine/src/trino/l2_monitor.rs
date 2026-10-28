//! Explicit lifecycle for deployment-owned L2 ACL attestation.

use std::sync::Arc;
use std::time::Duration;

use super::TrinoEngine;
use crate::EngineError;

#[derive(Debug, Clone, Copy)]
pub struct L2MonitorOptions {
    pub interval: Duration,
    pub check_timeout: Duration,
}

impl Default for L2MonitorOptions {
    fn default() -> Self {
        Self { interval: Duration::from_secs(30), check_timeout: Duration::from_secs(20) }
    }
}

pub struct L2AclMonitor {
    l2: Arc<TrinoEngine>,
    generation: u64,
}

impl L2AclMonitor {
    pub async fn shutdown(self) -> Result<(), EngineError> {
        Ok(())
    }
}

impl Drop for L2AclMonitor {
    fn drop(&mut self) {
        if let Some(health)=&self.l2.l2_health { health.stop(self.generation); }
    }
}

impl TrinoEngine {
    pub async fn start_l2_acl_monitor(self: Arc<Self>, ontology: Arc<Self>) -> Result<L2AclMonitor, EngineError> {
        self.start_l2_acl_monitor_with_options(ontology,L2MonitorOptions::default()).await
    }

    pub async fn start_l2_acl_monitor_with_options(self: Arc<Self>, _ontology: Arc<Self>, _options:L2MonitorOptions) -> Result<L2AclMonitor, EngineError> {
        let generation=self.l2_health.as_ref().ok_or_else(||EngineError::Config("not an L2 engine".to_owned()))?.register_monitor()?;
        Ok(L2AclMonitor{l2:self,generation})
    }
}
