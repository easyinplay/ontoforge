//! Authenticated HTTP host sharing the ontology services and durable read audit.

use std::future::Future;
use std::sync::Arc;

use axum::Router;
use ontoforge_auth::authn::Authenticator;
use ontoforge_engine::Engine;
use ontoforge_engine::config::AuthConfig;
use ontoforge_query::TokenCodec;
use ontoforge_storage::store::EditStore;
use tokio::net::TcpListener;

pub use ontoforge_api as api;

pub struct ServerHost<E> {
    pub store: Arc<EditStore>,
    pub engine: Arc<E>,
    pub auth: Authenticator,
    pub auth_config: AuthConfig,
    pub tokens: TokenCodec,
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("HTTP listener failed")]
    Listener,
    #[error("read audit drain failed")]
    AuditDrain,
}

impl<E: Engine + Send + Sync + 'static> ServerHost<E> {
    pub async fn serve(
        self: Arc<Self>,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), HostError> {
        let served = axum::serve(listener, Router::new())
            .with_graceful_shutdown(shutdown)
            .await;
        let drained = self.store.read_audit().shutdown().await;
        served.map_err(|_| HostError::Listener)?;
        drained.map_err(|_| HostError::AuditDrain)
    }
}

/// Cargo package name of this crate.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }
}

#[cfg(test)]
#[path = "tracer.test.rs"]
mod tracer_tests;
