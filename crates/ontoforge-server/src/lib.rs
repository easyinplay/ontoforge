//! Authenticated HTTP host sharing the ontology services and durable read audit.

use std::future::Future;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Path, RawQuery, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::Authenticator;
use ontoforge_auth::scope::{PrincipalScope, ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::Engine;
use ontoforge_engine::config::AuthConfig;
use ontoforge_query::{
    ExecutionLimits, ObjectSet, QueryError, QueryRequest, QueryService, TokenCodec,
};
use ontoforge_schema::ir::{BaseType, ObjectTypeName};
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use serde_json::{Value, json};
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

/// Authenticated identity and metadata fixed for one request.
pub struct RequestContext {
    pub principal: Principal,
    pub active: Arc<ActiveDefinition>,
    pub scope: PrincipalScope,
}

impl<E: Engine + Send + Sync + 'static> ServerHost<E> {
    pub async fn serve(
        self: Arc<Self>,
        listener: TcpListener,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), HostError> {
        let router = Router::new()
            .route("/api/objects/{object_type}/{pk}", get(read_pk::<E>))
            .with_state(Arc::clone(&self));
        let served = axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await;
        let drained = self.store.read_audit().shutdown().await;
        served.map_err(|_| HostError::Listener)?;
        drained.map_err(|_| HostError::AuditDrain)
    }
}

async fn read_pk<E: Engine + Send + Sync + 'static>(
    State(host): State<Arc<ServerHost<E>>>,
    path: Result<Path<(String, String)>, axum::extract::rejection::PathRejection>,
    RawQuery(query): RawQuery,
    request: Request,
) -> Response {
    let transport_id = ulid::Ulid::generate().to_string();
    let Ok(Path((api_name, pk))) = path else {
        return wire_error(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "objects",
            &transport_id,
        );
    };
    let headers = request.headers();
    let bearer = (headers.get_all(header::AUTHORIZATION).iter().count() == 1)
        .then(|| headers.get(header::AUTHORIZATION))
        .flatten()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let Some(bearer) = bearer else {
        return wire_error(
            StatusCode::UNAUTHORIZED,
            "UNAUTHENTICATED",
            &api_name,
            &transport_id,
        );
    };
    let Ok(principal) = host.auth.authenticate(bearer).await else {
        return wire_error(
            StatusCode::UNAUTHORIZED,
            "UNAUTHENTICATED",
            &api_name,
            &transport_id,
        );
    };
    if query.is_some_and(|q| !q.is_empty())
        || ["x-principal", "x-roles", "x-scope", "x-limits"]
            .iter()
            .any(|name| headers.contains_key(*name))
        || to_bytes(request.into_body(), 1)
            .await
            .map_or(true, |body| !body.is_empty())
    {
        return wire_error(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            &api_name,
            &transport_id,
        );
    }
    let Ok(snapshot) = host.store.request_snapshot().await else {
        return wire_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORE_UNAVAILABLE",
            &api_name,
            &transport_id,
        );
    };
    // request_snapshot has ended the shared definition transaction before scope compilation.
    let Ok(compiled) = compile_scope(&ScopeInputs {
        ontology: &snapshot.active.ontology,
        profile: None,
        attrs: &attr_types(Some(&host.auth_config)),
        scope: &snapshot.scope,
        sources: None,
    }) else {
        return wire_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "SCOPE_UNAVAILABLE",
            &api_name,
            &transport_id,
        );
    };
    let scope = compiled.for_principal(&principal);
    let context = RequestContext {
        principal,
        active: snapshot.active,
        scope,
    };
    execute_pk(&host, &context, &api_name, pk, &transport_id).await
}

async fn execute_pk<E: Engine + Send + Sync + 'static>(
    host: &ServerHost<E>,
    context: &RequestContext,
    api_name: &str,
    pk: String,
    transport_id: &str,
) -> Response {
    let Ok(object_type) = ObjectTypeName::try_from(api_name.to_owned()) else {
        return wire_error(StatusCode::NOT_FOUND, "NOT_FOUND", api_name, transport_id);
    };
    let Some(ty) = context.active.ontology.object_types.get(&object_type) else {
        return wire_error(StatusCode::NOT_FOUND, "NOT_FOUND", api_name, transport_id);
    };
    let pk_type = ty
        .properties
        .get(&ty.primary_key)
        .map(|property| property.base_type);
    let pk = match pk_type {
        Some(BaseType::String | BaseType::Date | BaseType::Timestamp) => Value::String(pk),
        _ => match serde_json::from_str(&pk) {
            Ok(value) => value,
            Err(_) => {
                return wire_error(StatusCode::NOT_FOUND, "NOT_FOUND", api_name, transport_id);
            }
        },
    };
    let execution = QueryService {
        principal: &context.principal,
        scope: &context.scope,
        active: Arc::clone(&context.active),
        engine: &*host.engine,
        store: &host.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &host.tokens,
    }
    .execute_audited(QueryRequest {
        set: ObjectSet::Pk { object_type, pk },
        select: ty.properties.keys().cloned().collect(),
    })
    .await;
    let request_id = execution.request_id.as_deref().unwrap_or(transport_id);
    match execution.result {
        Ok(page) => match page.rows.into_iter().next() {
            Some(row) => wire_response(StatusCode::OK, request_id, json!(row)),
            None => wire_error(StatusCode::NOT_FOUND, "NOT_FOUND", api_name, request_id),
        },
        Err(error) => wire_error(query_status(&error), error.code(), api_name, request_id),
    }
}

fn query_status(error: &QueryError) -> StatusCode {
    match error {
        QueryError::NotFound => StatusCode::NOT_FOUND,
        QueryError::Source | QueryError::Store | QueryError::AuditUnavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        QueryError::AmbiguousObject => StatusCode::CONFLICT,
        _ => StatusCode::BAD_REQUEST,
    }
}

fn wire_error(status: StatusCode, code: &str, api_name: &str, request_id: &str) -> Response {
    wire_response(
        status,
        request_id,
        json!({"code": code, "apiName": api_name, "request_id": request_id}),
    )
}

fn wire_response(status: StatusCode, request_id: &str, value: Value) -> Response {
    let mut response = (status, Json(value)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("server-generated ULID"),
    );
    response
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
