#![allow(dead_code)]

use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

pub fn required(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => panic!("{name} is required for ignored real-service tests"),
    }
}

pub fn client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("HTTP client")
}

pub fn token() -> String {
    required("ONTOFORGE_TOKEN")
}

pub fn no_grant_token() -> String {
    required("ONTOFORGE_NO_GRANT_TOKEN")
}

pub async fn get(url: &str, path: &str, bearer: &str) -> reqwest::Response {
    client()
        .get(format!("{url}{path}"))
        .bearer_auth(bearer)
        .send()
        .await
        .expect("HTTP GET")
}

pub async fn post_json(url: &str, path: &str, bearer: &str, body: &Value) -> reqwest::Response {
    client()
        .post(format!("{url}{path}"))
        .bearer_auth(bearer)
        .json(body)
        .send()
        .await
        .expect("HTTP POST")
}

pub async fn json(response: reqwest::Response) -> (reqwest::StatusCode, Value) {
    let status = response.status();
    let body = response.json().await.expect("JSON response");
    (status, body)
}

pub async fn health(url: &str) -> serde_json::Value {
    let response = client()
        .get(format!("{url}/health"))
        .send()
        .await
        .expect("server health request");
    let status = response.status();
    let body: serde_json::Value = response.json().await.expect("health JSON");
    assert!(
        status.is_success() || status.as_u16() == 503,
        "{status}: {body}"
    );
    body
}
