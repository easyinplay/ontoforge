//! Failure behaviour of the Trino client: error classification, retries and cancellation.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ontoforge_engine::printer::ast::{Expr, Literal, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{Dialect, Engine, EngineError, EngineStatus, PrintedSql, QueryContext};
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
        connect_timeout: Duration::from_millis(1000),
        max_page_bytes: 64 * 1024 * 1024,
        extra_origins: Vec::new(),
    }
}

fn engine_for(server: &MockServer) -> TrinoEngine {
    TrinoEngine::new(connection(&server.uri())).expect("engine")
}

fn ctx() -> QueryContext {
    let mut ctx = QueryContext::new("trace-1".to_owned(), vec!["ontoforge".to_owned()]);
    ctx.timeout = Duration::from_secs(30);
    ctx
}

/// A local port nothing listens on: bind, note the port, release it.
fn closed_port_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

fn error_page(error_name: &str) -> serde_json::Value {
    json!({
        "id": "q1",
        "error": { "message": "boom", "errorName": error_name, "errorType": "INTERNAL_ERROR" },
        "stats": { "state": "FAILED" }
    })
}

async fn mount_post(server: &MockServer, template: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .respond_with(template)
        .mount(server)
        .await;
}

#[tokio::test]
async fn closed_port_is_unavailable() {
    let engine = TrinoEngine::new(connection(&closed_port_url())).expect("engine");
    let started = Instant::now();
    let err = engine
        .execute(&select_one(), &ctx())
        .await
        .expect_err("nothing listens");
    assert!(
        matches!(
            err,
            EngineError::Unavailable {
                retryable: true,
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(err.code(), "ENGINE_UNAVAILABLE");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn dns_failure_is_unavailable() {
    let engine =
        TrinoEngine::new(connection("http://ontoforge-unresolvable.invalid:8080")).expect("engine");
    let err = engine
        .execute(&select_one(), &ctx())
        .await
        .expect_err("name does not resolve");
    assert!(
        matches!(
            err,
            EngineError::Unavailable {
                retryable: true,
                ..
            }
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn server_starting_up_is_unavailable() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        ResponseTemplate::new(200).set_body_json(error_page("SERVER_STARTING_UP")),
    )
    .await;
    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("server is starting");
    assert!(
        matches!(
            err,
            EngineError::Unavailable {
                retryable: true,
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(err.code(), "ENGINE_UNAVAILABLE");
}

#[tokio::test]
async fn server_shutting_down_is_unavailable() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        ResponseTemplate::new(200).set_body_json(error_page("SERVER_SHUTTING_DOWN")),
    )
    .await;
    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("server is shutting down");
    assert!(
        matches!(
            err,
            EngineError::Unavailable {
                retryable: true,
                ..
            }
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn http_503_with_shutting_down_body_is_unavailable() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        ResponseTemplate::new(503).set_body_json(error_page("SERVER_SHUTTING_DOWN")),
    )
    .await;
    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("503");
    assert!(
        matches!(
            err,
            EngineError::Unavailable {
                retryable: true,
                ..
            }
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn unauthorized_is_not_unavailable() {
    let server = MockServer::start().await;
    mount_post(&server, ResponseTemplate::new(401)).await;
    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("401");
    assert!(matches!(err, EngineError::Unauthorized { .. }), "{err:?}");
    assert_eq!(err.code(), "ENGINE_UNAUTHORIZED");

    let forbidden = MockServer::start().await;
    mount_post(&forbidden, ResponseTemplate::new(403)).await;
    let err = engine_for(&forbidden)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("403");
    assert!(matches!(err, EngineError::Unauthorized { .. }), "{err:?}");
}

#[tokio::test]
async fn syntax_error_is_query() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        ResponseTemplate::new(200).set_body_json(error_page("SYNTAX_ERROR")),
    )
    .await;
    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("syntax error");
    match err {
        EngineError::Query { error_name, .. } => assert_eq!(error_name, "SYNTAX_ERROR"),
        other => panic!("expected a Query error, got {other:?}"),
    }
}

#[tokio::test]
async fn status_reports_unavailable() {
    let engine = TrinoEngine::new(connection(&closed_port_url())).expect("engine");
    match engine.status().await {
        EngineStatus::Unavailable { reason } => {
            assert!(!reason.is_empty());
            assert!(!reason.to_lowercase().contains("authorization"), "{reason}");
        }
        EngineStatus::Available => panic!("a closed port is not available"),
    }
}

// ---- retries ----

fn done_page() -> serde_json::Value {
    json!({
        "id": "q1",
        "columns": [{ "name": "_col0", "type": "integer" }],
        "data": [[1]],
        "stats": { "state": "FINISHED" }
    })
}

fn next(server: &MockServer, page: &str) -> String {
    format!("{}/v1/statement/q1/{page}", server.uri())
}

/// POST answering with a page that points at `/v1/statement/q1/1`.
async fn mount_submit(server: &MockServer) {
    mount_post(
        server,
        ResponseTemplate::new(200).set_body_json(json!({
            "id": "q1", "nextUri": next(server, "1"), "stats": { "state": "QUEUED" }
        })),
    )
    .await;
}

/// GET page 1 answering `template` for the first `times` requests only.
async fn mount_get_times(server: &MockServer, template: ResponseTemplate, times: u64) {
    Mock::given(method("GET"))
        .and(path("/v1/statement/q1/1"))
        .respond_with(template)
        .up_to_n_times(times)
        .mount(server)
        .await;
}

async fn mount_get_always(server: &MockServer, template: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/v1/statement/q1/1"))
        .respond_with(template)
        .mount(server)
        .await;
}

/// POST answering `template` for the first request only.
async fn mount_post_once(server: &MockServer, template: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .respond_with(template)
        .up_to_n_times(1)
        .mount(server)
        .await;
}

async fn count(server: &MockServer, http_method: &str) -> usize {
    server
        .received_requests()
        .await
        .expect("recorded")
        .iter()
        .filter(|r| r.method.as_str() == http_method)
        .count()
}

fn assert_unavailable(err: &EngineError) {
    assert!(
        matches!(
            err,
            EngineError::Unavailable {
                retryable: true,
                ..
            }
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn get_503_then_success() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_times(&server, ResponseTemplate::new(503), 2).await;
    mount_get_always(
        &server,
        ResponseTemplate::new(200).set_body_json(done_page()),
    )
    .await;

    let output = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("succeeds after two 503");
    assert_eq!(output.rows.len(), 1);
    assert_eq!(count(&server, "GET").await, 3);
    assert_eq!(count(&server, "POST").await, 1);
}

#[tokio::test]
async fn get_502_and_504_are_retried() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_times(&server, ResponseTemplate::new(502), 1).await;
    mount_get_times(&server, ResponseTemplate::new(504), 1).await;
    mount_get_always(
        &server,
        ResponseTemplate::new(200).set_body_json(done_page()),
    )
    .await;

    engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("succeeds after 502 and 504");
    assert_eq!(count(&server, "GET").await, 3);
}

#[tokio::test]
async fn exhausted_503_is_unavailable() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_always(&server, ResponseTemplate::new(503)).await;

    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("503 forever");
    assert_unavailable(&err);
    // One original request and five retries.
    assert_eq!(count(&server, "GET").await, 6);
}

#[tokio::test]
async fn retry_after_is_honoured() {
    let server = MockServer::start().await;
    mount_post_once(
        &server,
        ResponseTemplate::new(429).insert_header("Retry-After", "1"),
    )
    .await;
    mount_post(
        &server,
        ResponseTemplate::new(200).set_body_json(done_page()),
    )
    .await;

    let started = Instant::now();
    engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("succeeds after the 429");
    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(count(&server, "POST").await, 2);
}

#[tokio::test]
async fn retry_after_is_capped() {
    let server = MockServer::start().await;
    mount_post_once(
        &server,
        ResponseTemplate::new(429).insert_header("Retry-After", "600"),
    )
    .await;
    mount_post(
        &server,
        ResponseTemplate::new(200).set_body_json(done_page()),
    )
    .await;

    let started = Instant::now();
    engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("succeeds after the 429");
    let waited = started.elapsed();
    assert!(waited >= Duration::from_secs(5), "{waited:?}");
    assert!(waited < Duration::from_secs(8), "{waited:?}");
}

#[tokio::test]
async fn post_502_is_not_retried() {
    let server = MockServer::start().await;
    mount_post(&server, ResponseTemplate::new(502)).await;

    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("502 on submission");
    assert_unavailable(&err);
    assert_eq!(count(&server, "POST").await, 1);
}

#[tokio::test]
async fn post_503_starting_up_is_retried() {
    let server = MockServer::start().await;
    mount_post_once(
        &server,
        ResponseTemplate::new(503).set_body_json(error_page("SERVER_STARTING_UP")),
    )
    .await;
    mount_post(
        &server,
        ResponseTemplate::new(200).set_body_json(done_page()),
    )
    .await;

    let output = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("succeeds once the server is up");
    assert_eq!(output.rows.len(), 1);
    assert_eq!(count(&server, "POST").await, 2);
}

#[tokio::test]
async fn post_plain_503_is_not_retried() {
    let bodies = [
        ResponseTemplate::new(503),
        ResponseTemplate::new(503).set_body_string("<html>Bad gateway</html>"),
    ];
    for template in bodies {
        let server = MockServer::start().await;
        mount_post(&server, template).await;

        let err = engine_for(&server)
            .execute(&select_one(), &ctx())
            .await
            .expect_err("plain 503 on submission");
        match &err {
            EngineError::Unavailable {
                retryable: true,
                reason,
            } => assert!(
                reason.contains("not retried because the statement may have been accepted"),
                "{reason}"
            ),
            other => panic!("expected Unavailable, got {other:?}"),
        }
        assert_eq!(count(&server, "POST").await, 1);
    }
}

#[tokio::test]
async fn connect_failure_is_retried_with_backoff() {
    let mut conn = connection(&closed_port_url());
    conn.connect_timeout = Duration::from_millis(200);
    let engine = TrinoEngine::new(conn).expect("engine");

    let started = Instant::now();
    let err = engine
        .execute(&select_one(), &ctx())
        .await
        .expect_err("nothing listens");
    assert_unavailable(&err);
    // Five retries wait 50 + 100 + 200 + 400 + 800 ms between them.
    assert!(
        started.elapsed() >= Duration::from_millis(1500),
        "{:?}",
        started.elapsed()
    );
}

// ---- cancellation, limits and the page size cap ----

/// Requests of `http_method` the server has seen; waits up to 2 s for at least `at_least`.
async fn wait_for_requests(
    server: &MockServer,
    http_method: &str,
    at_least: usize,
) -> Vec<wiremock::Request> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let found: Vec<_> = server
            .received_requests()
            .await
            .expect("recorded")
            .into_iter()
            .filter(|r| r.method.as_str() == http_method)
            .collect();
        if found.len() >= at_least || Instant::now() >= deadline {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn mount_delete(server: &MockServer) {
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount(server)
        .await;
}

/// GET `/v1/statement/q1/{page}` with `rows` varchar rows and, unless `last`, a nextUri to
/// the following page.
async fn mount_rows_page(server: &MockServer, page: u32, rows: usize, last: bool) {
    let mut body = json!({
        "id": "q1",
        "columns": [{ "name": "c", "type": "varchar" }],
        "data": vec![json!(["aaaaaaaaaa"]); rows],
        "stats": { "state": "RUNNING" }
    });
    if !last {
        body["nextUri"] = json!(next(server, &(page + 1).to_string()));
    }
    Mock::given(method("GET"))
        .and(path(format!("/v1/statement/q1/{page}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

/// Three pages of three rows each, behind a submission that only returns the first nextUri.
async fn mount_three_pages(server: &MockServer) {
    mount_submit(server).await;
    mount_rows_page(server, 1, 3, false).await;
    mount_rows_page(server, 2, 3, false).await;
    mount_rows_page(server, 3, 3, true).await;
}

#[tokio::test]
async fn timeout_sends_delete() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_always(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(done_page())
            .set_delay(Duration::from_secs(2)),
    )
    .await;
    mount_delete(&server).await;
    let mut context = ctx();
    context.timeout = Duration::from_millis(300);

    let started = Instant::now();
    let err = engine_for(&server)
        .execute(&select_one(), &context)
        .await
        .expect_err("the page is slower than the timeout");
    assert_eq!(err, EngineError::Timeout { after_ms: 300 });
    assert_eq!(err.code(), "ENGINE_TIMEOUT");
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "{:?}",
        started.elapsed()
    );

    let deletes = wait_for_requests(&server, "DELETE", 1).await;
    assert_eq!(deletes.len(), 1);
    assert_eq!(deletes[0].url.path(), "/v1/statement/q1/1");
}

#[tokio::test]
async fn hanging_submission_times_out_without_delete() {
    let server = MockServer::start().await;
    mount_post(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(done_page())
            .set_delay(Duration::from_secs(2)),
    )
    .await;
    let mut context = ctx();
    context.timeout = Duration::from_millis(300);

    let err = engine_for(&server)
        .execute(&select_one(), &context)
        .await
        .expect_err("the submission is slower than the timeout");
    assert_eq!(err, EngineError::Timeout { after_ms: 300 });
    // The server has not told us a nextUri, so there is nothing to cancel.
    assert_eq!(count(&server, "DELETE").await, 0);
}

#[tokio::test]
async fn retries_respect_timeout() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_always(&server, ResponseTemplate::new(503)).await;
    let mut context = ctx();
    context.timeout = Duration::from_millis(300);

    let started = Instant::now();
    let err = engine_for(&server)
        .execute(&select_one(), &context)
        .await
        .expect_err("503 forever");
    assert_eq!(err, EngineError::Timeout { after_ms: 300 });
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn dropped_future_sends_delete() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_always(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(done_page())
            .set_delay(Duration::from_secs(5)),
    )
    .await;
    mount_delete(&server).await;
    let engine = Arc::new(engine_for(&server));
    let sql = select_one();
    let context = ctx();

    let runner = Arc::clone(&engine);
    let task = tokio::spawn(async move { runner.execute(&sql, &context).await });
    let gets = wait_for_requests(&server, "GET", 1).await;
    assert_eq!(gets.len(), 1, "the query never reached its first GET");
    task.abort();
    assert!(task.await.expect_err("aborted").is_cancelled());

    let deletes = wait_for_requests(&server, "DELETE", 1).await;
    assert_eq!(deletes.len(), 1);
    assert_eq!(deletes[0].url.path(), "/v1/statement/q1/1");
    assert_eq!(
        deletes[0]
            .headers
            .get("x-trino-user")
            .and_then(|v| v.to_str().ok()),
        Some("ontoforge")
    );
}

#[tokio::test]
async fn finished_query_sends_no_delete() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_always(
        &server,
        ResponseTemplate::new(200).set_body_json(done_page()),
    )
    .await;
    mount_delete(&server).await;

    engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("query succeeds");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(count(&server, "DELETE").await, 0);
}

#[tokio::test]
async fn failed_query_sends_no_delete() {
    let server = MockServer::start().await;
    mount_submit(&server).await;
    mount_get_always(
        &server,
        ResponseTemplate::new(200).set_body_json(error_page("TABLE_NOT_FOUND")),
    )
    .await;
    mount_delete(&server).await;

    let err = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect_err("query failed");
    assert!(matches!(err, EngineError::Query { .. }), "{err:?}");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(count(&server, "DELETE").await, 0);
}

#[tokio::test]
async fn max_rows_truncates_and_deletes() {
    let server = MockServer::start().await;
    mount_three_pages(&server).await;
    mount_delete(&server).await;
    let mut context = ctx();
    context.max_rows = 4;

    let output = engine_for(&server)
        .execute(&select_one(), &context)
        .await
        .expect("truncated, not failed");
    assert_eq!(output.rows.len(), 4);
    assert!(output.truncated);

    let deletes = wait_for_requests(&server, "DELETE", 1).await;
    assert_eq!(deletes.len(), 1);
    // The nextUri given with page 2; page 3 was never fetched.
    assert_eq!(deletes[0].url.path(), "/v1/statement/q1/3");
    let gets: Vec<_> = server
        .received_requests()
        .await
        .expect("recorded")
        .into_iter()
        .filter(|r| r.method.as_str() == "GET")
        .map(|r| r.url.path().to_owned())
        .collect();
    assert_eq!(gets, ["/v1/statement/q1/1", "/v1/statement/q1/2"]);
}

#[tokio::test]
async fn max_bytes_truncates_and_deletes() {
    let server = MockServer::start().await;
    mount_three_pages(&server).await;
    mount_delete(&server).await;
    // A row is about 14 bytes of JSON: page 1 (3 rows) fits, the first row of page 2 does not.
    let mut context = ctx();
    context.max_bytes = 50;

    let output = engine_for(&server)
        .execute(&select_one(), &context)
        .await
        .expect("truncated, not failed");
    assert_eq!(output.rows.len(), 3);
    assert!(output.truncated);

    let deletes = wait_for_requests(&server, "DELETE", 1).await;
    assert_eq!(deletes.len(), 1);
    assert_eq!(deletes[0].url.path(), "/v1/statement/q1/3");
}

#[tokio::test]
async fn limits_not_reached_do_not_truncate() {
    let server = MockServer::start().await;
    mount_three_pages(&server).await;
    mount_delete(&server).await;

    let output = engine_for(&server)
        .execute(&select_one(), &ctx())
        .await
        .expect("all nine rows");
    assert_eq!(output.rows.len(), 9);
    assert!(!output.truncated);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(count(&server, "DELETE").await, 0);
}

/// A bare TCP server that speaks just enough HTTP/1.1 to serve one query whose second page
/// is sent chunked, without `Content-Length`, and larger than the client allows. Every request
/// line it sees is appended to the returned log.
fn chunked_oversize_server() -> (u16, Arc<Mutex<Vec<String>>>) {
    fn handle(mut stream: TcpStream, port: u16, log: &Mutex<Vec<String>>) {
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            if stream.read(&mut byte).unwrap_or(0) == 0 {
                return;
            }
            head.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&head).into_owned();
        let request_line = head.lines().next().unwrap_or_default().to_owned();
        let content_length = head
            .lines()
            .find_map(|l| {
                l.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(str::to_owned)
            })
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let mut body = vec![0u8; content_length];
        let _ = stream.read_exact(&mut body);
        let mut parts = request_line.split_whitespace();
        let (verb, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        log.lock().expect("log").push(format!("{verb} {target}"));
        match (verb, target) {
            ("POST", "/v1/statement") => {
                let page = format!(
                    r#"{{"id":"q1","nextUri":"http://127.0.0.1:{port}/v1/statement/q1/1"}}"#
                );
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                    page.len()
                );
            }
            ("GET", _) => {
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                );
                for _ in 0..8 {
                    let chunk = "x".repeat(512);
                    if write!(stream, "{:x}\r\n{chunk}\r\n", chunk.len()).is_err() {
                        return;
                    }
                }
                let _ = write!(stream, "0\r\n\r\n");
            }
            _ => {
                let _ = write!(
                    stream,
                    "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n"
                );
            }
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let log = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::clone(&log);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let log = Arc::clone(&shared);
            std::thread::spawn(move || handle(stream, port, &log));
        }
    });
    (port, log)
}

#[tokio::test]
async fn oversized_page_is_protocol_error() {
    let (port, log) = chunked_oversize_server();
    let mut conn = connection(&format!("http://127.0.0.1:{port}"));
    conn.max_page_bytes = 1024;
    let engine = TrinoEngine::new(conn).expect("engine");

    let err = engine
        .execute(&select_one(), &ctx())
        .await
        .expect_err("the second page is 4096 bytes");
    match err {
        EngineError::Protocol(message) => assert!(message.contains("1024"), "{message}"),
        other => panic!("expected a Protocol error, got {other:?}"),
    }

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let seen = log.lock().expect("log").clone();
        if seen.iter().any(|l| l == "DELETE /v1/statement/q1/1") {
            break;
        }
        assert!(Instant::now() < deadline, "no DELETE seen, got {seen:?}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
