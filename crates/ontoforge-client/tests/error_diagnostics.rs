#[path = "support/mod.rs"]
mod support;

use ontoforge_api::dto::ActionRequest;
use ontoforge_client::{ClientConfig, ClientError, OntoforgeClient};
use serde_json::{Value, json};

#[tokio::test]
async fn rust_client_action_error_diagnostics_survive_real_http() {
    let f = support::fixture().await.start().await;
    let api = OntoforgeClient::new(ClientConfig::new(&f.url).with_bearer_token(&f.token)).unwrap();
    for validate_only in [true, false] {
        for (params, field, expected) in [
            (
                json!({"delayMinutes": "wrong"}),
                "parameterErrors",
                json!([
                    {"itemOrdinal": 0, "name": "delayMinutes", "reason": "expected an integer, found a string"},
                    {"itemOrdinal": 0, "name": "flight", "reason": "required parameter is missing"}
                ]),
            ),
            (
                json!({"flight": "F-1001", "delayMinutes": 0}),
                "message",
                json!("Delay must be positive and cancelled flights cannot be delayed"),
            ),
        ] {
            let request: ActionRequest = serde_json::from_value(json!({
                "commandId": format!("client-diagnostics-{validate_only}-{field}"),
                "validateOnly": validate_only,
                "items": [{"params": params}]
            }))
            .unwrap();
            let error = api
                .execute_action("delayFlight", &request)
                .await
                .unwrap_err();
            let envelope = serde_json::to_value(error.envelope().unwrap()).unwrap();
            assert_eq!(envelope[field], expected);
            assert_eq!(envelope["apiName"], "delayFlight");
            assert!(!envelope["request_id"].as_str().unwrap().is_empty());
        }
    }
    f.finish().await;
}

async fn error_from_peer(body: Value, status: u16) -> ClientError {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).unwrap() > 0);
        let body = body.to_string();
        write!(stream, "HTTP/1.1 {status} Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    });
    let error = OntoforgeClient::new(ClientConfig::new(url))
        .unwrap()
        .metadata()
        .await
        .unwrap_err();
    task.join().unwrap();
    error
}

#[tokio::test]
async fn rust_client_action_error_diagnostics_reject_malformed_fields() {
    for diagnostic in [
        json!({"parameterErrors": "wrong"}),
        json!({"parameterErrors": [{}]}),
        json!({"parameterErrors": [{"itemOrdinal": -1, "name": "flight", "reason": "bad"}]}),
        json!({"parameterErrors": [{"itemOrdinal": 4_294_967_296_u64, "name": "flight", "reason": "bad"}]}),
        json!({"parameterErrors": [{"itemOrdinal": 0, "name": 1, "reason": "bad"}]}),
        json!({"parameterErrors": [{"itemOrdinal": 0, "name": "flight", "reason": false}]}),
        json!({"message": 42}),
    ] {
        let mut body = json!({"code": "INVALID_PARAMETERS", "apiName": "delayFlight", "request_id": "request-1"});
        body.as_object_mut()
            .unwrap()
            .extend(diagnostic.as_object().unwrap().clone());
        let error = error_from_peer(body, 400).await;
        assert!(
            matches!(error, ClientError::Decode { .. }),
            "{diagnostic}: {error:?}"
        );
    }
}

#[tokio::test]
async fn rust_client_recognizes_envelope_before_validating_diagnostics() {
    for body in [
        json!({"message": {"reason": "upstream"}}),
        json!({"parameterErrors": "upstream"}),
        json!({"code": "UPSTREAM", "message": {"reason": "upstream"}}),
        json!({"code": "UPSTREAM", "apiName": "gateway", "message": {"reason": "upstream"}}),
        json!({"code": "UPSTREAM", "apiName": "gateway", "request_id": false, "message": {"reason": "upstream"}}),
    ] {
        let error = error_from_peer(body.clone(), 502).await;
        assert!(
            matches!(error, ClientError::HttpStatus { .. }),
            "{body}: {error:?}"
        );
        assert_eq!(error.status().unwrap().as_u16(), 502);
        assert_eq!(
            error.code(),
            Some(body["code"].as_str().unwrap_or_default())
        );
        let envelope = serde_json::to_value(error.envelope().unwrap()).unwrap();
        assert!(envelope.get("parameterErrors").is_none());
        assert!(envelope.get("message").is_none());
    }
}

#[tokio::test]
async fn rust_client_action_error_diagnostics_keep_legacy_envelope_aliases() {
    let mut body = json!({
        "code": "INVALID_PARAMETERS", "api_name": "delayFlight", "requestId": "request-legacy",
        "parameterErrors": [{"itemOrdinal": 1, "name": "flight", "reason": "required parameter is missing"}]
    });
    let error = error_from_peer(body.clone(), 400).await;
    assert_eq!(error.api_name(), Some("delayFlight"));
    assert_eq!(error.request_id(), Some("request-legacy"));
    let envelope = serde_json::to_value(error.envelope().unwrap()).unwrap();
    assert_eq!(envelope["parameterErrors"], body["parameterErrors"]);
    body["parameterErrors"] = json!("wrong");
    assert!(matches!(
        error_from_peer(body, 400).await,
        ClientError::Decode { .. }
    ));
}
