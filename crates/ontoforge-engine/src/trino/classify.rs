//! The one table that turns HTTP statuses, transport failures and Trino error names into
//! [`EngineError`]s (ENGINE-02, ADR-0017).
//!
//! Everything that makes the engine unreachable ends up as `Unavailable { retryable: true }`;
//! nothing here falls back to an empty or partial result. Reasons describe the phenomenon
//! only: never a URL, a header or a credential.

use super::decode::QueryResults;
use crate::error::EngineError;

/// What one exchange with the coordinator came to.
pub(super) enum Outcome {
    /// A page without an error.
    Page(QueryResults),
    /// The exchange failed for good.
    Fail(EngineError),
}

/// The error names Trino uses while the server is not ready to run queries
/// (`StandardErrorCode.SERVER_STARTING_UP` / `SERVER_SHUTTING_DOWN`).
const SERVER_STARTING_UP: &str = "SERVER_STARTING_UP";
const SERVER_SHUTTING_DOWN: &str = "SERVER_SHUTTING_DOWN";

/// Maps a response status. `None` means the status is 200 and the body is a page.
///
/// `error_name` is the `error.errorName` of the response body when it could be parsed.
pub(super) fn classify_status(status: u16, error_name: Option<&str>) -> Option<Outcome> {
    let error = match status {
        200 => return None,
        401 | 403 => EngineError::Unauthorized {
            reason: format!("HTTP {status}"),
        },
        429 | 502 | 503 | 504 => EngineError::Unavailable {
            retryable: true,
            reason: match error_name {
                Some(name) => format!("Trino returned {status} ({name})"),
                None => format!("Trino returned {status}"),
            },
        },
        _ => EngineError::Protocol(format!("unexpected HTTP status {status}")),
    };
    Some(Outcome::Fail(error))
}

/// Maps an error reported inside a page. A server that is starting or stopping is
/// unavailable; every other name is a failure of this query.
pub(super) fn classify_query_error(error_name: &str, message: &str) -> EngineError {
    match error_name {
        SERVER_STARTING_UP | SERVER_SHUTTING_DOWN => EngineError::Unavailable {
            retryable: true,
            reason: format!("Trino reported {error_name}"),
        },
        _ => EngineError::Query {
            error_name: error_name.to_owned(),
            message: message.to_owned(),
        },
    }
}

/// Maps a failure to talk to the coordinator: connection refused, DNS, TLS handshake or a
/// connection that broke mid-response. The URL (it holds the query id) is stripped from the
/// text; the cause chain is kept.
pub(super) fn transport_failure(err: reqwest::Error) -> EngineError {
    use std::error::Error as _;

    let err = err.without_url();
    if err.is_builder() {
        return EngineError::Config("the HTTP request cannot be built".to_owned());
    }
    let mut reason = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    EngineError::Unavailable {
        retryable: true,
        reason,
    }
}
