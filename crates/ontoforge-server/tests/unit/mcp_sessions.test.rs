use crate::mcp::mcp_sessions::{BoundedSessionManager, SessionVerifyError};
use rmcp::transport::streamable_http_server::session::SessionManager;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

use crate::rest_tests::support;

#[tokio::test]
async fn cancelled_close_keeps_real_session_counted_until_removal() {
    let manager = Arc::new(BoundedSessionManager::new(1, Duration::from_secs(30)));
    let (session, transport) = manager.create_session().await.unwrap();
    manager.bind(&session, "alice").unwrap();
    let cancellation = transport.cancel_token();
    let write = manager.inner.sessions.write().await;
    let mut close = Box::pin(manager.close_session(&session));
    std::future::poll_fn(|context| {
        assert!(close.as_mut().poll(context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    assert!(cancellation.is_cancelled());
    assert_eq!(
        manager.bind(&session, "alice"),
        Err(SessionVerifyError::Missing)
    );
    assert_eq!(
        manager.verify(&session, "alice"),
        Err(SessionVerifyError::Missing)
    );
    drop(close);
    drop(write);
    let (_next, _next_transport) = manager.create_session().await.unwrap();
    assert_eq!(
        manager.inner.sessions.read().await.len(),
        1,
        "cancelled close must retain capacity accounting until the real map entry is removed"
    );
    manager.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelled_close_is_retried_by_reaper_without_new_requests() {
    let manager = Arc::new(BoundedSessionManager::new(1, Duration::from_secs(30)));
    manager.start();
    let (session, transport) = manager.create_session().await.unwrap();
    let cancellation = transport.cancel_token();
    let write = manager.inner.sessions.write().await;
    let mut close = Box::pin(manager.close_session(&session));
    std::future::poll_fn(|context| {
        assert!(close.as_mut().poll(context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    assert!(cancellation.is_cancelled());
    assert_eq!(write.len(), 1);
    drop(close);
    drop(write);
    wait_empty(&manager).await;
    manager.shutdown().await.unwrap();
}

#[tokio::test]
async fn real_http_sessions_never_exceed_capacity() {
    let fixture = support::fixture().await;
    let state = crate::state::for_host(&fixture.host);
    let manager = Arc::clone(&state.mcp_sessions);
    let running = fixture.start().await;
    let mut peak = 0;
    for _ in 0..257 {
        let response = running.client.post(format!("{}/mcp", running.url))
            .bearer_auth(&running.token)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"capacity","version":"0"}}}))
            .send().await.unwrap();
        assert!(response.status().is_success());
        peak = peak.max(manager.inner.sessions.read().await.len());
    }
    let _ = running.finish().await;
    assert!(
        peak <= 256,
        "real LocalSessionManager peak was {peak}, limit 256"
    );
    assert_eq!(manager.inner.sessions.read().await.len(), 0);
}

fn install(
    fixture: &support::Fixture,
    manager: BoundedSessionManager,
) -> Arc<BoundedSessionManager> {
    let manager = Arc::new(manager);
    super::registry().lock().unwrap().insert(
        super::key(&fixture.host),
        Arc::new(super::ServerState {
            mcp_sessions: Arc::clone(&manager),
            ..Default::default()
        }),
    );
    manager
}

async fn initialize(running: &support::Running) -> reqwest::Response {
    running.client.post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"lifecycle","version":"0"}}}))
        .send().await.unwrap()
}

fn id(response: &reqwest::Response) -> String {
    assert!(response.status().is_success(), "{}", response.status());
    response
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

async fn wait_empty(manager: &BoundedSessionManager) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while !manager.inner.sessions.read().await.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("real sessions must be reclaimed without another request");
}

#[tokio::test]
async fn pending_sessions_count_toward_cap_and_evicted_binding_cannot_revive() {
    let manager = Arc::new(BoundedSessionManager::new(2, Duration::from_secs(30)));
    manager.start();
    let (first, _transport) = manager.create_session().await.unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(16));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let manager = Arc::clone(&manager);
        let barrier = Arc::clone(&barrier);
        tasks.spawn(async move {
            barrier.wait().await;
            let session = manager.create_session().await.unwrap();
            assert!(manager.inner.sessions.read().await.len() <= 2);
            session
        });
    }
    let mut pending = Vec::new();
    while let Some(result) = tasks.join_next().await {
        pending.push(result.unwrap());
    }
    assert_eq!(manager.inner.sessions.read().await.len(), 2);
    assert!(!manager.inner.has_session(&first).await.unwrap());
    assert_eq!(
        manager.bind(&first, "alice"),
        Err(SessionVerifyError::Missing)
    );
    manager.shutdown().await.unwrap();
    assert_eq!(manager.inner.sessions.read().await.len(), 0);
    assert!(manager.create_session().await.is_err());
}

#[tokio::test]
async fn real_http_concurrent_initialize_stays_bounded_and_releases_at_shutdown() {
    let fixture = support::fixture().await;
    let manager = install(
        &fixture,
        BoundedSessionManager::new(4, Duration::from_secs(30)),
    );
    let running = Arc::new(fixture.start().await);
    let barrier = Arc::new(tokio::sync::Barrier::new(24));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..24 {
        let running = Arc::clone(&running);
        let manager = Arc::clone(&manager);
        let barrier = Arc::clone(&barrier);
        tasks.spawn(async move {
            barrier.wait().await;
            let response = initialize(&running).await;
            assert!(manager.inner.sessions.read().await.len() <= 4);
            response.status().is_success()
        });
    }
    let mut successes = 0;
    while let Some(result) = tasks.join_next().await {
        successes += usize::from(result.unwrap());
    }
    assert!(successes > 0);
    assert!(manager.inner.sessions.read().await.len() <= 4);
    let running = Arc::try_unwrap(running).ok().unwrap();
    let _ = running.finish().await;
    assert_eq!(manager.inner.sessions.read().await.len(), 0);
    let weak = Arc::downgrade(&manager);
    drop(manager);
    tokio::time::timeout(Duration::from_secs(2), async {
        while weak.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("host tasks must release their session manager");
}

#[tokio::test]
async fn real_http_eviction_delete_and_hosts_are_isolated() {
    let first = support::fixture().await;
    let first_manager = install(
        &first,
        BoundedSessionManager::new(1, Duration::from_secs(30)),
    );
    let second = support::fixture().await;
    let second_manager = install(
        &second,
        BoundedSessionManager::new(1, Duration::from_secs(30)),
    );
    let first = first.start().await;
    let second = second.start().await;
    let old = id(&initialize(&first).await);
    let other = id(&initialize(&second).await);
    let current = id(&initialize(&first).await);
    assert!(
        !first_manager
            .inner
            .has_session(&old.clone().into())
            .await
            .unwrap()
    );
    assert_eq!(
        first_manager.bind(&old, "ontoforge-dev|alice"),
        Err(SessionVerifyError::Missing)
    );
    let missing = first
        .client
        .get(format!("{}/mcp", first.url))
        .bearer_auth(&first.token)
        .header("accept", "text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &old)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);
    let deleted = first
        .client
        .delete(format!("{}/mcp", first.url))
        .bearer_auth(&first.token)
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &current)
        .send()
        .await
        .unwrap();
    assert!(deleted.status().is_success());
    assert_eq!(first_manager.inner.sessions.read().await.len(), 0);
    assert!(
        second_manager
            .inner
            .has_session(&other.into())
            .await
            .unwrap()
    );
    let _ = first.finish().await;
    assert_eq!(second_manager.inner.sessions.read().await.len(), 1);
    let _ = second.finish().await;
    assert_eq!(second_manager.inner.sessions.read().await.len(), 0);
}

#[tokio::test]
async fn real_http_idle_ttl_reclaims_without_requests() {
    let fixture = support::fixture().await;
    let manager = install(
        &fixture,
        BoundedSessionManager::new(2, Duration::from_millis(150)),
    );
    let running = fixture.start().await;
    let session = id(&initialize(&running).await);
    assert_eq!(manager.inner.sessions.read().await.len(), 1);
    wait_empty(&manager).await;
    assert_eq!(
        manager.bind(&session, "ontoforge-dev|alice"),
        Err(SessionVerifyError::Missing)
    );
    let _ = running.finish().await;
}

#[tokio::test]
async fn real_http_wrong_owner_never_refreshes_ttl() {
    let fixture = support::fixture().await;
    let key = ontoforge_auth::authn::dev::DevKey::load_or_create(
        &camino::Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap(),
    )
    .unwrap();
    let bob = ontoforge_auth::authn::dev::issue_dev_token(
        &key,
        &ontoforge_auth::authn::dev::DevClaims {
            sub: "bob".to_owned(),
            groups: vec!["dispatchers".to_owned()],
            attrs: serde_json::Map::default(),
            ttl: Duration::from_secs(60),
        },
    )
    .unwrap();
    let manager = install(
        &fixture,
        BoundedSessionManager::new(2, Duration::from_millis(150)),
    );
    let running = fixture.start().await;
    let session = id(&initialize(&running).await);
    for _ in 0..12 {
        let response = running
            .client
            .get(format!("{}/mcp", running.url))
            .bearer_auth(&bob)
            .header("accept", "text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .header("mcp-session-id", &session)
            .send()
            .await
            .unwrap();
        assert!(matches!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::NOT_FOUND
        ));
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert_eq!(
        manager.inner.sessions.read().await.len(),
        0,
        "unauthorized traffic must not extend TTL"
    );
    let _ = running.finish().await;
}

#[tokio::test]
async fn real_http_natural_worker_exit_removes_owner_through_wrapper() {
    let fixture = support::fixture().await;
    let mut manager = BoundedSessionManager::new(2, Duration::from_secs(30));
    manager.inner.session_config.keep_alive = Some(Duration::from_millis(100));
    let manager = install(&fixture, manager);
    let running = fixture.start().await;
    let session = id(&initialize(&running).await);
    wait_empty(&manager).await;
    assert_eq!(
        manager.verify(&session, "ontoforge-dev|alice"),
        Err(SessionVerifyError::Missing)
    );
    let _ = running.finish().await;
}

#[tokio::test]
async fn initialization_failure_and_cancelled_request_release_real_sessions() {
    use rmcp::model::GetExtensions;
    let manager = Arc::new(BoundedSessionManager::new(2, Duration::from_secs(30)));
    manager.start();
    let (failed, transport) = manager.create_session().await.unwrap();
    drop(transport);
    let message = serde_json::from_value(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"cancel","version":"0"}}})).unwrap();
    assert!(manager.initialize_session(&failed, message).await.is_err());
    assert_eq!(manager.inner.sessions.read().await.len(), 0);
    let (cancelled, _transport) = manager.create_session().await.unwrap();
    let guard = manager.initialization_guard();
    let mut request = axum::http::Request::new(());
    request.extensions_mut().insert(guard.slot());
    let mut message: rmcp::model::ClientJsonRpcMessage = serde_json::from_value(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"cancel","version":"0"}}})).unwrap();
    if let rmcp::model::ClientJsonRpcMessage::Request(request_message) = &mut message {
        request_message
            .request
            .extensions_mut()
            .insert(request.into_parts().0);
    }
    assert!(
        tokio::time::timeout(
            Duration::from_millis(30),
            manager.initialize_session(&cancelled, message)
        )
        .await
        .is_err()
    );
    drop(guard);
    wait_empty(&manager).await;
    assert_eq!(
        manager.bind(&cancelled, "alice"),
        Err(SessionVerifyError::Missing)
    );
    manager.shutdown().await.unwrap();
}

#[derive(Clone)]
struct ControlledInitialize {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    failure: bool,
}

impl rmcp::ServerHandler for ControlledInitialize {
    async fn initialize(
        &self,
        request: rmcp::model::InitializeRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::InitializeResult, rmcp::ErrorData> {
        self.entered.notify_one();
        if self.failure {
            return Err(rmcp::ErrorData::internal_error("initialize failed", None));
        }
        self.release.notified().await;
        context.peer.set_peer_info(request.clone());
        self.negotiate_initialize(&request)
    }
}

struct HandshakeServer {
    manager: Arc<BoundedSessionManager>,
    client: reqwest::Client,
    url: String,
    token: String,
    handler: ControlledInitialize,
    stop: tokio_util::sync::CancellationToken,
    task: tokio::task::JoinHandle<()>,
    host: Arc<crate::ServerHost<support::ControlledEngine>>,
    _dir: tempfile::TempDir,
}

impl HandshakeServer {
    async fn start(failure: bool) -> Self {
        let fixture = support::fixture().await;
        let manager = install(
            &fixture,
            BoundedSessionManager::new(2, Duration::from_secs(30)),
        );
        let state = crate::state::for_host(&fixture.host);
        manager.start();
        let host = Arc::new(fixture.host);
        let handler = ControlledInitialize {
            entered: Arc::new(tokio::sync::Notify::new()),
            release: Arc::new(tokio::sync::Notify::new()),
            failure,
        };
        let service_handler = handler.clone();
        let service = rmcp::transport::streamable_http_server::StreamableHttpService::new(
            move || Ok(service_handler.clone()),
            Arc::clone(&manager),
            rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default(),
        );
        let router = axum::Router::new()
            .nest_service("/mcp", service)
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&host),
                crate::context::authenticate_mcp::<support::ControlledEngine>,
            ))
            .layer(axum::middleware::from_fn_with_state(
                (
                    Arc::new(tokio::sync::Semaphore::new(4)),
                    Duration::from_millis(300),
                    state,
                ),
                crate::context::admit,
            ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let stop = tokio_util::sync::CancellationToken::new();
        let shutdown = stop.clone();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown.cancelled_owned())
                .await
                .unwrap();
        });
        Self {
            manager,
            client: reqwest::Client::new(),
            url,
            token: fixture.token,
            handler,
            stop,
            task,
            host,
            _dir: fixture.dir,
        }
    }

    fn request(&self) -> reqwest::RequestBuilder {
        self.client.post(&self.url).bearer_auth(&self.token)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"controlled-init","version":"0"}}}))
    }

    async fn finish(self) {
        self.stop.cancel();
        self.task.await.unwrap();
        crate::lifecycle::ServerLifecycle::new(&self.host)
            .shutdown()
            .await
            .unwrap();
        assert_eq!(self.manager.inner.sessions.read().await.len(), 0);
    }
}

#[tokio::test]
async fn real_http_cancelled_initialize_cleans_before_slow_handler_returns() {
    let server = HandshakeServer::start(false).await;
    let request = server.request();
    let pending = tokio::spawn(async move { request.send().await.unwrap() });
    tokio::time::timeout(Duration::from_secs(2), server.handler.entered.notified())
        .await
        .unwrap();
    let session = server
        .manager
        .inner
        .sessions
        .read()
        .await
        .keys()
        .next()
        .unwrap()
        .clone();
    let response = pending.await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::GATEWAY_TIMEOUT);
    wait_empty(&server.manager).await;
    assert_eq!(
        server.manager.bind(&session, "ontoforge-dev|alice"),
        Err(SessionVerifyError::Missing)
    );
    server.handler.release.notify_one();
    let weak = Arc::downgrade(&server.manager);
    server.finish().await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while weak.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("late initialize callback must finish without retaining the manager");
}

#[tokio::test]
async fn real_http_failed_initialize_leaves_no_session() {
    let server = HandshakeServer::start(true).await;
    let response = server.request().send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(
        !status.is_success() || body.contains("error"),
        "{status}: {body}"
    );
    wait_empty(&server.manager).await;
    server.finish().await;
}
