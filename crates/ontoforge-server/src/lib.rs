//! axum REST server (Phase 6) and rmcp MCP server (Phase 7) (ADR-0011).

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
