use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use ontoforge_auth::Principal;
use ontoforge_auth::scope::{PrincipalScope, ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_storage::store::ActiveDefinition;

use crate::{ServerHost, error};

#[derive(Clone)]
pub struct AdmissionPermit(pub Arc<Mutex<Option<tokio::sync::OwnedSemaphorePermit>>>);

static ACTION_TASKS: OnceLock<Mutex<HashMap<usize, Vec<tokio::task::JoinHandle<()>>>>> =
    OnceLock::new();

fn task_registry() -> &'static Mutex<HashMap<usize, Vec<tokio::task::JoinHandle<()>>>> {
    ACTION_TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_action<E>(host: &ServerHost<E>, task: tokio::task::JoinHandle<()>) {
    let key = std::ptr::from_ref(&host.store) as usize;
    task_registry()
        .lock()
        .unwrap()
        .entry(key)
        .or_default()
        .push(task);
}

pub async fn wait_actions<E>(host: &ServerHost<E>) {
    let key = std::ptr::from_ref(&host.store) as usize;
    let tasks = task_registry()
        .lock()
        .unwrap()
        .remove(&key)
        .unwrap_or_default();
    for task in tasks {
        let _ = task.await;
    }
}

#[derive(Clone)]
pub struct RequestTrace {
    id: Arc<Mutex<String>>,
    fault: Arc<Mutex<Option<SourceFault>>>,
}

#[derive(Clone, Copy)]
enum SourceFault {
    Timeout,
    Cancelled,
}

impl RequestTrace {
    pub(crate) fn new() -> Self {
        Self {
            id: Arc::new(Mutex::new(ulid::Ulid::generate().to_string())),
            fault: Arc::new(Mutex::new(None)),
        }
    }
    pub fn reserved(&self, id: &str) {
        id.clone_into(&mut self.id.lock().unwrap());
    }
    pub fn id(&self) -> String {
        self.id.lock().unwrap().clone()
    }
    pub fn source_failure(&self, api_name: &str, id: &str) -> Option<Response> {
        match *self.fault.lock().unwrap() {
            Some(SourceFault::Timeout) => Some(error::wire(
                StatusCode::GATEWAY_TIMEOUT,
                "DEADLINE_EXCEEDED",
                api_name,
                id,
            )),
            Some(SourceFault::Cancelled) => Some(error::wire(
                StatusCode::REQUEST_TIMEOUT,
                "CANCELLED",
                api_name,
                id,
            )),
            None => None,
        }
    }
}

pub async fn admit(
    axum::extract::State((slots, deadline)): axum::extract::State<(
        Arc<tokio::sync::Semaphore>,
        Duration,
    )>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let trace = RequestTrace::new();
    let api_name = request
        .uri()
        .path()
        .split('/')
        .nth(2)
        .unwrap_or("unknown")
        .to_owned();
    let Ok(permit) = slots.try_acquire_owned() else {
        return error::wire(
            StatusCode::TOO_MANY_REQUESTS,
            "CONCURRENCY_LIMIT",
            &api_name,
            &trace.id(),
        );
    };
    let permit_slot = Arc::new(Mutex::new(Some(permit)));
    request
        .extensions_mut()
        .insert(AdmissionPermit(Arc::clone(&permit_slot)));
    request.extensions_mut().insert(trace.clone());
    let response = match tokio::time::timeout(deadline, next.run(request)).await {
        Ok(response) => response,
        Err(_) => error::wire(
            StatusCode::GATEWAY_TIMEOUT,
            "DEADLINE_EXCEEDED",
            &api_name,
            &trace.id(),
        ),
    };
    let _ = permit_slot.lock().unwrap().take();
    response
}

pub struct ScopedEngine<'a, E> {
    pub inner: &'a E,
    pub trace: &'a RequestTrace,
    pub limits: crate::ServerLimits,
}

impl<E: Engine + Sync> Engine for ScopedEngine<'_, E> {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        let mut ctx = ctx.clone();
        ctx.max_rows = ctx.max_rows.min(self.limits.execution.max_raw_candidates);
        ctx.max_bytes = ctx.max_bytes.min(self.limits.execution.max_decoded_bytes);
        ctx.timeout = ctx.timeout.min(self.limits.deadline);
        let result = self.inner.execute(sql, &ctx).await;
        let fault = match &result {
            Err(EngineError::Timeout { .. }) => Some(SourceFault::Timeout),
            Err(EngineError::Cancelled) => Some(SourceFault::Cancelled),
            _ => None,
        };
        if let Some(fault) = fault {
            *self.trace.fault.lock().unwrap() = Some(fault);
        }
        result
    }
    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.inner.describe_tables(tables, ctx).await
    }
    async fn status(&self) -> EngineStatus {
        self.inner.status().await
    }
}

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
