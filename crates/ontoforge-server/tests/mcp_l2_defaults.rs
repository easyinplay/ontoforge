use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

#[path = "support/security_evidence.rs"]
mod evidence;
mod support;

struct L2Proof {
    ontology: ontoforge_engine::trino::TrinoEngine,
    l2: Arc<ontoforge_engine::trino::TrinoEngine>,
}

#[derive(Default)]
struct L2Control {
    failure: AtomicBool,
    block: AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    traces: Mutex<Vec<String>>,
    proof: std::sync::OnceLock<L2Proof>,
    all_statements: AtomicUsize,
}

async fn attested_l2(
    store: &ontoforge_storage::store::EditStore,
) -> (
    tokio_util::task::AbortOnDropHandle<()>,
    Arc<Mutex<Vec<String>>>,
    Arc<L2Control>,
) {
    attested_l2_catalog(store, "editstore").await
}

async fn attested_l2_catalog(
    store: &ontoforge_storage::store::EditStore,
    edit_catalog: &'static str,
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
                failure.all_statements.fetch_add(1, Ordering::SeqCst);
                let l2 = headers["x-trino-user"] == "l2";
                let tool_request = headers.get("x-trino-client-tags").and_then(|value| value.to_str().ok()).is_some_and(|tags| tags.split(',').any(|tag| tag == "mcp"));
                if tool_request {
                    recorded.lock().unwrap().push(sql.clone());
                    failure.traces.lock().unwrap().push(headers["x-trino-trace-token"].to_str().unwrap().to_owned());
                }
                if tool_request && failure.block.load(Ordering::SeqCst) {
                    failure.entered.notify_one();
                    failure.release.notified().await;
                }
                let body = if tool_request && failure.failure.load(Ordering::SeqCst) {
                    serde_json::json!({"error":{"errorName":"INTERNAL_ERROR","message":"physical_relation SELECT secret_token=sentinel"}})
                } else if sql.contains("query_id") {
                    serde_json::json!({"columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}],"data":if l2 {serde_json::json!([])} else {serde_json::json!([["probe_1","RUNNING"]])}})
                } else if l2 && !tool_request && (sql.contains("\"outside\"") || sql.contains(&format!("\"{edit_catalog}\""))) {
                    serde_json::json!({"error":{"errorName":"PERMISSION_DENIED","message":"denied"}})
                } else if sql == "SHOW CATALOGS" {
                    serde_json::json!({"columns":[{"name":"Catalog","type":"varchar"}],"data":[["demo"],[edit_catalog],[edit_catalog.to_uppercase()],["system"],["outside"]]})
                } else if sql.starts_with("SHOW SCHEMAS") {
                    serde_json::json!({"columns":[{"name":"Schema","type":"varchar"}],"data":[["aviation"],["information_schema"],["system"]]})
                } else if sql.starts_with("SHOW TABLES") {
                    serde_json::json!({"columns":[{"name":"Table","type":"varchar"}],"data":[["aircraft"],["aircraft$history"]]})
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
    ).replace("catalog = \"editstore\"", &format!("catalog = \"{edit_catalog}\""))).unwrap().trino.unwrap();
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
    state::set_l2_engine(store, Arc::clone(&l2)).unwrap();
    assert!(
        failure
            .proof
            .set(L2Proof {
                ontology: engines.ontology,
                l2
            })
            .is_ok()
    );
    statements.lock().unwrap().clear();
    (serving, statements, failure)
}

#[derive(Clone, Default)]
struct RecordingEngine {
    sql: Arc<Mutex<Vec<String>>>,
    tables: Arc<Mutex<Vec<TableRef>>>,
    status_calls: Arc<AtomicUsize>,
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
        self.status_calls.fetch_add(1, Ordering::SeqCst);
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

#[tokio::test]
async fn mcp_l2_local_rejections_skip_status_execute_and_audit() {
    let engine = RecordingEngine::default();
    let fixture = support::fixture().await;
    let principal = principal();
    let scope = raw_scope();
    let tools = McpL2Tools {
        principal: &principal,
        scope: &scope,
        active: fixture.store.active_definition().await.unwrap(),
        engine: &engine,
        edit_catalog: "editstore",
        store: &fixture.store,
        limits: ServerLimits::default(),
        on_reserved: None,
    };
    let results = [
        tools.execute_query("DROP TABLE demo.tiny.nation").await,
        tools.explain_query("SELECT 1", Some("INVALID")).await,
        tools
            .explain_query("DELETE FROM demo.tiny.nation", None)
            .await,
        tools.list_schemas(None).await,
        tools.list_tables(Some("demo"), None).await,
        tools.list_tables(Some("outside"), Some("tiny")).await,
        tools
            .get_table_schema("outside.tiny.nation", None, None)
            .await,
        tools
            .get_table_schema("nation; DROP TABLE x", Some("demo"), Some("tiny"))
            .await,
    ];
    assert!(results.iter().all(Result::is_err), "{results:?}");
    state::set_mcp_defaults(
        &fixture.store,
        McpDefaults {
            catalog: Some("outside".to_owned()),
            schema: Some("tiny".to_owned()),
        },
    );
    assert!(matches!(
        tools.get_table_schema("nation", None, None).await,
        Err(McpL2Error::Denied)
    ));
    assert!(matches!(
        tools.list_schemas(None).await,
        Err(McpL2Error::Denied)
    ));
    for missing_toolset in [true, false] {
        let mut denied_scope = raw_scope();
        if missing_toolset {
            denied_scope.toolsets.clear();
        } else {
            denied_scope.sql_levels.clear();
        }
        let denied = McpL2Tools {
            scope: &denied_scope,
            active: Arc::clone(&tools.active),
            ..tools
        };
        let results = [
            denied.execute_query("SELECT 1").await,
            denied.explain_query("SELECT 1", None).await,
            denied.list_catalogs().await,
            denied.list_schemas(Some("demo")).await,
            denied.list_tables(Some("demo"), Some("tiny")).await,
            denied
                .get_table_schema("demo.tiny.nation", None, None)
                .await,
        ];
        assert!(
            results
                .iter()
                .all(|result| matches!(result, Err(McpL2Error::Denied)))
        );
    }
    assert_eq!(
        engine.status_calls.load(Ordering::SeqCst),
        0,
        "local validation must precede the network status probe"
    );
    assert!(engine.sql.lock().unwrap().is_empty());
    assert!(engine.tables.lock().unwrap().is_empty());
    fixture.store.read_audit().shutdown().await.unwrap();
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .unwrap();
    let audits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_read_audit")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audits, 0, "local rejections must not reserve read audit");
    pool.close().await;
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

async fn grant_edit_catalog(fixture: &support::Fixture, catalog: &str) {
    let mut scope = fixture.store.request_snapshot().await.unwrap().scope;
    let role = scope.roles.get_mut("maintenanceLead").unwrap();
    role.l2.catalogs.push(catalog.to_owned());
    role.l2.schemas.push(format!("{catalog}.aviation"));
    evidence::apply_scope(&fixture.store, &scope).await;
}

#[tokio::test]
async fn mcp_l2_custom_edit_catalog_denies_before_any_upstream_request() {
    edit_catalog_denials("private_edits").await;
}

#[tokio::test]
async fn mcp_l2_default_edit_catalog_denies_before_any_upstream_request() {
    edit_catalog_denials("editstore").await;
}

async fn edit_catalog_denials(edit_catalog: &'static str) {
    use serde_json::json;
    let mut fixture = support::fixture().await;
    let (_trino, statements, control) = attested_l2_catalog(&fixture.store, edit_catalog).await;
    authorize_l2_fixture(&mut fixture);
    grant_edit_catalog(&fixture, edit_catalog).await;
    state::set_mcp_defaults(
        &fixture.store,
        McpDefaults {
            catalog: Some(edit_catalog.to_owned()),
            schema: Some("aviation".to_owned()),
        },
    );
    let mut running = fixture.start().await;
    running.client = evidence::client();
    evidence::ready(&running.client, &running.url).await;
    let session = initialize_l2(&running).await;
    let probes = [
        (
            "execute_query",
            json!({"query":format!("SELECT * FROM {edit_catalog}.aviation.aircraft")}),
            "INVALID_REQUEST",
        ),
        (
            "execute_query",
            json!({"query":format!("SELECT * FROM \"{}\".other.other_table", edit_catalog.to_uppercase())}),
            "INVALID_REQUEST",
        ),
        (
            "explain_query",
            json!({"query":format!("SELECT * FROM {edit_catalog}.other.other_table")}),
            "INVALID_REQUEST",
        ),
        ("list_schemas", json!({"catalog":edit_catalog}), "NOT_FOUND"),
        ("list_schemas", json!({}), "NOT_FOUND"),
        (
            "list_tables",
            json!({"catalog":edit_catalog,"schema":"other"}),
            "NOT_FOUND",
        ),
        ("list_tables", json!({}), "NOT_FOUND"),
        (
            "get_table_schema",
            json!({"table":format!("{edit_catalog}.other.other_table"),"catalog":"demo","schema":"aviation"}),
            "NOT_FOUND",
        ),
        ("get_table_schema", json!({"table":"aircraft"}), "NOT_FOUND"),
    ];
    let mut failures = Vec::new();
    for (index, (tool, args, code)) in probes.into_iter().enumerate() {
        let before = control.all_statements.load(Ordering::SeqCst);
        let business_before = statements.lock().unwrap().len();
        let source_before = running.engine.calls.load(Ordering::SeqCst);
        let response = l2_request(&running, &session, index as u64 + 2, tool, args).await;
        let upstream = control.all_statements.load(Ordering::SeqCst) - before;
        let business = statements.lock().unwrap().len() - business_before;
        if response["error"]["data"]["code"] != code || upstream != 0 || business != 0 {
            failures.push(format!("{tool}: expected {code}, upstream={upstream}, business={business}, response={response}"));
        }
        assert_eq!(running.engine.calls.load(Ordering::SeqCst), source_before);
    }
    let audit = running.finish().await;
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(audit.is_empty(), "local rejections must not reserve reads");
}

#[tokio::test]
async fn mcp_l2_custom_edit_catalog_filtered_and_read_catalog_still_works() {
    edit_catalog_metadata("private_edits").await;
}

#[tokio::test]
async fn mcp_l2_default_edit_catalog_filtered_and_read_catalog_still_works() {
    edit_catalog_metadata("editstore").await;
}

async fn edit_catalog_metadata(edit_catalog: &'static str) {
    use serde_json::json;
    let mut fixture = support::fixture().await;
    let (_trino, statements, control) = attested_l2_catalog(&fixture.store, edit_catalog).await;
    authorize_l2_fixture(&mut fixture);
    grant_edit_catalog(&fixture, edit_catalog).await;
    let mut running = fixture.start().await;
    running.client = evidence::client();
    evidence::ready(&running.client, &running.url).await;
    let session = initialize_l2(&running).await;
    let before = control.all_statements.load(Ordering::SeqCst);
    let source_before = running.engine.calls.load(Ordering::SeqCst);
    let probes = [
        ("list_catalogs", json!({}), "names", json!(["demo"])),
        ("list_schemas", json!({}), "names", json!(["aviation"])),
        ("list_tables", json!({}), "names", json!(["aircraft"])),
        (
            "get_table_schema",
            json!({"table":"aircraft"}),
            "columns",
            json!([]),
        ),
        (
            "execute_query",
            json!({"query":"SELECT * FROM demo.aviation.aircraft"}),
            "rows",
            json!([[1]]),
        ),
        (
            "explain_query",
            json!({"query":"SELECT * FROM demo.aviation.aircraft"}),
            "rows",
            json!([[1]]),
        ),
    ];
    let mut failures = Vec::new();
    for (index, (tool, args, field, expected)) in probes.into_iter().enumerate() {
        let response = l2_request(&running, &session, index as u64 + 2, tool, args).await;
        if response.get("error").is_some()
            || response["result"]["structuredContent"][field] != expected
        {
            failures.push(format!(
                "{tool}: expected {field}={expected}, response={response}"
            ));
        }
    }
    assert_eq!(statements.lock().unwrap().len(), 6);
    assert_eq!(control.all_statements.load(Ordering::SeqCst), before + 12);
    assert_eq!(running.engine.calls.load(Ordering::SeqCst), source_before);
    let sql = statements.lock().unwrap().clone();
    assert_eq!(sql[0], "SHOW CATALOGS");
    assert_eq!(sql[1], "SHOW SCHEMAS FROM \"demo\"");
    assert_eq!(sql[2], "SHOW TABLES FROM \"demo\".\"aviation\"");
    assert!(sql[3].contains("\"demo\".\"information_schema\".\"columns\""));
    assert!(sql.iter().all(|sql| !sql.contains(edit_catalog)));
    let audit = running.finish().await;
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(audit.len(), 6);
    assert!(audit.iter().all(|row| row["status"] == "success"));
}

#[tokio::test]
async fn mcp_l2_without_trino_remains_unavailable_for_all_six_tools() {
    use serde_json::json;
    let mut fixture = support::fixture().await;
    authorize_l2_fixture(&mut fixture);
    let mut running = fixture.start().await;
    running.client = evidence::client();
    evidence::ready(&running.client, &running.url).await;
    let session = initialize_l2(&running).await;
    let source_before = running.engine.calls.load(Ordering::SeqCst);
    for (index, (tool, args)) in [
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
        let response = l2_request(&running, &session, index as u64 + 2, tool, args).await;
        assert_eq!(
            response["error"]["data"]["code"], "SOURCE_UNAVAILABLE",
            "{tool}: {response}"
        );
        assert_eq!(running.engine.calls.load(Ordering::SeqCst), source_before);
    }
    assert!(running.finish().await.is_empty());
}

#[tokio::test]
async fn mcp_l2_typed_explain_concurrency_limit_releases_permit() {
    for format in ["LOGICAL", "DISTRIBUTED", "VALIDATE", "IO"] {
        typed_explain_concurrency_limit(format).await;
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "keep typed request admission, saturation, release and audit in temporal order"
)]
async fn typed_explain_concurrency_limit(format: &'static str) {
    use serde_json::json;
    use std::time::Duration;
    let mut fixture = support::fixture().await;
    let (_trino, statements, control) = attested_l2(&fixture.store).await;
    authorize_l2_fixture(&mut fixture);
    let mut running = fixture
        .start_with_limits(ServerLimits {
            max_concurrent_requests: 1,
            ..ServerLimits::default()
        })
        .await;
    running.client = evidence::client();
    let session = evidence::session(&running.client, &running.url, &running.token).await;
    let baseline = control.all_statements.load(Ordering::SeqCst);
    control.block.store(true, Ordering::SeqCst);
    let pending = {
        let client = running.client.clone();
        let url = running.url.clone();
        let token = running.token.clone();
        let session = session.clone();
        tokio::spawn(async move {
            evidence::rpc(&client, &url, &token, &session, json!({
                "jsonrpc":"2.0", "id":2, "method":"tools/call",
                "params":{"name":"explain_query", "arguments":{"query":"SELECT 1", "format":format}}
            })).await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), control.entered.notified())
        .await
        .expect("typed EXPLAIN reached upstream while holding the only permit");
    let source_calls = running.engine.calls.load(Ordering::SeqCst);
    assert_eq!(statements.lock().unwrap().len(), 1);
    assert_eq!(control.all_statements.load(Ordering::SeqCst), baseline + 2);
    let denied = running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .timeout(Duration::from_secs(2))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0", "id":3, "method":"tools/call",
            "params":{"name":"explain_query", "arguments":{"query":"SELECT 1", "format":format}}
        }))
        .send()
        .await
        .expect("saturated admission must reject without waiting for upstream");
    assert_eq!(denied.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
    let denied: serde_json::Value = denied.json().await.unwrap();
    assert_eq!(denied["code"], "CONCURRENCY_LIMIT", "{format}: {denied}");
    assert!(
        !pending.is_finished(),
        "the first request still owns the permit"
    );
    assert_eq!(
        statements.lock().unwrap().len(),
        1,
        "rejection sends no L2 SQL"
    );
    assert_eq!(
        control.all_statements.load(Ordering::SeqCst),
        baseline + 2,
        "rejection sends nothing with any identity or client tags"
    );
    assert_eq!(
        running.engine.calls.load(Ordering::SeqCst),
        source_calls,
        "rejection does not reroute to the source engine"
    );
    control.block.store(false, Ordering::SeqCst);
    control.release.notify_one();
    let first = tokio::time::timeout(Duration::from_secs(3), pending)
        .await
        .expect("first request completes after explicit release")
        .unwrap();
    assert!(
        first["result"].is_object() && first.get("error").is_none(),
        "{first}"
    );
    let third = l2_request(
        &running,
        &session,
        4,
        "explain_query",
        json!({"query":"SELECT 1", "format":format}),
    )
    .await;
    assert!(
        third["result"].is_object() && third.get("error").is_none(),
        "{third}"
    );
    assert_eq!(control.all_statements.load(Ordering::SeqCst), baseline + 4);
    assert_eq!(running.engine.calls.load(Ordering::SeqCst), source_calls);
    let sql = statements.lock().unwrap().clone();
    assert_eq!(sql.len(), 2);
    assert!(
        sql.iter()
            .all(|sql| sql.starts_with(&format!("EXPLAIN (TYPE {format}) SELECT 1")))
    );
    let trace_ids = control.traces.lock().unwrap().clone();
    assert_eq!(trace_ids.len(), 2);
    assert_ne!(trace_ids[0], trace_ids[1]);
    let audit = running.finish().await;
    assert_eq!(
        audit.len(),
        2,
        "admission rejection must not reserve a read audit"
    );
    for id in trace_ids {
        let rows: Vec<_> = audit.iter().filter(|row| row["request_id"] == id).collect();
        assert_eq!(rows.len(), 1, "one audit per accepted typed request");
        assert_eq!(rows[0]["status"], "success");
        assert_eq!(rows[0]["tool"], "explain_query");
        assert_eq!(rows[0]["row_count"], 1);
        assert_ne!(denied["request_id"], id);
    }
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "keep same-session revoke and recovery matrix together"
)]
async fn mcp_l2_same_session_revoke_defaults_and_qualified_zero_send() {
    use serde_json::json;
    let mut fixture = support::fixture().await;
    let (_trino, statements, control) = attested_l2(&fixture.store).await;
    authorize_l2_fixture(&mut fixture);
    let original = fixture.store.request_snapshot().await.unwrap().scope;
    let mut running = fixture.start().await;
    running.client = evidence::client();
    let session = evidence::session(&running.client, &running.url, &running.token).await;
    let before = evidence::counts(&running.path).await;
    let probes = [
        ("execute_query", json!({"query":"SELECT 1"})),
        ("get_table_schema", json!({"table":"aircraft"})),
        (
            "get_table_schema",
            json!({"table":"demo.aviation.aircraft","catalog":"outside","schema":"secret"}),
        ),
        ("list_tables", json!({})),
    ];
    let mut id = 2;
    for revoke in ["toolset", "sql_level", "binding", "allowlist"] {
        let mut scope = original.clone();
        let role = scope.roles.get_mut("maintenanceLead").unwrap();
        match revoke {
            "toolset" => {
                role.toolsets.retain(|toolset| *toolset != Toolset::SqlRaw);
            }
            "sql_level" => {
                role.sql_levels.clear();
                role.toolsets.retain(|toolset| *toolset != Toolset::SqlRaw);
                role.l2.catalogs.clear();
                role.l2.schemas.clear();
            }
            "binding" => {
                scope.bindings.clear();
            }
            "allowlist" => {
                role.l2.catalogs.clear();
                role.l2.schemas.clear();
            }
            _ => unreachable!(),
        }
        evidence::apply_scope(&running.store, &scope).await;
        let count = statements.lock().unwrap().len();
        let all_requests = control.all_statements.load(Ordering::SeqCst);
        let source_calls = running.engine.calls.load(Ordering::SeqCst);
        for (name, args) in &probes {
            if revoke == "allowlist" && *name == "execute_query" {
                continue;
            }
            let denied = l2_request(&running, &session, id, name, args.clone()).await;
            id += 1;
            assert!(denied["error"].is_object(), "{revoke} {name}: {denied}");
            assert_eq!(
                statements.lock().unwrap().len(),
                count,
                "{revoke} {name} must reject before send"
            );
            assert_eq!(
                control.all_statements.load(Ordering::SeqCst),
                all_requests,
                "{revoke} {name}: all identities and tags must remain silent"
            );
            assert_eq!(
                running.engine.calls.load(Ordering::SeqCst),
                source_calls,
                "denied L2 must not reroute to the source engine"
            );
        }
        let health = running
            .get("/health")
            .await
            .json::<serde_json::Value>()
            .await
            .unwrap();
        assert_eq!(
            health["l2"]["status"], "enabled",
            "principal revoke must not alter shared ACL proof: {health}"
        );
        evidence::apply_scope(&running.store, &original).await;
        let recovered = l2_request(
            &running,
            &session,
            id,
            "get_table_schema",
            json!({"table":"aircraft"}),
        )
        .await;
        id += 1;
        assert!(recovered.get("error").is_none(), "{revoke}: {recovered}");
        assert_eq!(
            statements.lock().unwrap().len(),
            count + 1,
            "same session must regain current grant"
        );
    }
    assert_eq!(evidence::counts(&running.path).await, before);
    let audit = running.finish().await;
    assert_eq!(audit.len(), 4, "only four recovered calls reserve reads");
    assert!(audit.iter().all(|row| row["status"] == "success"));
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "keep real proof expiry and recovery assertions in temporal order"
)]
async fn mcp_l2_same_session_real_proof_expiry_zero_send_and_recovery() {
    use ontoforge_engine::trino::l2_health::L2HealthStatus;
    use serde_json::json;
    let mut fixture = support::fixture().await;
    let (_trino, statements, control) = attested_l2(&fixture.store).await;
    authorize_l2_fixture(&mut fixture);
    let mut running = fixture.start().await;
    running.client = evidence::client();
    let session = evidence::session(&running.client, &running.url, &running.token).await;
    let admitted = l2_request(
        &running,
        &session,
        2,
        "execute_query",
        json!({"query":"SELECT 1"}),
    )
    .await;
    assert!(admitted.get("error").is_none(), "{admitted}");
    assert_eq!(statements.lock().unwrap().len(), 1);
    let health = running
        .get("/health")
        .await
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(health["l2"]["status"], "enabled");
    // Production attestation uses std::time::Instant and a fixed sixty-second TTL.
    tokio::time::sleep(std::time::Duration::from_secs(61)).await;
    let health = running
        .get("/health")
        .await
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(health["status"], "degraded", "{health}");
    assert_eq!(health["l2"]["reason"], "EXPIRED", "{health}");
    assert_eq!(health["engineStatus"], "available", "{health}");
    let count = statements.lock().unwrap().len();
    let all_requests = control.all_statements.load(Ordering::SeqCst);
    let source_calls = running.engine.calls.load(Ordering::SeqCst);
    for (offset, (name, args)) in [
        ("execute_query", json!({"query":"SELECT 1"})),
        ("list_tables", json!({})),
        (
            "get_table_schema",
            json!({"table":"demo.aviation.aircraft"}),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let denied = l2_request(&running, &session, offset as u64 + 3, name, args).await;
        assert_eq!(
            denied["error"]["data"]["code"], "SOURCE_UNAVAILABLE",
            "{denied}"
        );
        assert_eq!(
            statements.lock().unwrap().len(),
            count,
            "expired production proof must prevent MCP upstream sends"
        );
        assert_eq!(
            control.all_statements.load(Ordering::SeqCst),
            all_requests,
            "proof expiry must prevent requests with any identity or tags"
        );
        assert_eq!(running.engine.calls.load(Ordering::SeqCst), source_calls);
    }
    let proof = control.proof.get().unwrap();
    assert_eq!(
        proof.l2.recheck_l2_acl(&proof.ontology, "probe_1").await,
        L2HealthStatus::Enabled
    );
    let health = running
        .get("/health")
        .await
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(health["status"], "healthy", "{health}");
    assert_eq!(health["l2"]["status"], "enabled");
    let recovered = l2_request(
        &running,
        &session,
        6,
        "get_table_schema",
        json!({"table":"demo.aviation.aircraft"}),
    )
    .await;
    assert!(recovered.get("error").is_none(), "{recovered}");
    assert_eq!(statements.lock().unwrap().len(), count + 1);
    let audit = running.finish().await;
    assert_eq!(audit.len(), 2, "only admitted calls reserve reads");
    assert!(audit.iter().all(|row| row["status"] == "success"));
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

async fn wait_for_live_cancelled_audit(running: &support::Running, request_id: &str) -> bool {
    use std::time::Duration;
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &running.path,
    ))
    .await
    .unwrap();
    let observed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let rows: Vec<(String, String)> =
                sqlx::query_as("SELECT request_id, status FROM of_read_audit")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            if rows.len() == 1 && rows[0].0 == request_id && rows[0].1 == "cancelled" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .is_ok();
    pool.close().await;
    observed
}

async fn notify_cancelled(running: &support::Running, session: &str) {
    let response = running.client.post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token).header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26").header("mcp-session-id", session)
        .json(&serde_json::json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}))
        .send().await.unwrap();
    assert!(response.status().is_success());
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
            let audit_id = {
                let traces = control.traces.lock().unwrap();
                assert_eq!(traces.len(), 1);
                traces[0].clone()
            };
            if !shutdown {
                notify_cancelled(&running, &session).await;
                let cancelled = wait_for_live_cancelled_audit(&running, &audit_id).await;
                let health = tokio::time::timeout(Duration::from_secs(2), running.get("/health"))
                    .await
                    .expect("server must remain online before shutdown");
                assert!(health.status().is_success());
                if !cancelled {
                    running.finish().await;
                    panic!(
                        "no unique cancelled audit for {audit_id} while server was running; shutdown cannot supply cancellation evidence"
                    );
                }
                eprintln!(
                    "{tool}: unique cancelled audit {audit_id} observed before shutdown; health remains online"
                );
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
            assert_eq!(audit[0]["request_id"], audit_id);
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
            edit_catalog: "editstore",
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
        edit_catalog: "editstore",
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
        edit_catalog: "editstore",
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
