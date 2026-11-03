//! Bounded shutdown ordering for the HTTP host.

use ontoforge_engine::Engine;
use ontoforge_engine::EngineError;
use std::time::Duration;

use crate::{ServerHost, context, state};

pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("server shutdown exceeded its deadline")]
    Timeout,
    #[error("read audit drain failed")]
    Audit,
    #[error("L2 monitor shutdown failed: {0}")]
    Monitor(EngineError),
    #[error("accepted action task failed")]
    Action,
    #[error("engine cancellation drain failed: {0}")]
    Cancellation(EngineError),
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
        self.shutdown_before(tokio::time::Instant::now() + SHUTDOWN_TIMEOUT)
            .await
    }

    pub(crate) async fn shutdown_before(
        self,
        deadline: tokio::time::Instant,
    ) -> Result<(), LifecycleError> {
        let server_state = state::for_host(self.host);
        context::stop_admission(self.host);
        let actions = tokio::time::timeout_at(
            deadline - Duration::from_secs(5),
            context::wait_actions(self.host),
        )
        .await
        .map_err(|_| LifecycleError::Timeout)
        .and_then(|result| result.map_err(|_| LifecycleError::Action));
        server_state.cancel_reads();
        let drained = tokio::time::timeout_at(deadline - Duration::from_secs(5), async {
            server_state.read_tasks.close();
            server_state.read_tasks.wait().await;
            Ok(())
        })
        .await
        .unwrap_or(Err(LifecycleError::Timeout));
        context::detach_actions(self.host);
        let audit = tokio::time::timeout_at(
            deadline - Duration::from_secs(4),
            self.host.store.read_audit().shutdown(),
        )
        .await
        .map_err(|_| LifecycleError::Timeout)
        .and_then(|result| result.map_err(|_| LifecycleError::Audit));
        let monitor = server_state.take_monitor();
        let monitor_cleanup = monitor
            .as_ref()
            .map(ontoforge_engine::trino::l2_monitor::L2AclMonitor::cleanup_handle);
        let monitor = if let Some(monitor) = monitor {
            tokio::time::timeout_at(deadline - Duration::from_secs(2), monitor.shutdown())
                .await
                .map_err(|_| LifecycleError::Timeout)
                .and_then(|result| result.map_err(LifecycleError::Monitor))
        } else {
            Ok(())
        };
        let l2 = server_state.l2_engine();
        let cancellations = tokio::time::timeout_at(deadline, async {
            let (core, l2, monitor) = tokio::join!(
                self.host.engine.drain_cancellations(),
                async {
                    if let Some(engine) = l2 {
                        engine.drain_cancellations().await
                    } else {
                        Ok(())
                    }
                },
                async {
                    if let Some(cleanup) = monitor_cleanup {
                        cleanup.drain().await
                    } else {
                        Ok(())
                    }
                }
            );
            core.and(l2).and(monitor)
        })
        .await
        .map_err(|_| LifecycleError::Timeout)
        .and_then(|result| result.map_err(LifecycleError::Cancellation));
        state::remove(self.host);
        actions
            .and(drained)
            .and(audit)
            .and(monitor)
            .and(cancellations)
    }
}
