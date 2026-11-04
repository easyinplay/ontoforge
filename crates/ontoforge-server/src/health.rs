//! D-03 health endpoint and stable, non-secret status mapping.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ontoforge_engine::{Engine, EngineStatus};
use serde_json::json;
use std::sync::Arc;

use crate::{ServerHost, error, state};

pub async fn endpoint<E: Engine + Send + Sync + 'static>(
    State(host): State<Arc<ServerHost<E>>>,
) -> Response {
    let core = host.engine.status().await;
    let server_state = state::for_host(&host);
    let l2 = server_state.l2_status();
    let drift = match host.store.request_snapshot().await {
        Ok(snapshot) => server_state.drift_view(&snapshot.active).degraded(),
        Err(_) => None,
    };
    let core_available = matches!(core, EngineStatus::Available);
    let (status, http_status) = match core {
        EngineStatus::Available if drift.is_some() => {
            let degraded = drift == Some(true)
                || l2.is_some_and(|status| {
                    !matches!(
                        status,
                        ontoforge_engine::trino::l2_health::L2HealthStatus::Enabled
                    ) && !matches!(
                        status,
                        ontoforge_engine::trino::l2_health::L2HealthStatus::Disabled {
                            reason:
                                ontoforge_engine::trino::l2_health::L2DisableReason::NotConfigured
                        }
                    )
                });
            (
                if degraded { "degraded" } else { "healthy" },
                StatusCode::OK,
            )
        }
        EngineStatus::Available | EngineStatus::Unavailable { .. } => {
            ("unhealthy", StatusCode::SERVICE_UNAVAILABLE)
        }
    };
    let mut body = json!({
        "status": status,
        "engineStatus": if core_available { "available" } else { "unavailable" },
    });
    if let Some(status) = l2 {
        if let ontoforge_engine::trino::l2_health::L2HealthStatus::Disabled { reason } = status {
            if reason != ontoforge_engine::trino::l2_health::L2DisableReason::NotConfigured {
                body["l2"] = json!({
                    "status": "disabled",
                    "reason": state::reason_code(reason),
                });
            }
        } else {
            body["l2"] = json!({"status": "enabled"});
        }
    }
    error::response(http_status, &ulid::Ulid::generate().to_string(), body)
}
