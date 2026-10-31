#[path = "support/real.rs"]
mod support;

#[path = "support/mod.rs"]
mod fixtures;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::printer::l2::McpDefaults;
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_schema::load_scope_dir;
use ontoforge_server::{ServerHost, state};
use ontoforge_storage::reload::{ReloadOptions, reload};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;
use tokio::sync::oneshot;
use tokio_util::task::AbortOnDropHandle;

struct DelayedTrino {
    inner: TrinoEngine,
    delay: AtomicBool,
    entered: Notify,
    release: Notify,
    cancelled: Notify,
}

struct PendingSource<'a> {
    cancelled: &'a Notify,
    released: bool,
}

impl Drop for PendingSource<'_> {
    fn drop(&mut self) {
        if !self.released {
            self.cancelled.notify_one();
        }
    }
}

impl Engine for DelayedTrino {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        if self.delay.load(Ordering::SeqCst) {
            let mut pending = PendingSource {
                cancelled: &self.cancelled,
                released: false,
            };
            self.entered.notify_one();
            self.release.notified().await;
            pending.released = true;
        }
        self.inner.execute(sql, ctx).await
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

async fn finish_statement(mut page: Value) -> Value {
    let client = support::client();
    let mut rows = Vec::new();
    for _ in 0..200 {
        if let Some(data) = page.get_mut("data").and_then(Value::as_array_mut) {
            rows.append(data);
        }
        let Some(next) = page["nextUri"].as_str() else {
            page["data"] = Value::Array(rows);
            return page;
        };
        page = client
            .get(next)
            .send()
            .await
            .expect("Trino next page")
            .error_for_status()
            .expect("Trino page status")
            .json()
            .await
            .expect("Trino page JSON");
    }
    panic!("Trino query exceeded 200 pages");
}

fn parse_rpc(body: &str) -> Value {
    body.lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|value| serde_json::from_str(value).ok())
        })
        .or_else(|| serde_json::from_str(body).ok())
        .unwrap_or(Value::Null)
}

async fn mcp_call(
    server: &str,
    bearer: &str,
    session: &str,
    id: u64,
    name: &str,
    arguments: Value,
) -> Value {
    let response = support::client()
        .post(format!("{server}/mcp"))
        .bearer_auth(bearer)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }))
        .send()
        .await
        .expect("MCP tools/call")
        .text()
        .await
        .expect("MCP response body");
    let payload = parse_rpc(&response);
    assert_eq!(
        payload["jsonrpc"], "2.0",
        "invalid MCP response: {response}"
    );
    payload
}

async fn initialize_mcp(server: &str, bearer: &str) -> (reqwest::Client, String) {
    let client = support::client();
    let response = client
        .post(format!("{server}/mcp"))
        .bearer_auth(bearer)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "phase7-real", "version": "0"}}
        }))
        .send()
        .await
        .expect("MCP initialize");
    assert!(response.status().is_success());
    let session = response
        .headers()
        .get("mcp-session-id")
        .expect("MCP session")
        .to_str()
        .expect("MCP session header")
        .to_owned();
    let initialized = client
        .post(format!("{server}/mcp"))
        .bearer_auth(bearer)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .expect("MCP initialized notification");
    assert!(initialized.status().is_success());
    (client, session)
}

#[tokio::test]
#[ignore = "requires real Trino and an authenticated ontoforge HTTP server"]
async fn trino_http_long_read_releases_snapshot_lock() {
    let trino = support::required("ONTOFORGE_TRINO_URL");
    assert!(trino.starts_with("http://") || trino.starts_with("https://"));
    let info = support::client()
        .get(format!("{trino}/v1/info"))
        .send()
        .await
        .expect("Trino info")
        .json::<serde_json::Value>()
        .await
        .expect("Trino info JSON");
    assert_eq!(info["starting"], false, "{info}");
    assert_eq!(info["nodeVersion"]["version"], "483", "{info}");
    let fixture = fixtures::fixture().await;
    let config = parse_server_config(&format!(
        "[trino]\nurl = \"{trino}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n"
    ))
    .expect("Trino config")
    .trino
    .expect("Trino");
    let engines =
        build_trino_engines(&config, Utf8Path::new("."), &|_: &str| None).expect("Trino engine");
    let engine = Arc::new(DelayedTrino {
        inner: engines.ontology,
        delay: AtomicBool::new(true),
        entered: Notify::new(),
        release: Notify::new(),
        cancelled: Notify::new(),
    });
    let store = Arc::clone(&fixture.store);
    let host = ServerHost {
        store: Arc::clone(&store),
        engine: Arc::clone(&engine),
        auth: fixture.host.auth,
        auth_config: fixture.host.auth_config,
        tokens: fixture.host.tokens,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let server = format!("http://{}", listener.local_addr().expect("address"));
    let (stop, rx) = oneshot::channel();
    let task = AbortOnDropHandle::new(tokio::spawn(Arc::new(host).serve(listener, async {
        let _ = rx.await;
    })));
    let bearer = fixture.token.clone();
    let request_server = server.clone();
    let pending = AbortOnDropHandle::new(tokio::spawn(async move {
        support::get(&request_server, "/api/objects/Flight/F-1001", &bearer).await
    }));
    tokio::time::timeout(Duration::from_secs(5), engine.entered.notified())
        .await
        .expect("authenticated read reaches real source barrier");
    let other = ontoforge_storage::store::EditStore::open(fixtures::settings(&fixture.path))
        .await
        .expect("second metadata connection");
    let snapshot = tokio::time::timeout(Duration::from_secs(3), other.request_snapshot())
        .await
        .expect("metadata lock released before source wait")
        .expect("metadata snapshot");
    let mut ontology = snapshot.active.ontology.clone();
    ontology.description = Some("reload while real Trino source is pending".to_owned());
    tokio::time::timeout(
        Duration::from_secs(3),
        reload(&other, &ontology, &ReloadOptions::default()),
    )
    .await
    .expect("reload during source wait")
    .expect("reload");
    engine.delay.store(false, Ordering::SeqCst);
    engine.release.notify_one();
    let response = tokio::time::timeout(Duration::from_secs(30), pending)
        .await
        .expect("Trino source completed")
        .expect("read task");
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request id")
        .to_owned();
    let (status, object) = support::json(response).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{object}");
    assert_eq!(object["properties"]["flightId"], "F-1001");

    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .expect("audit pool");
    let writes_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .expect("business writes before independent read");
    engine.delay.store(true, Ordering::SeqCst);
    let address = server.strip_prefix("http://").expect("listener URL");
    let mut socket = tokio::net::TcpStream::connect(address)
        .await
        .expect("independent read connection");
    socket
        .write_all(
            format!(
                "GET /api/objects/Flight/F-1001 HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {}\r\n\r\n",
                fixture.token
            )
            .as_bytes(),
        )
        .await
        .expect("independent HTTP read");
    tokio::time::timeout(Duration::from_secs(5), engine.entered.notified())
        .await
        .expect("independent read reaches real source barrier");
    drop(socket);
    let cancelled = tokio::time::timeout(Duration::from_secs(3), engine.cancelled.notified()).await;
    engine.delay.store(false, Ordering::SeqCst);
    engine.release.notify_waiters();
    stop.send(()).expect("stop source host");
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("host shutdown deadline")
        .expect("host task")
        .expect("audit drain");
    assert!(
        cancelled.is_ok(),
        "disconnect must cancel the independent source read"
    );
    let version: i64 =
        sqlx::query_scalar("SELECT def_version FROM of_read_audit WHERE request_id = ?")
            .bind(&request_id)
            .fetch_one(&pool)
            .await
            .expect("durable source audit");
    assert_eq!(version, snapshot.active.def_version);
    let audit_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_read_audit")
        .fetch_one(&pool)
        .await
        .expect("durable audit count");
    assert_eq!(
        audit_rows, 2,
        "one successful read and one cancelled read must each reserve one audit row"
    );
    let success_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM of_read_audit WHERE request_id = ? AND status = 'success' AND row_count = 1",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("durable successful audit");
    assert_eq!(success_audits, 1);
    let cancelled_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM of_read_audit WHERE request_id <> ? AND status = 'cancelled' AND row_count = 0",
    )
    .bind(request_id)
    .fetch_one(&pool)
    .await
    .expect("durable cancellation audit");
    assert_eq!(
        cancelled_audits, 1,
        "the disconnected read must leave exactly one cancelled audit row"
    );
    let writes_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .expect("business writes after cancellation");
    assert_eq!(
        writes_after, writes_before,
        "cancelled reads must not create business writes"
    );
    other
        .read_audit()
        .shutdown()
        .await
        .expect("second store drain");
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires real Trino L2 ACL services and an authenticated ontoforge HTTP server"]
async fn trino_http_and_mcp_six_l2_tools_all_formats() {
    let l2 = support::required("ONTOFORGE_TRINO_L2_URL");
    let loose = support::required("ONTOFORGE_TRINO_L2_LOOSE_URL");

    for url in [&l2, &loose] {
        let info = support::client()
            .get(format!("{url}/v1/info"))
            .send()
            .await
            .expect("L2 Trino info")
            .json::<serde_json::Value>()
            .await
            .expect("L2 Trino info JSON");
        assert_eq!(info["starting"], false, "{info}");
        assert_eq!(info["nodeVersion"]["version"], "483", "{info}");
    }

    let l2_allowed = support::client()
        .post(format!("{l2}/v1/statement"))
        .header("X-Trino-User", "l2")
        .body("SELECT 1 FROM demo.tiny.nation LIMIT 1")
        .send()
        .await
        .expect("L2 allow query")
        .json::<serde_json::Value>()
        .await
        .expect("L2 allow JSON");
    let l2_allowed = finish_statement(l2_allowed).await;
    assert!(l2_allowed.get("error").is_none(), "{l2_allowed}");
    assert!(
        !l2_allowed["data"]
            .as_array()
            .expect("allowed query rows")
            .is_empty()
    );

    let l2_denied = support::client()
        .post(format!("{l2}/v1/statement"))
        .header("X-Trino-User", "l2")
        .body("SELECT 1 FROM outside.tiny.nation LIMIT 1")
        .send()
        .await
        .expect("L2 deny query")
        .json::<serde_json::Value>()
        .await
        .expect("L2 deny JSON");
    let l2_denied = finish_statement(l2_denied).await;
    assert_eq!(
        l2_denied["error"]["errorName"], "PERMISSION_DENIED",
        "{l2_denied}"
    );

    let loose_allowed = support::client()
        .post(format!("{loose}/v1/statement"))
        .header("X-Trino-User", "l2")
        .body("SELECT 1 FROM outside.tiny.nation LIMIT 1")
        .send()
        .await
        .expect("loose ACL query")
        .json::<serde_json::Value>()
        .await
        .expect("loose ACL JSON");
    let loose_allowed = finish_statement(loose_allowed).await;
    assert!(loose_allowed.get("error").is_none(), "{loose_allowed}");
    assert!(
        !loose_allowed["data"]
            .as_array()
            .expect("loose query rows")
            .is_empty()
    );

    let fixture = fixtures::fixture().await;
    let mut scope = load_scope_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/scopes/aviation"
    )))
    .scope
    .expect("aviation scope");
    let maintenance = scope
        .roles
        .values_mut()
        .find(|role| role.api_name.as_str() == "maintenanceLead")
        .expect("maintenance role");
    maintenance.l2.schemas = vec!["demo.tiny".to_owned()];
    let active = fixture.store.active_definition().await.expect("definition");
    let compiled = compile_scope(&ScopeInputs {
        ontology: &active.ontology,
        profile: None,
        attrs: &attr_types(Some(&fixture.host.auth_config)),
        scope: &scope,
        sources: None,
    })
    .expect("compile L2 scope");
    let props = compiled.row_filter_properties();
    let plan = fixture
        .store
        .scope_plan(&scope, &props)
        .await
        .expect("scope plan");
    fixture
        .store
        .scope_apply(&scope, &props, &plan.hash, "real-l2-test")
        .await
        .expect("L2 scope apply");
    let key = DevKey::load_or_create(
        &Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).expect("key path"),
    )
    .expect("key");
    let make_token = |subject: &str| {
        issue_dev_token(
            &key,
            &DevClaims {
                sub: subject.to_owned(),
                groups: vec![],
                attrs: Default::default(),
                ttl: Duration::from_secs(600),
            },
        )
        .expect("real test principal token")
    };
    let bearer = make_token("mia");
    let denied_bearer = make_token("bob");
    let config = parse_server_config(&format!(
        "[trino]\nurl = \"{l2}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl]\nactive_query_sql = \"SELECT count(*) FROM demo.sf100000.orders\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    )).expect("L2 config").trino.expect("Trino");
    let engines =
        build_trino_engines(&config, Utf8Path::new("."), &|_: &str| None).expect("L2 engines");
    let engine = Arc::new(engines.l2.expect("L2 identity"));
    let monitor = Arc::clone(&engine)
        .start_l2_acl_monitor(Arc::new(engines.ontology))
        .await
        .expect("real ACL monitor");
    let status_engine = Arc::clone(&engine);
    let store = Arc::clone(&fixture.host.store);
    let host = ServerHost {
        store: Arc::clone(&store),
        engine,
        auth: fixture.host.auth,
        auth_config: fixture.host.auth_config,
        tokens: fixture.host.tokens,
    }
    .with_l2_status_provider(move || status_engine.l2_status());
    assert!(host.install_l2_monitor(monitor).is_ok());
    state::set_mcp_defaults(
        &store,
        McpDefaults {
            catalog: Some("demo".to_owned()),
            schema: Some("tiny".to_owned()),
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("MCP listener");
    let server = format!("http://{}", listener.local_addr().expect("address"));
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::new(host).serve(listener, async {
        let _ = rx.await;
    }));
    let (_client, session) = initialize_mcp(&server, &bearer).await;
    let (_denied_client, denied_session) = initialize_mcp(&server, &denied_bearer).await;
    for (id, (name, args)) in [
        ("list_catalogs", json!({})),
        ("list_schemas", json!({"catalog":"demo"})),
        ("list_tables", json!({"catalog":"demo","schema":"tiny"})),
        (
            "get_table_schema",
            json!({"table":"nation","catalog":"demo","schema":"tiny"}),
        ),
        (
            "execute_query",
            json!({"query":"SELECT 1 FROM demo.tiny.nation LIMIT 1"}),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let payload = mcp_call(&server, &bearer, &session, id as u64 + 2, name, args).await;
        assert!(payload["error"].is_null(), "{name}: {payload}");
        assert!(payload["result"].is_object(), "{name}: {payload}");
        let denied = mcp_call(
            &server,
            &denied_bearer,
            &denied_session,
            id as u64 + 2,
            name,
            json!({}),
        )
        .await;
        assert!(denied["error"].is_object(), "ungranted {name}: {denied}");
    }
    for (offset, format) in ["LOGICAL", "DISTRIBUTED", "VALIDATE", "IO"]
        .into_iter()
        .enumerate()
    {
        let payload = mcp_call(
            &server,
            &bearer,
            &session,
            20 + offset as u64,
            "explain_query",
            json!({"query":"SELECT 1 FROM demo.tiny.nation LIMIT 1","format":format}),
        )
        .await;
        assert!(payload["error"].is_null(), "{format}: {payload}");
        assert!(payload["result"].is_object(), "{format}: {payload}");
    }
    let denied_explain = mcp_call(
        &server,
        &denied_bearer,
        &denied_session,
        30,
        "explain_query",
        json!({"query":"SELECT 1"}),
    )
    .await;
    assert!(denied_explain["error"].is_object(), "{denied_explain}");
    let denied_catalog = mcp_call(
        &server,
        &bearer,
        &session,
        31,
        "execute_query",
        json!({"query":"SELECT * FROM outside.tiny.nation"}),
    )
    .await;
    assert!(denied_catalog["error"].is_object(), "{denied_catalog}");
    stop.send(()).expect("stop MCP host");
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("host shutdown")
        .expect("host join")
        .expect("monitor and audit drain");
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .expect("audit database");
    let tools: Vec<String> = sqlx::query_scalar("SELECT DISTINCT tool FROM of_read_audit")
        .fetch_all(&pool)
        .await
        .expect("durable audit");
    for tool in [
        "execute_query",
        "explain_query",
        "list_catalogs",
        "list_schemas",
        "list_tables",
        "get_table_schema",
    ] {
        assert!(
            tools.iter().any(|actual| actual == tool),
            "missing durable audit for {tool}: {tools:?}"
        );
    }
    pool.close().await;
}
