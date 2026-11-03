use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use camino::Utf8PathBuf;
use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_auth::{Principal, PrincipalId};
use ontoforge_engine::printer::l2::McpDefaults;
use ontoforge_engine::{
    Column, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PhysicalType, PrintedSql,
    QueryContext, QueryOutput, TableRef, Value,
};
use ontoforge_schema::ir::{SqlLevel, Toolset};
use ontoforge_server::mcp::mcp_l2::{McpL2Error, McpL2Tools};
use ontoforge_server::{ServerLimits, state};

mod support;

#[derive(Default)]
struct L2Control {
    failure: AtomicBool,
    block: AtomicBool,
    entered: tokio::sync::Notify,
    traces: Mutex<Vec<String>>,
}

async fn attested_l2(
    store: &ontoforge_storage::store::EditStore,
) -> (
    tokio_util::task::AbortOnDropHandle<()>,
    Arc<Mutex<Vec<String>>>,
    Arc<L2Control>,
) {
    let statements = Arc::new(Mutex::new(Vec::<String>::new()));
    let recorded = Arc::clone(&statements);
    let failure = Arc::new(L2Control::default());
    let server_failure = Arc::clone(&failure);
    let app = axum::Router::new().route(
        "/v1/statement",
        axum::routing::post(move |headers: axum::http::HeaderMap, sql: String| {
            let recorded = Arc::clone(&recorded);
            let failure = Arc::clone(&server_failure);
            async move {
                let l2 = headers["x-trino-user"] == "l2";
                let tool_request = headers.get("x-trino-client-tags").and_then(|value| value.to_str().ok()).is_some_and(|tags| tags.split(',').any(|tag| tag == "mcp"));
                if tool_request {
                    recorded.lock().unwrap().push(sql.clone());
                    failure.traces.lock().unwrap().push(headers["x-trino-trace-token"].to_str().unwrap().to_owned());
                }
                if tool_request && failure.block.load(Ordering::SeqCst) {
                    failure.entered.notify_one();
                    std::future::pending::<()>().await;
                }
                let body = if tool_request && failure.failure.load(Ordering::SeqCst) {
                    serde_json::json!({"error":{"errorName":"INTERNAL_ERROR","message":"physical_relation SELECT secret_token=sentinel"}})
                } else if sql.contains("query_id") {
                    serde_json::json!({"columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}],"data":if l2 {serde_json::json!([])} else {serde_json::json!([["probe_1","RUNNING"]])}})
                } else if l2 && (sql.contains("\"outside\"") || sql.contains("\"editstore\"")) {
                    serde_json::json!({"error":{"errorName":"PERMISSION_DENIED","message":"denied"}})
                } else if sql.contains("information_schema") {
                    serde_json::json!({"columns":[],"data":[]})
                } else {
                    serde_json::json!({"columns":[{"name":"value","type":"bigint"}],"data":[[1]]})
                };
                axum::Json(body)
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let serving = tokio_util::task::AbortOnDropHandle::new(tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    }));
    let config = ontoforge_engine::config::parse_server_config(&format!(
        "[trino]\nurl = \"{url}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    )).unwrap().trino.unwrap();
    let engines =
        ontoforge_engine::config::build_trino_engines(&config, camino::Utf8Path::new("."), &|_| {
            None
        })
        .unwrap();
    let l2 = Arc::new(engines.l2.unwrap());
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "probe_1").await,
        ontoforge_engine::trino::l2_health::L2HealthStatus::Enabled,
    );
    state::set_l2_engine(store, l2).unwrap();
    statements.lock().unwrap().clear();
    (serving, statements, failure)
}

#[derive(Clone, Default)]
struct RecordingEngine {
    sql: Arc<Mutex<Vec<String>>>,
    tables: Arc<Mutex<Vec<TableRef>>>,
    block: Arc<AtomicBool>,
    entered: Arc<tokio::sync::Notify>,
}

impl Engine for RecordingEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Trino
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        _ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.sql.lock().unwrap().push(sql.as_str().to_owned());
        if self.block.load(Ordering::SeqCst) {
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(QueryOutput {
            columns: vec![Column {
                name: "name".to_owned(),
                physical: PhysicalType::Varchar { length: None },
            }],
            rows: vec![vec![Value::String("demo".to_owned())]],
            truncated: false,
        })
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        _ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.tables.lock().unwrap().extend(tables.iter().cloned());
        if self.block.load(Ordering::SeqCst) {
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(tables
            .iter()
            .cloned()
            .map(|table| {
                (
                    table,
                    vec![PhysicalColumn {
                        name: "id".to_owned(),
                        physical: PhysicalType::Varchar { length: None },
                        nullable: false,
                    }],
                )
            })
            .collect())
    }

    fn status(&self) -> impl Future<Output = EngineStatus> + Send {
        std::future::ready(EngineStatus::Available)
    }
}

#[expect(
    clippy::default_trait_access,
    reason = "Attrs is not re-exported by the public auth API"
)]
fn principal() -> Principal {
    Principal {
        id: PrincipalId::new("ontoforge-dev", "alice"),
        issuer: "ontoforge-dev".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Default::default(),
    }
}

fn raw_scope() -> PrincipalScope {
    PrincipalScope {
        toolsets: [Toolset::SqlRaw].into_iter().collect(),
        sql_levels: [SqlLevel::Raw].into_iter().collect(),
        l2_catalogs: ["demo".to_owned()].into_iter().collect(),
        l2_schemas: ["demo.tiny".to_owned()].into_iter().collect(),
        ..PrincipalScope::default()
    }
}

async fn l2_request(
    running: &support::Running,
    session: &str,
    id: u64,
    name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    let body = running.client.post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
        .json(&serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}}))
        .send().await.unwrap().text().await.unwrap();
    body.lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|data| serde_json::from_str(data).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .expect("MCP response")
}

async fn initialize_l2(running: &support::Running) -> String {
    let initialized = running.client.post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token).header("accept", "application/json, text/event-stream")
        .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"l2-errors","version":"0"}}}))
        .send().await.unwrap();
    initialized.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned()
}

fn authorize_l2_fixture(fixture: &mut support::Fixture) {
    let key_path = Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap();
    let key = DevKey::load_or_create(&key_path).unwrap();
    fixture.token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "mia".to_owned(),
            groups: Vec::new(),
            attrs: serde_json::Map::new(),
            ttl: std::time::Duration::from_secs(600),
        },
    )
    .unwrap();
    state::set_mcp_defaults(
        &fixture.store,
        McpDefaults {
            catalog: Some("demo".to_owned()),
            schema: Some("aviation".to_owned()),
        },
    );
}

#[tokio::test]
async fn mcp_l2_errors_are_safe_and_preserve_reserved_audit_ids() {
    use serde_json::json;
    let mut fixture = support::fixture().await;
    let (_trino, statements, failure) = attested_l2(&fixture.store).await;
    authorize_l2_fixture(&mut fixture);
    let running = fixture.start().await;
    let session = initialize_l2(&running).await;
    failure.failure.store(true, Ordering::SeqCst);
    let mut reserved = Vec::new();
    for (offset, (name, arguments)) in [
        ("execute_query", json!({"query":"SELECT 1"})),
        ("explain_query", json!({"query":"SELECT 1"})),
        ("list_catalogs", json!({})),
        ("list_schemas", json!({})),
        ("list_tables", json!({})),
        ("get_table_schema", json!({"table":"aircraft"})),
    ]
    .into_iter()
    .enumerate()
    {
        let response = l2_request(&running, &session, offset as u64 + 2, name, arguments).await;
        assert_eq!(
            response["error"]["data"]["code"], "SOURCE_UNAVAILABLE",
            "{response}"
        );
        assert_eq!(response["error"]["data"]["apiName"], name);
        for secret in [
            "physical_relation",
            "secret_token",
            "sentinel",
            "INTERNAL_ERROR",
        ] {
            assert!(!response.to_string().contains(secret), "{response}");
        }
        reserved.push((
            name,
            response["error"]["data"]["request_id"]
                .as_str()
                .unwrap()
                .to_owned(),
        ));
    }
    let sent = statements.lock().unwrap().len();
    let invalid = l2_request(
        &running,
        &session,
        8,
        "explain_query",
        json!({"query":"SELECT 1", "format":"secret_token=sentinel"}),
    )
    .await;
    assert_eq!(
        invalid["error"]["data"]["code"], "INVALID_REQUEST",
        "{invalid}"
    );
    assert!(!invalid.to_string().contains("secret_token"));
    assert!(
        invalid["error"]["data"]["request_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(statements.lock().unwrap().len(), sent);
    running.store.read_audit().shutdown().await.unwrap();
    let unavailable = l2_request(
        &running,
        &session,
        9,
        "execute_query",
        json!({"query":"SELECT 1"}),
    )
    .await;
    assert_eq!(
        unavailable["error"]["data"]["code"], "AUDIT_UNAVAILABLE",
        "{unavailable}"
    );
    assert_eq!(statements.lock().unwrap().len(), sent);
    let audit = running.finish().await;
    assert_eq!(audit.len(), reserved.len());
    for (tool, id) in reserved {
        assert!(audit.iter().any(|row| row["request_id"] == id
            && row["tool"] == tool
            && row["status"] == "source_error"));
        assert_ne!(invalid["error"]["data"]["request_id"], id);
        assert_ne!(unavailable["error"]["data"]["request_id"], id);
    }
}

#[tokio::test]
async fn mcp_l2_cancellation_and_shutdown_keep_reserved_audit_ids() {
    use serde_json::{Value as JsonValue, json};
    use std::time::Duration;
    for shutdown in [false, true] {
        for (tool, arguments) in [
            ("execute_query", json!({"query":"SELECT 1"})),
            ("get_table_schema", json!({"table":"aircraft"})),
        ] {
            let mut fixture = support::fixture().await;
            let (_trino, _, control) = attested_l2(&fixture.store).await;
            authorize_l2_fixture(&mut fixture);
            let running = fixture.start().await;
            let session = initialize_l2(&running).await;
            control.block.store(true, Ordering::SeqCst);
            let request = running.client.post(format!("{}/mcp", running.url))
                .bearer_auth(&running.token).header("accept", "application/json, text/event-stream")
                .header("mcp-protocol-version", "2025-03-26").header("mcp-session-id", &session)
                .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":tool,"arguments":arguments}}));
            let pending =
                tokio::spawn(async move { request.send().await.unwrap().text().await.unwrap() });
            tokio::time::timeout(Duration::from_secs(3), control.entered.notified())
                .await
                .expect("L2 source entered after audit reservation");
            if !shutdown {
                let cancelled = running.client.post(format!("{}/mcp", running.url))
                    .bearer_auth(&running.token).header("accept", "application/json, text/event-stream")
                    .header("mcp-protocol-version", "2025-03-26").header("mcp-session-id", &session)
                    .json(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}))
                    .send().await.unwrap();
                assert!(cancelled.status().is_success());
            }
            let stopping = tokio::spawn(running.finish());
            let body = tokio::time::timeout(Duration::from_secs(7), pending)
                .await
                .unwrap()
                .unwrap();
            let response: Option<JsonValue> = body
                .lines()
                .find_map(|line| {
                    line.strip_prefix("data: ")
                        .and_then(|data| serde_json::from_str(data).ok())
                })
                .or_else(|| serde_json::from_str(&body).ok());
            let code = if shutdown {
                "SERVER_SHUTTING_DOWN"
            } else {
                "CANCELLED"
            };
            let audit = stopping.await.unwrap();
            assert_eq!(audit.len(), 1);
            assert_eq!(audit[0]["status"], "cancelled");
            assert_eq!(audit[0]["request_id"], control.traces.lock().unwrap()[0]);
            if let Some(response) = response {
                assert_eq!(response["error"]["data"]["code"], code, "{response}");
                assert_eq!(
                    response["error"]["data"]["request_id"],
                    audit[0]["request_id"]
                );
            }
        }
    }
}

#[tokio::test]
async fn l2_reservation_observer_runs_before_source_await_and_survives_drop() {
    for schema in [false, true] {
        let fixture = support::fixture().await;
        let engine = RecordingEngine::default();
        engine.block.store(true, Ordering::SeqCst);
        let observed = Mutex::new(None::<String>);
        let record_id = |id: &str| *observed.lock().unwrap() = Some(id.to_owned());
        let active = fixture.store.active_definition().await.unwrap();
        let principal = principal();
        let scope = raw_scope();
        let tools = McpL2Tools {
            principal: &principal,
            scope: &scope,
            active,
            engine: &engine,
            store: &fixture.store,
            limits: ServerLimits::default(),
            on_reserved: Some(&record_id),
        };
        {
            let request = async {
                if schema {
                    tools.get_table_schema("demo.tiny.nation", None, None).await
                } else {
                    tools.execute_query("SELECT 1").await
                }
            };
            tokio::pin!(request);
            tokio::select! {
                result = &mut request => panic!("source should be pending: {result:?}"),
                () = engine.entered.notified() => {},
            }
            assert!(
                observed.lock().unwrap().is_some(),
                "reservation must be observable before the pending source resolves"
            );
        }
        let id = observed.lock().unwrap().clone().unwrap();
        let audit = fixture.start().await.finish().await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0]["request_id"], id);
        assert_eq!(audit[0]["status"], "cancelled");
    }
}

#[tokio::test]
async fn mcp_l2_optional_context_defaults_and_qualified_override() {
    let engine = RecordingEngine::default();
    let fixture = support::fixture().await;
    let active = fixture.store.active_definition().await.unwrap();
    let principal = principal();
    let mut scope = raw_scope();
    scope.l2_catalogs.insert("other".to_owned());
    scope.l2_schemas.insert("other.空间".to_owned());
    state::set_mcp_defaults(
        &fixture.store,
        McpDefaults {
            catalog: Some("demo".to_owned()),
            schema: Some("tiny".to_owned()),
        },
    );
    let tools = McpL2Tools {
        principal: &principal,
        scope: &scope,
        active,
        engine: &engine,
        store: &fixture.store,
        limits: ServerLimits::default(),
        on_reserved: None,
    };

    let defaulted = tools
        .get_table_schema("航班", None, None)
        .await
        .expect("deployment defaults qualify a bare table");
    assert_eq!(defaulted["columns"][0]["name"], "id");
    assert_eq!(
        engine.tables.lock().unwrap().as_slice(),
        &[TableRef {
            catalog: "demo".to_owned(),
            schema: "tiny".to_owned(),
            table: "航班".to_owned(),
        }]
    );
    engine.tables.lock().unwrap().clear();

    let result = tools
        .get_table_schema("\"other\".\"空间\".\"航班\"", Some("demo"), Some("tiny"))
        .await;
    assert!(
        result.is_ok(),
        "qualified table should be accepted: {result:?}"
    );
    assert_eq!(
        engine.tables.lock().unwrap().as_slice(),
        &[TableRef {
            catalog: "other".to_owned(),
            schema: "空间".to_owned(),
            table: "航班".to_owned(),
        }]
    );

    let schemas = tools
        .list_schemas(None)
        .await
        .expect("deployment catalog default");
    assert_eq!(schemas["names"], serde_json::json!(["demo"]));
    let tables = tools
        .list_tables(None, None)
        .await
        .expect("deployment catalog/schema defaults");
    assert_eq!(tables["names"], serde_json::json!(["demo"]));
}

#[tokio::test]
async fn mcp_l2_missing_context_and_outside_allowlist_deny_before_send() {
    let engine = RecordingEngine::default();
    let fixture = support::fixture().await;
    let active = fixture.store.active_definition().await.unwrap();
    let principal = principal();
    let mut scope = raw_scope();
    scope.l2_catalogs.clear();
    scope.l2_schemas.clear();
    let tools = McpL2Tools {
        principal: &principal,
        scope: &scope,
        active,
        engine: &engine,
        store: &fixture.store,
        limits: ServerLimits::default(),
        on_reserved: None,
    };

    let missing = tools.list_schemas(None).await;
    assert_eq!(
        missing
            .as_ref()
            .map_err(ToString::to_string)
            .expect_err("missing catalog context must be rejected"),
        "missing trusted L2 context: catalog"
    );
    assert!(engine.sql.lock().unwrap().is_empty());

    let denied = tools
        .get_table_schema("outside", Some("other"), Some("secret"))
        .await;
    assert!(matches!(denied, Err(McpL2Error::Denied)));
    assert!(engine.sql.lock().unwrap().is_empty());

    let invalid = tools
        .get_table_schema("tiny; DROP TABLE secret", Some("demo"), Some("tiny"))
        .await;
    assert!(matches!(invalid, Err(McpL2Error::Invalid(_))));
    assert!(engine.tables.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mcp_l2_defaults_real_listener_consumes_deployment_context() {
    let mut fixture = support::fixture().await;
    let (_trino, statements, _) = attested_l2(&fixture.store).await;
    let key_path = Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap();
    let key = DevKey::load_or_create(&key_path).unwrap();
    fixture.token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "mia".to_owned(),
            groups: Vec::new(),
            attrs: serde_json::Map::new(),
            ttl: std::time::Duration::from_secs(600),
        },
    )
    .unwrap();
    state::set_mcp_defaults(
        &fixture.store,
        McpDefaults {
            catalog: Some("demo".to_owned()),
            schema: Some("aviation".to_owned()),
        },
    );
    let running = fixture.start().await;
    let client = reqwest::Client::new();
    let url = format!("{}/mcp", running.url);
    let initialize = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "defaults", "version": "0"}}
        }))
        .send()
        .await
        .unwrap();
    let session = initialize
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let call = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": "get_table_schema", "arguments": {"table": "aircraft"}}
        }))
        .send()
        .await
        .unwrap();
    let body = call.text().await.unwrap();
    assert!(
        body.contains("columns"),
        "listener response must contain schema: {body}"
    );
    assert!(
        !body.contains("catalog is required"),
        "deployment defaults were not consumed: {body}"
    );
    assert!(
        statements.lock().unwrap().iter().any(|sql| {
            sql.contains("\"demo\".\"information_schema\".\"columns\"")
                && sql.contains("'aviation'")
                && sql.contains("'aircraft'")
        }),
        "metadata SQL must consume the trusted deployment defaults"
    );
    let _ = running.finish().await;
}
