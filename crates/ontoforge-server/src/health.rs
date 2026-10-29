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
    let l2 = state::for_host(&host).l2_status();
    let core_available = matches!(core, EngineStatus::Available);
    let (status, http_status) = match core {
        EngineStatus::Available => {
            let degraded = l2.is_some_and(|status| {
                !matches!(
                    status,
                    ontoforge_engine::trino::l2_health::L2HealthStatus::Enabled
                ) && !matches!(
                    status,
                    ontoforge_engine::trino::l2_health::L2HealthStatus::Disabled {
                        reason: ontoforge_engine::trino::l2_health::L2DisableReason::NotConfigured
                    }
                )
            });
            (
                if degraded { "degraded" } else { "healthy" },
                StatusCode::OK,
            )
        }
        EngineStatus::Unavailable { .. } => ("unhealthy", StatusCode::SERVICE_UNAVAILABLE),
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
