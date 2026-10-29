//! Bounded shutdown ordering for the HTTP host.

use ontoforge_engine::Engine;
use ontoforge_engine::EngineError;
use std::time::Duration;

use crate::{ServerHost, context, state};

#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("server shutdown exceeded its deadline")]
    Timeout,
    #[error("read audit drain failed")]
    Audit,
    #[error("L2 monitor shutdown failed: {0}")]
    Monitor(EngineError),
}

pub struct ServerLifecycle<'a, E> {
    host: &'a ServerHost<E>,
}

impl<'a, E> ServerLifecycle<'a, E> {
    pub fn new(host: &'a ServerHost<E>) -> Self {
        Self { host }
    }
}

impl<E: Engine + Send + Sync + 'static> ServerLifecycle<'_, E> {
    pub async fn shutdown(self) -> Result<(), LifecycleError> {
        let server_state = state::for_host(self.host);
        server_state.stop_admission();
        tokio::time::timeout(Duration::from_secs(5), context::wait_actions(self.host))
            .await
            .map_err(|_| LifecycleError::Timeout)?;
        self.host
            .store
            .read_audit()
            .shutdown()
            .await
            .map_err(|_| LifecycleError::Audit)?;
        if let Some(monitor) = server_state.take_monitor() {
            monitor.shutdown().await.map_err(LifecycleError::Monitor)?;
        }
        state::remove(self.host);
        Ok(())
    }
}
