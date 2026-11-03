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
    let key = Arc::as_ptr(&host.store) as usize;
    task_registry()
        .lock()
        .unwrap()
        .entry(key)
        .or_default()
        .push(task);
}

pub(crate) fn spawn_action<E, F>(host: &ServerHost<E>, action: F) -> Result<(), ()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let mut tasks = task_registry().lock().unwrap();
    if !crate::state::for_host(host).accepting() {
        return Err(());
    }
    tasks
        .entry(Arc::as_ptr(&host.store) as usize)
        .or_default()
        .push(tokio::spawn(action));
    Ok(())
}

pub(crate) fn stop_admission<E>(host: &ServerHost<E>) {
    let _tasks = task_registry().lock().unwrap();
    crate::state::for_host(host).stop_admission();
}

pub async fn wait_actions<E>(host: &ServerHost<E>) -> Result<(), tokio::task::JoinError> {
    let key = Arc::as_ptr(&host.store) as usize;
    let tasks = task_registry()
        .lock()
        .unwrap()
        .remove(&key)
        .unwrap_or_default();
    let mut failure = None;
    for task in tasks {
        if let Err(error) = task.await {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn detach_actions<E>(host: &ServerHost<E>) {
    task_registry()
        .lock()
        .unwrap()
        .remove(&(Arc::as_ptr(&host.store) as usize));
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
    axum::extract::State((slots, deadline, server_state)): axum::extract::State<(
        Arc<tokio::sync::Semaphore>,
        Duration,
        Arc<crate::state::ServerState>,
    )>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let trace = RequestTrace::new();
    let _read = server_state.read_tasks.token();
    if !server_state.accepting() {
        return error::wire(
            StatusCode::SERVICE_UNAVAILABLE,
            "SHUTTING_DOWN",
            "server",
            &trace.id(),
        );
    }
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
    let response = tokio::select! {
        biased;
        () = server_state.read_cancellation.cancelled() => error::wire(StatusCode::SERVICE_UNAVAILABLE, "SHUTTING_DOWN", &api_name, &trace.id()),
        result = tokio::time::timeout(deadline, next.run(request)) => match result {
            Ok(response) => response,
            Err(_) => error::wire(
            StatusCode::GATEWAY_TIMEOUT,
            "DEADLINE_EXCEEDED",
            &api_name,
            &trace.id(),
            ),
        },
    };
    let _ = permit_slot.lock().unwrap().take();
    response
}

/// Source admission is shared; only independent queries inherit transport limits.
#[derive(Debug, Clone, Copy)]
pub enum EnginePurpose {
    Query,
    Action,
}

pub struct ScopedEngine<'a, E> {
    pub inner: &'a E,
    pub trace: &'a RequestTrace,
    pub limits: crate::ServerLimits,
    pub drift: &'a crate::drift::DriftView,
    pub purpose: EnginePurpose,
}

impl<E: Engine + Sync> Engine for ScopedEngine<'_, E> {
    fn check_source_target(
        &self,
        target: &ontoforge_schema::ir::BindingTarget,
    ) -> Result<(), ontoforge_engine::SourceTargetError> {
        self.drift.check(target)?;
        self.inner.check_source_target(target)
    }
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        if matches!(self.purpose, EnginePurpose::Action) {
            return self.inner.execute(sql, ctx).await;
        }
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
    pub drift: Arc<crate::drift::DriftView>,
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
            drift: crate::state::for_host(host).drift_view(&snapshot.active),
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

pub async fn authenticate_mcp<E: Engine + Send + Sync + 'static>(
    axum::extract::State(host): axum::extract::State<Arc<ServerHost<E>>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let (ctx, id) = match context_from_headers(&host, request.headers().clone(), "mcp").await {
        Ok(context) => context,
        Err(response) => return response,
    };
    let owners = crate::mcp::mcp_sessions::global();
    let session = request
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if let Some(session) = &session
        && let Err(reason) = owners.verify(session, ctx.principal.id.as_str())
    {
        let status = match reason {
            crate::mcp::mcp_sessions::SessionVerifyError::Missing => StatusCode::NOT_FOUND,
            crate::mcp::mcp_sessions::SessionVerifyError::Mismatched => StatusCode::FORBIDDEN,
        };
        return error::wire(status, "SESSION_OWNER_UNAVAILABLE", "mcp", &id);
    }
    let deleting = request.method() == axum::http::Method::DELETE;
    let response = next.run(request).await;
    if response.status().is_success() {
        if deleting {
            if let Some(session) = session {
                owners.remove(&session);
            }
        } else if session.is_none()
            && let Some(created) = response
                .headers()
                .get("mcp-session-id")
                .and_then(|value| value.to_str().ok())
            && owners.admit(created, ctx.principal.id.as_str()).is_err()
        {
            return error::wire(
                StatusCode::SERVICE_UNAVAILABLE,
                "SESSION_OWNER_UNAVAILABLE",
                "mcp",
                &id,
            );
        }
    }
    response
}
