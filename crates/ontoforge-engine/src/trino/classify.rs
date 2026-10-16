//! The one table that turns HTTP statuses, transport failures and Trino error names into
//! [`EngineError`]s and retry decisions (ENGINE-02, ADR-0017).
//!
//! Everything that makes the engine unreachable ends up as `Unavailable { retryable: true }`;
//! nothing here falls back to an empty or partial result. Reasons describe the phenomenon
//! only: never a URL, a header or a credential.
//!
//! Retry rules (Trino client protocol, spike 1, RESEARCH A13):
//!
//! - `GET nextUri` is idempotent: 502 / 503 / 504 and failed connects are retried with a
//!   backoff of 50 ms doubling up to 1 s, at most [`MAX_RETRIES`] times.
//! - 429 is retried for GET and POST after `Retry-After` seconds (default 1 s, capped at 5 s).
//! - `POST /v1/statement` is retried only when the statement provably was not accepted: 429,
//!   a 503 whose body says `SERVER_STARTING_UP`, or a connect that never completed. A plain
//!   503 (a gateway can answer it for a statement the coordinator did take), 502 and 504 are
//!   not retried, so an expensive query is not submitted twice (E16).

use std::time::Duration;

use super::decode::QueryResults;
use crate::error::EngineError;

/// Retries after the first request, so a request is sent at most `MAX_RETRIES + 1` times.
pub(super) const MAX_RETRIES: u32 = 5;

const BACKOFF_START: Duration = Duration::from_millis(50);
const BACKOFF_CAP: Duration = Duration::from_secs(1);
const RETRY_AFTER_DEFAULT: Duration = Duration::from_secs(1);
const RETRY_AFTER_CAP: Duration = Duration::from_secs(5);

/// The error names Trino uses while the server is not ready to run queries
/// (`StandardErrorCode.SERVER_STARTING_UP` / `SERVER_SHUTTING_DOWN`).
const SERVER_STARTING_UP: &str = "SERVER_STARTING_UP";
const SERVER_SHUTTING_DOWN: &str = "SERVER_SHUTTING_DOWN";

/// What one exchange with the coordinator came to.
pub(super) enum Outcome {
    /// A page without an error.
    Page(QueryResults),
    /// Send the same request again after `after`.
    Retry { after: Duration },
    /// The exchange failed for good.
    Fail(EngineError),
}

/// Delay before retry number `attempt + 1` (`attempt` counts the retries already made).
fn backoff(attempt: u32) -> Duration {
    BACKOFF_START
        .saturating_mul(2u32.saturating_pow(attempt.min(8)))
        .min(BACKOFF_CAP)
}

/// `Retry-After` as whole seconds; anything else (a date, garbage, no header) waits the
/// default. A server cannot make the client wait longer than the cap.
fn retry_after_delay(header: Option<&str>) -> Duration {
    header
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map_or(RETRY_AFTER_DEFAULT, |secs| {
            Duration::from_secs(secs).min(RETRY_AFTER_CAP)
        })
}

/// Retry after `after`, or give up as unavailable once the retries are used up.
fn retry_or_give_up(status: u16, attempt: u32, after: Duration) -> Outcome {
    if attempt < MAX_RETRIES {
        Outcome::Retry { after }
    } else {
        Outcome::Fail(EngineError::Unavailable {
            retryable: true,
            reason: format!("Trino returned {status} after {} attempts", attempt + 1),
        })
    }
}

/// Maps a response status. `None` means the status is 200 and the body is a page.
///
/// `retry_after` is the `Retry-After` header, `error_name` the `error.errorName` of the
/// response body when it could be parsed, `is_post` marks the statement submission and
/// `attempt` counts the retries already made.
pub(super) fn classify_status(
    status: u16,
    retry_after: Option<&str>,
    error_name: Option<&str>,
    is_post: bool,
    attempt: u32,
) -> Option<Outcome> {
    let outcome = match status {
        200 => return None,
        401 | 403 => Outcome::Fail(EngineError::Unauthorized {
            reason: format!("HTTP {status}"),
        }),
        429 => retry_or_give_up(status, attempt, retry_after_delay(retry_after)),
        502..=504 => {
            let provably_not_accepted = status == 503 && error_name == Some(SERVER_STARTING_UP);
            if is_post && !provably_not_accepted {
                let named = error_name.map_or_else(String::new, |name| format!(" ({name})"));
                Outcome::Fail(EngineError::Unavailable {
                    retryable: true,
                    reason: format!(
                        "Trino returned {status}{named} to the statement submission; \
                         not retried because the statement may have been accepted"
                    ),
                })
            } else {
                retry_or_give_up(status, attempt, backoff(attempt))
            }
        }
        _ => Outcome::Fail(EngineError::Protocol(format!(
            "unexpected HTTP status {status}"
        ))),
    };
    Some(outcome)
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

/// Maps a failure to send a request. A connection that was never established (refused, DNS,
/// TLS handshake) provably did not deliver the request, so it is retried for GET and POST
/// alike; everything else fails at once.
pub(super) fn classify_transport(err: reqwest::Error, attempt: u32) -> Outcome {
    if err.is_connect() && attempt < MAX_RETRIES {
        Outcome::Retry {
            after: backoff(attempt),
        }
    } else {
        Outcome::Fail(transport_failure(err))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn retry_delay(outcome: Option<&Outcome>) -> Duration {
        match outcome {
            Some(Outcome::Retry { after }) => *after,
            _ => panic!("expected a retry"),
        }
    }

    #[test]
    fn backoff_doubles_from_50ms_up_to_one_second() {
        let waits: Vec<_> = (0..MAX_RETRIES).map(backoff).collect();
        let millis: Vec<_> = waits.iter().map(Duration::as_millis).collect();
        assert_eq!(millis, [50, 100, 200, 400, 800]);
        assert_eq!(backoff(6), Duration::from_secs(1));
        assert_eq!(backoff(u32::MAX), Duration::from_secs(1));
    }

    #[test]
    fn retry_after_is_whole_seconds_capped_at_five() {
        let delay = |header| retry_delay(classify_status(429, header, None, true, 0).as_ref());
        assert_eq!(delay(Some("5")), Duration::from_secs(5));
        assert_eq!(delay(Some("6")), Duration::from_secs(5));
        assert_eq!(delay(Some("600")), Duration::from_secs(5));
        assert_eq!(delay(Some(" 2 ")), Duration::from_secs(2));
        assert_eq!(delay(Some("0")), Duration::ZERO);
        assert_eq!(delay(None), Duration::from_secs(1));
        assert_eq!(delay(Some("soon")), Duration::from_secs(1));
        assert_eq!(delay(Some("-3")), Duration::from_secs(1));
        assert_eq!(
            delay(Some("Wed, 21 Oct 2026 07:28:00 GMT")),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn the_fifth_retry_is_the_last() {
        let last = classify_status(503, None, None, false, MAX_RETRIES - 1);
        assert_eq!(retry_delay(last.as_ref()), Duration::from_millis(800));
        match classify_status(503, None, None, false, MAX_RETRIES) {
            Some(Outcome::Fail(EngineError::Unavailable { retryable, reason })) => {
                assert!(retryable);
                assert!(reason.contains("after 6 attempts"), "{reason}");
            }
            _ => panic!("expected the retries to be used up"),
        }
    }

    #[test]
    fn post_retries_only_what_was_not_accepted() {
        let post = |status, name| classify_status(status, None, name, true, 0);
        assert!(matches!(post(429, None), Some(Outcome::Retry { .. })));
        assert!(matches!(
            post(503, Some("SERVER_STARTING_UP")),
            Some(Outcome::Retry { .. })
        ));
        for failing in [
            post(503, None),
            post(503, Some("SERVER_SHUTTING_DOWN")),
            post(502, Some("SERVER_STARTING_UP")),
            post(504, None),
        ] {
            assert!(matches!(
                failing,
                Some(Outcome::Fail(EngineError::Unavailable { .. }))
            ));
        }
    }

    #[test]
    fn statuses_without_a_retry_rule() {
        assert!(classify_status(200, None, None, true, 0).is_none());
        assert!(matches!(
            classify_status(401, None, None, false, 0),
            Some(Outcome::Fail(EngineError::Unauthorized { .. }))
        ));
        assert!(matches!(
            classify_status(403, None, None, true, 0),
            Some(Outcome::Fail(EngineError::Unauthorized { .. }))
        ));
        for status in [400, 404, 500, 501] {
            assert!(matches!(
                classify_status(status, None, None, false, 0),
                Some(Outcome::Fail(EngineError::Protocol(_)))
            ));
        }
    }
}
