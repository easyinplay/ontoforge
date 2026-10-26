//! Fail-closed ACL attestation for the dedicated L2 identity.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::TrinoEngine;
use crate::printer::l2::{ProbeTable, acl_query_probe, acl_table_probe};
use crate::{EngineError, QueryContext, QueryOutput, Value};

/// Explicit deployment declaration and actual relations used for ACL attestation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct L2AclConfig {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L2HealthStatus {
    Enabled,
    Disabled { reason: L2DisableReason },
}

#[derive(Debug)]
pub(super) struct L2Health {
    config: Option<L2AclConfig>,
    status: Arc<Mutex<(L2HealthStatus, Instant)>>,
    recheck_lock: tokio::sync::Mutex<()>,
    pub(super) admission: tokio::sync::RwLock<()>,
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
            status: Arc::new(Mutex::new((
                L2HealthStatus::Disabled { reason },
                Instant::now(),
            ))),
            recheck_lock: tokio::sync::Mutex::new(()),
            admission: tokio::sync::RwLock::new(()),
        }
    }

    pub(super) fn status(&self) -> L2HealthStatus {
        self.status.lock().map_or(
            L2HealthStatus::Disabled {
                reason: L2DisableReason::NotVerified,
            },
            |guard| {
                let (status, checked_at) = &*guard;
                if *status == L2HealthStatus::Enabled
                    && checked_at.elapsed() >= Duration::from_secs(60)
                {
                    L2HealthStatus::Disabled {
                        reason: L2DisableReason::Expired,
                    }
                } else {
                    *status
                }
            },
        )
    }

    fn set(&self, status: L2HealthStatus) -> L2HealthStatus {
        match self.status.lock() {
            Ok(mut current) => {
                *current = (status, Instant::now());
                status
            }
            Err(_) => L2HealthStatus::Disabled {
                reason: L2DisableReason::NotVerified,
            },
        }
    }

    fn disable(&self, reason: L2DisableReason) -> L2HealthStatus {
        self.set(L2HealthStatus::Disabled { reason })
    }
}

fn probe_context() -> QueryContext {
    QueryContext {
        trace_token: "ontoforge-l2-acl-check".to_owned(),
        client_tags: vec!["ontoforge".to_owned()],
        timeout: Duration::from_secs(5),
        max_rows: 2,
        max_bytes: 4096,
    }
}

fn failure(error: &EngineError, otherwise: L2DisableReason) -> L2DisableReason {
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
    let _recheck = health.recheck_lock.lock().await;
    let _admission = health.admission.write().await;
    health.disable(L2DisableReason::NotVerified);
    let Some(config) = &health.config else {
        return health.disable(L2DisableReason::NotConfigured);
    };
    if l2.conn.user == ontology.conn.user
        || config.allowed == config.outside
        || config.allowed == config.edit
        || config.outside == config.edit
        || config.allowed.catalog == config.outside.catalog
        || config.allowed.catalog == config.edit.catalog
    {
        return health.disable(L2DisableReason::InvalidProbe);
    }
    let Ok(queries) = acl_query_probe(other_query_id) else {
        return health.disable(L2DisableReason::InvalidProbe);
    };
    let Ok(allowed) = acl_table_probe(&config.allowed) else {
        return health.disable(L2DisableReason::InvalidProbe);
    };
    let Ok(outside) = acl_table_probe(&config.outside) else {
        return health.disable(L2DisableReason::InvalidProbe);
    };
    let Ok(edit) = acl_table_probe(&config.edit) else {
        return health.disable(L2DisableReason::InvalidProbe);
    };
    let ctx = probe_context();
    for relation in [&allowed, &outside, &edit] {
        match ontology.execute_unchecked(relation, &ctx).await {
            Ok(output) if confirmed(&output) => {}
            Err(error) => {
                return health.disable(failure(&error, L2DisableReason::ObjectUnconfirmed));
            }
            _ => return health.disable(L2DisableReason::ObjectUnconfirmed),
        }
    }
    match l2.execute_unchecked(&allowed, &ctx).await {
        Ok(output) if confirmed(&output) => {}
        Err(error) => return health.disable(failure(&error, L2DisableReason::AllowedProbeFailed)),
        _ => return health.disable(L2DisableReason::AllowedProbeFailed),
    }
    for relation in [&outside, &edit] {
        match l2.execute_unchecked(relation, &ctx).await {
            Err(EngineError::Query { error_name, .. }) if error_name == "PERMISSION_DENIED" => {}
            Err(error) => {
                return health.disable(failure(&error, L2DisableReason::NegativeProbeFailed));
            }
            _ => return health.disable(L2DisableReason::NegativeProbeFailed),
        }
    }
    match ontology.execute_unchecked(&queries, &ctx).await {
        Ok(output)
            if !output.truncated
                && output.rows.as_slice() == [vec![Value::String(other_query_id.to_owned())]] => {}
        Err(error) => {
            return health.disable(failure(&error, L2DisableReason::OtherQueryUnconfirmed));
        }
        _ => return health.disable(L2DisableReason::OtherQueryUnconfirmed),
    }
    match l2.execute_unchecked(&queries, &ctx).await {
        Ok(output) if !output.truncated && output.rows.is_empty() => {
            health.set(L2HealthStatus::Enabled)
        }
        Err(error) => health.disable(failure(&error, L2DisableReason::QueryVisibilityFailed)),
        _ => health.disable(L2DisableReason::QueryVisibilityFailed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_health_expired_attestation_is_disabled() {
        let health = L2Health::new(None);
        health.set(L2HealthStatus::Enabled);
        health.status.lock().expect("status lock").1 = Instant::now()
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
