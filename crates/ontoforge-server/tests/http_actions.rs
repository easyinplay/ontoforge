#[path = "support/mod.rs"]
mod support;

use reqwest::StatusCode;
use serde_json::{Value, json};
use support::fixture;

#[tokio::test]
async fn http_action_validate_submit_history_result() {
    let f = fixture().await.start().await;
    let validate = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 15}}],
                "validateOnly": true
            }),
        )
        .await;
    assert_eq!(validate.status(), StatusCode::OK);
    assert_eq!(validate.json::<Value>().await.unwrap()["validated"], true);

    let command_id = "http-action-validate-submit";
    let submit = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "commandId": command_id,
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 15}}]
            }),
        )
        .await;
    assert_eq!(submit.status(), StatusCode::OK);
    let receipt = submit.json::<Value>().await.unwrap();
    assert_eq!(receipt["commandId"], command_id);

    let result = f.get(&format!("/api/commands/{command_id}")).await;
    assert_eq!(result.status(), StatusCode::OK);
    assert_eq!(result.json::<Value>().await.unwrap(), receipt);

    let history = f.get("/api/objects/Flight/F-1001/history?limit=1").await;
    assert_eq!(history.status(), StatusCode::OK);
    let history = history.json::<Value>().await.unwrap();
    assert!(history["entries"].as_array().is_some());
    assert!(
        history["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["action"] == "delayFlight" })
    );
    assert_eq!(f.finish_actions().await.len(), 1);
}

#[tokio::test]
async fn http_action_empty_invalid_and_masked_history() {
    let f = fixture().await.start().await;
    let empty = f
        .post(
            "/api/actions/delayFlight",
            &json!({"commandId": "empty", "items": []}),
        )
        .await;
    assert_eq!(empty.status(), StatusCode::BAD_REQUEST);

    let mismatch = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "action": "recordMaintenance",
                "commandId": "mismatch",
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 1}}]
            }),
        )
        .await;
    assert_eq!(mismatch.status(), StatusCode::BAD_REQUEST);

    let history = f.get("/api/objects/Flight/F-1001/history?limit=1").await;
    assert_eq!(history.status(), StatusCode::OK);
    let body = history.json::<Value>().await.unwrap();
    assert!(body["entries"].as_array().is_some());
    assert!(body["truncated"].is_boolean());
    assert_eq!(f.finish().await.len(), 1);
}
