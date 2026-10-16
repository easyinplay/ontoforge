//! Protocol tests for the Trino client against a wiremock server.

use std::time::Duration;

use ontoforge_engine::printer::ast::{Expr, Literal, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{
    Column, Dialect, Engine, EngineError, PhysicalType, PrintedSql, QueryContext, QueryOutput,
    Value,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn select_one() -> PrintedSql {
    let select = Select {
        projection: vec![SelectItem {
            expr: Expr::Literal(Literal::Long(1)),
            alias: None,
        }],
        from: None,
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    print_select(&select, Dialect::Trino).expect("printable")
}

fn connection(url: &str) -> TrinoConnection {
    TrinoConnection {
        url: reqwest::Url::parse(url).expect("url"),
        user: "ontoforge".to_owned(),
        auth: Auth::None,
        ca_pem: None,
        connect_timeout: Duration::from_secs(5),
        max_page_bytes: 64 * 1024 * 1024,
        extra_origins: Vec::new(),
    }
}

fn engine_for(server: &MockServer) -> TrinoEngine {
    TrinoEngine::new(connection(&server.uri())).expect("engine")
}

fn ctx() -> QueryContext {
    let mut ctx = QueryContext::new(
        "trace-1".to_owned(),
        vec!["ontoforge".to_owned(), "validate-online".to_owned()],
    );
    ctx.timeout = Duration::from_secs(30);
    ctx
}

async fn mount_post(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn mount_get(server: &MockServer, page: &str, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path(format!("/v1/statement/q1/{page}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

fn next(server: &MockServer, page: &str) -> String {
    format!("{}/v1/statement/q1/{page}", server.uri())
}

#[tokio::test]
async fn select_one_round_trip() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "nextUri": next(&server, "1"), "stats": { "state": "QUEUED" } }),
    )
    .await;
    mount_get(
        &server,
        "1",
        json!({
            "id": "q1",
            "columns": [{ "name": "_col0", "type": "integer" }],
            "data": [[1]],
            "stats": { "state": "FINISHED" }
        }),
    )
    .await;

    let output = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("query succeeds");
    assert_eq!(
        output,
        QueryOutput {
            columns: vec![Column {
                name: "_col0".to_owned(),
                physical: PhysicalType::Integer,
            }],
            rows: vec![vec![Value::Long(1)]],
            truncated: false,
        }
    );
}

#[tokio::test]
async fn request_headers() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "nextUri": next(&server, "1"), "stats": { "state": "QUEUED" } }),
    )
    .await;
    mount_get(
        &server,
        "1",
        json!({
            "id": "q1",
            "columns": [{ "name": "_col0", "type": "integer" }],
            "data": [[1]],
            "stats": { "state": "FINISHED" }
        }),
    )
    .await;
    let sql = select_one();
    engine_for(&server)
        .execute(&sql, &ctx())
        .await
        .expect("query succeeds");

    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(requests.len(), 2);
    for request in &requests {
        let header = |name: &str| {
            request
                .headers
                .get(name)
                .map(|v| v.to_str().expect("ascii").to_owned())
        };
        assert_eq!(header("x-trino-user").as_deref(), Some("ontoforge"));
        assert_eq!(header("x-trino-source").as_deref(), Some("ontoforge"));
        assert_eq!(header("x-trino-trace-token").as_deref(), Some("trace-1"));
        assert_eq!(
            header("x-trino-client-tags").as_deref(),
            Some("ontoforge,validate-online")
        );
        assert_eq!(header("x-trino-time-zone").as_deref(), Some("UTC"));
        assert_eq!(header("authorization"), None);
    }
    let post = &requests[0];
    assert_eq!(post.method.as_str(), "POST");
    assert_eq!(
        post.headers
            .get("x-trino-session")
            .and_then(|v| v.to_str().ok()),
        Some("query_max_execution_time=35s")
    );
    assert_eq!(String::from_utf8_lossy(&post.body), sql.as_str());
    let get = &requests[1];
    assert_eq!(get.method.as_str(), "GET");
    assert!(
        get.headers.get("x-trino-session").is_none(),
        "the session header is only sent with the POST"
    );
}

#[tokio::test]
async fn data_spans_pages() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "nextUri": next(&server, "1"), "stats": { "state": "QUEUED" } }),
    )
    .await;
    mount_get(
        &server,
        "1",
        json!({
            "id": "q1",
            "nextUri": next(&server, "2"),
            "columns": [{ "name": "n", "type": "integer" }],
            "stats": { "state": "RUNNING" }
        }),
    )
    .await;
    mount_get(
        &server,
        "2",
        json!({ "id": "q1", "nextUri": next(&server, "3"), "data": [[1], [2]], "stats": { "state": "RUNNING" } }),
    )
    .await;
    mount_get(
        &server,
        "3",
        json!({ "id": "q1", "data": [[3]], "stats": { "state": "FINISHED" } }),
    )
    .await;

    let output = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("query succeeds");
    assert_eq!(
        output.rows,
        vec![
            vec![Value::Long(1)],
            vec![Value::Long(2)],
            vec![Value::Long(3)]
        ]
    );
    assert_eq!(output.columns.len(), 1);
    assert!(!output.truncated);
}

#[tokio::test]
async fn foreign_next_uri_is_rejected() {
    let server = MockServer::start().await;
    let other = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "nextUri": next(&other, "1"), "stats": { "state": "QUEUED" } }),
    )
    .await;

    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("foreign nextUri");
    assert!(matches!(err, EngineError::Protocol(_)), "{err:?}");
    assert_eq!(
        other.received_requests().await.expect("recorded").len(),
        0,
        "nothing may be sent to a foreign origin"
    );
}

#[tokio::test]
async fn query_error_maps_to_query() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({
            "id": "q1",
            "error": {
                "message": "line 1:8: Column 'x' cannot be resolved",
                "errorCode": 47,
                "errorName": "SYNTAX_ERROR",
                "errorType": "USER_ERROR"
            },
            "stats": { "state": "FAILED" }
        }),
    )
    .await;

    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("query error");
    match err {
        EngineError::Query { error_name, .. } => assert_eq!(error_name, "SYNTAX_ERROR"),
        other => panic!("expected a Query error, got {other:?}"),
    }
}

#[tokio::test]
async fn set_session_header_is_ignored() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("X-Trino-Set-Session", "a=b")
                .set_body_json(json!({
                    "id": "q1",
                    "columns": [{ "name": "_col0", "type": "integer" }],
                    "data": [[1]],
                    "stats": { "state": "FINISHED" }
                })),
        )
        .mount(&server)
        .await;

    let engine = engine_for(&server);
    engine
        .execute(&select_one(), &ctx())
        .await
        .expect("first query");
    engine
        .execute(&select_one(), &ctx())
        .await
        .expect("second query");

    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(requests.len(), 2);
    let session = requests[1]
        .headers
        .get("x-trino-session")
        .and_then(|v| v.to_str().ok())
        .expect("session header");
    assert_eq!(session, "query_max_execution_time=35s");
}

#[tokio::test]
async fn bad_trace_token_is_config_error() {
    let server = MockServer::start().await;
    let engine = engine_for(&server);
    for token in ["bad\r\ntoken", "", "has space", &"t".repeat(129)] {
        let mut c = ctx();
        c.trace_token = token.to_owned();
        let err = engine
            .execute(&select_one(), &c)
            .await
            .expect_err("invalid token");
        assert!(matches!(err, EngineError::Config(_)), "{token:?}: {err:?}");
    }
    let mut c = ctx();
    c.client_tags = vec!["a,b".to_owned()];
    assert!(matches!(
        engine.execute(&select_one(), &c).await,
        Err(EngineError::Config(_))
    ));
    c.client_tags = vec!["x".repeat(65)];
    assert!(matches!(
        engine.execute(&select_one(), &c).await,
        Err(EngineError::Config(_))
    ));
    assert_eq!(server.received_requests().await.expect("recorded").len(), 0);
}

#[tokio::test]
async fn trace_token_of_128_characters_is_accepted() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "columns": [], "stats": { "state": "FINISHED" } }),
    )
    .await;
    let mut c = ctx();
    c.trace_token = "t".repeat(128);
    engine_for(&server)
        .execute(&select_one(), &c)
        .await
        .expect("128 characters are fine");
}
