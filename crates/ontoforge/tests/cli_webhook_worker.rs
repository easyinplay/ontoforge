use std::collections::{BTreeMap, BTreeSet};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
---
kind: ActionType
apiName: createGate
parameters:
  code: { type: string, required: true }
  name: { type: string, required: true }
rules:
  - kind: createObject
    objectType: Gate
    properties:
      code: { param: code }
      name: { param: name }
";

async fn committed(dir: &tempfile::TempDir) -> String {
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("store");
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &store,
        &loaded.ontology.expect("ontology"),
        &ReloadOptions::default(),
    )
    .await
    .expect("reload")
    .def_version
    .expect("definition version");
    let principal = Principal {
        id: PrincipalId::new("test", "alice"),
        issuer: "test".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("createGate".to_owned().try_into().expect("action"));
    scope.types.insert(
        "Gate".to_owned().try_into().expect("type"),
        TypeScope {
            row: Pred::Const(true),
            columns: ["code", "name"]
                .into_iter()
                .map(|name| {
                    (
                        name.to_owned().try_into().expect("column"),
                        Pred::Const(true),
                    )
                })
                .collect(),
        },
    );
    let tokens = TokenCodec::new(b"worker-cli-test-key".to_vec()).expect("tokens");
    let service = ActionService::new(
        &store,
        ActionLimits::new(8, 8, 65536).expect("limits"),
        &tokens,
    );
    let outcome = service
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "createGate".to_owned(),
                command_id: Some("worker-cli-command".to_owned()),
                items: vec![ActionItem {
                    params: serde_json::from_value(
                        serde_json::json!({"code":"G1", "name":"Private Name"}),
                    )
                    .expect("params"),
                    expected_versions: BTreeMap::new(),
                }],
                validate_only: false,
            },
        )
        .await
        .expect("Action submit");
    let ActionOutcome::Committed(receipt) = outcome else {
        panic!("expected committed Action")
    };
    receipt.event_id
}

fn write_config(dir: &tempfile::TempDir, target: &str) -> Utf8PathBuf {
    let config =
        Utf8PathBuf::from_path_buf(dir.path().join("ontoforge-server.toml")).expect("utf8 path");
    std::fs::write(&config, format!("[store]\nkind = \"sqlite\"\npath = \"edit.db\"\n\n[webhook]\ntarget = \"{target}/events\"\npoll_interval_ms = 50\nrequest_timeout_ms = 500\nlease_duration_ms = 2000\nbatch_limit = 4\n")).expect("config");
    config
}

struct Worker(Child);

impl Worker {
    fn start(config: &Utf8PathBuf) -> Self {
        Self(
            Command::new(env!("CARGO_BIN_EXE_ontoforge"))
                .args(["webhook-worker", "--server-config", config.as_str()])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn worker"),
        )
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn wait_requests(server: &MockServer, count: usize) -> Vec<wiremock::Request> {
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let requests = server.received_requests().await.expect("received requests");
            if requests.len() >= count {
                break requests;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("worker should deliver the committed event")
}

async fn reject_first_attempt(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/events"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/events"))
        .respond_with(ResponseTemplate::new(204))
        .mount(server)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn committed_action_is_delivered_by_running_cli_worker() {
    let dir = tempfile::tempdir().expect("tempdir");
    let event_id = committed(&dir).await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/events"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let mut worker = Worker::start(&write_config(&dir, &server.uri()));
    let requests = wait_requests(&server, 1).await;
    assert!(
        worker.0.try_wait().expect("worker status").is_none(),
        "worker must keep running after delivery"
    );
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .headers
            .get("X-Ontoforge-Event-Id")
            .expect("event header"),
        event_id.as_str()
    );
    let payload: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("payload");
    assert_eq!(payload["event_id"], event_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn running_cli_worker_retries_after_http_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let event_id = committed(&dir).await;
    let server = MockServer::start().await;
    reject_first_attempt(&server).await;
    let mut worker = Worker::start(&write_config(&dir, &server.uri()));
    let requests = wait_requests(&server, 2).await;
    assert!(
        worker.0.try_wait().expect("worker status").is_none(),
        "worker must keep polling"
    );
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(
            request
                .headers
                .get("X-Ontoforge-Event-Id")
                .expect("event header"),
            event_id.as_str()
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn restarted_cli_worker_recovers_failed_delivery() {
    let dir = tempfile::tempdir().expect("tempdir");
    let event_id = committed(&dir).await;
    let server = MockServer::start().await;
    reject_first_attempt(&server).await;
    let config = write_config(&dir, &server.uri());
    let first = Worker::start(&config);
    let initial = wait_requests(&server, 1).await;
    assert_eq!(initial.len(), 1);
    drop(first);
    let next = Worker::start(&config);
    let requests = wait_requests(&server, 2).await;
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(
            request
                .headers
                .get("X-Ontoforge-Event-Id")
                .expect("event header"),
            event_id.as_str()
        );
    }
    drop(next);
}
