//! Fail-closed ACL attestation for the dedicated L2 identity.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::TrinoEngine;
use crate::printer::l2::{ProbeTable, acl_query_probe, acl_table_probe};
use crate::{EngineError, QueryContext, QueryOutput, Value};

/// Explicit deployment declaration and actual relations used for ACL attestation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct L2AclConfig {
    /// Deployment-owned read-only workload that remains RUNNING during attestation.
    #[serde(default)]
    pub active_query_sql: Option<String>,
    pub allowed: ProbeTable,
    pub outside: ProbeTable,
    pub edit: ProbeTable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L2DisableReason {
    NotConfigured,
    NotVerified,
    InvalidProbe,
    ObjectUnconfirmed,
    AllowedProbeFailed,
    NegativeProbeFailed,
    OtherQueryUnconfirmed,
    QueryVisibilityFailed,
    TimedOut,
    Expired,
    Stopped,
    CleanupFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L2HealthStatus {
    Enabled,
    Disabled { reason: L2DisableReason },
}

#[derive(Debug)]
pub(super) struct L2Health {
    pub(super) config: Option<L2AclConfig>,
    state: Mutex<HealthState>,
    pub(super) recheck_lock: tokio::sync::Mutex<()>,
    pub(super) admission: tokio::sync::RwLock<()>,
}

#[derive(Debug)]
struct HealthState {
    status: L2HealthStatus,
    checked_at: Instant,
    generation: u64,
    check_epoch: u64,
    monitoring: bool,
    stopped: bool,
}

impl L2Health {
    pub(super) fn new(config: Option<L2AclConfig>) -> Self {
        let reason = if config.is_some() {
            L2DisableReason::NotVerified
        } else {
            L2DisableReason::NotConfigured
        };
        Self {
            config,
            state: Mutex::new(HealthState {
                status: L2HealthStatus::Disabled { reason },
                checked_at: Instant::now(),
                generation: 0,
                check_epoch: 0,
                monitoring: false,
                stopped: false,
            }),
            recheck_lock: tokio::sync::Mutex::new(()),
            admission: tokio::sync::RwLock::new(()),
        }
    }

    pub(super) fn status(&self) -> L2HealthStatus {
        self.state.lock().map_or(
            L2HealthStatus::Disabled {
                reason: L2DisableReason::NotVerified,
            },
            |guard| {
                if guard.status == L2HealthStatus::Enabled
                    && guard.checked_at.elapsed() >= Duration::from_secs(60)
                {
                    L2HealthStatus::Disabled {
                        reason: L2DisableReason::Expired,
                    }
                } else {
                    guard.status
                }
            },
        )
    }

    pub(super) fn complete(&self, ticket: (u64, u64), status: L2HealthStatus) -> L2HealthStatus {
        match self.state.lock() {
            Ok(mut current) => {
                if (current.generation, current.check_epoch) == ticket && !current.stopped {
                    current.status = status;
                    current.checked_at = Instant::now();
                }
                current.status
            }
            Err(_) => L2HealthStatus::Disabled {
                reason: L2DisableReason::NotVerified,
            },
        }
    }

    pub(super) fn begin(&self, owner: Option<u64>) -> Option<(u64, u64)> {
        let mut current = self.state.lock().ok()?;
        if current.stopped || owner.is_some_and(|owner| owner != current.generation) {
            return None;
        }
        current.check_epoch = current.check_epoch.wrapping_add(1);
        current.status = L2HealthStatus::Disabled {
            reason: L2DisableReason::NotVerified,
        };
        Some((current.generation, current.check_epoch))
    }

    pub(super) fn register_monitor(&self) -> Result<u64, EngineError> {
        let mut current = self
            .state
            .lock()
            .map_err(|_| EngineError::Config("L2 health unavailable".to_owned()))?;
        if current.monitoring {
            return Err(EngineError::Config(
                "L2 ACL monitor already running".to_owned(),
            ));
        }
        current.generation = current.generation.wrapping_add(1);
        current.monitoring = true;
        current.stopped = false;
        current.status = L2HealthStatus::Disabled {
            reason: L2DisableReason::NotVerified,
        };
        Ok(current.generation)
    }

    pub(super) fn stop(&self, generation: u64) {
        if let Ok(mut current) = self.state.lock()
            && current.generation == generation
        {
            current.stopped = true;
            current.monitoring = false;
            current.status = L2HealthStatus::Disabled {
                reason: L2DisableReason::Stopped,
            };
        }
    }
}

pub(super) fn probe_context() -> QueryContext {
    QueryContext {
        trace_token: "ontoforge-l2-acl-check".to_owned(),
        client_tags: vec!["ontoforge".to_owned()],
        timeout: Duration::from_secs(5),
        max_rows: 2,
        max_bytes: 4096,
    }
}

pub(super) fn failure(error: &EngineError, otherwise: L2DisableReason) -> L2DisableReason {
    if matches!(error, EngineError::Timeout { .. }) {
        L2DisableReason::TimedOut
    } else {
        otherwise
    }
}

fn confirmed(output: &QueryOutput) -> bool {
    !output.truncated && output.rows.len() == 1
}

/// The negative probes are never filtered by the application allow-list. The ontology identity
/// first establishes that their physical relations exist, so missing objects cannot pass.
pub(super) async fn recheck(
    l2: &TrinoEngine,
    ontology: &TrinoEngine,
    other_query_id: &str,
) -> L2HealthStatus {
    let Some(health) = &l2.l2_health else {
        return L2HealthStatus::Disabled {
            reason: L2DisableReason::NotConfigured,
        };
    };
    let Some(ticket) = health.begin(None) else {
        return health.status();
    };
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let _recheck = health.recheck_lock.lock().await;
        let _admission = health.admission.write().await;
        let result = run_checks(l2, ontology, other_query_id).await;
        health.complete(
            ticket,
            match result {
                Ok(()) => L2HealthStatus::Enabled,
                Err(reason) => L2HealthStatus::Disabled { reason },
            },
        )
    })
    .await;
    let status = match result {
        Ok(status) => return status,
        Err(_) => L2HealthStatus::Disabled {
            reason: L2DisableReason::TimedOut,
        },
    };
    health.complete(ticket, status)
}

pub(super) async fn run_checks(
    l2: &TrinoEngine,
    ontology: &TrinoEngine,
    other_query_id: &str,
) -> Result<(), L2DisableReason> {
    let health = l2
        .l2_health
        .as_ref()
        .ok_or(L2DisableReason::NotConfigured)?;
    let Some(config) = &health.config else {
        return Err(L2DisableReason::NotConfigured);
    };
    if l2.conn.user == ontology.conn.user
        || l2.statement_url != ontology.statement_url
        || config.allowed == config.outside
        || config.allowed == config.edit
        || config.outside == config.edit
        || config.allowed.catalog == config.outside.catalog
        || config.allowed.catalog == config.edit.catalog
    {
        return Err(L2DisableReason::InvalidProbe);
    }
    let Ok(queries) = acl_query_probe(other_query_id) else {
        return Err(L2DisableReason::InvalidProbe);
    };
    let Ok(allowed) = acl_table_probe(&config.allowed) else {
        return Err(L2DisableReason::InvalidProbe);
    };
    let Ok(outside) = acl_table_probe(&config.outside) else {
        return Err(L2DisableReason::InvalidProbe);
    };
    let Ok(edit) = acl_table_probe(&config.edit) else {
        return Err(L2DisableReason::InvalidProbe);
    };
    let ctx = probe_context();
    for relation in [&allowed, &outside, &edit] {
        match ontology.execute_unchecked(relation, &ctx).await {
            Ok(output) if confirmed(&output) => {}
            Err(error) => {
                return Err(failure(&error, L2DisableReason::ObjectUnconfirmed));
            }
            _ => return Err(L2DisableReason::ObjectUnconfirmed),
        }
    }
    match l2.execute_unchecked(&allowed, &ctx).await {
        Ok(output) if confirmed(&output) => {}
        Err(error) => return Err(failure(&error, L2DisableReason::AllowedProbeFailed)),
        _ => return Err(L2DisableReason::AllowedProbeFailed),
    }
    for relation in [&outside, &edit] {
        match l2.execute_unchecked(relation, &ctx).await {
            Err(EngineError::Query { error_name, .. }) if error_name == "PERMISSION_DENIED" => {}
            Err(error) => {
                return Err(failure(&error, L2DisableReason::NegativeProbeFailed));
            }
            _ => return Err(L2DisableReason::NegativeProbeFailed),
        }
    }
    confirm_running(ontology, &queries, other_query_id, &ctx).await?;
    match l2.execute_unchecked(&queries, &ctx).await {
        Ok(output) if !output.truncated && output.rows.is_empty() => {}
        Err(error) => return Err(failure(&error, L2DisableReason::QueryVisibilityFailed)),
        _ => return Err(L2DisableReason::QueryVisibilityFailed),
    }
    confirm_running(ontology, &queries, other_query_id, &ctx).await
}

async fn confirm_running(
    ontology: &TrinoEngine,
    queries: &crate::PrintedSql,
    other_query_id: &str,
    ctx: &QueryContext,
) -> Result<(), L2DisableReason> {
    match ontology.execute_unchecked(queries, ctx).await {
        Ok(output)
            if !output.truncated
                && output.rows.as_slice()
                    == [vec![
                        Value::String(other_query_id.to_owned()),
                        Value::String("RUNNING".to_owned()),
                    ]] =>
        {
            Ok(())
        }
        Err(error) => Err(failure(&error, L2DisableReason::OtherQueryUnconfirmed)),
        _ => Err(L2DisableReason::OtherQueryUnconfirmed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_health_expired_attestation_is_disabled() {
        let health = L2Health::new(None);
        health.complete((0, 0), L2HealthStatus::Enabled);
        health.state.lock().expect("status lock").checked_at = Instant::now()
            .checked_sub(Duration::from_secs(61))
            .expect("clock supports sixty seconds");
        assert_eq!(
            health.status(),
            L2HealthStatus::Disabled {
                reason: L2DisableReason::Expired,
            }
        );
    }
}
