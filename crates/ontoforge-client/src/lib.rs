//! Lightweight Rust client for the Ontoforge REST API.

use std::collections::BTreeMap;
use std::time::Duration;

use ontoforge_api::dto;
use reqwest::{Method, StatusCode, Url};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub bearer_token: Option<String>,
    pub timeout: Duration,
}

impl ClientConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            bearer_token: None,
            timeout: Duration::from_secs(30),
        }
    }
    #[must_use]
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("invalid REST base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("REST transport failed: {0}")]
    Transport(#[source] reqwest::Error),
    #[error("REST request failed with HTTP {status}")]
    HttpStatus {
        status: StatusCode,
        envelope: dto::ErrorEnvelope,
    },
    #[error("REST response decode failed for {api_name}: {message}")]
    Decode { api_name: String, message: String },
}

impl ClientError {
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::HttpStatus { envelope, .. } => Some(envelope.code.as_str()),
            _ => None,
        }
    }
    pub fn api_name(&self) -> Option<&str> {
        match self {
            Self::HttpStatus { envelope, .. } => Some(envelope.api_name.as_str()),
            _ => None,
        }
    }
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::HttpStatus { envelope, .. } => Some(envelope.request_id.as_str()),
            _ => None,
        }
    }
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn envelope(&self) -> Option<&dto::ErrorEnvelope> {
        match self {
            Self::HttpStatus { envelope, .. } => Some(envelope),
            _ => None,
        }
    }
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Transport(error) if error.is_timeout())
    }
}

#[derive(Debug, Clone)]
pub enum ActionResponse {
    Committed(dto::ActionReceipt),
    Validated(dto::ValidationResponse),
}

#[derive(Clone)]
pub struct OntoforgeClient {
    http: reqwest::Client,
    base_url: Url,
    bearer_token: Option<String>,
}

impl OntoforgeClient {
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        let mut base_url = Url::parse(&config.base_url)
            .map_err(|error| ClientError::InvalidBaseUrl(error.to_string()))?;
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(ClientError::InvalidBaseUrl(
                "base URL must use http or https".to_owned(),
            ));
        }
        let path = base_url.path().trim_end_matches('/').to_owned();
        base_url.set_path(&path);
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ClientError::Transport)?;
        Ok(Self {
            http,
            base_url,
            bearer_token: config.bearer_token,
        })
    }

    pub async fn metadata(&self) -> Result<dto::Metadata, ClientError> {
        decode_metadata(&self.send(Method::GET, "/api/ontology", None).await?)
    }
    pub async fn get_object(
        &self,
        object_type: &str,
        pk: &Value,
    ) -> Result<dto::ObjectRow, ClientError> {
        let path = format!(
            "/api/objects/{}/{}",
            encode_path(object_type),
            encode_path(&pk_path(pk))
        );
        decode_object_row(&self.send(Method::GET, &path, None).await?)
    }
    pub async fn query(&self, request: &dto::QueryRequest) -> Result<dto::ListPage, ClientError> {
        decode_list_page(&self.send_json(Method::POST, "/api/query", request).await?)
    }
    pub async fn aggregate(
        &self,
        request: &dto::AggregateRequest,
    ) -> Result<dto::AggregateResult, ClientError> {
        decode_aggregate_result(
            &self
                .send_json(Method::POST, "/api/aggregate", request)
                .await?,
        )
    }
    pub async fn health(&self) -> Result<dto::HealthResponse, ClientError> {
        decode_health(&self.send(Method::GET, "/health", None).await?)
    }
    pub async fn execute_action(
        &self,
        action_name: &str,
        request: &dto::ActionRequest,
    ) -> Result<ActionResponse, ClientError> {
        let path = format!("/api/actions/{}", encode_path(action_name));
        let value = self.send_json(Method::POST, &path, request).await?;
        if value.get("validated").and_then(Value::as_bool) == Some(true) {
            return Ok(ActionResponse::Validated(dto::ValidationResponse {
                validated: true,
            }));
        }
        decode_action_receipt(&value).map(ActionResponse::Committed)
    }
    pub async fn validate_action(
        &self,
        action_name: &str,
        request: &dto::ActionRequest,
    ) -> Result<dto::ValidationResponse, ClientError> {
        let mut request = request.clone();
        request.validate_only = true;
        match self.execute_action(action_name, &request).await? {
            ActionResponse::Validated(value) => Ok(value),
            ActionResponse::Committed(_) => Err(ClientError::Decode {
                api_name: action_name.to_owned(),
                message: "validateOnly response was a receipt".to_owned(),
            }),
        }
    }
    pub async fn history(
        &self,
        object_type: &str,
        pk: &Value,
        limit: Option<u32>,
        before_seq: Option<i64>,
    ) -> Result<dto::HistoryPage, ClientError> {
        let mut path = format!(
            "/api/objects/{}/{}/history",
            encode_path(object_type),
            encode_path(&pk_path(pk))
        );
        let mut query = Vec::new();
        if let Some(limit) = limit {
            query.push(format!("limit={limit}"));
        }
        if let Some(before_seq) = before_seq {
            query.push(format!("beforeSeq={before_seq}"));
        }
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }
        decode_history_page(&self.send(Method::GET, &path, None).await?)
    }
    pub async fn get_command_result(
        &self,
        command_id: &str,
    ) -> Result<dto::ActionReceipt, ClientError> {
        let path = format!("/api/commands/{}", encode_path(command_id));
        decode_action_receipt(&self.send(Method::GET, &path, None).await?)
    }
    async fn send_json<T: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: &T,
    ) -> Result<Value, ClientError> {
        let body = serde_json::to_value(body).map_err(|error| ClientError::Decode {
            api_name: path.to_owned(),
            message: error.to_string(),
        })?;
        self.send(method, path, Some(body)).await
    }
    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, ClientError> {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| ClientError::InvalidBaseUrl(error.to_string()))?;
        let mut request = self.http.request(method, url);
        if let Some(token) = &self.bearer_token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(ClientError::Transport)?;
        let status = response.status();
        let api_name = path
            .split('/')
            .rfind(|part| !part.is_empty())
            .unwrap_or("unknown")
            .split('?')
            .next()
            .unwrap_or("unknown")
            .to_owned();
        let value = response
            .json::<Value>()
            .await
            .map_err(|error| ClientError::Decode {
                api_name: api_name.clone(),
                message: error.to_string(),
            })?;
        if !status.is_success() {
            return Err(ClientError::HttpStatus {
                status,
                envelope: decode_error_envelope(&value),
            });
        }
        Ok(value)
    }
}

fn decode_error_envelope(value: &Value) -> dto::ErrorEnvelope {
    dto::ErrorEnvelope {
        code: string_field(value, "code"),
        api_name: value
            .get("apiName")
            .or_else(|| value.get("api_name"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        request_id: value
            .get("request_id")
            .or_else(|| value.get("requestId"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    }
}
fn decode_metadata(value: &Value) -> Result<dto::Metadata, ClientError> {
    decode("ontology", || {
        Ok(dto::Metadata {
            api_name: string_field(value, "apiName"),
            display_name: optional_string(value, "displayName"),
            description: optional_string(value, "description"),
            object_types: value
                .get("objectTypes")
                .and_then(Value::as_array)
                .ok_or_else(|| "objectTypes must be an array".to_owned())?
                .iter()
                .map(decode_object_metadata)
                .collect::<Result<_, _>>()?,
            links: value
                .get("links")
                .and_then(Value::as_array)
                .ok_or_else(|| "links must be an array".to_owned())?
                .iter()
                .map(decode_link_metadata)
                .collect(),
            actions: value
                .get("actions")
                .and_then(Value::as_array)
                .ok_or_else(|| "actions must be an array".to_owned())?
                .iter()
                .map(decode_action_metadata)
                .collect(),
            def_version: value
                .get("defVersion")
                .and_then(Value::as_i64)
                .ok_or_else(|| "defVersion must be an integer".to_owned())?,
            engine_status: decode_engine_status(value, "engineStatus")?,
        })
    })
}
fn decode_object_metadata(value: &Value) -> Result<dto::ObjectMetadata, String> {
    Ok(dto::ObjectMetadata {
        api_name: string_field(value, "apiName"),
        display_name: optional_string(value, "displayName"),
        description: optional_string(value, "description"),
        aliases: string_array(value, "aliases")?,
        primary_key: string_field(value, "primaryKey"),
        title_key: string_field(value, "titleKey"),
        properties: value
            .get("properties")
            .and_then(Value::as_array)
            .ok_or_else(|| "properties must be an array".to_owned())?
            .iter()
            .map(|property| {
                Ok(dto::PropertyMetadata {
                    api_name: string_field(property, "apiName"),
                    r#type: string_field(property, "type"),
                    required: property
                        .get("required")
                        .and_then(Value::as_bool)
                        .ok_or_else(|| "required must be a boolean".to_owned())?,
                    display_name: optional_string(property, "displayName"),
                    description: optional_string(property, "description"),
                })
            })
            .collect::<Result<_, String>>()?,
    })
}
fn decode_link_metadata(value: &Value) -> dto::LinkMetadata {
    dto::LinkMetadata {
        api_name: string_field(value, "apiName"),
        display_name: optional_string(value, "displayName"),
        description: optional_string(value, "description"),
        from: string_field(value, "from"),
        to: string_field(value, "to"),
    }
}
fn decode_action_metadata(value: &Value) -> dto::ActionMetadata {
    dto::ActionMetadata {
        api_name: string_field(value, "apiName"),
        display_name: optional_string(value, "displayName"),
        description: optional_string(value, "description"),
    }
}
fn decode_object_row(value: &Value) -> Result<dto::ObjectRow, ClientError> {
    decode("objects", || decode_object_row_value(value))
}
fn decode_list_page(value: &Value) -> Result<dto::ListPage, ClientError> {
    decode("query", || {
        Ok(dto::ListPage {
            rows: value
                .get("rows")
                .and_then(Value::as_array)
                .ok_or_else(|| "rows must be an array".to_owned())?
                .iter()
                .map(decode_object_row_value)
                .collect::<Result<_, _>>()?,
            has_more: bool_field(value, "hasMore"),
            truncated: bool_field(value, "truncated"),
            cursor: optional_string(value, "cursor"),
        })
    })
}
fn decode_object_row_value(value: &Value) -> Result<dto::ObjectRow, String> {
    Ok(dto::ObjectRow {
        object_type: string_field(value, "objectType"),
        pk: value
            .get("pk")
            .cloned()
            .ok_or_else(|| "pk is required".to_owned())?,
        properties: object_field(value, "properties")?,
        version: string_field(value, "_version"),
    })
}
fn decode_aggregate_result(value: &Value) -> Result<dto::AggregateResult, ClientError> {
    decode("aggregate", || {
        Ok(dto::AggregateResult {
            buckets: value
                .get("buckets")
                .and_then(Value::as_array)
                .ok_or_else(|| "buckets must be an array".to_owned())?
                .iter()
                .map(|bucket| {
                    Ok(dto::AggregateBucket {
                        group: bucket
                            .get("group")
                            .and_then(Value::as_array)
                            .ok_or_else(|| "group must be an array".to_owned())?
                            .clone(),
                        values: bucket
                            .get("values")
                            .and_then(Value::as_array)
                            .ok_or_else(|| "values must be an array".to_owned())?
                            .clone(),
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    })
}
fn decode_health(value: &Value) -> Result<dto::HealthResponse, ClientError> {
    decode("health", || {
        Ok(dto::HealthResponse {
            status: string_field(value, "status"),
            engine_status: decode_engine_status(value, "engineStatus")?,
        })
    })
}
fn decode_history_page(value: &Value) -> Result<dto::HistoryPage, ClientError> {
    decode("history", || {
        Ok(dto::HistoryPage {
            entries: value
                .get("entries")
                .and_then(Value::as_array)
                .ok_or_else(|| "entries must be an array".to_owned())?
                .iter()
                .map(decode_history_entry)
                .collect::<Result<_, String>>()?,
            truncated: bool_field(value, "truncated"),
            next_before_seq: value.get("nextBeforeSeq").and_then(Value::as_i64),
        })
    })
}
fn decode_history_entry(value: &Value) -> Result<dto::HistoryEntry, String> {
    Ok(dto::HistoryEntry {
        seq: integer_field(value, "seq")?,
        event_id: string_field(value, "eventId"),
        actor: string_field(value, "actor"),
        action: string_field(value, "action"),
        committed_at: string_field(value, "committedAt"),
        op: string_field(value, "op"),
        version: integer_field(value, "version")?,
        values: object_field(value, "values")?,
        unset: string_array_default(value, "unset")?,
        masked_changes: string_array_default(value, "maskedChanges")?,
        params: value.get("params").cloned(),
    })
}
fn decode_action_receipt(value: &Value) -> Result<dto::ActionReceipt, ClientError> {
    decode("commands", || {
        Ok(dto::ActionReceipt {
            command_id: string_field(value, "commandId"),
            event_id: string_field(value, "eventId"),
            log_seq: integer_field(value, "logSeq")?,
            committed_at: string_field(value, "committedAt"),
            chosen_current_time: string_field(value, "chosenCurrentTime"),
            items: value
                .get("items")
                .and_then(Value::as_array)
                .ok_or_else(|| "items must be an array".to_owned())?
                .iter()
                .map(|item| {
                    let edit = item
                        .get("edits")
                        .and_then(Value::as_array)
                        .and_then(|edits| edits.first())
                        .ok_or_else(|| "receipt item must contain an edit".to_owned())?;
                    Ok(dto::ItemReceipt {
                        item_ordinal: u32::try_from(integer_field_any(
                            item,
                            &["itemOrdinal", "item_ordinal"],
                        )?)
                        .map_err(|_| {
                            "itemOrdinal must be an integer in 0..=4294967295".to_owned()
                        })?,
                        object_type: string_field_any(edit, &["typeName", "type_name"]),
                        pk: edit
                            .get("pk")
                            .cloned()
                            .ok_or_else(|| "receipt edit pk is required".to_owned())?,
                        version: integer_field_any(edit, &["versionAfter", "version_after"])?
                            .to_string(),
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    })
}
fn decode_engine_status(value: &Value, field: &str) -> Result<dto::EngineAvailability, String> {
    match string_field(value, field).as_str() {
        "available" => Ok(dto::EngineAvailability::Available),
        "unavailable" => Ok(dto::EngineAvailability::Unavailable),
        other => Err(format!("{field} has invalid value {other}")),
    }
}
fn decode<T>(api_name: &str, result: impl FnOnce() -> Result<T, String>) -> Result<T, ClientError> {
    result().map_err(|message| ClientError::Decode {
        api_name: api_name.to_owned(),
        message,
    })
}
fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
fn optional_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}
fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}
fn integer_field(value: &Value, field: &str) -> Result<i64, String> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{field} must be an integer"))
}
fn integer_field_any(value: &Value, fields: &[&str]) -> Result<i64, String> {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(Value::as_i64))
        .ok_or_else(|| format!("{} must be an integer", fields.join(" or ")))
}
fn string_field_any(value: &Value, fields: &[&str]) -> String {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned()
}
fn string_array(value: &Value, field: &str) -> Result<Vec<String>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("{field} must contain strings"))
        })
        .collect()
}
fn string_array_default(value: &Value, field: &str) -> Result<Vec<String>, String> {
    if value.get(field).is_none_or(Value::is_null) {
        return Ok(Vec::new());
    }
    string_array(value, field)
}
fn object_field(value: &Value, field: &str) -> Result<BTreeMap<String, Value>, String> {
    value
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{field} must be an object"))
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
}
fn pk_path(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}
fn encode_path(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            byte => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CRATE_NAME, ClientError, decode_action_receipt};
    use serde_json::{Value, json};

    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }

    fn receipt_with_ordinal(field: &str, ordinal: i64) -> Value {
        let mut value = json!({
            "commandId": "command-1",
            "eventId": "event-1",
            "logSeq": 1,
            "committedAt": "2026-10-31T00:00:00Z",
            "chosenCurrentTime": "2026-10-31T00:00:00Z",
            "items": [{
                "edits": [{"typeName": "Airport", "pk": "BER", "versionAfter": 1}]
            }]
        });
        value["items"][0][field] = json!(ordinal);
        value
    }

    #[test]
    fn receipt_rejects_negative_item_ordinal() {
        for field in ["itemOrdinal", "item_ordinal"] {
            let error = decode_action_receipt(&receipt_with_ordinal(field, -1)).unwrap_err();
            assert!(
                matches!(error, ClientError::Decode { api_name, .. } if api_name == "commands")
            );
        }
    }

    #[test]
    fn receipt_rejects_item_ordinal_above_u32_max() {
        for field in ["itemOrdinal", "item_ordinal"] {
            let error =
                decode_action_receipt(&receipt_with_ordinal(field, 4_294_967_296)).unwrap_err();
            assert!(
                matches!(error, ClientError::Decode { api_name, .. } if api_name == "commands")
            );
        }
    }

    #[test]
    fn receipt_preserves_item_ordinal_u32_boundaries() {
        for field in ["itemOrdinal", "item_ordinal"] {
            for (ordinal, expected) in [(0, 0), (4_294_967_295, u32::MAX)] {
                let receipt = decode_action_receipt(&receipt_with_ordinal(field, ordinal)).unwrap();
                assert_eq!(receipt.items[0].item_ordinal, expected);
            }
        }
    }
}
