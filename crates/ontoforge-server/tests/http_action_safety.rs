#[path = "support/security_evidence.rs"]
mod evidence;
#[path = "support/mod.rs"]
mod support;

use reqwest::StatusCode;
use serde_json::{Value, json};
use support::fixture;

#[tokio::test]
async fn http_same_command_race_one_commit_original_receipt() {
    let f = fixture().await.start().await;
    let command_id = "http-race";
    let body = json!({
        "commandId": command_id,
        "items": [{"params": {"flight": "F-1001", "delayMinutes": 20}}]
    });
    let left = f.post("/api/actions/delayFlight", &body);
    let right = f.post("/api/actions/delayFlight", &body);
    let (left, right) = tokio::join!(left, right);
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);
    assert_eq!(
        left.json::<Value>().await.unwrap(),
        right.json::<Value>().await.unwrap()
    );
    assert_eq!(f.finish_actions().await.len(), 0);
}

#[tokio::test]
async fn http_replay_revoke_cross_principal_and_changed_params() {
    let f = fixture().await.start().await;
    let first = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "commandId": "http-replay",
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 10}}]
            }),
        )
        .await;
    assert_eq!(first.status(), StatusCode::OK);
    let receipt = first.json::<Value>().await.unwrap();

    let replay = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "commandId": "http-replay",
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 10}}]
            }),
        )
        .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(replay.json::<Value>().await.unwrap(), receipt);

    let changed = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "commandId": "http-replay",
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 11}}]
            }),
        )
        .await;
    assert_eq!(changed.status(), StatusCode::CONFLICT);

    let result = f.get("/api/commands/http-replay").await;
    assert_eq!(result.status(), StatusCode::OK);
    assert_eq!(result.json::<Value>().await.unwrap(), receipt);
    assert_eq!(f.finish_actions().await.len(), 0);
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "keep stale submit, stale validation and successful replay in temporal order"
)]
async fn http_stale_and_validate_conflicts_never_retry() {
    use ontoforge_storage::reload::{ReloadOptions, reload};
    use ontoforge_storage::store::EditStore;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let mut f = fixture().await.start().await;
    f.client = evidence::client();
    let other = EditStore::open(support::settings(&f.path)).await.unwrap();
    let before = evidence::counts(&f.path).await;
    let before_rows = evidence::durable_rows(&f.path).await;
    for validate_only in [false, true] {
        evidence::ready(&f.client, &f.url).await;
        let old = f.store.active_definition().await.unwrap();
        let calls = f.engine.calls.load(Ordering::SeqCst);
        f.engine.block.store(true, Ordering::SeqCst);
        let client = f.client.clone();
        let url = f.url.clone();
        let token = f.token.clone();
        let pending = tokio::spawn(async move {
            client
                .post(format!("{url}/api/actions/delayFlight"))
                .bearer_auth(token)
                .json(
                    &json!({"commandId":format!("http-definition-stale-{validate_only}"),
                    "validateOnly":validate_only,
                    "items":[{"params":{"flight":"F-1001","delayMinutes":5}}]}),
                )
                .send()
                .await
                .unwrap()
        });
        tokio::time::timeout(Duration::from_secs(5), f.engine.entered.notified())
            .await
            .unwrap();
        let mut changed = old.ontology.clone();
        changed.description = Some(format!("external-reload-{validate_only}"));
        tokio::time::timeout(
            Duration::from_secs(5),
            reload(&other, &changed, &ReloadOptions::default()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_ne!(
            other.active_definition().await.unwrap().def_version,
            old.def_version
        );
        f.engine.block.store(false, Ordering::SeqCst);
        f.engine.release.notify_one();
        let response = tokio::time::timeout(Duration::from_secs(10), pending)
            .await
            .unwrap()
            .unwrap();
        let status = response.status();
        let error = response.json::<Value>().await.unwrap();
        assert_eq!(status, StatusCode::CONFLICT, "{error}");
        assert_eq!(error["code"], "DEFINITION_STALE", "{error}");
        assert_eq!(
            evidence::counts(&f.path).await,
            before,
            "stale definition must write no business table"
        );
        assert_eq!(evidence::durable_rows(&f.path).await, before_rows);
        assert_eq!(
            f.engine.calls.load(Ordering::SeqCst) - calls,
            1,
            "the adapter must not retry against the new definition"
        );
    }
    evidence::ready(&f.client, &f.url).await;
    let body = json!({"commandId":"http-after-stale","items":[{"params":{"flight":"F-1001","delayMinutes":5}}]});
    let mut validate = body.clone();
    validate["validateOnly"] = json!(true);
    assert_eq!(
        support::success(f.post("/api/actions/delayFlight", &validate).await).await["validated"],
        true
    );
    assert_eq!(evidence::durable_rows(&f.path).await, before_rows);
    let receipt = support::success(f.post("/api/actions/delayFlight", &body).await).await;
    let committed_rows = evidence::durable_rows(&f.path).await;
    evidence::assert_one_flight_commit(&before_rows, &committed_rows);
    assert_eq!(
        support::success(f.post("/api/actions/delayFlight", &body).await).await,
        receipt
    );
    assert_eq!(evidence::durable_rows(&f.path).await, committed_rows);
    assert_eq!(
        evidence::delta(evidence::counts(&f.path).await, before),
        [1; 5]
    );
    other.read_audit().shutdown().await.unwrap();
    assert_eq!(f.finish_actions().await.len(), 0);
}
