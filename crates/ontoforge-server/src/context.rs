use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::Response;
use ontoforge_auth::Principal;
use ontoforge_auth::scope::{PrincipalScope, ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_storage::store::ActiveDefinition;
use rmcp::transport::streamable_http_server::session::SessionManager;

use crate::{ServerHost, ServerLimits, error};

#[derive(Clone)]
pub struct AdmissionPermit(pub Arc<Mutex<Option<tokio::sync::OwnedSemaphorePermit>>>);

#[cfg(test)]
#[path = "../tests/unit/mcp_admission.test.rs"]
mod admission_tests;

#[derive(Clone)]
struct McpAdmission {
    reservation: Option<AdmissionPermit>,
    business: bool,
    controls: Arc<tokio::sync::Semaphore>,
    limits: ServerLimits,
}

fn mcp_endpoint(path: &str) -> bool {
    matches!(path, "/mcp" | "/mcp/")
}

fn admission_permit(permit: tokio::sync::OwnedSemaphorePermit) -> AdmissionPermit {
    AdmissionPermit(Arc::new(Mutex::new(Some(permit))))
}

fn concurrency_limit(api_name: &str, id: &str) -> Response {
    error::wire(
        StatusCode::TOO_MANY_REQUESTS,
        "CONCURRENCY_LIMIT",
        api_name,
        id,
    )
}

fn reserve_mcp(
    request: &axum::extract::Request,
    slots: Arc<tokio::sync::Semaphore>,
    controls: Arc<tokio::sync::Semaphore>,
    limits: ServerLimits,
) -> Result<McpAdmission, ()> {
    let (reservation, business) = if request.method() == Method::GET {
        (None, false)
    } else if request.method() == Method::DELETE
        && request
            .headers()
            .get("mcp-session-id")
            .is_some_and(|id| !id.is_empty())
    {
        (
            Some(controls.clone().try_acquire_owned().map_err(|_| ())?),
            false,
        )
    } else {
        match slots.try_acquire_owned() {
            Ok(permit) => (Some(permit), true),
            Err(_) if request.method() == Method::POST => (
                Some(controls.clone().try_acquire_owned().map_err(|_| ())?),
                false,
            ),
            Err(_) => return Err(()),
        }
    };
    Ok(McpAdmission {
        reservation: reservation.map(admission_permit),
        business,
        controls,
        limits,
    })
}

#[derive(serde::Deserialize)]
struct CancellationEnvelope {
    jsonrpc: String,
    method: String,
    params: Box<serde_json::value::RawValue>,
}

#[derive(serde::Deserialize)]
struct CancellationTarget {
    #[serde(rename = "requestId")]
    request_id: rmcp::model::RequestId,
}

fn cancellation_notification(bytes: &[u8]) -> bool {
    use rmcp::model::{CancelledNotificationParam, ClientJsonRpcMessage, ClientNotification};
    // Required typed fields reject duplicates before Value can collapse keys.
    let Ok(envelope) = serde_json::from_slice::<CancellationEnvelope>(bytes) else {
        return false;
    };
    if envelope.jsonrpc != "2.0" || envelope.method != "notifications/cancelled" {
        return false;
    }
    let Ok(target) = serde_json::from_str::<CancellationTarget>(envelope.params.get()) else {
        return false;
    };
    let Ok(params) = serde_json::from_str::<CancelledNotificationParam>(envelope.params.get())
    else {
        return false;
    };
    if params.request_id.as_ref() != Some(&target.request_id) {
        return false;
    }
    let Ok(serde_json::Value::Object(object)) = serde_json::from_slice::<serde_json::Value>(bytes)
    else {
        return false;
    };
    if object.contains_key("id") {
        return false;
    }
    matches!(serde_json::from_slice::<ClientJsonRpcMessage>(bytes),
        Ok(ClientJsonRpcMessage::Notification(notification))
        if matches!(notification.notification, ClientNotification::CancelledNotification(ref cancelled)
            if cancelled.params == params))
}

async fn admit_mcp_request(
    mut request: axum::extract::Request,
    id: &str,
) -> Result<axum::extract::Request, Response> {
    let Some(mut admission) = request.extensions_mut().remove::<McpAdmission>() else {
        return Ok(request);
    };
    if request.method() == Method::POST {
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, admission.limits.max_body_bytes)
            .await
            .map_err(|_| {
                error::wire(StatusCode::PAYLOAD_TOO_LARGE, "INVALID_REQUEST", "mcp", id)
            })?;
        if cancellation_notification(&bytes) {
            if admission.business {
                let control = admission
                    .controls
                    .try_acquire_owned()
                    .map_err(|_| concurrency_limit("mcp", id))?;
                admission.reservation = Some(admission_permit(control));
            }
        } else if !admission.business {
            return Err(concurrency_limit("mcp", id));
        }
        request = axum::extract::Request::from_parts(parts, axum::body::Body::from(bytes));
    }
    if let Some(permit) = admission.reservation {
        request.extensions_mut().insert(permit);
    }
    Ok(request)
}

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
    axum::extract::State((slots, limits, server_state)): axum::extract::State<(
        Arc<tokio::sync::Semaphore>,
        ServerLimits,
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
    let _http_permit = if mcp_endpoint(request.uri().path()) {
        let Ok(admission) = reserve_mcp(
            &request,
            slots,
            Arc::clone(&server_state.mcp_controls),
            limits,
        ) else {
            return concurrency_limit(&api_name, &trace.id());
        };
        request.extensions_mut().insert(admission);
        None
    } else {
        let Ok(permit) = slots.try_acquire_owned() else {
            return concurrency_limit(&api_name, &trace.id());
        };
        let permit = admission_permit(permit);
        request.extensions_mut().insert(permit.clone());
        // REST body extractors can drop Parts before the handler awaits.
        // Keep this reference until the response; Action may take its contents.
        Some(permit)
    };
    request.extensions_mut().insert(trace.clone());
    tokio::select! {
        biased;
        () = server_state.read_cancellation.cancelled() => error::wire(StatusCode::SERVICE_UNAVAILABLE, "SHUTTING_DOWN", &api_name, &trace.id()),
        result = tokio::time::timeout(limits.deadline, next.run(request)) => match result {
            Ok(response) => response,
            Err(_) => error::wire(
            StatusCode::GATEWAY_TIMEOUT,
            "DEADLINE_EXCEEDED",
            &api_name,
            &trace.id(),
            ),
        },
    }
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
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let (ctx, id) = match context_from_headers(&host, request.headers().clone(), "mcp").await {
        Ok(context) => context,
        Err(response) => return response,
    };
    let server_state = crate::state::for_host(&host);
    let owners = &server_state.mcp_sessions;
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
    request = match admit_mcp_request(request, &id).await {
        Ok(request) => request,
        Err(response) => return response,
    };
    let initialization = owners.initialization_guard();
    request.extensions_mut().insert(initialization.slot());
    let response = next.run(request).await;
    if response.status().is_success()
        && session.is_none()
        && let Some(created) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
    {
        if owners.bind(created, ctx.principal.id.as_str()).is_err() {
            let _ = owners.close_session(&created.into()).await;
            return error::wire(
                StatusCode::SERVICE_UNAVAILABLE,
                "SESSION_OWNER_UNAVAILABLE",
                "mcp",
                &id,
            );
        }
        initialization.disarm();
    }
    response
}
