use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use ontoforge_auth::Principal;
use ontoforge_auth::scope::{PrincipalScope, ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::Engine;
use ontoforge_storage::store::ActiveDefinition;

use crate::{ServerHost, error};

pub struct RequestContext {
    pub principal: Principal,
    pub active: Arc<ActiveDefinition>,
    pub scope: PrincipalScope,
}

pub async fn build<E: Engine + Send + Sync + 'static>(
    host: &ServerHost<E>,
    headers: &HeaderMap,
    api_name: &str,
) -> Result<(RequestContext, String), Response> {
    let request_id = ulid::Ulid::generate().to_string();
    let bearer = (headers.get_all(header::AUTHORIZATION).iter().count() == 1)
        .then(|| headers.get(header::AUTHORIZATION))
        .flatten()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let Some(bearer) = bearer else {
        return Err(error::wire(
            StatusCode::UNAUTHORIZED,
            "UNAUTHENTICATED",
            api_name,
            &request_id,
        ));
    };
    let Ok(principal) = host.auth.authenticate(bearer).await else {
        return Err(error::wire(
            StatusCode::UNAUTHORIZED,
            "UNAUTHENTICATED",
            api_name,
            &request_id,
        ));
    };
    let Ok(snapshot) = host.store.request_snapshot().await else {
        return Err(error::wire(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORE_UNAVAILABLE",
            api_name,
            &request_id,
        ));
    };
    let Ok(compiled) = compile_scope(&ScopeInputs {
        ontology: &snapshot.active.ontology,
        profile: None,
        attrs: &attr_types(Some(&host.auth_config)),
        scope: &snapshot.scope,
        sources: None,
    }) else {
        return Err(error::wire(
            StatusCode::SERVICE_UNAVAILABLE,
            "SCOPE_UNAVAILABLE",
            api_name,
            &request_id,
        ));
    };
    Ok((
        RequestContext {
            principal: principal.clone(),
            active: snapshot.active,
            scope: compiled.for_principal(&principal),
        },
        request_id,
    ))
}

pub fn has_forged_context(headers: &HeaderMap) -> bool {
    ["x-principal", "x-roles", "x-scope", "x-limits"]
        .iter()
        .any(|name| headers.contains_key(*name))
}

pub async fn context_from_headers<E: Engine + Send + Sync + 'static>(
    host: &ServerHost<E>,
    headers: HeaderMap,
    api_name: &str,
) -> Result<(RequestContext, String), Response> {
    if has_forged_context(&headers) {
        let id = ulid::Ulid::generate().to_string();
        return Err(error::wire(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            api_name,
            &id,
        ));
    }
    build(host, &headers, api_name).await
}
