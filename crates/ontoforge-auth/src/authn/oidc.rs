//! OIDC bearer token verification.
//!
//! The algorithm is chosen from the token header, checked against the issuer's allow-list
//! and only then used to build the `Validation`. The unverified payload is never used to
//! choose an algorithm or a key; it is read only to pick the issuer when several are
//! trusted, and the verification that follows enforces that same issuer.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

use camino::Utf8Path;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, Validation, decode};
use ontoforge_engine::config::{OidcConfig, PrincipalAttrConfig};
use reqwest::Url;
use serde_json::{Map, Value};

use super::AuthnError;
use super::jwks::{JwksCache, JwksPolicy};
use super::principal::{Principal, principal_from_claims};

/// Largest accepted `ca_file`, in bytes.
const MAX_CA_BYTES: u64 = 1024 * 1024;

pub(crate) struct OidcVerifier {
    issuer: String,
    audience: String,
    algorithms: Vec<Algorithm>,
    leeway_secs: u64,
    groups_claim: String,
    jwks: JwksCache,
}

impl OidcVerifier {
    pub(crate) fn from_config(cfg: &OidcConfig, config_dir: &Utf8Path) -> Result<Self, AuthnError> {
        let mut algorithms = Vec::new();
        for name in &cfg.algorithms {
            let algorithm = Algorithm::from_str(name).map_err(|_| {
                AuthnError::Config(format!(
                    "issuer `{}`: unknown algorithm `{name}`",
                    cfg.issuer
                ))
            })?;
            if algorithm.family() == jsonwebtoken::AlgorithmFamily::Hmac {
                return Err(AuthnError::Config(format!(
                    "issuer `{}`: symmetric algorithm `{name}` is not allowed",
                    cfg.issuer
                )));
            }
            algorithms.push(algorithm);
        }
        let uri = Url::parse(&cfg.jwks_uri).map_err(|_| {
            AuthnError::Config(format!(
                "issuer `{}`: `jwks_uri` is not a valid URL",
                cfg.issuer
            ))
        })?;
        let policy = JwksPolicy::default();
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(policy.timeout)
            .user_agent(concat!("ontoforge/", env!("CARGO_PKG_VERSION")));
        if let Some(file) = &cfg.ca_file {
            builder = builder.tls_certs_merge(read_ca(&cfg.issuer, config_dir, file)?);
        }
        let client = builder.build().map_err(|_| {
            AuthnError::Config(format!(
                "issuer `{}`: cannot build the HTTP client",
                cfg.issuer
            ))
        })?;
        Ok(Self {
            issuer: cfg.issuer.clone(),
            audience: cfg.audience.clone(),
            algorithms,
            leeway_secs: cfg.leeway_secs,
            groups_claim: cfg.groups_claim.clone(),
            jwks: JwksCache::new(uri, client, policy),
        })
    }

    pub(crate) fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Verifies `token` signed with `alg` (already read from the header) under `kid`.
    pub(crate) async fn verify(
        &self,
        token: &str,
        alg: Algorithm,
        kid: &str,
        namespaced: bool,
        attrs: &BTreeMap<String, PrincipalAttrConfig>,
    ) -> Result<Principal, AuthnError> {
        if !self.algorithms.contains(&alg) {
            return Err(AuthnError::UnsupportedAlgorithm);
        }
        let key = self.jwks.key(kid, alg).await?;
        // `alg` was checked against the allow-list above, so this accepts exactly one
        // algorithm that the operator configured for this issuer.
        let mut validation = Validation::new(alg);
        validation.leeway = self.leeway_secs;
        validation.validate_nbf = true;
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let data =
            decode::<Map<String, Value>>(token, &key, &validation).map_err(|e| map_error(&e))?;
        principal_from_claims(
            &self.issuer,
            &data.claims,
            &self.groups_claim,
            namespaced,
            attrs,
        )
    }
}

/// Maps a library error to ours without carrying any token or claim content.
fn map_error(error: &jsonwebtoken::errors::Error) -> AuthnError {
    match error.kind() {
        ErrorKind::ExpiredSignature => AuthnError::Expired,
        ErrorKind::InvalidSignature => AuthnError::InvalidSignature,
        ErrorKind::InvalidAlgorithm | ErrorKind::UnsupportedAlgorithm => {
            AuthnError::UnsupportedAlgorithm
        }
        ErrorKind::InvalidIssuer => AuthnError::UnknownIssuer,
        ErrorKind::InvalidAudience => {
            AuthnError::InvalidClaims("claim `aud` does not match".to_owned())
        }
        ErrorKind::ImmatureSignature => {
            AuthnError::InvalidClaims("the token is not valid yet".to_owned())
        }
        ErrorKind::MissingRequiredClaim(name) => {
            AuthnError::InvalidClaims(format!("claim `{name}` is missing"))
        }
        ErrorKind::InvalidClaimFormat(name) => {
            AuthnError::InvalidClaims(format!("claim `{name}` has an invalid format"))
        }
        _ => AuthnError::Malformed,
    }
}

fn read_ca(
    issuer: &str,
    config_dir: &Utf8Path,
    file: &str,
) -> Result<Vec<reqwest::Certificate>, AuthnError> {
    let path = config_dir.join(file);
    let fail =
        |what: &str| AuthnError::Config(format!("issuer `{issuer}`: ca_file `{path}` {what}"));
    let too_large = std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_CA_BYTES);
    if too_large {
        return Err(fail("is too large"));
    }
    let pem = std::fs::read(&path).map_err(|_| fail("cannot be read"))?;
    match reqwest::Certificate::from_pem_bundle(&pem) {
        Ok(certs) if !certs.is_empty() => Ok(certs),
        _ => Err(fail("holds no PEM certificate")),
    }
}
