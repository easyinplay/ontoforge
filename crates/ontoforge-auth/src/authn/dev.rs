//! Development tokens: HS256 tokens signed with a local key (AUTH-02).

use std::collections::BTreeMap;
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::config::PrincipalAttrConfig;
use secrecy::SecretBox;
use serde_json::{Map, Value};

use super::AuthnError;
use super::principal::Principal;

/// Issuer and audience of every development token.
pub const DEV_ISSUER: &str = "ontoforge-dev";

/// Proof that the caller knowingly enables insecure development tokens.
pub struct DevMode {
    _priv: (),
}

impl DevMode {
    #[must_use]
    pub fn enable_insecure() -> Self {
        Self { _priv: () }
    }
}

/// The 32 byte signing key of development tokens.
pub struct DevKey(#[allow(dead_code)] SecretBox<[u8; 32]>);

impl DevKey {
    pub fn load_or_create(_path: &Utf8Path) -> Result<Self, AuthnError> {
        Err(AuthnError::Config("not implemented".to_owned()))
    }
}

/// What a development token says about its subject.
pub struct DevClaims {
    pub sub: String,
    pub groups: Vec<String>,
    pub attrs: Map<String, Value>,
    pub ttl: Duration,
}

pub fn issue_dev_token(_key: &DevKey, _claims: &DevClaims) -> Result<String, AuthnError> {
    Err(AuthnError::Config("not implemented".to_owned()))
}

/// Verifies development tokens.
pub struct DevVerifier {
    _priv: (),
}

impl DevVerifier {
    #[must_use]
    pub fn new(
        _mode: DevMode,
        _key: DevKey,
        _attrs: &BTreeMap<String, PrincipalAttrConfig>,
    ) -> Self {
        Self { _priv: () }
    }

    pub(crate) fn verify(&self, _token: &str) -> Result<Principal, AuthnError> {
        Err(AuthnError::Malformed)
    }
}
