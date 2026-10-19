//! Edit store, definition reload and migration, edit log and import (ADR-0011, ADR-0023).
//!
//! Dependency direction: storage -> schema, expr, auth, engine. `ontoforge-query` (Phase 4) and
//! `ontoforge-action` depend on this crate, never the other way round.

pub mod ddl;
pub mod diff;
pub mod error;
pub mod naming;
pub mod physical;
pub mod reload;
mod sql_exit;
pub mod store;

/// Cargo package name of this crate.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }
}
