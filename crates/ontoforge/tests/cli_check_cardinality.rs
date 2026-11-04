//! Explicit source OneToOne checks through the real CLI and SQLite fixture engine.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const OBJECTS: &str = "\
kind: ObjectType
apiName: Aircraft
primaryKey: id
properties:
  id: { type: string, required: true }
backing: editOnly
---
kind: ObjectType
apiName: MaintenanceRecord
primaryKey: id
properties:
  id: { type: string, required: true }
  aircraftId: { type: string }
backing: source
---
kind: LinkType
apiName: aircraftMaintenance
cardinality: oneToOne
from: { objectType: Aircraft, apiName: maintenance }
to: { objectType: MaintenanceRecord, apiName: aircraft, foreignKey: aircraftId }
";
const BINDING: &str = "\
kind: SourceBinding
target: { objectType: MaintenanceRecord }
source: { catalog: demo, schema: aviation, table: records }
columns:
  id: { column: record_id }
  aircraftId: { column: aircraft_id }
fixture: { file: fixtures/records.csv }
";

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("write");
}

fn ontology(csv: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "ontology.yaml",
        "apiName: demo\ndefaultProfile: dev\n",
    );
    write(dir.path(), "objects.yaml", OBJECTS);
    write(dir.path(), "bindings/dev/record.yaml", BINDING);
    write(dir.path(), "bindings/dev/fixtures/records.csv", csv);
    dir
}

fn run(root: &Path, format: &str) -> Output {
    run_with(
        root,
        &["--online", "--check-cardinality", "--format", format],
    )
}

fn run_with(root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .arg("validate")
        .arg(root)
        .args(extra)
        .args(["--color", "never"])
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("CLI")
}

fn diagnostics(out: &Output) -> Vec<Value> {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn has_code(out: &Output, code: &str) -> bool {
    diagnostics(out).iter().any(|diag| diag["code"] == code)
}

fn server_config(root: &Path, url: &str, limits: &str) -> String {
    write(
        root,
        "server.toml",
        &format!(
            "[trino]\nurl = \"{url}\"\nconnect_timeout_ms = 1000\n{limits}\n[trino.identities.ontology]\nuser = \"ontoforge\"\nauth = \"none\"\n[profiles.dev]\nengine = \"trino\"\n"
        ),
    );
    root.join("server.toml").to_str().expect("utf8").to_owned()
}

async fn run_remote(root: &Path, config: &str, extra: &[&str]) -> Output {
    let root = root.to_owned();
    let config = config.to_owned();
    let extra: Vec<String> = extra.iter().map(|value| (*value).to_owned()).collect();
    tokio::task::spawn_blocking(move || {
        let mut args = vec!["--online", "--format", "json", "--server-config", &config];
        args.extend(extra.iter().map(String::as_str));
        run_with(&root, &args)
    })
    .await
    .expect("join CLI")
}

fn info_rows() -> Value {
    json!([
        ["aviation", "records", "record_id", "varchar", "NO"],
        ["aviation", "records", "aircraft_id", "varchar", "YES"]
    ])
}

fn page(columns: Value, rows: Value) -> Value {
    json!({"id":"q1", "columns":columns, "data":rows, "stats":{"state":"FINISHED"}})
}

fn result_page(rows: Value) -> Value {
    page(
        json!([{"name":"fk","type":"varchar"},{"name":"n","type":"bigint"}]),
        rows,
    )
}

async fn mount_info(server: &MockServer, rows: Value) {
    Mock::given(method("POST")).and(path("/v1/statement"))
        .and(body_string_contains("\"information_schema\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(
            json!([{"name":"table_schema","type":"varchar"},{"name":"table_name","type":"varchar"},{"name":"column_name","type":"varchar"},{"name":"data_type","type":"varchar"},{"name":"is_nullable","type":"varchar"}]), rows)))
        .mount(server).await;
}

async fn mount_cardinality(server: &MockServer, rows: Value) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("count(DISTINCT"))
        .respond_with(ResponseTemplate::new(200).set_body_json(result_page(rows)))
        .mount(server)
        .await;
}

async fn cardinality_requests(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .expect("requests")
        .iter()
        .filter(|request| String::from_utf8_lossy(&request.body).contains("count(DISTINCT"))
        .count()
}

#[test]
fn cardinality_flag_contract() {
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM1,A1\nM2,A1\n");
    let missing_online = run_with(dir.path(), &["--check-cardinality"]);
    assert_eq!(missing_online.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_online.stderr).contains("--online"));
    let single = run(dir.path(), "json");
    assert_eq!(single.status.code(), Some(0));
    assert!(has_code(&single, "ONT-W18"));
    assert!(!has_code(&single, "ONT-V18"));
    let both = run_with(
        dir.path(),
        &[
            "--online",
            "--check-pk",
            "--check-cardinality",
            "--format",
            "json",
        ],
    );
    assert_eq!(both.status.code(), Some(1));
    assert!(has_code(&both, "ONT-W18"));
    assert!(has_code(&both, "ONT-V18"));
}

#[test]
fn warning_output_and_exit_codes() {
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let ordinary = run(dir.path(), "json");
    let denied = run_with(
        dir.path(),
        &[
            "--online",
            "--check-cardinality",
            "--deny-warnings",
            "--format",
            "json",
        ],
    );
    assert_eq!(ordinary.status.code(), Some(0));
    assert_eq!(denied.status.code(), Some(1));
    assert_eq!(diagnostics(&ordinary), diagnostics(&denied));
    let warnings = diagnostics(&ordinary);
    let warning = warnings
        .iter()
        .find(|d| d["code"] == "ONT-W18")
        .expect("warning");
    assert_eq!(warning["severity"], "warning");
    assert_eq!(warning["line"], 6);
    assert!(
        warning["help"]
            .as_str()
            .expect("help")
            .contains("was not checked")
    );
    for (deny, expected) in [(false, 0), (true, 1)] {
        let mut args = vec!["--online", "--check-cardinality"];
        if deny {
            args.push("--deny-warnings");
        }
        let out = run_with(dir.path(), &args);
        assert_eq!(out.status.code(), Some(expected));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains("ONT-W18"));
        assert!(!text.contains("ok: ontology"));
    }
    write(
        dir.path(),
        "bindings/dev/record.yaml",
        &BINDING.replace("  aircraftId: { column: aircraft_id }\n", ""),
    );
    write(
        dir.path(),
        "bindings/dev/fixtures/records.csv",
        "record_id,aircraftId\nM1,A1\nM2,A1\n",
    );
    let fallback = run(dir.path(), "json");
    assert_eq!(fallback.status.code(), Some(0));
    let diags = diagnostics(&fallback);
    assert_eq!(
        diags
            .iter()
            .find(|d| d["code"] == "ONT-W18")
            .expect("fallback location")["line"],
        3
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_flag_means_no_cardinality_scan() {
    let server = MockServer::start().await;
    mount_info(&server, info_rows()).await;
    mount_cardinality(&server, json!([["A1", 2]])).await;
    Mock::given(method("POST"))
        .and(body_string_contains("HAVING count(*)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(result_page(json!([]))))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("IS NULL"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page(json!([{"name":"n","type":"bigint"}]), json!([[0]]))),
        )
        .mount(&server)
        .await;
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let config = server_config(dir.path(), &server.uri(), "");
    for extra in [vec![], vec!["--check-pk"]] {
        let out = run_remote(dir.path(), &config, &extra).await;
        assert_eq!(out.status.code(), Some(0), "{:?}", diagnostics(&out));
        assert_eq!(cardinality_requests(&server).await, 0);
    }
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(
        requests.len(),
        4,
        "two metadata requests and two PK queries"
    );
    let explicit = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
    assert_eq!(explicit.status.code(), Some(0));
    assert!(has_code(&explicit, "ONT-W18"));
    assert_eq!(cardinality_requests(&server).await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn non_candidates_are_not_scanned() {
    let edit_target = format!(
        "{}---\nkind: ObjectType\napiName: BackupRecord\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  aircraftId: {{ type: string }}\nbacking: editOnly\n",
        OBJECTS.replace(
            "to: { objectType: MaintenanceRecord",
            "to: { objectType: BackupRecord"
        )
    );
    for objects in [
        OBJECTS.replace("oneToOne", "oneToMany"),
        OBJECTS
            .replace(
                "cardinality: oneToOne",
                "cardinality: manyToMany\nbacking: editOnly",
            )
            .replace(", foreignKey: aircraftId", ""),
        edit_target,
    ] {
        let server = MockServer::start().await;
        mount_info(&server, info_rows()).await;
        let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
        write(dir.path(), "objects.yaml", &objects);
        let config = server_config(dir.path(), &server.uri(), "");
        let out = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
        assert_eq!(out.status.code(), Some(0), "{:?}", diagnostics(&out));
        assert_eq!(
            server
                .received_requests()
                .await
                .expect("metadata executed")
                .len(),
            1
        );
        assert_eq!(cardinality_requests(&server).await, 0);
        assert!(!has_code(&out, "ONT-W18"));
    }
}

#[test]
fn long_sample_values_are_bounded() {
    let mut csv = String::from("record_id,aircraft_id\n");
    for i in 0..22 {
        let value = format!("{i:02}{}", "界".repeat(80));
        csv.push_str(&format!("M{i}a,{value}\nM{i}b,{value}\n"));
    }
    let dir = ontology(&csv);
    let out = run(dir.path(), "json");
    assert_eq!(out.status.code(), Some(0));
    let diags = diagnostics(&out);
    let warning = diags
        .iter()
        .find(|d| d["code"] == "ONT-W18")
        .expect("warning");
    let message = warning["message"].as_str().expect("message");
    assert_eq!(message.matches("distinct objects").count(), 20);
    assert!(message.contains("and more (showing up to 20)"));
    assert!(message.contains(&format!("00{}…", "界".repeat(62))));
    assert!(!message.contains(&"界".repeat(63)));
    assert!(message.chars().count() < 2000, "bounded message");
}

fn three_targets() -> TempDir {
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let mut objects = OBJECTS.to_owned();
    for (object, link, accessor, table) in [
        ("BackupRecord", "betaMaintenance", "backup", "records_b"),
        ("OtherRecord", "gammaMaintenance", "other", "records_c"),
    ] {
        objects.push_str(&format!("---\nkind: ObjectType\napiName: {object}\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  aircraftId: {{ type: string }}\nbacking: source\n---\nkind: LinkType\napiName: {link}\ncardinality: oneToOne\nfrom: {{ objectType: Aircraft, apiName: {accessor} }}\nto: {{ objectType: {object}, apiName: aircraft, foreignKey: aircraftId }}\n"));
        write(
            dir.path(),
            &format!("bindings/dev/{table}.yaml"),
            &BINDING
                .replace("MaintenanceRecord", object)
                .replace("table: records", &format!("table: {table}")),
        );
    }
    write(dir.path(), "objects.yaml", &objects);
    dir
}

fn three_info_rows() -> Value {
    let mut rows = Vec::new();
    for table in ["records", "records_b", "records_c"] {
        rows.extend([
            json!(["aviation", table, "record_id", "varchar", "NO"]),
            json!(["aviation", table, "aircraft_id", "varchar", "YES"]),
        ]);
    }
    Value::Array(rows)
}

async fn mount_slow_scan(server: &MockServer, table: &str, next: &str) {
    Mock::given(method("POST")).and(body_string_contains("count(DISTINCT"))
        .and(body_string_contains(format!("\"{table}\"")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"slow", "nextUri":format!("{}{next}",server.uri()), "stats":{"state":"QUEUED"}})))
        .mount(server).await;
    Mock::given(method("GET"))
        .and(path(next))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(result_page(json!([])))
                .set_delay(std::time::Duration::from_secs(2)),
        )
        .mount(server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(next))
        .respond_with(ResponseTemplate::new(204))
        .mount(server)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cardinality_total_deadline_is_shared() {
    let server = MockServer::start().await;
    mount_info(&server, three_info_rows()).await;
    Mock::given(method("POST"))
        .and(body_string_contains("count(DISTINCT"))
        .and(body_string_contains("\"records\""))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(result_page(json!([["A1", 2]])))
                .set_delay(std::time::Duration::from_millis(80)),
        )
        .mount(&server)
        .await;
    let next = "/v1/statement/executing/cardinality/1";
    mount_slow_scan(&server, "records_b", next).await;
    Mock::given(method("POST"))
        .and(body_string_contains("count(DISTINCT"))
        .and(body_string_contains("\"records_c\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(result_page(json!([]))))
        .mount(&server)
        .await;
    let dir = three_targets();
    let config = server_config(dir.path(), &server.uri(), "query_timeout_ms = 500");
    let out = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
    assert_eq!(out.status.code(), Some(3), "{:?}", diagnostics(&out));
    assert!(has_code(&out, "ONT-W18"), "earlier warning must survive");
    assert!(has_code(&out, "ONT-V19"));
    assert_eq!(
        cardinality_requests(&server).await,
        2,
        "third target must not be queried after the total deadline"
    );
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == wiremock::http::Method::DELETE
                && request.url.path() == next)
            .count(),
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn precheck_errors_skip_cardinality_scan() {
    for rows in [
        json!([]),
        json!([["aviation", "records", "record_id", "varchar", "NO"]]),
        json!([
            ["aviation", "records", "record_id", "varchar", "NO"],
            ["aviation", "records", "aircraft_id", "bigint", "YES"]
        ]),
    ] {
        let server = MockServer::start().await;
        mount_info(&server, rows).await;
        let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
        let config = server_config(dir.path(), &server.uri(), "");
        let out = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
        assert_eq!(out.status.code(), Some(1), "{:?}", diagnostics(&out));
        assert!(has_code(&out, "ONT-V17"));
        assert_eq!(cardinality_requests(&server).await, 0);
        assert_eq!(server.received_requests().await.expect("metadata").len(), 1);
    }
    let server = MockServer::start().await;
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let config = server_config(dir.path(), &server.uri(), "");
    write(
        dir.path(),
        "objects.yaml",
        &OBJECTS.replace("cardinality: oneToOne", "cardinality: invalid"),
    );
    let offline = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
    assert_eq!(offline.status.code(), Some(1));
    assert!(
        server
            .received_requests()
            .await
            .expect("no requests")
            .is_empty()
    );
    write(dir.path(), "objects.yaml", OBJECTS);
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let metadata = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
    assert_eq!(metadata.status.code(), Some(3));
    assert!(has_code(&metadata, "ONT-V19"));
    assert_eq!(cardinality_requests(&server).await, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn incomplete_scan_is_not_clean() {
    for (limits, response) in [
        (
            "max_rows = 2",
            result_page(json!([["A", 2], ["B", 2], ["C", 2]])),
        ),
        (
            "max_bytes = 300",
            result_page(json!([["x".repeat(400), 2]])),
        ),
        ("", result_page(json!([["A", 1]]))),
        ("", result_page(json!([["A", "invalid count"]]))),
        (
            "",
            json!({"id":"q1", "error":{"errorName":"PERMISSION_DENIED", "message":"denied"}}),
        ),
    ] {
        let server = MockServer::start().await;
        mount_info(&server, info_rows()).await;
        Mock::given(method("POST"))
            .and(body_string_contains("count(DISTINCT"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
        let config = server_config(dir.path(), &server.uri(), limits);
        let out = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
        assert_eq!(
            out.status.code(),
            Some(3),
            "{limits}: {:?}",
            diagnostics(&out)
        );
        assert!(has_code(&out, "ONT-V19"));
        assert!(!has_code(&out, "ONT-W18"));
        assert_eq!(
            cardinality_requests(&server).await,
            1,
            "must reach the data scan"
        );
    }
    let server = MockServer::start().await;
    mount_info(&server, info_rows()).await;
    mount_slow_scan(&server, "records", "/v1/statement/executing/slow/1").await;
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let config = server_config(dir.path(), &server.uri(), "query_timeout_ms = 200");
    let out = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
    assert_eq!(out.status.code(), Some(3));
    assert!(has_code(&out, "ONT-V19"));
    assert_eq!(cardinality_requests(&server).await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn failure_preserves_findings_and_drains() {
    for engine_wide in [false, true] {
        let server = MockServer::start().await;
        mount_info(&server, three_info_rows()).await;
        Mock::given(method("POST"))
            .and(body_string_contains("count(DISTINCT"))
            .and(body_string_contains("\"records\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(result_page(json!([["A", 2]]))))
            .mount(&server)
            .await;
        let next = "/v1/statement/executing/failing/1";
        Mock::given(method("POST")).and(body_string_contains("count(DISTINCT"))
            .and(body_string_contains("\"records_b\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"q2","nextUri":format!("{}{next}",server.uri()),"stats":{"state":"QUEUED"}})))
            .mount(&server).await;
        let response = if engine_wide {
            ResponseTemplate::new(401)
        } else {
            ResponseTemplate::new(200).set_body_string("{")
        };
        Mock::given(method("GET"))
            .and(path(next))
            .respond_with(response)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path(next))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_string_contains("count(DISTINCT"))
            .and(body_string_contains("\"records_c\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(result_page(json!([]))))
            .mount(&server)
            .await;
        let dir = three_targets();
        let config = server_config(dir.path(), &server.uri(), "");
        let out = run_remote(dir.path(), &config, &["--check-cardinality"]).await;
        assert_eq!(out.status.code(), Some(3), "{:?}", diagnostics(&out));
        assert!(has_code(&out, "ONT-W18"));
        assert!(has_code(&out, "ONT-V19"));
        assert_eq!(
            cardinality_requests(&server).await,
            if engine_wide { 2 } else { 3 }
        );
        let requests = server.received_requests().await.expect("requests");
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.method == wiremock::http::Method::DELETE
                    && request.url.path() == next)
                .count(),
            1,
            "cleanup request must complete before process exit"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cardinality_budget_starts_after_precheck_and_pk() {
    let server = MockServer::start().await;
    let columns = json!([{"name":"table_schema","type":"varchar"},{"name":"table_name","type":"varchar"},{"name":"column_name","type":"varchar"},{"name":"data_type","type":"varchar"},{"name":"is_nullable","type":"varchar"}]);
    Mock::given(method("POST"))
        .and(body_string_contains("\"information_schema\""))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page(columns, info_rows()))
                .set_delay(std::time::Duration::from_millis(180)),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("HAVING count(*)"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(result_page(json!([])))
                .set_delay(std::time::Duration::from_millis(180)),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("IS NULL"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(page(json!([{"name":"n","type":"bigint"}]), json!([[0]])))
                .set_delay(std::time::Duration::from_millis(180)),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("count(DISTINCT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(result_page(json!([["A", 2]])))
                .set_delay(std::time::Duration::from_millis(180)),
        )
        .mount(&server)
        .await;
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let config = server_config(dir.path(), &server.uri(), "query_timeout_ms = 300");
    let out = run_remote(dir.path(), &config, &["--check-cardinality", "--check-pk"]).await;
    assert_eq!(out.status.code(), Some(0), "{:?}", diagnostics(&out));
    assert!(has_code(&out, "ONT-W18"));
    assert!(!has_code(&out, "ONT-V19"));
    assert_eq!(server.received_requests().await.expect("requests").len(), 4);
}

#[test]
fn cardinality_tracer_reports_conflict() {
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let out = run(dir.path(), "json");
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected warning exit0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let diagnostics: Vec<Value> = serde_json::from_slice(&out.stdout).expect("diagnostics JSON");
    let warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d["code"] == "ONT-W18")
        .collect();
    assert_eq!(warnings.len(), 1, "{diagnostics:#?}");
    let warning = warnings[0];
    assert_eq!(warning["severity"], "warning");
    let message = warning["message"].as_str().expect("message");
    for expected in ["aircraftMaintenance", "aircraft_id", "A1", "2"] {
        assert!(message.contains(expected), "{message}");
    }
    assert!(
        warning["file"]
            .as_str()
            .expect("file")
            .ends_with("bindings/dev/record.yaml"),
        "{warning}"
    );
    assert_eq!(warning["line"], 6, "FK mapping location: {warning}");
    assert!(
        warning["help"]
            .as_str()
            .expect("help")
            .contains("referential integrity")
    );

    let human = run(dir.path(), "human");
    assert_eq!(human.status.code(), Some(0));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&human.stdout),
        String::from_utf8_lossy(&human.stderr)
    );
    assert!(text.contains("ONT-W18"), "{text}");
    assert!(text.contains("1 warning"), "{text}");
    assert!(
        !text.contains("ok: ontology"),
        "warning must not be summarized as clean: {text}"
    );
}

fn file_snapshot(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn collect(root: &Path, dir: &Path, files: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).expect("read directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                collect(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).expect("relative").to_owned(),
                    std::fs::read(path).expect("read file"),
                ));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

#[test]
fn repeated_check_is_read_only() {
    let dir = ontology("record_id,aircraft_id\nM4,B\nM2,A\nM1,A\nM3,B\nM5,C\nM6,C\nM7,C\nM1,A\n");
    let before = file_snapshot(dir.path());
    let first = run(dir.path(), "json");
    let second = run(dir.path(), "json");
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    let diagnostics: Vec<Value> = serde_json::from_slice(&first.stdout).expect("JSON");
    let warning = diagnostics
        .iter()
        .find(|d| d["code"] == "ONT-W18")
        .expect("warning");
    let message = warning["message"].as_str().expect("message");
    assert!(
        message
            .contains("`C`: 3 distinct objects, `A`: 2 distinct objects, `B`: 2 distinct objects"),
        "{message}"
    );
    assert_eq!(
        before,
        file_snapshot(dir.path()),
        "no source changes or edit store files"
    );
}
