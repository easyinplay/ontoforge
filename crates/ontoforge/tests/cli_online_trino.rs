//! `ontoforge validate --online --server-config`: the CLI against a Trino that is a wiremock
//! server (BIND-03, ENGINE-01, ENGINE-02).

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
  flightNumber: { type: string }
  status: { type: string }
  delayMinutes: { type: integer }
backing: source
";

const AIRPORT: &str = "\
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
backing: source
";

const FLIGHT_BINDING: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightNumber: { column: flight_no }
  delayMinutes: { column: delay_min }
fixture: { file: fixtures/flights.csv }
";

const AIRPORT_BINDING: &str = "\
kind: SourceBinding
target: { objectType: Airport }
source: { catalog: other, schema: aviation, table: airports }
fixture: { file: fixtures/airports.csv }
";

const FLIGHT_CSV: &str = "flightId,flight_no,status,delay_min\nF1,LH1,ok,5\n";
const AIRPORT_CSV: &str = "code\nFRA\n";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// An ontology with the source object type `Flight`, and `Airport` when `with_airport`.
fn ontology(with_airport: bool) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "ontology.yaml", MANIFEST);
    let objects = if with_airport {
        format!("{FLIGHT}---\n{AIRPORT}")
    } else {
        FLIGHT.to_owned()
    };
    write(dir.path(), "objects.yaml", &objects);
    write(dir.path(), "bindings/dev/flight.yaml", FLIGHT_BINDING);
    write(dir.path(), "bindings/dev/fixtures/flights.csv", FLIGHT_CSV);
    if with_airport {
        write(dir.path(), "bindings/dev/airport.yaml", AIRPORT_BINDING);
        write(
            dir.path(),
            "bindings/dev/fixtures/airports.csv",
            AIRPORT_CSV,
        );
    }
    dir
}

fn trino_config(url: &str, extra_trino: &str) -> String {
    format!(
        "[trino]\nurl = \"{url}\"\nconnect_timeout_ms = 1000\n{extra_trino}\n\
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

/// Runs the CLI in a blocking task so that the wiremock server keeps being served.
async fn ontoforge(args: Vec<String>, envs: Vec<(String, String)>) -> Output {
    tokio::task::spawn_blocking(move || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ontoforge"));
        command
            .args(args)
            .env_remove("ONTOFORGE_PROFILE")
            .env("NO_COLOR", "1");
        for (key, value) in envs {
            command.env(key, value);
        }
        command.output().expect("spawn ontoforge")
    })
    .await
    .expect("join")
}

fn online_args(dir: &Path, config: &str, extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "validate".to_owned(),
        dir.to_str().expect("utf-8 path").to_owned(),
        "--online".to_owned(),
        "--server-config".to_owned(),
        config.to_owned(),
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

fn info_columns() -> Value {
    let column = |name: &str| json!({ "name": name, "type": "varchar" });
    json!([
        column("table_schema"),
        column("table_name"),
        column("column_name"),
        column("data_type"),
        column("is_nullable")
    ])
}

fn page(rows: &Value) -> Value {
    json!({ "id": "q1", "columns": info_columns(), "data": rows, "stats": { "state": "FINISHED" } })
}

/// The `information_schema.columns` rows of a Flight table that matches `FLIGHT_BINDING`.
fn flight_rows() -> Value {
    json!([
        ["aviation", "flights", "flightid", "varchar", "NO"],
        ["aviation", "flights", "flight_no", "varchar", "YES"],
        ["aviation", "flights", "status", "varchar", "YES"],
        ["aviation", "flights", "delay_min", "integer", "YES"]
    ])
}

async fn mount_catalog(server: &MockServer, catalog: &str, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains(format!(
            "\"{catalog}\".\"information_schema\""
        )))
        .respond_with(response)
        .mount(server)
        .await;
}

async fn mount_rows(server: &MockServer, catalog: &str, rows: &Value) {
    mount_catalog(
        server,
        catalog,
        ResponseTemplate::new(200).set_body_json(page(rows)),
    )
    .await;
}

/// A local port nothing listens on.
fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("addr").port()
}

fn offset_of(text: &str, needle: &str) -> usize {
    text.find(needle).expect("needle in text")
}

/// 1-based line and column of byte `offset` in `text`.
fn line_col(text: &str, offset: usize) -> (u64, u64) {
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line as u64, column as u64)
}

#[tokio::test(flavor = "multi_thread")]
async fn trino_profile_clean() {
    let server = MockServer::start().await;
    mount_rows(&server, "demo", &flight_rows()).await;
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri(), ""));

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--format", "json"]),
        Vec::new(),
    )
    .await;
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    let d = &diags[0];
    assert_eq!(d["code"], "ONT-V17");
    assert_eq!(d["severity"], "warning");
    assert_eq!(d["file"], "bindings/dev/flight.yaml");
    assert!(d["message"].as_str().unwrap().contains("fixture"));
    let offset = offset_of(FLIGHT_BINDING, "fixtures/flights.csv");
    assert_eq!(d["span"]["offset"], offset);

    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(requests.len(), 1, "one request for one catalog");
    let request = &requests[0];
    let token = request
        .headers
        .get("X-Trino-Trace-Token")
        .expect("trace token")
        .to_str()
        .expect("ascii");
    assert!(token.starts_with("ontoforge-cli-"), "{token}");
    assert_eq!(
        request
            .headers
            .get("X-Trino-Client-Tags")
            .expect("client tags")
            .to_str()
            .expect("ascii"),
        "ontoforge,validate-online"
    );
    let body = String::from_utf8_lossy(&request.body);
    assert!(
        !body.to_uppercase().contains("GROUP BY"),
        "no full table scan without --check-pk: {body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn trino_profile_missing_column() {
    let server = MockServer::start().await;
    mount_rows(
        &server,
        "demo",
        &json!([
            ["aviation", "flights", "flightid", "varchar", "NO"],
            ["aviation", "flights", "flight_no", "varchar", "YES"],
            ["aviation", "flights", "delay_min", "integer", "YES"]
        ]),
    )
    .await;
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri(), ""));

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--format", "json"]),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    let errors: Vec<Value> = diagnostics(&out)
        .into_iter()
        .filter(|d| d["severity"] == "error")
        .collect();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0]["code"], "ONT-V17");
    assert_eq!(errors[0]["file"], "bindings/dev/flight.yaml");
    assert!(errors[0]["message"].as_str().unwrap().contains("status"));
}

#[tokio::test(flavor = "multi_thread")]
async fn trino_type_mismatch() {
    let server = MockServer::start().await;
    mount_rows(
        &server,
        "demo",
        &json!([
            ["aviation", "flights", "flightid", "varchar", "NO"],
            ["aviation", "flights", "flight_no", "varchar", "YES"],
            ["aviation", "flights", "status", "varchar", "YES"],
            ["aviation", "flights", "delay_min", "varchar", "YES"]
        ]),
    )
    .await;
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri(), ""));

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--format", "json"]),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    let errors: Vec<Value> = diagnostics(&out)
        .into_iter()
        .filter(|d| d["severity"] == "error")
        .collect();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    let d = &errors[0];
    assert_eq!(d["code"], "ONT-V17");
    assert_eq!(d["file"], "bindings/dev/flight.yaml");
    assert_eq!(d["span"]["offset"], offset_of(FLIGHT_BINDING, "delay_min"));
    assert!(d["message"].as_str().unwrap().contains("delayMinutes"));
}

#[tokio::test(flavor = "multi_thread")]
async fn trino_missing_table() {
    let server = MockServer::start().await;
    mount_rows(&server, "demo", &json!([])).await;
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri(), ""));

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--format", "json"]),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    let errors: Vec<Value> = diagnostics(&out)
        .into_iter()
        .filter(|d| d["severity"] == "error")
        .collect();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    let d = &errors[0];
    assert_eq!(d["code"], "ONT-V17");
    let offset = offset_of(FLIGHT_BINDING, "table: flights") + "table: ".len();
    assert_eq!(d["span"]["offset"], offset);
}

#[tokio::test(flavor = "multi_thread")]
async fn one_catalog_query_error_is_v19_others_checked() {
    let server = MockServer::start().await;
    // `demo` answers with a table that lacks the `status` column.
    mount_rows(
        &server,
        "demo",
        &json!([
            ["aviation", "flights", "flightid", "varchar", "NO"],
            ["aviation", "flights", "flight_no", "varchar", "YES"],
            ["aviation", "flights", "delay_min", "integer", "YES"]
        ]),
    )
    .await;
    mount_catalog(
        &server,
        "other",
        ResponseTemplate::new(200).set_body_json(json!({
            "id": "q2",
            "error": { "message": "denied", "errorName": "PERMISSION_DENIED" },
            "stats": { "state": "FAILED" }
        })),
    )
    .await;
    let dir = ontology(true);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri(), ""));

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--format", "json"]),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(3));
    let diags = diagnostics(&out);
    let v19: Vec<&Value> = diags.iter().filter(|d| d["code"] == "ONT-V19").collect();
    let v17: Vec<&Value> = diags
        .iter()
        .filter(|d| d["code"] == "ONT-V17" && d["severity"] == "error")
        .collect();
    assert_eq!(v19.len(), 1, "{diags:#?}");
    assert_eq!(v17.len(), 1, "{diags:#?}");
    assert_eq!(v19[0]["file"], "bindings/dev/airport.yaml");
    assert_eq!(
        v19[0]["span"]["offset"],
        offset_of(AIRPORT_BINDING, "{ catalog")
    );
    assert!(
        v19[0]["message"]
            .as_str()
            .unwrap()
            .contains("PERMISSION_DENIED")
    );
    assert_eq!(v17[0]["file"], "bindings/dev/flight.yaml");

    let human = ontoforge(online_args(dir.path(), &cfg, &[]), Vec::new()).await;
    assert_eq!(human.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(!stderr.contains("ok:"), "{stderr}");
}

#[tokio::test(flavor = "multi_thread")]
async fn timeout_flag_is_sent() {
    let server = MockServer::start().await;
    mount_rows(&server, "demo", &flight_rows()).await;
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri(), "query_timeout_ms = 10000\n"));

    let session = |request: &wiremock::Request| {
        request
            .headers
            .get("X-Trino-Session")
            .expect("session header")
            .to_str()
            .expect("ascii")
            .to_owned()
    };

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--timeout", "2000"]),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(0));
    let out = ontoforge(online_args(dir.path(), &cfg, &[]), Vec::new()).await;
    assert_eq!(out.status.code(), Some(0));

    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(requests.len(), 2);
    assert_eq!(session(&requests[0]), "query_max_execution_time=7s");
    assert_eq!(session(&requests[1]), "query_max_execution_time=15s");
}

#[tokio::test(flavor = "multi_thread")]
async fn trino_unavailable_exits_3() {
    let port = closed_port();
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&trino_config(&format!("http://127.0.0.1:{port}"), ""));

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--format", "json"]),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(3));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "only the engine failure is reported");
    assert_eq!(diags[0]["code"], "ONT-V19");
    assert!(
        diags[0]["message"]
            .as_str()
            .unwrap()
            .contains("Trino is unavailable"),
        "{diags:#?}"
    );

    let human = ontoforge(online_args(dir.path(), &cfg, &[]), Vec::new()).await;
    assert_eq!(human.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(!stderr.contains("ok:"), "{stderr}");
    assert!(
        stderr.contains("online check could not complete"),
        "{stderr}"
    );
    // The reason carries the operating system's wording and the port; neither belongs in a
    // snapshot that is compared on every platform.
    // The renderer wraps a long reason onto `│` continuation lines, which go with it.
    let stdout = String::from_utf8_lossy(&human.stdout).replace(&port.to_string(), "<port>");
    let mut in_reason = false;
    let stdout: String = stdout
        .lines()
        .filter_map(|line| match line.split_once("Trino is unavailable:") {
            Some((head, _)) => {
                in_reason = true;
                Some(format!("{head}Trino is unavailable: <reason>"))
            }
            None if in_reason && line.trim_start().starts_with('│') => None,
            None => {
                in_reason = false;
                Some(line.to_owned())
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("unavailable_human", stdout);
}

#[tokio::test(flavor = "multi_thread")]
async fn unauthorized_exits_3() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&trino_config(&server.uri(), ""));

    let out = ontoforge(
        online_args(dir.path(), &cfg, &["--format", "json"]),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(3));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0]["code"], "ONT-V19");
    assert!(
        diags[0]["message"]
            .as_str()
            .unwrap()
            .contains("authentication")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn secret_never_printed() {
    const SECRET: &str = "s3cr3t-token-value";
    let port = closed_port();
    let text = format!(
        "[trino]\nurl = \"https://127.0.0.1:{port}\"\nconnect_timeout_ms = 1000\n\
         query_timeout_ms = 3000\n\n\
         [trino.identities.ontology]\nuser = \"ontoforge\"\nauth = \"bearer\"\n\
         token_env = \"ONTOFORGE_TEST_TOKEN\"\n\n[profiles.dev]\nengine = \"trino\"\n"
    );
    let dir = ontology(false);
    let (_cfg_dir, cfg) = config_file(&text);
    let envs = vec![("ONTOFORGE_TEST_TOKEN".to_owned(), SECRET.to_owned())];

    for extra in [&[][..], &["--format", "json"][..]] {
        let out = ontoforge(online_args(dir.path(), &cfg, extra), envs.clone()).await;
        assert_eq!(out.status.code(), Some(3), "{extra:?}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!stdout.contains(SECRET), "{stdout}");
        assert!(!stderr.contains(SECRET), "{stderr}");
    }
}

/// CR-C01: a rejected plaintext secret, or the userinfo of `url`, never reaches the output,
/// also in the human format, which prints the lines around a span of a registered source.
#[tokio::test(flavor = "multi_thread")]
async fn rejected_plaintext_secret_never_printed() {
    const SECRET: &str = "Hunter2SuperSecret";
    let dir = ontology(false);
    let configs = [
        format!(
            "[trino]\nurl = \"https://127.0.0.1:9\"\n\n[trino.identities.ontology]\n\
             user = \"u\"\npassword = \"{SECRET}\"\n\n[profiles.dev]\nengine = \"trino\"\n"
        ),
        format!(
            "[trino]\nurl = \"https://127.0.0.1:9\"\n\n[trino.identities.ontology]\n\
             user = \"u\"\ntoken = \"{SECRET}\"\n\n[profiles.dev]\nengine = \"trino\"\n"
        ),
        format!(
            "[trino]\nurl = \"https://alice:{SECRET}@127.0.0.1:9\"\n\n\
             [trino.identities.ontology]\nuser = \"u\"\nauth = \"none\"\n\n\
             [profiles.dev]\nengine = \"trino\"\n"
        ),
    ];
    for text in configs {
        let (_cfg_dir, cfg) = config_file(&text);
        for extra in [&[][..], &["--format", "json"][..]] {
            let out = ontoforge(online_args(dir.path(), &cfg, extra), Vec::new()).await;
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{extra:?}\n{stdout}\n{stderr}");
            assert!(stdout.contains("ONT-V20"), "{extra:?}\n{stdout}");
            assert!(!stdout.contains(SECRET), "{extra:?}\n{stdout}");
            assert!(!stderr.contains(SECRET), "{extra:?}\n{stderr}");
            if extra.is_empty() {
                // The human format still names the place of the problem.
                let file_line = format!("{cfg}:");
                assert!(stdout.contains(&file_line), "{stdout}");
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn config_errors_are_v20_with_location() {
    let dir = ontology(false);
    let run = |text: String| {
        let dir_path = dir.path().to_path_buf();
        async move {
            let (cfg_dir, cfg) = config_file(&text);
            let out = ontoforge(
                online_args(&dir_path, &cfg, &["--format", "json"]),
                Vec::new(),
            )
            .await;
            drop(cfg_dir);
            (out, cfg, text)
        }
    };

    // A plaintext secret points at its key.
    let (out, cfg, text) = run(
        "[trino]\nurl = \"https://127.0.0.1:9\"\n\n[trino.identities.ontology]\nuser = \"u\"\npassword = \"x\"\n\n[profiles.dev]\nengine = \"trino\"\n"
            .to_owned(),
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0]["code"], "ONT-V20");
    assert_eq!(diags[0]["file"], cfg.as_str());
    let (line, column) = line_col(&text, offset_of(&text, "password"));
    assert_eq!(diags[0]["line"], line);
    assert_eq!(diags[0]["column"], column);
    assert!(diags[0]["message"].as_str().unwrap().contains("plaintext"));

    // A profile name must be lower case (E14).
    let (out, cfg, text) = run("[profiles.Prod]\nengine = \"sqlite\"\n".to_owned()).await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0]["code"], "ONT-V20");
    assert_eq!(diags[0]["file"], cfg.as_str());
    let (line, _) = line_col(&text, offset_of(&text, "Prod"));
    assert_eq!(diags[0]["line"], line);

    // The selected profile has no mapping.
    let (out, cfg, _) = run("[profiles.prod]\nengine = \"sqlite\"\n".to_owned()).await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0]["code"], "ONT-V20");
    assert_eq!(diags[0]["file"], cfg.as_str());
    assert!(diags[0]["message"].as_str().unwrap().contains("dev"));

    // Nothing but `engine` is allowed in a profile.
    let (out, _, text) =
        run("[profiles.dev]\nengine = \"sqlite\"\nurl = \"http://x\"\n".to_owned()).await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0]["code"], "ONT-V20");
    let (line, _) = line_col(&text, offset_of(&text, "url"));
    assert_eq!(diags[0]["line"], line);

    // A configuration file that does not exist.
    let missing = TempDir::new().expect("tempdir");
    let missing = missing.path().join("nope.toml");
    let out = ontoforge(
        online_args(
            dir.path(),
            missing.to_str().expect("utf-8"),
            &["--format", "json"],
        ),
        Vec::new(),
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert_eq!(diags[0]["code"], "ONT-V20");
}

#[test]
fn check_pk_requires_online() {
    let out = Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(["validate", AVIATION, "--check-pk"])
        .env_remove("ONTOFORGE_PROFILE")
        .output()
        .expect("spawn ontoforge");
    assert_eq!(out.status.code(), Some(2));
}

#[tokio::test(flavor = "multi_thread")]
async fn sqlite_profile_uses_fixtures() {
    let (_cfg_dir, cfg) = config_file("[profiles.dev]\nengine = \"sqlite\"\n");
    let out = ontoforge(
        vec![
            "validate".to_owned(),
            AVIATION.to_owned(),
            "--online".to_owned(),
            "--server-config".to_owned(),
            cfg,
            "--format".to_owned(),
            "json".to_owned(),
            "--color".to_owned(),
            "never".to_owned(),
        ],
        Vec::new(),
    )
    .await;
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "[]");
}
