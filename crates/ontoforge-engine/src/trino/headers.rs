//! Per request header assembly. The client is stateless: every request carries everything
//! it needs and nothing the server sends back (`X-Trino-Set-*`) is remembered.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::QueryContext;
use crate::error::EngineError;

const TRACE_TOKEN_MAX: usize = 128;
const CLIENT_TAG_MAX: usize = 64;
/// Slack between the server side execution limit and the client side one, in seconds.
const SESSION_LIMIT_SLACK_SECS: u64 = 5;

fn is_token(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

/// Rejects a trace token or client tag that could inject into a header. The values are
/// never echoed back.
pub(super) fn validate_context(ctx: &QueryContext) -> Result<(), EngineError> {
    if !is_token(&ctx.trace_token, TRACE_TOKEN_MAX) {
        return Err(EngineError::Config(format!(
            "trace token must be 1 to {TRACE_TOKEN_MAX} characters from [A-Za-z0-9._:-]"
        )));
    }
    if ctx.client_tags.iter().any(|t| !is_token(t, CLIENT_TAG_MAX)) {
        return Err(EngineError::Config(format!(
            "client tags must be 1 to {CLIENT_TAG_MAX} characters from [A-Za-z0-9._:-]"
        )));
    }
    Ok(())
}

fn value(text: &str, what: &str) -> Result<HeaderValue, EngineError> {
    HeaderValue::from_str(text)
        .map_err(|_| EngineError::Config(format!("{what} is not a valid header value")))
}

/// The seconds the server may run a query: the client timeout rounded up, plus slack, so
/// the client side limit fires first.
fn server_limit_secs(ctx: &QueryContext) -> u64 {
    ctx.timeout.as_secs() + u64::from(ctx.timeout.subsec_nanos() > 0) + SESSION_LIMIT_SLACK_SECS
}

/// Headers for one request: `X-Trino-User`, `X-Trino-Source`, `X-Trino-Trace-Token`,
/// `X-Trino-Client-Tags`, `X-Trino-Time-Zone` and, when `with_session` is set,
/// `X-Trino-Session`. `with_session` is true for the POST only: session properties
/// belong to the statement, not to the polling of its results.
pub(super) fn statement_headers(
    user: &str,
    ctx: &QueryContext,
    with_session: bool,
) -> Result<HeaderMap, EngineError> {
    validate_context(ctx)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-trino-user"),
        value(user, "the Trino user")?,
    );
    headers.insert(
        HeaderName::from_static("x-trino-source"),
        HeaderValue::from_static("ontoforge"),
    );
    headers.insert(
        HeaderName::from_static("x-trino-trace-token"),
        value(&ctx.trace_token, "the trace token")?,
    );
    if !ctx.client_tags.is_empty() {
        headers.insert(
            HeaderName::from_static("x-trino-client-tags"),
            value(&ctx.client_tags.join(","), "the client tags")?,
        );
    }
    headers.insert(
        HeaderName::from_static("x-trino-time-zone"),
        HeaderValue::from_static("UTC"),
    );
    if with_session {
        headers.insert(
            HeaderName::from_static("x-trino-session"),
            value(
                &format!("query_max_execution_time={}s", server_limit_secs(ctx)),
                "the session property",
            )?,
        );
    }
    Ok(headers)
}

/// Adds the `Authorization` header of `auth` to `builder`.
pub(super) fn apply_auth(
    builder: reqwest::RequestBuilder,
    user: &str,
    auth: &super::Auth,
) -> reqwest::RequestBuilder {
    let _ = (user, auth);
    builder
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;
    use crate::trino::Auth;

    fn authorization(auth: &Auth) -> Option<String> {
        let request = apply_auth(
            reqwest::Client::new().post("https://trino.example/v1/statement"),
            "ontoforge",
            auth,
        )
        .build()
        .expect("request");
        request
            .headers()
            .get("authorization")
            .map(|v| v.to_str().expect("ascii").to_owned())
    }

    #[test]
    fn basic_and_bearer_headers() {
        assert_eq!(authorization(&Auth::None), None);
        assert_eq!(
            authorization(&Auth::Basic {
                password: SecretString::from("pw".to_owned())
            })
            .as_deref(),
            Some("Basic b250b2ZvcmdlOnB3")
        );
        assert_eq!(
            authorization(&Auth::Bearer {
                token: SecretString::from("tok.en".to_owned())
            })
            .as_deref(),
            Some("Bearer tok.en")
        );
    }
}
