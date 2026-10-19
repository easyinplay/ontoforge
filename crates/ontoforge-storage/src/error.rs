//! Errors of the edit store.

/// A failure of the edit store. Messages never contain a connection string.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The database could not be reached or opened.
    #[error("cannot connect to the edit store: {0}")]
    Connect(String),
    /// A system migration failed.
    #[error("edit store migration failed: {0}")]
    Migrate(String),
    /// A statement failed. `sqlstate` is the database's error code when it reported one.
    #[error("edit store query failed{}: {message}", sqlstate_suffix(.sqlstate.as_ref()))]
    Query {
        sqlstate: Option<String>,
        message: String,
    },
    /// The store is configured in a way this build cannot serve.
    #[error("edit store configuration: {0}")]
    Config(String),
    /// Stored data does not have the shape this code wrote.
    #[error("edit store content is corrupt: {0}")]
    Corrupt(String),
}

fn sqlstate_suffix(sqlstate: Option<&String>) -> String {
    sqlstate.map(|s| format!(" (code {s})")).unwrap_or_default()
}

impl StoreError {
    /// Maps a driver error. Configuration errors can echo the connection string, so their text
    /// is replaced.
    pub(crate) fn from_sqlx(error: &sqlx::Error) -> Self {
        match error {
            sqlx::Error::Configuration(_) => Self::Config("invalid connection settings".to_owned()),
            sqlx::Error::Database(db) => Self::Query {
                sqlstate: db.code().map(std::borrow::Cow::into_owned),
                message: db.message().to_owned(),
            },
            other => Self::Query {
                sqlstate: None,
                message: other.to_string(),
            },
        }
    }

    pub(crate) fn postgres_unsupported() -> Self {
        Self::Config("the PostgreSQL edit store is not available in this build".to_owned())
    }
}
