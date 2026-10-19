//! Authentication and authorization (ADR-0011). Implemented in Phase 5.

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
