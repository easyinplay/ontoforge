//! Authentication: bearer token to [`Principal`].
//!
//! OIDC access tokens are verified against the issuer's JWKS (AUTH-01). Dev tokens (AUTH-02)
//! share the entry point: the token header's algorithm picks the route, and the dev route
//! exists only when the caller attached a verifier with [`Authenticator::with_dev`].

use std::collections::BTreeMap;

use camino::Utf8Path;
use jsonwebtoken::dangerous::insecure_decode_claims;
use jsonwebtoken::decode_header;
use jsonwebtoken::{Algorithm, AlgorithmFamily};
use ontoforge_engine::config::{AuthConfig, PrincipalAttrConfig};
use serde_json::Value;

pub mod dev;
pub mod jwks;
mod oidc;
pub mod principal;

use dev::DevVerifier;
use oidc::OidcVerifier;
pub use principal::{Principal, PrincipalId};

/// Why a bearer token was not accepted. `Display` names the kind of failure, and at most a
/// configured issuer, attribute or claim name; it never contains token or claim values.
#[derive(Debug, thiserror::Error)]
pub enum AuthnError {
    #[error("the token is malformed")]
    Malformed,
    #[error("the token algorithm is not accepted")]
    UnsupportedAlgorithm,
    #[error("the token issuer is not trusted")]
    UnknownIssuer,
    #[error("no signing key matches the token")]
    UnknownKey,
    #[error("the token signature is invalid")]
    InvalidSignature,
    #[error("the token has expired")]
    Expired,
    #[error("invalid token claims: {0}")]
    InvalidClaims(String),
    #[error("the signing keys are unavailable: {0}")]
    JwksUnavailable(String),
    #[error("invalid authentication configuration: {0}")]
    Config(String),
}

/// Verifies bearer tokens.
pub struct Authenticator {
    oidc: Vec<OidcVerifier>,
    attrs: BTreeMap<String, PrincipalAttrConfig>,
    dev: Option<DevVerifier>,
}

impl Authenticator {
    /// Builds the verifiers of `cfg`. Relative paths resolve against `config_dir`.
    pub fn from_config(cfg: &AuthConfig, config_dir: &Utf8Path) -> Result<Self, AuthnError> {
        let oidc = cfg
            .oidc
            .iter()
            .map(|entry| OidcVerifier::from_config(entry, config_dir))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            oidc,
            attrs: cfg.principal_attrs.clone(),
            dev: None,
        })
    }

    /// Also accepts development tokens verified by `dev`.
    #[must_use]
    pub fn with_dev(mut self, dev: DevVerifier) -> Self {
        self.dev = Some(dev);
        self
    }

    /// Authenticates `bearer` (the token without the `Bearer ` prefix).
    pub async fn authenticate(&self, bearer: &str) -> Result<Principal, AuthnError> {
        let header = decode_header(bearer).map_err(|_| AuthnError::Malformed)?;
        // The route is decided by the header algorithm alone. Symmetric algorithms have no
        // OIDC route: HS256 goes to the dev verifier when the caller attached one, every
        // other symmetric algorithm is refused. Asymmetric tokens never reach the dev route.
        if header.alg.family() == AlgorithmFamily::Hmac {
            return match (&self.dev, header.alg) {
                (Some(dev), Algorithm::HS256) => dev.verify(bearer),
                _ => Err(AuthnError::UnsupportedAlgorithm),
            };
        }
        let kid = header.kid.as_deref().ok_or(AuthnError::Malformed)?;
        let verifier = self.verifier_for(bearer)?;
        verifier
            .verify(bearer, header.alg, kid, self.oidc.len() > 1, &self.attrs)
            .await
    }

    /// The verifier for the token's issuer. With one trusted issuer it is that issuer; with
    /// several, the unverified `iss` selects it and the verification enforces it.
    fn verifier_for(&self, bearer: &str) -> Result<&OidcVerifier, AuthnError> {
        if let [only] = self.oidc.as_slice() {
            return Ok(only);
        }
        let claims: Value = insecure_decode_claims(bearer).map_err(|_| AuthnError::Malformed)?;
        let issuer = claims
            .get("iss")
            .and_then(Value::as_str)
            .ok_or(AuthnError::UnknownIssuer)?;
        self.oidc
            .iter()
            .find(|verifier| verifier.issuer() == issuer)
            .ok_or(AuthnError::UnknownIssuer)
    }
}
