//! Authenticated HTTP host sharing the ontology services and durable read audit.

use std::future::Future;
use std::sync::Arc;

use axum::Router;
use ontoforge_auth::authn::Authenticator;
use ontoforge_engine::Engine;
use ontoforge_engine::config::AuthConfig;
use ontoforge_query::{QueryError, TokenCodec};
use ontoforge_storage::store::EditStore;
use tokio::net::TcpListener;

pub mod context;
pub mod error;
pub mod rest;
pub use ontoforge_api as api;

pub struct ServerHost<E> {
    pub store: Arc<EditStore>,
    pub engine: Arc<E>,
    pub auth: Authenticator,
    pub auth_config: AuthConfig,
    pub tokens: TokenCodec,
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
        let router = Router::new()
            .route("/api/ontology", axum::routing::get(rest::ontology::<E>))
            .route(
                "/api/objects/{object_type}/{pk}",
                axum::routing::get(rest::object::<E>),
            )
            .route("/api/query", axum::routing::post(rest::query::<E>))
            .route("/api/aggregate", axum::routing::post(rest::aggregate::<E>))
            .fallback(error::not_found)
            .with_state(Arc::clone(&self));
        let served = axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await;
        let drained = self.store.read_audit().shutdown().await;
        served.map_err(|_| HostError::Listener)?;
        drained.map_err(|_| HostError::AuditDrain)
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
