//! `ontoforge validate --online --check-pk`: duplicate and NULL primary keys on the fixture
//! stand-in and on a Trino that is a wiremock server, and the exit code table (BIND-03).

use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

const MANIFEST: &str = "apiName: demo\ndefaultProfile: dev\n";

const FLIGHT: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
backing: source
";

const FLIGHT_BINDING: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
fixture: { file: fixtures/flights.csv }
";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// An ontology with the source object type `Flight` whose fixture has `csv` as its text.
fn ontology(csv: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "ontology.yaml", MANIFEST);
    write(dir.path(), "objects.yaml", FLIGHT);
    write(dir.path(), "bindings/dev/flight.yaml", FLIGHT_BINDING);
    write(dir.path(), "bindings/dev/fixtures/flights.csv", csv);
    dir
}

const CLEAN_CSV: &str = "flightId,status\nF001,ok\nF002,ok\n";

fn trino_config(url: &str) -> String {
    format!(
        "[trino]\nurl = \"{url}\"\nconnect_timeout_ms = 1000\n\n\
         [trino.identities.ontology]\nuser = \"ontoforge\"\nauth = \"none\"\n\n\
         [profiles.dev]\nengine = \"trino\"\n"
    )
}

/// Writes `text` as the server configuration into a directory of its own.
fn config_file(text: &str) -> (TempDir, String) {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "ontoforge-server.toml", text);
    let path = dir
        .path()
        .join("ontoforge-server.toml")
        .to_str()
        .expect("utf-8 path")
        .to_owned();
    (dir, path)
}

/// Runs the CLI in a blocking task so that a wiremock server keeps being served.
async fn ontoforge(args: Vec<String>) -> Output {
    tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_ontoforge"))
            .args(args)
            .env_remove("ONTOFORGE_PROFILE")
            .env("NO_COLOR", "1")
            .output()
            .expect("spawn ontoforge")
    })
    .await
    .expect("join")
}

fn args(dir: &Path, extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "validate".to_owned(),
        dir.to_str().expect("utf-8 path").to_owned(),
        "--color".to_owned(),
        "never".to_owned(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    args
}

fn diagnostics(out: &Output) -> Vec<Value> {
    let parsed: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    parsed.as_array().expect("JSON array").clone()
}

fn with_code<'a>(diags: &'a [Value], code: &str) -> Vec<&'a Value> {
    diags.iter().filter(|d| d["code"] == code).collect()
}

fn message(d: &Value) -> &str {
    d["message"].as_str().expect("message")
}

fn page(columns: &Value, rows: &Value) -> Value {
    json!({ "id": "q1", "columns": columns, "data": rows, "stats": { "state": "FINISHED" } })
}

/// Answers the `information_schema.columns` query of catalog `demo` with `rows`.
async fn mount_info(server: &MockServer, rows: Value) {
    let column = |name: &str| json!({ "name": name, "type": "varchar" });
    let columns = json!([
        column("table_schema"),
        column("table_name"),
        column("column_name"),
        column("data_type"),
        column("is_nullable")
    ]);
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("\"demo\".\"information_schema\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(&columns, &rows)))
        .mount(server)
        .await;
}

/// The `information_schema.columns` rows of a Flight table that matches `FLIGHT_BINDING`.
fn flight_rows() -> Value {
    json!([
        ["aviation", "flights", "flightid", "varchar", "NO"],
        ["aviation", "flights", "status", "varchar", "YES"]
    ])
}

/// Answers the duplicate query with `rows` and the NULL count with `nulls`.
async fn mount_pk(server: &MockServer, rows: Value, nulls: i64) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("GROUP BY"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(
            &json!([{ "name": "pk", "type": "varchar" }, { "name": "n", "type": "bigint" }]),
            &rows,
        )))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("IS NULL"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(
            &json!([{ "name": "n", "type": "bigint" }]),
            &json!([[nulls]]),
        )))
        .mount(server)
        .await;
}

/// A local port nothing listens on.
fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("addr").port()
}

fn request_bodies(requests: &[wiremock::Request]) -> Vec<String> {
    requests
        .iter()
        .map(|r| String::from_utf8_lossy(&r.body).into_owned())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn check_pk_clean_on_aviation() {
    let out = ontoforge(args(
        Path::new(AVIATION),
        &["--online", "--check-pk", "--format", "json"],
    ))
    .await;
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(diagnostics(&out).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn check_pk_reports_duplicates() {
    let dir = ontology("flightId,status\nF001,a\nF001,b\nF002,c\n");
    let out = ontoforge(args(
        dir.path(),
        &["--online", "--check-pk", "--format", "json"],
    ))
    .await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    let v18 = with_code(&diags, "ONT-V18");
    assert_eq!(v18.len(), 1, "{diags:#?}");
    let d = v18[0];
    assert_eq!(d["severity"], "error");
    assert_eq!(d["file"], "bindings/dev/flight.yaml");
    assert_eq!(
        d["span"]["offset"],
        FLIGHT_BINDING.find("flights }").expect("table")
    );
    assert!(message(d).contains("F001"), "{d:#?}");
    assert!(message(d).contains("×2"), "{d:#?}");
    assert!(!message(d).contains("F002"), "{d:#?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn check_pk_reports_null_keys() {
    let dir = ontology("flightId,status\n,a\nF001,b\n,c\n");
    let out = ontoforge(args(
        dir.path(),
        &["--online", "--check-pk", "--format", "json"],
    ))
    .await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    let v18 = with_code(&diags, "ONT-V18");
    assert_eq!(v18.len(), 1, "{diags:#?}");
    assert!(
        message(v18[0]).contains("2 row(s) with a NULL primary key"),
        "{diags:#?}"
    );
    assert!(!message(v18[0]).contains("duplicate"), "{diags:#?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn check_pk_without_flag_runs_no_scan() {
    let dir = ontology("flightId,status\nF001,a\nF001,b\n");
    let out = ontoforge(args(dir.path(), &["--online", "--format", "json"])).await;
    assert_eq!(out.status.code(), Some(0));
    assert!(with_code(&diagnostics(&out), "ONT-V18").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn check_pk_on_trino() {
    let server = MockServer::start().await;
    mount_info(&server, flight_rows()).await;
    mount_pk(&server, json!([["F001", 2]]), 0).await;
    let dir = ontology(CLEAN_CSV);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri()));

    let out = ontoforge(args(
        dir.path(),
        &[
            "--online",
            "--server-config",
            &cfg,
            "--check-pk",
            "--format",
            "json",
        ],
    ))
    .await;
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let diags = diagnostics(&out);
    let v18 = with_code(&diags, "ONT-V18");
    assert_eq!(v18.len(), 1, "{diags:#?}");
    assert!(message(v18[0]).contains("F001"), "{diags:#?}");

    let bodies = request_bodies(&server.received_requests().await.expect("recorded"));
    let duplicate_query = bodies
        .iter()
        .find(|b| b.contains("GROUP BY"))
        .expect("duplicate query sent");
    assert!(duplicate_query.contains("IS NOT NULL"), "{duplicate_query}");
    assert!(
        bodies
            .iter()
            .any(|b| b.contains("IS NULL") && !b.contains("GROUP BY")),
        "{bodies:#?}"
    );
}

/// WR-C02: an engine-wide failure while scanning keys keeps what the round already found
/// (the described bindings and the keys scanned before), next to the ONT-V19.
#[tokio::test(flavor = "multi_thread")]
async fn check_pk_engine_failure_keeps_earlier_findings() {
    const AIRPORT: &str = "\
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
backing: source
";
    const AIRPORT_BINDING: &str = "\
kind: SourceBinding
target: { objectType: Airport }
source: { catalog: demo, schema: aviation, table: airports }
fixture: { file: fixtures/airports.csv }
";
    let server = MockServer::start().await;
    mount_info(
        &server,
        json!([
            ["aviation", "airports", "code", "varchar", "NO"],
            ["aviation", "flights", "flightid", "varchar", "NO"],
            ["aviation", "flights", "status", "varchar", "YES"]
        ]),
    )
    .await;
    // `Airport` is scanned first and has a duplicate key; then Trino stops accepting the
    // service identity.
    let airports = "FROM \"demo\".\"aviation\".\"airports\"";
    let columns = json!([{ "name": "pk", "type": "varchar" }, { "name": "n", "type": "bigint" }]);
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("GROUP BY"))
        .and(body_string_contains(airports))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(page(&columns, &json!([["FRA", 2]]))),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("IS NULL"))
        .and(body_string_contains(airports))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(
            &json!([{ "name": "n", "type": "bigint" }]),
            &json!([[0]]),
        )))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains(
            "FROM \"demo\".\"aviation\".\"flights\"",
        ))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let dir = ontology(CLEAN_CSV);
    write(
        dir.path(),
        "objects.yaml",
        &format!("{FLIGHT}---\n{AIRPORT}"),
    );
    write(dir.path(), "bindings/dev/airport.yaml", AIRPORT_BINDING);
    write(
        dir.path(),
        "bindings/dev/fixtures/airports.csv",
        "code\nFRA\n",
    );
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri()));

    let out = ontoforge(args(
        dir.path(),
        &[
            "--online",
            "--server-config",
            &cfg,
            "--check-pk",
            "--format",
            "json",
        ],
    ))
    .await;
    let diags = diagnostics(&out);
    assert_eq!(out.status.code(), Some(3), "{diags:#?}");
    let v19 = with_code(&diags, "ONT-V19");
    assert_eq!(v19.len(), 1, "{diags:#?}");
    assert!(message(v19[0]).contains("authentication"), "{diags:#?}");
    let v18 = with_code(&diags, "ONT-V18");
    assert_eq!(v18.len(), 1, "{diags:#?}");
    assert_eq!(v18[0]["file"], "bindings/dev/airport.yaml");
    assert!(message(v18[0]).contains("FRA"), "{diags:#?}");
    // The ONT-V17 infos of the described bindings are kept as well.
    assert_eq!(with_code(&diags, "ONT-V17").len(), 2, "{diags:#?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn check_pk_skips_mismatched_bindings() {
    let server = MockServer::start().await;
    // The key column is missing, so the binding already has an ONT-V17 error.
    mount_info(
        &server,
        json!([["aviation", "flights", "status", "varchar", "YES"]]),
    )
    .await;
    mount_pk(&server, json!([["F001", 2]]), 0).await;
    let dir = ontology(CLEAN_CSV);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri()));

    let out = ontoforge(args(
        dir.path(),
        &[
            "--online",
            "--server-config",
            &cfg,
            "--check-pk",
            "--format",
            "json",
        ],
    ))
    .await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert!(with_code(&diags, "ONT-V18").is_empty(), "{diags:#?}");
    let bodies = request_bodies(&server.received_requests().await.expect("recorded"));
    assert_eq!(bodies.len(), 1, "only the description: {bodies:#?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn exit_code_table() {
    // 0: a clean online check.
    let out = ontoforge(args(Path::new(AVIATION), &["--online"])).await;
    assert_eq!(out.status.code(), Some(0), "clean");

    // 1: ONT-V17, a column the binding needs is missing.
    let server = MockServer::start().await;
    mount_info(
        &server,
        json!([["aviation", "flights", "flightid", "varchar", "NO"]]),
    )
    .await;
    let dir = ontology(CLEAN_CSV);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri()));
    let out = ontoforge(args(
        dir.path(),
        &["--online", "--server-config", &cfg, "--format", "json"],
    ))
    .await;
    assert_eq!(out.status.code(), Some(1), "V17");
    let diags = diagnostics(&out);
    assert!(
        with_code(&diags, "ONT-V17")
            .iter()
            .any(|d| d["severity"] == "error"),
        "{diags:#?}"
    );

    // 1: ONT-V18, a duplicate primary key.
    let dup = ontology("flightId,status\nF001,a\nF001,b\n");
    let out = ontoforge(args(
        dup.path(),
        &["--online", "--check-pk", "--format", "json"],
    ))
    .await;
    assert_eq!(out.status.code(), Some(1), "V18");
    assert_eq!(with_code(&diagnostics(&out), "ONT-V18").len(), 1);

    // 1: ONT-V20, the server configuration does not exist.
    let missing = dir.path().join("no-such-server.toml");
    let out = ontoforge(args(
        dir.path(),
        &[
            "--online",
            "--server-config",
            missing.to_str().expect("utf-8 path"),
            "--format",
            "json",
        ],
    ))
    .await;
    assert_eq!(out.status.code(), Some(1), "V20");
    assert_eq!(with_code(&diagnostics(&out), "ONT-V20").len(), 1);

    // 2: usage error, --check-pk needs --online.
    let out = ontoforge(args(dir.path(), &["--check-pk"])).await;
    assert_eq!(out.status.code(), Some(2), "usage");

    // 3: Trino cannot be reached.
    let (_closed_dir, closed) = config_file(&trino_config(&format!(
        "http://127.0.0.1:{}",
        closed_port()
    )));
    let out = ontoforge(args(
        dir.path(),
        &[
            "--online",
            "--server-config",
            &closed,
            "--check-pk",
            "--format",
            "json",
        ],
    ))
    .await;
    assert_eq!(out.status.code(), Some(3), "unavailable");
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0]["code"], "ONT-V19");
}
