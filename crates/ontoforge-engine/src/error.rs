//! Errors of the engine layer.
//!
//! `EngineError` and `PrintError` are `Clone`: one failing catalog request has to be copied
//! onto every binding that depends on it. Messages only carry the reason text the variant was
//! built with; callers building those texts must never put request headers or credentials in
//! them.

use thiserror::Error;

/// Why a [`crate::printer`] AST could not be turned into SQL text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PrintError {
    /// A string literal contained a NUL character.
    #[error("string literal contains a NUL character")]
    NulInString,
    /// An identifier was the empty string.
    #[error("identifier is empty")]
    EmptyIdentifier,
    /// An identifier contained a NUL character.
    #[error("identifier contains a NUL character")]
    NulInIdentifier,
    /// A double literal was NaN or infinite.
    #[error("double literal is not finite")]
    NonFiniteDouble,
    /// A decimal literal did not match `-?digits(.digits)?`.
    #[error("invalid decimal literal `{0}`")]
    InvalidDecimal(String),
    /// A date literal was not a valid `YYYY-MM-DD`.
    #[error("invalid date literal `{0}`")]
    InvalidDate(String),
    /// A timestamp literal was not a valid `YYYY-MM-DD HH:MM:SS(.fff)?`.
    #[error("invalid timestamp literal `{0}`")]
    InvalidTimestamp(String),
    /// A `SELECT` without any projection item.
    #[error("select has an empty projection")]
    EmptyProjection,
    /// An `IN` list without elements.
    #[error("IN list is empty")]
    EmptyInList,
    /// An `AND` / `OR` without operands.
    #[error("AND / OR has no operands")]
    EmptyBoolList,
}

/// A failure of an [`crate::Engine`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    /// The engine cannot be reached (connection, DNS, TLS, exhausted 5xx retries).
    #[error("engine unavailable: {reason}")]
    Unavailable { retryable: bool, reason: String },
    /// The engine rejected the credentials (401 / 403).
    #[error("engine rejected the credentials: {reason}")]
    Unauthorized { reason: String },
    /// The query ran longer than the allowed time.
    #[error("query timed out after {after_ms} ms")]
    Timeout { after_ms: u64 },
    /// The query was cancelled.
    #[error("query cancelled")]
    Cancelled,
    /// The engine reported a failure for this query.
    #[error("query failed ({error_name}): {message}")]
    Query { error_name: String, message: String },
    /// The engine broke the wire protocol.
    #[error("protocol error: {0}")]
    Protocol(String),
    /// The SQL could not be printed.
    #[error("cannot print SQL: {0}")]
    Print(#[from] PrintError),
    /// The engine is configured incorrectly or was asked something it cannot answer.
    #[error("engine configuration error: {0}")]
    Config(String),
    /// Fixture data could not be loaded into the stand-in engine.
    #[error("fixture error: {0}")]
    Fixture(String),
}

impl EngineError {
    /// Stable upper-snake error code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "ENGINE_UNAVAILABLE",
            Self::Unauthorized { .. } => "ENGINE_UNAUTHORIZED",
            Self::Timeout { .. } => "ENGINE_TIMEOUT",
            Self::Cancelled => "ENGINE_CANCELLED",
            Self::Query { .. } => "ENGINE_QUERY_FAILED",
            Self::Protocol(_) => "ENGINE_PROTOCOL_ERROR",
            Self::Print(_) => "SQL_PRINT_ERROR",
            Self::Config(_) => "ENGINE_CONFIG_ERROR",
            Self::Fixture(_) => "ENGINE_FIXTURE_ERROR",
        }
    }

    /// Whether the engine as a whole cannot serve this round of requests.
    ///
    /// `true`: the caller should abort the whole round. `false`: only the query target that
    /// produced the error is affected; the caller should record it on that target and go on
    /// with the others.
    #[must_use]
    pub fn is_engine_wide(&self) -> bool {
        matches!(
            self,
            Self::Unavailable { .. }
                | Self::Unauthorized { .. }
                | Self::Cancelled
                | Self::Config(_)
        )
    }
}
