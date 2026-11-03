#[path = "support/mod.rs"]
mod support;

use reqwest::StatusCode;
use serde_json::{Value, json};
use support::fixture;

#[tokio::test]
async fn http_action_parameter_diagnostics_keep_item_and_field() {
    let f = fixture().await.start().await;
    for validate_only in [true, false] {
        let response = f
            .post(
                "/api/actions/delayFlight",
                &json!({
                    "commandId": format!("diagnostics-params-{validate_only}"),
                    "validateOnly": validate_only,
                    "items": [
                        {"params": {"flight": "F-1001", "delayMinutes": 1}},
                        {"params": {"delayMinutes": "wrong", "extra": true}}
                    ]
                }),
            )
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let request_id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["code"], "INVALID_PARAMETERS");
        assert_eq!(body["apiName"], "delayFlight");
        assert_eq!(body["request_id"], request_id);
        assert_eq!(
            body["parameterErrors"],
            json!([
                {"itemOrdinal": 1, "name": "delayMinutes", "reason": "expected an integer, found a string"},
                {"itemOrdinal": 1, "name": "extra", "reason": "unknown parameter"},
                {"itemOrdinal": 1, "name": "flight", "reason": "required parameter is missing"}
            ])
        );
        assert!(body.get("message").is_none());
    }
    f.finish().await;
}

#[tokio::test]
async fn http_action_criteria_diagnostics_keep_only_configured_message() {
    let f = fixture().await.start().await;
    for validate_only in [true, false] {
        let response = f
            .post(
                "/api/actions/delayFlight",
                &json!({
                    "commandId": format!("diagnostics-criteria-{validate_only}"),
                    "validateOnly": validate_only,
                    "items": [{"params": {"flight": "F-1001", "delayMinutes": 0}}]
                }),
            )
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let request_id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["code"], "CRITERIA_FAILED");
        assert_eq!(
            body["message"],
            "Delay must be positive and cancelled flights cannot be delayed"
        );
        assert_eq!(body["request_id"], request_id);
        assert!(body.get("parameterErrors").is_none());
    }
    f.finish().await;
}

#[tokio::test]
async fn http_action_other_errors_omit_diagnostics() {
    let f = fixture().await.start().await;
    for (action, body, code) in [
        (
            "delayFlight",
            json!({"commandId": "diagnostics-empty", "items": []}),
            "INVALID_REQUEST",
        ),
        (
            "unknownAction",
            json!({"commandId": "diagnostics-unknown", "items": [{"params": {}}]}),
            "NOT_FOUND",
        ),
    ] {
        let response = f.post(&format!("/api/actions/{action}"), &body).await;
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["code"], code);
        assert!(body.get("message").is_none());
        assert!(body.get("parameterErrors").is_none());
        assert_eq!(body.as_object().unwrap().len(), 3);
    }
    f.finish().await;
}

#[tokio::test]
async fn http_action_store_error_keeps_database_details_private() {
    let f = fixture().await;
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    sqlx::query("DROP TABLE of_action_command")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let f = f.start().await;
    let response = f
        .post(
            "/api/actions/delayFlight",
            &json!({
                "commandId": "diagnostics-store",
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 1}}]
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], "STORE_UNAVAILABLE");
    assert_eq!(body.as_object().unwrap().len(), 3);
    assert!(!body.to_string().contains("of_action_command"));
    assert!(!body.to_string().contains("no such table"));
    f.finish().await;
}

#[tokio::test]
async fn http_action_commit_error_omits_internal_message() {
    let f = fixture().await.start().await;
    let request = json!({
        "commandId": "diagnostics-reused",
        "items": [{"params": {"flight": "F-1001", "delayMinutes": 1}}]
    });
    assert_eq!(
        f.post("/api/actions/delayFlight", &request).await.status(),
        StatusCode::OK
    );
    let mut changed = request;
    changed["items"][0]["params"]["delayMinutes"] = json!(2);
    let response = f.post("/api/actions/delayFlight", &changed).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], "COMMAND_ID_REUSED");
    assert_eq!(body.as_object().unwrap().len(), 3);
    f.finish_actions().await;
}
