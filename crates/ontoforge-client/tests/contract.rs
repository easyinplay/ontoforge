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
    assert!(serde_json::to_value(health).unwrap().get("l2").is_none());
    f.finish().await;
}

#[tokio::test]
async fn rust_client_health_preserves_l2_status_and_disable_reason() {
    use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};

    let cases = [
        (
            L2HealthStatus::Enabled,
            "healthy",
            Some(json!({"status": "enabled"})),
        ),
        (
            L2HealthStatus::Disabled {
                reason: L2DisableReason::Expired,
            },
            "degraded",
            Some(json!({"status": "disabled", "reason": "EXPIRED"})),
        ),
        (
            L2HealthStatus::Disabled {
                reason: L2DisableReason::NotConfigured,
            },
            "healthy",
            None,
        ),
    ];
    for (status, expected_status, expected_l2) in cases {
        let f = fixture().await.with_l2_status(status).start().await;
        let wire: Value = reqwest::get(format!("{}/health", f.url))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let decoded = client(&f.url, &f.token).health().await.unwrap();
        let decoded = serde_json::to_value(decoded).unwrap();
        f.finish().await;
        assert_eq!(wire["status"], expected_status);
        assert_eq!(wire.get("l2"), expected_l2.as_ref());
        assert_eq!(
            decoded, wire,
            "typed health must retain the actual HTTP response"
        );
    }
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
async fn rust_client_valid_principals_revoke_and_inflight_cancel_are_isolated() {
    use std::sync::atomic::Ordering;

    let f = fixture().await.start().await;
    let alice = client(&f.url, &f.token);
    let mia = client(&f.url, &f.principal_token("mia", vec![]));
    let (alice_meta, mia_meta) = tokio::join!(alice.metadata(), mia.metadata());
    let alice_types: Vec<_> = alice_meta
        .unwrap()
        .object_types
        .into_iter()
        .map(|t| t.api_name)
        .collect();
    let mia_types: Vec<_> = mia_meta
        .unwrap()
        .object_types
        .into_iter()
        .map(|t| t.api_name)
        .collect();
    assert!(alice_types.iter().any(|name| name == "Flight"));
    assert!(!alice_types.iter().any(|name| name == "Aircraft"));
    assert!(mia_types.iter().any(|name| name == "Aircraft"));
    assert!(!mia_types.iter().any(|name| name == "Flight"));
    let aircraft_pk = json!("N100");
    let flight_pk = json!("F-1001");
    let (alice_denied, mia_denied) = tokio::join!(
        alice.get_object("Aircraft", &aircraft_pk),
        mia.get_object("Flight", &flight_pk)
    );
    let alice_denied = alice_denied.unwrap_err();
    let mia_denied = mia_denied.unwrap_err();
    assert_eq!(alice_denied.code(), Some("NOT_FOUND"));
    assert_eq!(mia_denied.code(), Some("NOT_FOUND"));
    assert!(!alice_denied.request_id().unwrap().is_empty());
    assert!(!mia_denied.request_id().unwrap().is_empty());
    assert_ne!(alice_denied.request_id(), mia_denied.request_id());

    f.engine.block.store(true, Ordering::SeqCst);
    let cancelled_api = alice.clone();
    let pending =
        tokio::spawn(async move { cancelled_api.get_object("Flight", &json!("F-1001")).await });
    tokio::time::timeout(Duration::from_secs(3), f.engine.entered.notified())
        .await
        .unwrap();
    let independent_api = alice.clone();
    let independent =
        tokio::spawn(async move { independent_api.get_object("Flight", &json!("F-1001")).await });
    tokio::time::timeout(Duration::from_secs(3), f.engine.entered.notified())
        .await
        .unwrap();
    assert!(
        !pending.is_finished(),
        "the first request must still be in flight"
    );
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    assert!(!independent.is_finished());
    f.engine.block.store(false, Ordering::SeqCst);
    f.engine.release.notify_waiters();
    let independent = tokio::time::timeout(Duration::from_secs(3), independent)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(independent.pk, json!("F-1001"));

    f.revoke_dispatchers().await;
    let denied = alice
        .get_object("Flight", &json!("F-1001"))
        .await
        .unwrap_err();
    assert_eq!(denied.code(), Some("NOT_FOUND"));
    assert!(!denied.request_id().unwrap().is_empty());
    assert!(
        mia.metadata()
            .await
            .unwrap()
            .object_types
            .iter()
            .any(|t| t.api_name == "Aircraft")
    );
    f.finish().await;
}

#[tokio::test]
async fn rust_client_transport_failures_never_retry_and_replay_has_one_durable_commit() {
    use std::sync::atomic::Ordering;
    use support::security::FaultProxy;

    let f = support::multi_edit_fixture().await.start().await;
    let api = client(&f.url, &f.token);
    let request = ActionRequest {
        command_id: Some("client-loss-after-commit".to_owned()),
        items: vec![ontoforge_api::dto::ActionItem {
            params: std::collections::BTreeMap::from([
                ("flight".to_owned(), json!("F-1001")),
                ("delayMinutes".to_owned(), json!(17)),
            ]),
            expected_versions: std::collections::BTreeMap::new(),
        }],
        validate_only: false,
    };
    let before = f.durable_state().await;
    let before_rows = f.durable_rows().await;
    let early = FaultProxy::start(f.url.clone(), false).await;
    assert!(matches!(
        client(&early.url, &f.token)
            .execute_action("delayFlight", &request)
            .await,
        Err(ClientError::Transport(_))
    ));
    assert_eq!(early.requests.load(Ordering::SeqCst), 1);
    assert_eq!(early.forwards.load(Ordering::SeqCst), 0);
    assert_eq!(f.durable_state().await, before);
    assert_eq!(f.durable_rows().await, before_rows);
    early.finish().await;

    let lost = FaultProxy::start(f.url.clone(), true).await;
    assert!(matches!(
        client(&lost.url, &f.token)
            .execute_action("delayFlight", &request)
            .await,
        Err(ClientError::Transport(_))
    ));
    assert_eq!(lost.requests.load(Ordering::SeqCst), 1);
    assert_eq!(lost.forwards.load(Ordering::SeqCst), 1);
    let receipt = lost.receipt.lock().unwrap().clone().unwrap();
    lost.finish().await;
    let committed = f.durable_state().await;
    let committed_rows = f.durable_rows().await;
    for (table, count) in [
        ("of_edit_log", 1),
        ("of_edit_log_entry", 2),
        ("of_edit_history", 2),
        ("of_action_command", 1),
        ("of_webhook_outbox", 1),
    ] {
        assert_eq!(committed[table].0 - before[table].0, count, "{table}");
    }
    assert_eq!(committed["of_obj_flight"].0 - before["of_obj_flight"].0, 1);
    assert_eq!(committed["of_obj_flight"].1 - before["of_obj_flight"].1, 2);
    for (table, state) in &before {
        if (table.starts_with("of_obj_") || table.starts_with("of_lnk_"))
            && table != "of_obj_flight"
        {
            assert_eq!(&committed[table], state, "unrelated business table {table}");
            assert_eq!(committed_rows[table], before_rows[table]);
        }
    }
    let ActionResponse::Committed(replayed) =
        api.execute_action("delayFlight", &request).await.unwrap()
    else {
        panic!("replay must return the committed receipt");
    };
    assert_eq!(serde_json::to_value(replayed).unwrap(), receipt);
    assert_eq!(
        serde_json::to_value(
            api.get_command_result("client-loss-after-commit")
                .await
                .unwrap()
        )
        .unwrap(),
        receipt
    );
    assert_eq!(f.durable_state().await, committed);
    assert_eq!(f.durable_rows().await, committed_rows);
    assert_eq!(
        api.get_object("Flight", &json!("F-1001"))
            .await
            .unwrap()
            .properties["delayMinutes"],
        json!(17)
    );
    f.finish_actions().await;
}

#[tokio::test]
async fn rust_client_action_validate_history_result_and_replay() {
    let f = support::multi_edit_fixture().await.start().await;
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
    let receipt_wire = serde_json::to_value(&first_receipt).unwrap();
    assert_eq!(
        receipt_wire["items"][0]["edits"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(first_receipt.items[0].item_ordinal, 0);
    for (ordinal, edit) in first_receipt.items[0].edits.iter().enumerate() {
        assert_eq!(edit.rule_ordinal as usize, ordinal);
        assert_eq!(edit.edit_ordinal as usize, ordinal);
        assert_eq!(edit.kind, "object");
        assert_eq!(edit.type_name, "Flight");
        assert_eq!(edit.pk, json!("F-1001"));
        assert_eq!(edit.operation, "set");
        assert!(edit.version_after > 0);
    }
    let replay = api.get_command_result("client-replay").await.unwrap();
    assert_eq!(replay.event_id, first_receipt.event_id);
    assert_eq!(replay.log_seq, first_receipt.log_seq);
    assert_eq!(serde_json::to_value(&replay).unwrap(), receipt_wire);
    let ActionResponse::Committed(replayed) =
        api.execute_action("delayFlight", &request).await.unwrap()
    else {
        panic!("expected replayed receipt");
    };
    assert_eq!(serde_json::to_value(&replayed).unwrap(), receipt_wire);
    let history = api
        .history("Flight", &json!("F-1001"), Some(100), None)
        .await
        .unwrap();
    assert!(!history.entries.is_empty());
    assert_eq!(history.entries[0].event_id, first_receipt.event_id);
    assert_eq!(history.entries[0].committed_at, first_receipt.committed_at);
    f.finish_actions().await;
}

#[tokio::test]
async fn rust_client_history_paginates_with_limit_and_before_seq() {
    let f = fixture().await.start().await;
    let api = client(&f.url, &f.token);
    let mut receipts = Vec::new();
    for (command_id, delay) in [("history-first", 10), ("history-second", 20)] {
        let request = ActionRequest {
            command_id: Some(command_id.to_owned()),
            items: vec![ontoforge_api::dto::ActionItem {
                params: std::collections::BTreeMap::from([
                    ("flight".to_owned(), json!("F-1001")),
                    ("delayMinutes".to_owned(), json!(delay)),
                ]),
                expected_versions: std::collections::BTreeMap::new(),
            }],
            validate_only: false,
        };
        let ActionResponse::Committed(receipt) =
            api.execute_action("delayFlight", &request).await.unwrap()
        else {
            panic!("expected committed receipt");
        };
        receipts.push(receipt);
    }
    let limited = api
        .history("Flight", &json!("F-1001"), Some(1), None)
        .await
        .unwrap();
    let older = api
        .history(
            "Flight",
            &json!("F-1001"),
            Some(1),
            Some(receipts[1].log_seq),
        )
        .await
        .unwrap();
    let limited_seqs: Vec<_> = limited.entries.iter().map(|entry| entry.seq).collect();
    let older_seqs: Vec<_> = older.entries.iter().map(|entry| entry.seq).collect();
    assert_eq!(
        (limited_seqs, older_seqs),
        (vec![receipts[1].log_seq], vec![receipts[0].log_seq])
    );
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
    assert!(
        error
            .envelope()
            .is_some_and(|envelope| !envelope.request_id.is_empty())
    );
    assert_eq!(error.api_name(), Some("delayFlight"));
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
