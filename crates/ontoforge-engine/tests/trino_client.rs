//! Protocol tests for the Trino client against a wiremock server.

use std::time::Duration;

use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{
    Column, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PhysicalType, PrintedSql,
    QueryContext, QueryOutput, TableRef, Value,
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
async fn rows_past_the_row_limit_are_not_parsed() {
    // WR-E05: a page is parsed row by row, and only while rows are kept. The rows after the
    // limit here are not even arrays; parsing the page as a whole would reject it.
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({
            "id": "q1",
            "columns": [{ "name": "_col0", "type": "integer" }],
            "data": [[1], [2], "not a row", { "x": 1 }],
            "stats": { "state": "FINISHED" }
        }),
    )
    .await;
    let mut c = ctx();
    c.max_rows = 2;
    let output = engine_for(&server)
        .execute(&select_one(), &c)
        .await
        .expect("the rows past the limit are skipped");
    assert_eq!(
        output.rows,
        vec![vec![Value::Long(1)], vec![Value::Long(2)]]
    );
    assert!(output.truncated);
}

#[tokio::test]
async fn kept_row_that_is_not_an_array_is_protocol_error() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({
            "id": "q1",
            "columns": [{ "name": "_col0", "type": "varchar" }],
            "data": [["fine"], "cell-secret-value"],
            "stats": { "state": "FINISHED" }
        }),
    )
    .await;
    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("a row must be an array");
    let EngineError::Protocol(message) = &err else {
        panic!("{err:?}");
    };
    assert!(!message.contains("cell-secret-value"), "{message}");
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

#[tokio::test]
async fn next_uri_scheme_mismatch_is_rejected() {
    let server = MockServer::start().await;
    // Same host and port as the mock, but https.
    let downgraded = format!(
        "{}/v1/statement/q1/1",
        server.uri().replacen("http://", "https://", 1)
    );
    mount_post(
        &server,
        json!({ "id": "q1", "nextUri": downgraded, "stats": { "state": "QUEUED" } }),
    )
    .await;

    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("scheme mismatch");
    assert!(matches!(err, EngineError::Protocol(_)), "{err:?}");
    assert_eq!(
        server.received_requests().await.expect("recorded").len(),
        1,
        "only the POST may be sent"
    );
}

#[tokio::test]
async fn two_identities_use_separate_clients() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "columns": [], "stats": { "state": "FINISHED" } }),
    )
    .await;
    let mut ontology = connection(&server.uri());
    ontology.user = "ontoforge-ontology".to_owned();
    let mut l2 = connection(&server.uri());
    l2.user = "ontoforge-l2".to_owned();
    let ontology = TrinoEngine::new(ontology).expect("ontology engine");
    let l2 = TrinoEngine::new(l2).expect("l2 engine");

    ontology
        .execute(&select_one(), &ctx())
        .await
        .expect("ontology query");
    l2.execute(&select_one(), &ctx()).await.expect("l2 query");

    let users: Vec<String> = server
        .received_requests()
        .await
        .expect("recorded")
        .iter()
        .map(|r| {
            r.headers
                .get("x-trino-user")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(users, vec!["ontoforge-ontology", "ontoforge-l2"]);
}

// ---- value decoding -------------------------------------------------------------------

fn cols(spec: &[(&str, &str)]) -> serde_json::Value {
    serde_json::Value::Array(
        spec.iter()
            .map(|(name, ty)| json!({ "name": name, "type": ty }))
            .collect(),
    )
}

/// Runs a query whose single page is `columns` / `data`.
async fn run_page(columns: serde_json::Value, data: serde_json::Value) -> QueryOutput {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "columns": columns, "data": data, "stats": { "state": "FINISHED" } }),
    )
    .await;
    engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("query succeeds")
}

#[tokio::test]
async fn decode_types() {
    let out = run_page(
        cols(&[
            ("b", "boolean"),
            ("t", "tinyint"),
            ("s", "smallint"),
            ("i", "integer"),
            ("l", "bigint"),
            ("r", "real"),
            ("d", "double"),
            ("dec", "decimal(12,2)"),
            ("v", "varchar(10)"),
            ("c", "char(3)"),
            ("dt", "date"),
            ("ts", "timestamp(3)"),
            ("tz", "timestamp(3) with time zone"),
            ("a", "array(varchar)"),
            ("m", "map(varchar,integer)"),
            ("bad", "integer"),
        ]),
        json!([[
            true,
            1,
            2,
            3,
            9_007_199_254_740_993_i64,
            1.5,
            2.5,
            "1234.50",
            "abc",
            "ab ",
            "2026-03-29",
            "2026-03-29 01:30:00.000",
            "2026-03-29 01:30:00.000 Europe/Berlin",
            ["x", null],
            { "k": 1 },
            "oops"
        ]]),
    )
    .await;
    assert_eq!(
        out.rows,
        vec![vec![
            Value::Boolean(true),
            Value::Long(1),
            Value::Long(2),
            Value::Long(3),
            Value::Long(9_007_199_254_740_993),
            Value::Double(1.5),
            Value::Double(2.5),
            Value::Decimal("1234.50".to_owned()),
            Value::String("abc".to_owned()),
            Value::String("ab ".to_owned()),
            Value::Date("2026-03-29".to_owned()),
            Value::Timestamp("2026-03-29 01:30:00.000".to_owned()),
            Value::TimestampTz("2026-03-29 01:30:00.000 Europe/Berlin".to_owned()),
            Value::Array(vec![Value::String("x".to_owned()), Value::Null]),
            Value::Unsupported(json!({ "k": 1 })),
            Value::Unsupported(json!("oops")),
        ]]
    );
    assert_eq!(
        out.columns[7].physical,
        PhysicalType::Decimal {
            precision: 12,
            scale: 2
        }
    );
}

#[tokio::test]
async fn decode_special_doubles_and_zone_forms() {
    let out = run_page(
        cols(&[("d", "double")]),
        json!([
            [1.5],
            ["NaN"],
            ["Infinity"],
            ["-Infinity"],
            [null],
            ["junk"]
        ]),
    )
    .await;
    let cell = |i: usize| out.rows[i][0].clone();
    assert_eq!(cell(0), Value::Double(1.5));
    assert!(matches!(cell(1), Value::Double(x) if x.is_nan()));
    assert_eq!(cell(2), Value::Double(f64::INFINITY));
    assert_eq!(cell(3), Value::Double(f64::NEG_INFINITY));
    assert_eq!(cell(4), Value::Null);
    assert_eq!(cell(5), Value::Unsupported(json!("junk")));

    let out = run_page(
        cols(&[("tz", "timestamp(3) with time zone")]),
        json!([
            ["2026-03-29 01:30:00.000 Europe/Berlin"],
            ["2026-03-29 01:30:00.000 +01:00"]
        ]),
    )
    .await;
    assert_eq!(
        out.rows,
        vec![
            vec![Value::TimestampTz(
                "2026-03-29 01:30:00.000 Europe/Berlin".to_owned()
            )],
            vec![Value::TimestampTz(
                "2026-03-29 01:30:00.000 +01:00".to_owned()
            )],
        ]
    );
}

// ---- describe_tables and status -------------------------------------------------------

fn table(catalog: &str, schema: &str, name: &str) -> TableRef {
    TableRef {
        catalog: catalog.to_owned(),
        schema: schema.to_owned(),
        table: name.to_owned(),
    }
}

fn info_columns() -> serde_json::Value {
    cols(&[
        ("table_schema", "varchar"),
        ("table_name", "varchar"),
        ("column_name", "varchar"),
        ("data_type", "varchar"),
        ("is_nullable", "varchar"),
    ])
}

/// The metadata query the engine must send, built independently with the printer.
fn expected_describe_sql(catalog: &str, pairs: &[(&str, &str)]) -> String {
    let eq = |column: &str, value: &str| Expr::Compare {
        left: Box::new(Expr::Column(column.to_owned())),
        op: CompareOp::Eq,
        right: Box::new(Expr::Literal(Literal::String(value.to_owned()))),
    };
    let mut conditions: Vec<Expr> = pairs
        .iter()
        .map(|(schema, name)| Expr::And(vec![eq("table_schema", schema), eq("table_name", name)]))
        .collect();
    let filter = if conditions.len() == 1 {
        conditions.remove(0)
    } else {
        Expr::Or(conditions)
    };
    let item = |name: &str| SelectItem {
        expr: Expr::Column(name.to_owned()),
        alias: None,
    };
    let order = |name: &str| OrderItem {
        expr: Expr::Column(name.to_owned()),
        descending: false,
        nulls_first: false,
    };
    let select = Select {
        projection: vec![
            item("table_schema"),
            item("table_name"),
            item("column_name"),
            item("data_type"),
            item("is_nullable"),
        ],
        from: Some(table(catalog, "information_schema", "columns")),
        filter: Some(filter),
        group_by: Vec::new(),
        having: None,
        order_by: vec![
            order("table_schema"),
            order("table_name"),
            order("ordinal_position"),
        ],
        limit: None,
    };
    print_select(&select, Dialect::Trino)
        .expect("printable")
        .as_str()
        .to_owned()
}

fn page(rows: &serde_json::Value) -> serde_json::Value {
    json!({ "id": "q1", "columns": info_columns(), "data": rows, "stats": { "state": "FINISHED" } })
}

#[tokio::test]
async fn describe_tables_batches_one_catalog() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        page(&json!([
            ["aviation", "flights", "flight_id", "varchar", "NO"],
            ["aviation", "flights", "departure", "timestamp(3)", "YES"],
            ["aviation", "flights", "seats", "integer", "YES"],
            ["aviation", "pilots", "pilot_id", "varchar", "NO"],
            ["aviation", "pilots", "name", "varchar(100)", "YES"]
        ])),
    )
    .await;
    let flights = table("demo", "aviation", "flights");
    let pilots = table("demo", "aviation", "pilots");

    let described = engine_for(&server)
        .describe_tables(&[flights.clone(), pilots.clone()], &ctx())
        .await
        .expect("described");

    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(requests.len(), 1, "one query for the whole catalog");
    let body = String::from_utf8_lossy(&requests[0].body).into_owned();
    assert_eq!(
        body,
        expected_describe_sql("demo", &[("aviation", "flights"), ("aviation", "pilots")])
    );
    assert!(body.contains("\"demo\".\"information_schema\".\"columns\""));
    assert!(body.contains(" OR "));

    assert_eq!(described.len(), 2);
    let column = |name: &str, physical: PhysicalType, nullable: bool| PhysicalColumn {
        name: name.to_owned(),
        physical,
        nullable,
    };
    assert_eq!(
        described[&flights],
        vec![
            column("flight_id", PhysicalType::Varchar { length: None }, false),
            column(
                "departure",
                PhysicalType::Timestamp {
                    precision: 3,
                    with_time_zone: false
                },
                true
            ),
            column("seats", PhysicalType::Integer, true),
        ]
    );
    assert_eq!(
        described[&pilots][1],
        column("name", PhysicalType::Varchar { length: Some(100) }, true)
    );
}

#[tokio::test]
async fn describe_single_table_uses_a_single_condition() {
    let server = MockServer::start().await;
    mount_post(&server, page(&json!([]))).await;
    engine_for(&server)
        .describe_tables(&[table("demo", "aviation", "flights")], &ctx())
        .await
        .expect("described");
    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(
        String::from_utf8_lossy(&requests[0].body),
        expected_describe_sql("demo", &[("aviation", "flights")])
    );
}

#[tokio::test]
async fn describe_tables_lowercases_names() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        page(&json!([[
            "aviation",
            "flights",
            "flight_id",
            "varchar",
            "NO"
        ]])),
    )
    .await;
    let asked = table("Demo", "Aviation", "Flights");
    let described = engine_for(&server)
        .describe_tables(std::slice::from_ref(&asked), &ctx())
        .await
        .expect("described");

    let requests = server.received_requests().await.expect("recorded");
    let body = String::from_utf8_lossy(&requests[0].body).into_owned();
    assert!(
        body.contains("'aviation'") && body.contains("'flights'"),
        "{body}"
    );
    assert!(
        !body.contains("'Aviation'") && !body.contains("'Flights'"),
        "{body}"
    );
    assert_eq!(described.keys().collect::<Vec<_>>(), vec![&asked]);
}

#[tokio::test]
async fn describe_missing_tables_absent() {
    let flights = table("demo", "aviation", "flights");
    let pilots = table("demo", "aviation", "pilots");

    let server = MockServer::start().await;
    mount_post(&server, page(&json!([]))).await;
    let described = engine_for(&server)
        .describe_tables(&[flights.clone(), pilots.clone()], &ctx())
        .await
        .expect("no rows is not an error");
    assert!(described.is_empty());

    let server = MockServer::start().await;
    mount_post(
        &server,
        page(&json!([[
            "aviation",
            "flights",
            "flight_id",
            "varchar",
            "NO"
        ]])),
    )
    .await;
    let described = engine_for(&server)
        .describe_tables(&[flights.clone(), pilots.clone()], &ctx())
        .await
        .expect("described");
    assert!(described.contains_key(&flights));
    assert!(!described.contains_key(&pilots));

    for name in ["CATALOG_NOT_FOUND", "SCHEMA_NOT_FOUND"] {
        let server = MockServer::start().await;
        mount_post(
            &server,
            json!({ "id": "q1", "error": { "message": "gone", "errorName": name }, "stats": { "state": "FAILED" } }),
        )
        .await;
        let described = engine_for(&server)
            .describe_tables(std::slice::from_ref(&flights), &ctx())
            .await
            .expect("a missing catalog means missing tables");
        assert!(described.is_empty(), "{name}");
    }

    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "error": { "message": "denied", "errorName": "PERMISSION_DENIED" }, "stats": { "state": "FAILED" } }),
    )
    .await;
    let err = engine_for(&server)
        .describe_tables(std::slice::from_ref(&flights), &ctx())
        .await
        .expect_err("other errors are returned");
    assert!(matches!(err, EngineError::Query { .. }), "{err:?}");
}

#[tokio::test]
async fn describe_truncated_metadata_is_error() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        page(&json!([
            ["aviation", "flights", "a", "varchar", "NO"],
            ["aviation", "flights", "b", "varchar", "NO"],
            ["aviation", "flights", "c", "varchar", "NO"]
        ])),
    )
    .await;
    let mut c = ctx();
    c.max_rows = 2;
    let err = engine_for(&server)
        .describe_tables(&[table("demo", "aviation", "flights")], &c)
        .await
        .expect_err("truncated metadata");
    match err {
        EngineError::Protocol(message) => {
            assert!(
                message.contains("demo") && message.contains("truncated"),
                "{message}"
            );
        }
        other => panic!("expected a Protocol error, got {other:?}"),
    }
}

#[tokio::test]
async fn describe_mixed_catalogs_is_config_error() {
    let server = MockServer::start().await;
    let err = engine_for(&server)
        .describe_tables(&[table("a", "s", "t"), table("b", "s", "t")], &ctx())
        .await
        .expect_err("two catalogs");
    assert!(matches!(err, EngineError::Config(_)), "{err:?}");
    let none = engine_for(&server)
        .describe_tables(&[], &ctx())
        .await
        .expect("empty input");
    assert!(none.is_empty());
    assert_eq!(server.received_requests().await.expect("recorded").len(), 0);
}

#[tokio::test]
async fn status_available_and_unavailable() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        json!({ "id": "q1", "columns": [{ "name": "_col0", "type": "integer" }], "data": [[1]], "stats": { "state": "FINISHED" } }),
    )
    .await;
    assert_eq!(engine_for(&server).status().await, EngineStatus::Available);
    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(String::from_utf8_lossy(&requests[0].body), "SELECT 1");

    let down = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&down)
        .await;
    match engine_for(&down).status().await {
        EngineStatus::Unavailable { reason } => {
            assert!(!reason.is_empty());
            assert!(!reason.to_lowercase().contains("authorization"), "{reason}");
        }
        EngineStatus::Available => panic!("a 401 is not available"),
    }
}
