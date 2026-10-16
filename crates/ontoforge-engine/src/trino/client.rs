//! One request / response exchange with the coordinator.

use reqwest::{RequestBuilder, StatusCode};

use super::decode::QueryResults;
use crate::error::EngineError;

/// Sends `request` once and parses the page it returns.
///
/// Retries, timeouts and cancellation wrap this function; it does exactly one exchange.
/// Error texts never carry the URL (it holds the query id) or any header.
pub(super) async fn send_once(
    request: RequestBuilder,
    max_page_bytes: usize,
) -> Result<QueryResults, EngineError> {
    let mut response = request.send().await.map_err(map_transport)?;
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(EngineError::Unauthorized {
            reason: format!("HTTP {status}"),
        });
    }
    if status != StatusCode::OK {
        return Err(EngineError::Protocol(format!(
            "unexpected HTTP status {status}"
        )));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_transport)? {
        if body.len() + chunk.len() > max_page_bytes {
            return Err(EngineError::Protocol(format!(
                "response page exceeds {max_page_bytes} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|e| {
        // The serde message can quote cell values; keep to the position.
        EngineError::Protocol(format!(
            "invalid QueryResults JSON ({:?}) at line {} column {}",
            e.classify(),
            e.line(),
            e.column()
        ))
    })
}

fn map_transport(e: reqwest::Error) -> EngineError {
    use std::error::Error as _;

    let e = e.without_url();

    let mut reason = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    EngineError::Unavailable {
        retryable: e.is_connect(),
        reason,
    }
}
