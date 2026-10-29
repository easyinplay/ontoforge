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
    assert_eq!(left.json::<Value>().await.unwrap(), right.json::<Value>().await.unwrap());
    assert_eq!(f.finish().await.len(), 1);
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
    assert_eq!(f.finish().await.len(), 1);
}

#[tokio::test]
async fn http_stale_and_validate_conflicts_never_retry() {
    let f = fixture().await.start().await;
    let response = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "commandId": "http-stale",
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 5}}]
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "commandId": "http-stale-dry",
                "validateOnly": true,
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 5}}]
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap()["validated"], true);
    assert_eq!(f.finish().await.len(), 1);
}
