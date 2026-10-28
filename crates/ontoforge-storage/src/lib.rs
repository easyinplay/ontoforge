//! Edit store, definition reload and migration, edit log and import (ADR-0011, ADR-0023).
//!
//! Dependency direction: storage -> schema, expr, auth, engine. `ontoforge-query` (Phase 4) and
//! `ontoforge-action` depend on this crate, never the other way round.

pub mod commit;
pub mod ddl;
pub mod diff;
pub mod error;
pub mod history;
pub mod import;
mod index;
pub mod log;
pub mod migrate;
pub mod naming;
pub mod physical;
pub mod query_read;
pub mod read_audit;
pub mod reload;
pub mod scope_store;
mod sql_exit;
pub mod state;
pub mod store;
pub mod value;

/// Cargo package name of this crate.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// The complete metadata captured for one request, with no live transaction.
pub struct RequestSnapshot {
    pub active: std::sync::Arc<store::ActiveDefinition>,
    pub scope: ontoforge_schema::ir::ScopeSet,
}

impl store::EditStore {
    pub async fn request_snapshot(&self) -> Result<RequestSnapshot, error::StoreError> {
        Err(error::StoreError::Config("request snapshots are not implemented".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }
}
