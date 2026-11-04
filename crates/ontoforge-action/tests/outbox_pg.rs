use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{DateTime, Utc};
use ontoforge_action::outbox::{WebhookConfig, WebhookError, WebhookWorker};
use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "../../ontoforge-storage/tests/support/mod.rs"]
mod support;

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

async fn committed() -> (support::PgTestStore, String, DateTime<Utc>) {
    let pg = support::pg_store().await;
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &pg.store,
        &loaded.ontology.unwrap(),
        &ReloadOptions::default(),
    )
    .await
    .unwrap()
    .def_version
    .unwrap();
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
        .insert("createGate".to_owned().try_into().unwrap());
    scope.types.insert(
        "Gate".to_owned().try_into().unwrap(),
        TypeScope {
            row: Pred::Const(true),
            columns: ["code", "name"]
                .into_iter()
                .map(|name| (name.to_owned().try_into().unwrap(), Pred::Const(true)))
                .collect(),
        },
    );
    let tokens = TokenCodec::new(b"pg-outbox-key".to_vec()).unwrap();
    let result = ActionService::new(&pg.store, ActionLimits::new(8, 8, 65536).unwrap(), &tokens)
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "createGate".to_owned(),
                command_id: Some("pg-outbox".to_owned()),
                items: vec![ActionItem {
                    params: serde_json::from_value(
                        serde_json::json!({"code":"G1", "name":"Private Name"}),
                    )
                    .unwrap(),
                    expected_versions: BTreeMap::new(),
                }],
                validate_only: false,
            },
        )
        .await
        .unwrap();
    let ActionOutcome::Committed(receipt) = result else {
        panic!("expected commit")
    };
    let now = DateTime::parse_from_rfc3339(&receipt.committed_at)
        .unwrap()
        .to_utc()
        + chrono::Duration::seconds(1);
    (pg, receipt.event_id, now)
}

fn worker<'a>(
    store: &'a ontoforge_storage::store::EditStore,
    server: &MockServer,
) -> WebhookWorker<'a> {
    WebhookWorker::new(
        store,
        WebhookConfig::new(
            &server.uri(),
            Duration::from_secs(10),
            Duration::from_secs(2),
            1,
        )
        .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn postgres_outbox_parallel_workers_claim_one_generation() {
    let (pg, event_id, now) = committed().await;
    let other = pg.open_another(Duration::from_secs(5)).await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let a = worker(&pg.store, &server);
    let b = worker(&other, &server);
    let (first, second) = tokio::join!(a.poll_at(now), b.poll_at(now));
    assert_eq!(first.unwrap() + second.unwrap(), 1);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get("X-Ontoforge-Event-Id").unwrap(),
        event_id.as_str()
    );
    assert!(
        pg.store
            .claim_due_outbox(
                now + chrono::Duration::seconds(11),
                "third",
                now + chrono::Duration::seconds(21),
                1
            )
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn postgres_outbox_after_send_crash_reclaims_with_same_payload() {
    let (pg, event_id, now) = committed().await;
    let other = pg.open_another(Duration::from_secs(5)).await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let a = worker(&pg.store, &server);
    assert!(matches!(
        a.poll_at_with_after_send(now, |_| Err(WebhookError::Interrupted))
            .await,
        Err(WebhookError::Interrupted)
    ));
    assert_eq!(
        worker(&other, &server)
            .poll_at(now + chrono::Duration::seconds(9))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        worker(&other, &server)
            .poll_at(now + chrono::Duration::seconds(10))
            .await
            .unwrap(),
        1
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    assert_eq!(
        requests[0].headers.get("X-Ontoforge-Event-Id"),
        requests[1].headers.get("X-Ontoforge-Event-Id")
    );
    assert_eq!(
        requests[1].headers.get("X-Ontoforge-Event-Id").unwrap(),
        event_id.as_str()
    );
}
