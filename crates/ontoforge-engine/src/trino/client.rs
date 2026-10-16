//! One request / response exchange with the coordinator.

use reqwest::{RequestBuilder, Response};

use super::classify::{self, Outcome};
use super::decode::QueryResults;
use crate::error::EngineError;

/// Sends the request `build` makes and parses the page it returns, retrying as
/// [`classify`] decides. `build` is called again for every attempt.
///
/// `is_post` marks the statement submission, whose retries are narrower than a GET's.
/// Timeouts and cancellation wrap this function. Error texts never carry the URL (it holds
/// the query id) or any header.
pub(super) async fn send<F>(
    build: F,
    is_post: bool,
    max_page_bytes: usize,
) -> Result<QueryResults, EngineError>
where
    F: Fn() -> Result<RequestBuilder, EngineError>,
{
    let mut attempt = 0;
    loop {
        match exchange(build()?, is_post, attempt, max_page_bytes).await {
            Outcome::Page(page) => return Ok(page),
            Outcome::Fail(error) => return Err(error),
            Outcome::Retry { after } => {
                tokio::time::sleep(after).await;
                attempt += 1;
            }
        }
    }
}

/// One request and its response, classified.
async fn exchange(
    request: RequestBuilder,
    is_post: bool,
    attempt: u32,
    max_page_bytes: usize,
) -> Outcome {
    let mut response = match request.send().await {
        Ok(response) => response,
        Err(e) => return classify::classify_transport(e, attempt),
    };
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    // A gateway may answer 503 for a statement the server did accept; only the body tells a
    // server that was not ready (`SERVER_STARTING_UP`) from a plain gateway failure.
    let error_name = if is_post && status == 503 {
        peek_error_name(&mut response, max_page_bytes).await
    } else {
        None
    };
    if let Some(outcome) = classify::classify_status(
        status,
        retry_after.as_deref(),
        error_name.as_deref(),
        is_post,
        attempt,
    ) {
        return outcome;
    }
    match read_page(&mut response, max_page_bytes).await {
        Ok(page) => page_outcome(page),
        Err(e) => Outcome::Fail(e),
    }
}

fn page_outcome(mut page: QueryResults) -> Outcome {
    match page.error.take() {
        Some(error) => Outcome::Fail(classify::classify_query_error(
            error.error_name.as_deref().unwrap_or("UNKNOWN"),
            &error.message,
        )),
        None => Outcome::Page(page),
    }
}

/// The `error.errorName` of a non-200 body; `None` when the body is not a Trino page.
async fn peek_error_name(response: &mut Response, max_page_bytes: usize) -> Option<String> {
    let body = read_body(response, max_page_bytes).await.ok()?;
    let page: QueryResults = serde_json::from_slice(&body).ok()?;
    page.error?.error_name
}

async fn read_page(
    response: &mut Response,
    max_page_bytes: usize,
) -> Result<QueryResults, EngineError> {
    let body = read_body(response, max_page_bytes).await?;
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

/// Reads the body chunk by chunk and stops as soon as it outgrows `max_page_bytes`, whatever
/// `Content-Length` claims.
async fn read_body(response: &mut Response, max_page_bytes: usize) -> Result<Vec<u8>, EngineError> {
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(classify::transport_failure)?
    {
        if body.len() + chunk.len() > max_page_bytes {
            return Err(EngineError::Protocol(format!(
                "response page exceeds {max_page_bytes} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
