//! JWKS fetching and caching (ADR-0008: own cache, no extra crate).
//!
//! The key set address only ever comes from `ontoforge-server.toml`. A token header can
//! never point this module somewhere else.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::{Jwk, PublicKeyUse};
use jsonwebtoken::{Algorithm, DecodingKey};
use reqwest::Url;
use serde_json::Value;

use super::AuthnError;

/// Cache and fetch limits for one key set.
#[derive(Debug, Clone)]
pub struct JwksPolicy {
    /// Lifetime when the response carries no usable `Cache-Control: max-age`.
    pub default_ttl: Duration,
    pub min_ttl: Duration,
    pub max_ttl: Duration,
    /// Smallest gap between two fetches.
    pub min_refresh: Duration,
    /// Keys older than this are no longer used even when the identity provider is down.
    pub max_stale: Duration,
    pub max_body_bytes: usize,
    pub timeout: Duration,
}

impl Default for JwksPolicy {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_hours(1),
            min_ttl: Duration::from_mins(1),
            max_ttl: Duration::from_hours(24),
            min_refresh: Duration::from_secs(30),
            max_stale: Duration::from_hours(24),
            max_body_bytes: 1024 * 1024,
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Clone)]
struct CachedKey {
    key: Arc<DecodingKey>,
    /// The `alg` the key set declares for this key, when it does.
    alg: Option<Algorithm>,
}

struct CacheState {
    keys: HashMap<String, CachedKey>,
    expires_at: Option<Instant>,
}

/// The signing keys of one identity provider.
pub struct JwksCache {
    uri: Url,
    client: reqwest::Client,
    state: RwLock<CacheState>,
    policy: JwksPolicy,
}

impl JwksCache {
    #[must_use]
    pub fn new(uri: Url, client: reqwest::Client, policy: JwksPolicy) -> Self {
        Self {
            uri,
            client,
            state: RwLock::new(CacheState {
                keys: HashMap::new(),
                expires_at: None,
            }),
            policy,
        }
    }

    /// The verification key for `kid`, usable with `alg`.
    pub async fn key(&self, kid: &str, alg: Algorithm) -> Result<Arc<DecodingKey>, AuthnError> {
        if let Some(found) = self.cached(kid) {
            return usable(&found, alg);
        }
        let keys = self.fetch().await?;
        let found = keys.get(kid).cloned();
        let mut state = self.state.write().unwrap_or_else(PoisonError::into_inner);
        state.keys = keys;
        state.expires_at = Some(Instant::now() + self.policy.default_ttl);
        drop(state);
        found.map_or(Err(AuthnError::UnknownKey), |found| usable(&found, alg))
    }

    fn cached(&self, kid: &str) -> Option<CachedKey> {
        let state = self.state.read().unwrap_or_else(PoisonError::into_inner);
        let fresh = state.expires_at.is_some_and(|at| Instant::now() < at);
        if fresh {
            state.keys.get(kid).cloned()
        } else {
            None
        }
    }

    async fn fetch(&self) -> Result<HashMap<String, CachedKey>, AuthnError> {
        let unavailable = |reason: &str| AuthnError::JwksUnavailable(reason.to_owned());
        let mut response = self
            .client
            .get(self.uri.clone())
            .send()
            .await
            .map_err(|_| unavailable("the key set request failed"))?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(unavailable("the key set endpoint did not answer 200"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| unavailable("the key set response could not be read"))?
        {
            if body.len() + chunk.len() > self.policy.max_body_bytes {
                return Err(unavailable("the key set response is too large"));
            }
            body.extend_from_slice(&chunk);
        }
        parse_key_set(&body).ok_or_else(|| unavailable("the key set response is not a JWKS"))
    }
}

fn usable(found: &CachedKey, alg: Algorithm) -> Result<Arc<DecodingKey>, AuthnError> {
    if found.key.family() != alg.family() || found.alg.is_some_and(|declared| declared != alg) {
        return Err(AuthnError::UnsupportedAlgorithm);
    }
    Ok(Arc::clone(&found.key))
}

/// Signature keys by `kid`. Keys without a `kid`, meant for encryption, or that cannot be
/// built are skipped; a `kid` that appears twice is ambiguous and dropped.
fn parse_key_set(body: &[u8]) -> Option<HashMap<String, CachedKey>> {
    let document: Value = serde_json::from_slice(body).ok()?;
    let entries = document.get("keys")?.as_array()?;
    let mut keys = HashMap::new();
    let mut ambiguous = Vec::new();
    for entry in entries {
        let Ok(jwk) = serde_json::from_value::<Jwk>(entry.clone()) else {
            continue;
        };
        if matches!(&jwk.common.public_key_use, Some(usage) if *usage != PublicKeyUse::Signature) {
            continue;
        }
        let Some(kid) = jwk.common.key_id.clone() else {
            continue;
        };
        let Ok(key) = DecodingKey::try_from(&jwk) else {
            continue;
        };
        let alg = jwk
            .common
            .key_algorithm
            .and_then(|declared| declared.to_string().parse::<Algorithm>().ok());
        if keys
            .insert(
                kid.clone(),
                CachedKey {
                    key: Arc::new(key),
                    alg,
                },
            )
            .is_some()
        {
            ambiguous.push(kid);
        }
    }
    for kid in ambiguous {
        keys.remove(&kid);
    }
    Some(keys)
}
