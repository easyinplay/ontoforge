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
