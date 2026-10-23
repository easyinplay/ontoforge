use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use ontoforge_action::outbox::{WebhookConfig, WebhookError, WebhookWorker};
use ontoforge_action::{
    ActionItem, ActionLimits, ActionOutcome, ActionReceipt, ActionRequest, ActionService,
};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use sqlx::sqlite::SqlitePoolOptions;
use wiremock::matchers::{method, path as request_path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

#[tokio::test]
async fn payload_is_minimal_and_versioned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("open store");
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
    .expect("version");
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
    let tokens = TokenCodec::new(b"outbox-test-key".to_vec()).expect("tokens");
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
                command_id: Some("cmd-one".to_owned()),
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
        .expect("Action commit");
    let ActionOutcome::Committed(receipt) = outcome else {
        panic!("expected committed Action")
    };
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&path))
        .await
        .expect("inspect store");
    let payload: String =
        sqlx::query_scalar("SELECT payload FROM of_webhook_outbox WHERE event_id = ?")
            .bind(&receipt.event_id)
            .fetch_one(&pool)
            .await
            .expect("outbox payload");
    let value: serde_json::Value = serde_json::from_str(&payload).expect("JSON payload");
    assert_eq!(
        value["schema_version"], 1,
        "webhook payload must declare its format"
    );
    assert_eq!(value["event_id"], receipt.event_id);
    assert!(
        !payload.contains("Private Name"),
        "row values must not leave the outbox"
    );
}

async fn committed() -> (tempfile::TempDir, Utf8PathBuf, EditStore, ActionReceipt) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("open store");
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
    .expect("version");
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
    let tokens = TokenCodec::new(b"outbox-test-key".to_vec()).expect("tokens");
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
                command_id: Some("cmd-one".to_owned()),
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
        .expect("Action commit");
    let ActionOutcome::Committed(receipt) = outcome else {
        panic!("expected commit")
    };
    (dir, path, store, receipt)
}

fn poll_time(receipt: &ActionReceipt) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&receipt.committed_at)
        .expect("commit time")
        .to_utc()
        + chrono::Duration::seconds(1)
}

fn worker<'a>(store: &'a EditStore, server: &MockServer) -> WebhookWorker<'a> {
    WebhookWorker::new(
        store,
        WebhookConfig::new(
            &format!("{}/events", server.uri()),
            Duration::from_secs(10),
            Duration::from_secs(2),
            4,
        )
        .expect("trusted config"),
    )
    .expect("worker")
}

async fn outbox_row(path: &Utf8PathBuf) -> (i64, String, Option<String>, Option<String>) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(path))
        .await
        .expect("inspection pool");
    sqlx::query_as(
        "SELECT attempt_count, next_attempt_at, delivered_at, last_error FROM of_webhook_outbox",
    )
    .fetch_one(&pool)
    .await
    .expect("outbox row")
}

async fn add_second_outbox_event(path: &Utf8PathBuf, event_id: &str) -> String {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(path))
        .await
        .expect("inspection pool");
    let second = format!("{event_id}-second");
    sqlx::query("INSERT INTO of_edit_log (event_id, actor, action, committed_at, def_version, params) SELECT ?, actor, action, committed_at, def_version, params FROM of_edit_log WHERE event_id = ?")
        .bind(&second)
        .bind(event_id)
        .execute(&pool)
        .await
        .expect("second log event");
    let (payload, due): (String, String) =
        sqlx::query_as("SELECT payload, next_attempt_at FROM of_webhook_outbox WHERE event_id = ?")
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .expect("first outbox event");
    let mut payload: serde_json::Value = serde_json::from_str(&payload).expect("JSON payload");
    payload["event_id"] = serde_json::Value::String(second.clone());
    sqlx::query(
        "INSERT INTO of_webhook_outbox (event_id, payload, next_attempt_at) VALUES (?, ?, ?)",
    )
    .bind(&second)
    .bind(payload.to_string())
    .bind(due)
    .execute(&pool)
    .await
    .expect("second outbox event");
    second
}

#[tokio::test]
async fn retry_delay_starts_after_slow_response() {
    let (_dir, path, store, receipt) = committed().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503).set_delay(Duration::from_millis(300)))
        .mount(&server)
        .await;
    let now = poll_time(&receipt);
    assert_eq!(worker(&store, &server).poll_at(now).await.unwrap(), 1);
    let next = DateTime::parse_from_rfc3339(&outbox_row(&path).await.1)
        .unwrap()
        .to_utc();
    assert!(
        next >= now + chrono::Duration::milliseconds(1200),
        "retry must be scheduled after the response: {next}"
    );
}

#[tokio::test]
async fn later_event_is_unclaimed_while_first_delivery_is_in_flight() {
    let (_dir, path, store, receipt) = committed().await;
    let second = add_second_outbox_event(&path, &receipt.event_id).await;
    let server = MockServer::start().await;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let started_tx = std::sync::Mutex::new(Some(started_tx));
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            if let Some(tx) = started_tx.lock().unwrap().take() {
                let _ = tx.send(());
            }
            ResponseTemplate::new(204).set_delay(Duration::from_millis(400))
        })
        .mount(&server)
        .await;
    let sender = worker(&store, &server);
    let inspect = async {
        started_rx.await.expect("first request started");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlite_connect_options(&path))
            .await
            .expect("inspection pool");
        let owner: Option<String> =
            sqlx::query_scalar("SELECT lease_owner FROM of_webhook_outbox WHERE event_id = ?")
                .bind(&second)
                .fetch_one(&pool)
                .await
                .expect("second lease");
        assert!(owner.is_none(), "later event was leased before its send");
    };
    let (delivered, ()) = tokio::join!(sender.poll_at(poll_time(&receipt)), inspect);
    assert_eq!(delivered.unwrap(), 2);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn poll_does_not_exceed_batch_limit() {
    let (_dir, path, store, receipt) = committed().await;
    add_second_outbox_event(&path, &receipt.event_id).await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let sender = WebhookWorker::new(
        &store,
        WebhookConfig::new(
            &format!("{}/events", server.uri()),
            Duration::from_secs(10),
            Duration::from_secs(2),
            1,
        )
        .unwrap(),
    )
    .unwrap();
    let now = poll_time(&receipt);
    assert_eq!(sender.poll_at(now).await.unwrap(), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_eq!(sender.poll_at(now).await.unwrap(), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn crash_before_send_committed_event_survives_restart() {
    let (_dir, path, store, receipt) = committed().await;
    drop(store);
    let reopened = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("reopen store");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(request_path("/events"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    assert_eq!(
        worker(&reopened, &server)
            .poll_at(poll_time(&receipt))
            .await
            .expect("delivery"),
        1
    );
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .headers
            .get("X-Ontoforge-Event-Id")
            .expect("event header"),
        receipt.event_id.as_str()
    );
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("payload");
    assert_eq!(body["schema_version"], 1);
    assert_eq!(body["event_id"], receipt.event_id);
    assert_eq!(body["receipt"]["log_seq"], receipt.log_seq);
    assert!(!String::from_utf8_lossy(&requests[0].body).contains("Private Name"));
    assert!(outbox_row(&path).await.2.is_some(), "delivery persisted");
}

#[tokio::test]
async fn http_failure_retries_with_same_event_id() {
    let (_dir, path, store, receipt) = committed().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(request_path("/events"))
        .respond_with(ResponseTemplate::new(503).set_body_string("SECRET RESPONSE"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(request_path("/events"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let first = poll_time(&receipt);
    let sender = worker(&store, &server);
    assert_eq!(sender.poll_at(first).await.expect("first attempt"), 1);
    let row = outbox_row(&path).await;
    assert_eq!(row.0, 1);
    assert_eq!(row.3.as_deref(), Some("http_status_503"));
    assert!(!row.3.as_deref().unwrap().contains("SECRET"));
    assert_eq!(sender.poll_at(first).await.expect("not due"), 0);
    let retry_at = DateTime::parse_from_rfc3339(&row.1)
        .expect("retry time")
        .to_utc();
    assert_eq!(sender.poll_at(retry_at).await.expect("retry"), 1);
    assert!(outbox_row(&path).await.2.is_some());
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    assert_eq!(
        requests[0].headers.get("X-Ontoforge-Event-Id"),
        requests[1].headers.get("X-Ontoforge-Event-Id")
    );
    assert_eq!(
        requests[1].headers.get("X-Ontoforge-Event-Id").unwrap(),
        receipt.event_id.as_str()
    );
}

#[tokio::test]
async fn crash_after_success_before_mark_redelivers_same_event_id() {
    let (_dir, path, store, receipt) = committed().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(request_path("/events"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let first = poll_time(&receipt);
    let sender = worker(&store, &server);
    assert!(matches!(
        sender
            .poll_at_with_after_send(first, |_| Err(WebhookError::Interrupted))
            .await,
        Err(WebhookError::Interrupted)
    ));
    assert!(outbox_row(&path).await.2.is_none());
    assert_eq!(
        worker(&store, &server)
            .poll_at(first + chrono::Duration::seconds(9))
            .await
            .expect("leased"),
        0
    );
    assert_eq!(
        worker(&store, &server)
            .poll_at(first + chrono::Duration::seconds(10))
            .await
            .expect("recovered"),
        1
    );
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    assert_eq!(
        requests[0].headers.get("X-Ontoforge-Event-Id"),
        requests[1].headers.get("X-Ontoforge-Event-Id")
    );
    assert_eq!(
        requests[1].headers.get("X-Ontoforge-Event-Id").unwrap(),
        receipt.event_id.as_str()
    );
    assert!(outbox_row(&path).await.2.is_some());
}

#[tokio::test]
async fn lease_expiry_reclaims_and_stale_attempt_is_fenced() {
    let (_dir, path, store, receipt) = committed().await;
    let now = poll_time(&receipt);
    let old = store
        .claim_due_outbox(now, "worker", now + chrono::Duration::seconds(10), 1)
        .await
        .expect("claim")
        .remove(0);
    assert!(
        store
            .claim_due_outbox(
                now + chrono::Duration::seconds(9),
                "other",
                now + chrono::Duration::seconds(19),
                1
            )
            .await
            .expect("unexpired")
            .is_empty()
    );
    let new = store
        .claim_due_outbox(
            now + chrono::Duration::seconds(10),
            "worker",
            now + chrono::Duration::seconds(20),
            1,
        )
        .await
        .expect("reclaim")
        .remove(0);
    assert_ne!(
        old.lease_owner, new.lease_owner,
        "every generation needs a new fencing token"
    );
    assert_eq!(old.event_id, new.event_id);
    assert_eq!(old.payload, new.payload);
    assert!(
        !store
            .mark_delivered(&old.event_id, &old.lease_owner, now)
            .await
            .expect("stale mark")
    );
    assert!(
        !store
            .record_retry(&old.event_id, &old.lease_owner, 1, now, "transport_error")
            .await
            .expect("stale retry")
    );
    assert_eq!(outbox_row(&path).await.0, 0);
    assert!(
        store
            .mark_delivered(&new.event_id, &new.lease_owner, now)
            .await
            .expect("current mark")
    );
    assert!(outbox_row(&path).await.2.is_some());
}

#[tokio::test]
async fn lease_parallel_workers_claim_one_attempt() {
    let (_dir, path, store, receipt) = committed().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(request_path("/events"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let a = worker(&store, &server);
    let b = worker(&store, &server);
    let (first, second) = tokio::join!(
        a.poll_at(poll_time(&receipt)),
        b.poll_at(poll_time(&receipt))
    );
    assert_eq!(first.expect("first") + second.expect("second"), 1);
    assert_eq!(server.received_requests().await.expect("requests").len(), 1);
    assert!(outbox_row(&path).await.2.is_some());
}

#[tokio::test]
async fn retry_backoff_caps_at_five_minutes_without_leaking_response() {
    let (_dir, path, store, receipt) = committed().await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(request_path("/events"))
        .respond_with(ResponseTemplate::new(503).set_body_string("SECRET RESPONSE"))
        .mount(&server)
        .await;
    let sender = worker(&store, &server);
    let mut due = poll_time(&receipt);
    for attempt in 1..=10 {
        assert_eq!(sender.poll_at(due).await.expect("failed delivery"), 1);
        let row = outbox_row(&path).await;
        assert_eq!(row.0, attempt);
        assert_eq!(row.3.as_deref(), Some("http_status_503"));
        let next = DateTime::parse_from_rfc3339(&row.1)
            .expect("next attempt")
            .to_utc();
        let delay = (1_i64 << (attempt - 1)).min(300);
        assert!(next - due >= chrono::Duration::seconds(delay));
        assert!(next - due < chrono::Duration::seconds(delay + 1));
        assert_eq!(sender.poll_at(due).await.expect("not yet due"), 0);
        due = next;
    }
    assert_eq!(
        server.received_requests().await.expect("requests").len(),
        10
    );
}

#[tokio::test]
async fn payload_target_is_only_trusted_configuration() {
    let (_dir, _path, store, receipt) = committed().await;
    let trusted = MockServer::start().await;
    let untrusted = MockServer::start().await;
    Mock::given(method("POST"))
        .and(request_path("/events"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&trusted)
        .await;
    let override_request = serde_json::json!({
        "action": "createGate", "commandId": "other", "items": [],
        "webhookUrl": format!("{}/events", untrusted.uri())
    });
    assert!(serde_json::from_value::<ActionRequest>(override_request).is_err());
    assert_eq!(
        worker(&store, &trusted)
            .poll_at(poll_time(&receipt))
            .await
            .expect("delivery"),
        1
    );
    assert_eq!(
        trusted
            .received_requests()
            .await
            .expect("trusted requests")
            .len(),
        1
    );
    assert!(
        untrusted
            .received_requests()
            .await
            .expect("untrusted requests")
            .is_empty()
    );
    let payload = &trusted.received_requests().await.expect("payload")[0].body;
    let text = String::from_utf8_lossy(payload);
    for secret in [
        "Private Name",
        "old_value",
        "current_value",
        "physical_name",
        "scope",
    ] {
        assert!(!text.contains(secret), "outbox leaked {secret}");
    }
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(payload).unwrap()["event_id"],
        receipt.event_id
    );
}
