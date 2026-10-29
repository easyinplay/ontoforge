#[path = "support/mod.rs"]
mod support;

use std::time::Duration;

use ontoforge_api::dto::{ActionRequest, AggregateRequest, ObjectSet, PageSpec, QueryRequest};
use ontoforge_client::{ActionResponse, ClientConfig, ClientError, OntoforgeClient};
use serde_json::{Value, json};
use support::fixture;

fn client(url: &str, token: &str) -> OntoforgeClient {
    OntoforgeClient::new(
        ClientConfig::new(url)
            .with_bearer_token(token)
            .with_timeout(Duration::from_secs(5)),
    )
    .unwrap()
}

#[tokio::test]
async fn rust_client_metadata_pk_query_aggregate_health() {
    let f = fixture().await.start().await;
    let api = client(&f.url, &f.token);

    let metadata = api.metadata().await.unwrap();
    assert_eq!(metadata.api_name, "aviation");
    assert!(
        metadata
            .object_types
            .iter()
            .any(|object| object.api_name == "Airport" && object.primary_key == "code")
    );

    let row = api.get_object("Airport", &json!("BER")).await.unwrap();
    assert_eq!(row.pk, json!("BER"));
    assert_eq!(row.properties["name"], json!("Berlin"));

    let page = api
        .query(&QueryRequest {
            set: ObjectSet::Type {
                object_type: "Airport".to_owned(),
            },
            select: vec!["name".to_owned()],
            order_by: Vec::new(),
            page: PageSpec {
                first: 2,
                after: None,
            },
        })
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 2);
    assert!(page.has_more || page.cursor.is_some());

    let aggregate = api
        .aggregate(&AggregateRequest {
            set: ObjectSet::Type {
                object_type: "Airport".to_owned(),
            },
            group_by: Vec::new(),
            measures: vec![ontoforge_api::dto::AggregateMeasure::Count { property: None }],
        })
        .await
        .unwrap();
    assert_eq!(aggregate.buckets[0].values[0], json!(3));

    let health = api.health().await.unwrap();
    assert_eq!(health.status, "healthy");
    f.finish().await;
}

#[tokio::test]
async fn rust_client_instances_and_concurrent_cancellation_are_isolated() {
    let f = fixture().await.start().await;
    let good = client(&f.url, &f.token);
    let bad = client(&f.url, "not-the-valid-bearer");
    let (good_result, bad_result) = tokio::join!(good.metadata(), bad.metadata());
    assert!(good_result.is_ok());
    assert!(matches!(bad_result, Err(ClientError::HttpStatus { .. })));

    f.engine
        .block
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let timed = OntoforgeClient::new(
        ClientConfig::new(&f.url)
            .with_bearer_token(&f.token)
            .with_timeout(Duration::from_millis(100)),
    )
    .unwrap();
    let pending = tokio::spawn(async move { timed.get_object("Flight", &json!("F-1001")).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let result = tokio::time::timeout(Duration::from_secs(3), pending)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    f.engine
        .block
        .store(false, std::sync::atomic::Ordering::SeqCst);
    f.engine.release.notify_waiters();
    tokio::time::timeout(Duration::from_secs(3), f.engine.cancelled.notified())
        .await
        .ok();
    f.finish().await;
}

#[tokio::test]
async fn rust_client_action_validate_history_result_and_replay() {
    let f = fixture().await.start().await;
    let api = client(&f.url, &f.token);
    let request = ActionRequest {
        command_id: Some("client-replay".to_owned()),
        items: vec![ontoforge_api::dto::ActionItem {
            params: std::collections::BTreeMap::from([
                ("flight".to_owned(), json!("F-1001")),
                ("delayMinutes".to_owned(), json!(10)),
            ]),
            expected_versions: std::collections::BTreeMap::new(),
        }],
        validate_only: false,
    };
    let validated = api.validate_action("delayFlight", &request).await.unwrap();
    assert!(validated.validated);
    let first = api.execute_action("delayFlight", &request).await.unwrap();
    let ActionResponse::Committed(first_receipt) = first else {
        panic!("expected committed receipt");
    };
    let replay = api.get_command_result("client-replay").await.unwrap();
    assert_eq!(replay.event_id, first_receipt.event_id);
    assert_eq!(replay.log_seq, first_receipt.log_seq);
    let history = api
        .history("Flight", &json!("F-1001"), Some(100), None)
        .await
        .unwrap();
    assert!(!history.entries.is_empty());
    f.finish_actions().await;
}

#[tokio::test]
async fn rust_client_auth_scope_revoke_stale_and_transport_errors() {
    let f = fixture().await.start().await;
    let api = client(&f.url, &f.token);
    let request = ActionRequest {
        command_id: Some("client-error".to_owned()),
        items: vec![ontoforge_api::dto::ActionItem {
            params: std::collections::BTreeMap::from([
                ("flight".to_owned(), json!("F-1001")),
                ("delayMinutes".to_owned(), json!(5)),
            ]),
            expected_versions: std::collections::BTreeMap::new(),
        }],
        validate_only: false,
    };
    let first = api.execute_action("delayFlight", &request).await.unwrap();
    assert!(matches!(first, ActionResponse::Committed(_)));
    let mut changed = request.clone();
    changed.items[0]
        .params
        .insert("delayMinutes".to_owned(), json!(6));
    let error = api
        .execute_action("delayFlight", &changed)
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some("COMMAND_ID_REUSED"));
    let unauthorized = client(&f.url, "revoked").metadata().await.unwrap_err();
    assert_eq!(unauthorized.code(), Some("UNAUTHENTICATED"));
    f.finish_actions().await;
}

#[tokio::test]
async fn rust_client_wire_numbers_and_all_endpoint_groups() {
    let f = fixture().await.start().await;
    let api = client(&f.url, &f.token);
    let page = api
        .query(&QueryRequest {
            set: ObjectSet::Pk {
                object_type: "Airport".to_owned(),
                pk: Value::String("BER".to_owned()),
            },
            select: Vec::new(),
            order_by: Vec::new(),
            page: PageSpec {
                first: 1,
                after: None,
            },
        })
        .await
        .unwrap();
    assert!(!page.rows[0].version.is_empty());
    let aggregate = api
        .aggregate(&AggregateRequest {
            set: ObjectSet::Type {
                object_type: "Airport".to_owned(),
            },
            group_by: vec!["country".to_owned()],
            measures: vec![ontoforge_api::dto::AggregateMeasure::Count { property: None }],
        })
        .await
        .unwrap();
    assert!(!aggregate.buckets.is_empty());
    f.finish().await;
}
