//! Authenticated HTTP host sharing the ontology services and durable read audit.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use ontoforge_auth::authn::Authenticator;
use ontoforge_engine::Engine;
use ontoforge_engine::config::AuthConfig;
use ontoforge_query::{QueryError, TokenCodec};
use ontoforge_storage::store::EditStore;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

pub mod context;
pub mod error;
pub mod health;
pub mod lifecycle;
pub mod rest;
pub mod state;
pub use ontoforge_api as api;

pub struct ServerHost<E> {
    pub store: Arc<EditStore>,
    pub engine: Arc<E>,
    pub auth: Authenticator,
    pub auth_config: AuthConfig,
    pub tokens: TokenCodec,
}

#[derive(Debug, Clone, Copy)]
pub struct ServerLimits {
    pub max_body_bytes: usize,
    pub max_concurrent_requests: usize,
    pub deadline: Duration,
    pub execution: ontoforge_query::ExecutionLimits,
}

impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: 1024 * 1024,
            max_concurrent_requests: 32,
            deadline: Duration::from_millis(ontoforge_engine::DEFAULT_QUERY_TIMEOUT_MS),
            execution: ontoforge_query::ExecutionLimits::phase4_default(),
        }
    }
}

impl ServerLimits {
    fn validate(self) -> Result<Self, HostError> {
        let maximum = ontoforge_query::ExecutionLimits::phase4_default();
        if self.max_body_bytes == 0
            || self.max_body_bytes > 16 * 1024 * 1024
            || self.max_concurrent_requests == 0
            || self.max_concurrent_requests > 1024
            || self.deadline.is_zero()
            || self.deadline > Duration::from_secs(300)
            || self.execution.max_backend_calls == 0
            || self.execution.max_backend_calls > maximum.max_backend_calls
            || self.execution.max_raw_candidates == 0
            || self.execution.max_raw_candidates > maximum.max_raw_candidates
            || self.execution.max_backfill_keys == 0
            || self.execution.max_backfill_keys > maximum.max_backfill_keys
            || self.execution.max_decoded_bytes == 0
            || self.execution.max_decoded_bytes > maximum.max_decoded_bytes
        {
            return Err(HostError::Limits);
        }
        Ok(self)
    }
}

pub const REST_ROUTE_PATHS: &[&str] = &[
    "/api/ontology",
    "/api/query",
    "/api/aggregate",
    "/api/objects/{object_type}/{pk}",
];

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("invalid HTTP host limits")]
    Limits,
    #[error("HTTP listener failed")]
    Listener,
    #[error("read audit drain failed")]
    AuditDrain,
}

impl<E: Engine + Send + Sync + 'static> ServerHost<E> {
    pub fn new(
        store: Arc<EditStore>,
        engine: Arc<E>,
        auth: Authenticator,
        auth_config: AuthConfig,
        signing_key: Vec<u8>,
    ) -> Result<Self, QueryError> {
        Ok(Self {
            store,
            engine,
            auth,
            auth_config,
            tokens: TokenCodec::new(signing_key)?,
        })
    }

    pub async fn serve(
        self: Arc<Self>,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), HostError> {
        self.serve_with_limits(listener, shutdown, ServerLimits::default())
            .await
    }

    pub async fn serve_with_limits(
        self: Arc<Self>,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
        limits: ServerLimits,
    ) -> Result<(), HostError> {
        let limits = limits.validate()?;
        let admission = (
            Arc::new(Semaphore::new(limits.max_concurrent_requests)),
            limits.deadline,
        );
        let router = Router::new()
            .route("/health", axum::routing::get(health::endpoint::<E>))
            .route("/api/ontology", axum::routing::get(rest::ontology::<E>))
            .route(
                "/api/objects/{object_type}/{pk}",
                axum::routing::get(rest::object::<E>),
            )
            .route("/api/query", axum::routing::post(rest::query::<E>))
            .route("/api/aggregate", axum::routing::post(rest::aggregate::<E>))
            .route(
                "/api/actions/{action}",
                axum::routing::post(rest::action::<E>),
            )
            .route(
                "/api/commands/{command_id}",
                axum::routing::get(rest::command_result::<E>),
            )
            .route(
                "/api/objects/{object_type}/{pk}/history",
                axum::routing::get(rest::history::<E>),
            )
            .fallback(error::not_found)
            .method_not_allowed_fallback(error::method_not_allowed)
            .layer(axum::Extension(limits))
            .layer(axum::middleware::from_fn_with_state(
                admission,
                context::admit,
            ))
            .with_state(Arc::clone(&self));
        let served = axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await;
        let lifecycle = lifecycle::ServerLifecycle::new(&self).shutdown().await;
        served.map_err(|_| HostError::Listener)?;
        lifecycle.map_err(|error| match error {
            lifecycle::LifecycleError::Audit | lifecycle::LifecycleError::Monitor(_) => {
                HostError::AuditDrain
            }
        })
    }

    pub fn with_l2_status_provider<F>(self, provider: F) -> Self
    where
        F: Fn() -> Option<ontoforge_engine::trino::l2_health::L2HealthStatus>
            + Send
            + Sync
            + 'static,
    {
        state::for_host(&self).set_l2_status_provider(provider);
        self
    }

    pub fn install_l2_monitor(
        &self,
        monitor: ontoforge_engine::trino::l2_monitor::L2AclMonitor,
    ) -> Result<(), ontoforge_engine::trino::l2_monitor::L2AclMonitor> {
        state::for_host(self).install_monitor(monitor)
    }
}

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;
    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }
}

#[cfg(test)]
extern crate self as ontoforge_server;

#[cfg(test)]
#[path = "../tests/unit/rest.test.rs"]
mod rest_tests;

#[cfg(test)]
#[path = "../tests/unit/tracer.test.rs"]
mod tracer_tests;
