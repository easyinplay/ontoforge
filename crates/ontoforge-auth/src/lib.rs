//! Authentication and authorization (ADR-0011, ADR-0023).
//!
//! This crate owns:
//!
//! - authentication ([`authn`]): OIDC JWT verification with a cached JWKS, and the
//!   [`Principal`] it produces. Dev tokens join in a later plan.
//! - role and scope compilation and merging (added by later plans).
//!
//! Dependency direction is fixed by ADR-0023: this crate depends on `ontoforge-schema`,
//! `ontoforge-expr` and `ontoforge-engine`, never on storage.

pub mod authn;

pub use authn::{Authenticator, AuthnError, Principal, PrincipalId};

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
