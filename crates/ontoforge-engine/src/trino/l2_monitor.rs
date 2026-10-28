//! Explicit lifecycle for deployment-owned L2 ACL attestation.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::{Method, RequestBuilder};
use tokio::task::JoinHandle;

use super::TrinoEngine;
use super::l2_health::{self, L2DisableReason, L2HealthStatus};
use crate::printer::l2::{L2Policy, acl_query_probe, validate_l2};
use crate::{EngineError, PrintedSql, QueryContext};

#[derive(Debug, Clone, Copy)]
pub struct L2MonitorOptions {
    /// Must be nonzero and at most 30 seconds, below the 60 second attestation TTL.
    pub interval: Duration,
    pub check_timeout: Duration,
}

impl Default for L2MonitorOptions {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(20),
        }
    }
}

pub struct L2AclMonitor {
    l2: Arc<TrinoEngine>,
    generation: u64,
    pending: Arc<PendingProbe>,
    task: Option<JoinHandle<()>>,
}

impl L2AclMonitor {
    /// Immediately disables admission, joins the monitor and awaits bounded cleanup.
    pub async fn shutdown(mut self) -> Result<(), EngineError> {
        self.stop();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.pending
            .cancel(Some(self.generation))
            .await
            .and_then(|()| self.pending.confirm_cleared_through(self.generation))
            .map_err(|_| EngineError::Unavailable {
                reason: "L2 probe cleanup failed".to_owned(),
                retryable: true,
            })
    }

    fn stop(&self) {
        if let Some(health) = &self.l2.l2_health {
            health.stop(self.generation);
        }
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

impl Drop for L2AclMonitor {
    fn drop(&mut self) {
        self.stop();
        let pending = self.pending.clone();
        let generation = self.generation;
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = pending.cancel(Some(generation)).await;
            });
        }
    }
}

impl TrinoEngine {
    /// Awaits the first attestation and returns the lease required to keep L2 enabled.
    /// Transient attestation failures return a disabled monitor that retries each interval.
    /// The host must retain this handle and call `shutdown` during graceful shutdown.
    /// A dropped startup future or handle disables L2 and cancels the active probe.
    ///
    /// ```ignore
    /// let ontology = Arc::new(engines.ontology);
    /// let l2 = Arc::new(engines.l2.expect("configured L2"));
    /// let monitor = l2.clone().start_l2_acl_monitor(ontology).await?;
    /// // Serve requests using these same engine instances.
    /// monitor.shutdown().await?;
    /// ```
    pub async fn start_l2_acl_monitor(
        self: Arc<Self>,
        ontology: Arc<Self>,
    ) -> Result<L2AclMonitor, EngineError> {
        self.start_l2_acl_monitor_with_options(ontology, L2MonitorOptions::default())
            .await
    }

    pub async fn start_l2_acl_monitor_with_options(
        self: Arc<Self>,
        ontology: Arc<Self>,
        options: L2MonitorOptions,
    ) -> Result<L2AclMonitor, EngineError> {
        if options.interval.is_zero()
            || options.interval > Duration::from_secs(30)
            || options.check_timeout.is_zero()
            || options.check_timeout > Duration::from_secs(20)
        {
            return Err(EngineError::Config(
                "invalid L2 ACL monitor bounds".to_owned(),
            ));
        }
        let health = self
            .l2_health
            .as_ref()
            .ok_or_else(|| EngineError::Config("not an L2 engine".to_owned()))?;
        let generation = health.register_monitor()?;
        let pending = health.pending.clone();
        // Construct the lease before the first await so startup cancellation runs Drop.
        let mut monitor = L2AclMonitor {
            l2: self.clone(),
            generation,
            pending: pending.clone(),
            task: None,
        };
        run_cycle(&self, &ontology, generation, &pending, options).await;
        monitor.task = Some(tokio::spawn(async move {
            let _exit = MonitorExit {
                l2: self.clone(),
                generation,
            };
            loop {
                tokio::time::sleep(options.interval).await;
                run_cycle(&self, &ontology, generation, &pending, options).await;
            }
        }));
        Ok(monitor)
    }
}

struct MonitorExit {
    l2: Arc<TrinoEngine>,
    generation: u64,
}
impl Drop for MonitorExit {
    fn drop(&mut self) {
        if let Some(health) = &self.l2.l2_health {
            health.stop(self.generation);
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct PendingProbe {
    state: Mutex<PendingState>,
    cancellation: tokio::sync::Mutex<()>,
    last_id: Mutex<Option<String>>,
}

#[derive(Debug, Default)]
struct PendingState {
    next_token: u64,
    delete: Option<PendingDelete>,
}

#[derive(Debug)]
struct PendingDelete {
    token: u64,
    generation: u64,
    request: RequestBuilder,
}

impl PendingProbe {
    fn confirm_cleared_through(&self, generation: u64) -> Result<(), L2DisableReason> {
        if self
            .state
            .lock()
            .map_err(|_| L2DisableReason::CleanupFailed)?
            .delete
            .as_ref()
            .is_some_and(|delete| delete.generation <= generation)
        {
            Err(L2DisableReason::CleanupFailed)
        } else {
            Ok(())
        }
    }

    fn record(&self, generation: u64, request: RequestBuilder) -> Result<(), L2DisableReason> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| L2DisableReason::CleanupFailed)?;
        state.next_token = state.next_token.wrapping_add(1);
        state.delete = Some(PendingDelete {
            token: state.next_token,
            generation,
            request,
        });
        Ok(())
    }

    async fn cancel(&self, owner: Option<u64>) -> Result<(), L2DisableReason> {
        tokio::time::timeout(Duration::from_secs(2), self.cancel_inner(owner))
            .await
            .map_err(|_| L2DisableReason::CleanupFailed)?
    }

    async fn cancel_inner(&self, owner: Option<u64>) -> Result<(), L2DisableReason> {
        let _serial = self.cancellation.lock().await;
        let delete = self
            .state
            .lock()
            .map_err(|_| L2DisableReason::CleanupFailed)?
            .delete
            .as_ref()
            .filter(|delete| owner.is_none_or(|owner| owner == delete.generation))
            .map(|delete| {
                delete
                    .request
                    .try_clone()
                    .map(|request| (delete.token, request))
                    .ok_or(L2DisableReason::CleanupFailed)
            })
            .transpose()?;
        let Some((token, delete)) = delete else {
            return Ok(());
        };
        match delete.send().await {
            Ok(response) if response.status().is_success() || response.status().as_u16() == 404 => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| L2DisableReason::CleanupFailed)?;
                if state
                    .delete
                    .as_ref()
                    .is_some_and(|delete| delete.token == token)
                {
                    state.delete = None;
                }
                Ok(())
            }
            _ => Err(L2DisableReason::CleanupFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn old_cleanup_response_and_owner_cannot_clear_or_cancel_new_probe() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/old"))
            .respond_with(ResponseTemplate::new(204).set_delay(Duration::from_millis(150)))
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/new"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let pending = Arc::new(PendingProbe::default());
        let client = reqwest::Client::new();
        pending
            .record(1, client.delete(format!("{}/old", server.uri())))
            .expect("old");
        let old_pending = pending.clone();
        let old = tokio::spawn(async move { old_pending.cancel(Some(1)).await });
        tokio::time::timeout(Duration::from_secs(1), async {
            while server
                .received_requests()
                .await
                .expect("requests")
                .is_empty()
            {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("old DELETE in flight");
        assert!(
            !old.is_finished(),
            "old response must still be pending before replacing token"
        );
        pending
            .record(2, client.delete(format!("{}/new", server.uri())))
            .expect("new");
        old.await.expect("old join").expect("old cleanup");
        assert!(
            pending.confirm_cleared_through(2).is_err(),
            "old response must preserve the new token"
        );
        pending.cancel(Some(1)).await.expect("old owner ignored");
        assert_eq!(
            server.received_requests().await.expect("requests").len(),
            1,
            "old Drop must not DELETE the new probe"
        );
        pending.cancel(Some(2)).await.expect("new cleanup");
        pending.confirm_cleared_through(2).expect("cleared");
        assert_eq!(server.received_requests().await.expect("requests").len(), 2);
    }
}

async fn run_cycle(
    l2: &TrinoEngine,
    ontology: &TrinoEngine,
    generation: u64,
    pending: &PendingProbe,
    options: L2MonitorOptions,
) {
    let Some(health) = &l2.l2_health else {
        return;
    };
    let Some(ticket) = health.begin(Some(generation)) else {
        return;
    };
    // The deadline starts before either lock; emergency cancellation has its own 2s bound.
    let result = tokio::time::timeout(options.check_timeout, async {
        let _serial = health.recheck_lock.lock().await;
        let _admission = health.admission.write().await;
        let checks = async {
            pending.cancel(None).await?;
            let sql = probe_sql(l2, ontology)?;
            let query_id = start_probe(ontology, &sql, pending, generation).await?;
            l2_health::run_checks(l2, ontology, &query_id).await
        }
        .await;
        let cleanup = pending.cancel(Some(generation)).await;
        let status = match cleanup.and(checks) {
            Ok(()) => L2HealthStatus::Enabled,
            Err(reason) => L2HealthStatus::Disabled { reason },
        };
        health.complete(ticket, status);
    })
    .await;
    if result.is_err() {
        let reason = match pending.cancel(Some(generation)).await {
            Ok(()) => L2DisableReason::TimedOut,
            Err(reason) => reason,
        };
        health.complete(ticket, L2HealthStatus::Disabled { reason });
    }
}

fn probe_sql(l2: &TrinoEngine, ontology: &TrinoEngine) -> Result<PrintedSql, L2DisableReason> {
    if l2.conn.user == ontology.conn.user
        || l2.statement_url != ontology.statement_url
        || ontology.l2_health.is_some()
    {
        return Err(L2DisableReason::InvalidProbe);
    }
    let config = l2
        .l2_health
        .as_ref()
        .and_then(|health| health.config.as_ref())
        .ok_or(L2DisableReason::NotConfigured)?;
    let raw = config
        .active_query_sql
        .as_deref()
        .ok_or(L2DisableReason::NotConfigured)?;
    validate_l2(
        raw,
        &L2Policy {
            raw_allowed: true,
            catalogs: BTreeSet::from([config.allowed.catalog.clone()]),
            schemas: BTreeSet::new(),
            edit_catalog: config.edit.catalog.clone(),
            max_rows: 1,
        },
    )
    .map_err(|_| L2DisableReason::InvalidProbe)
}

async fn start_probe(
    ontology: &TrinoEngine,
    sql: &PrintedSql,
    pending: &PendingProbe,
    generation: u64,
) -> Result<String, L2DisableReason> {
    let ctx = QueryContext {
        timeout: Duration::from_secs(20),
        trace_token: "ontoforge-l2-active-probe".to_owned(),
        ..l2_health::probe_context()
    };
    // Only this deployment-owned probe needs a queued-time limit in addition to execution.
    let mut page = super::client::send(
        || {
            let mut request =
                ontology.request(Method::POST, ontology.statement_url.clone(), &ctx, false)?;
            request = request.header(
                "X-Trino-Session",
                "query_max_execution_time=25s,query_max_run_time=25s",
            );
            Ok(request.body(sql.as_str().to_owned()))
        },
        true,
        ontology.conn.max_page_bytes,
    )
    .await
    .map_err(|e| l2_health::failure(&e, L2DisableReason::OtherQueryUnconfirmed))?;
    let mut query_id = None;
    loop {
        let next = page
            .next_uri
            .as_deref()
            .map(|uri| ontology.checked_next_uri(uri))
            .transpose()
            .map_err(|_| L2DisableReason::InvalidProbe)?;
        if let Some(next) = &next {
            let delete = ontology
                .request(Method::DELETE, next.clone(), &ctx, false)
                .map_err(|_| L2DisableReason::InvalidProbe)?;
            pending.record(generation, delete)?;
        }
        let id = page
            .id
            .as_deref()
            .ok_or(L2DisableReason::OtherQueryUnconfirmed)?;
        acl_query_probe(id).map_err(|_| L2DisableReason::InvalidProbe)?;
        if let Some(expected) = &query_id {
            if expected != id {
                return Err(L2DisableReason::OtherQueryUnconfirmed);
            }
        } else {
            let mut last = pending
                .last_id
                .lock()
                .map_err(|_| L2DisableReason::OtherQueryUnconfirmed)?;
            if last.as_deref() == Some(id) {
                return Err(L2DisableReason::OtherQueryUnconfirmed);
            }
            *last = Some(id.to_owned());
            query_id = Some(id.to_owned());
        }
        let state = page.stats.as_ref().and_then(|stats| stats.state.as_deref());
        let next = next.ok_or(L2DisableReason::OtherQueryUnconfirmed)?;
        match state {
            Some("RUNNING") => return query_id.ok_or(L2DisableReason::OtherQueryUnconfirmed),
            Some("QUEUED" | "WAITING_FOR_RESOURCES" | "DISPATCHING" | "PLANNING" | "STARTING") => {}
            _ => return Err(L2DisableReason::OtherQueryUnconfirmed),
        }
        page = super::client::send(
            || ontology.request(Method::GET, next.clone(), &ctx, false),
            false,
            ontology.conn.max_page_bytes,
        )
        .await
        .map_err(|e| l2_health::failure(&e, L2DisableReason::OtherQueryUnconfirmed))?;
    }
}
