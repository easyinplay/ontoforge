//! JWKS fetching and caching (ADR-0008: own cache, no extra crate).
//!
//! The key set address only ever comes from `ontoforge-server.toml`. A token header can
//! never point this module somewhere else.
//!
//! Policy, in order of what an attacker or a flaky identity provider can cause:
//!
//! - The cache lives for the response's `Cache-Control: max-age`, clamped to
//!   `[min_ttl, max_ttl]`, or `default_ttl` without one.
//! - A `kid` that is not in the cache triggers one fetch, shared by all waiting callers
//!   (single flight). Fetches are at least `min_refresh` apart, so random `kid`s cannot
//!   turn this service into a request amplifier; unknown `kid`s are also remembered.
//! - When a fetch fails the previous key set keeps being used until `max_stale` has passed
//!   since it was fetched; after that the cache fails closed.
//! - A response is accepted only with status 200 and at most `max_body_bytes`; a fetch
//!   takes at most `timeout`.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::{Jwk, PublicKeyUse};
use jsonwebtoken::{Algorithm, DecodingKey};
use reqwest::Url;
use reqwest::header::CACHE_CONTROL;
use serde_json::Value;

use super::AuthnError;

/// Most unknown `kid`s remembered at once.
const MAX_NEGATIVE_ENTRIES: usize = 256;

/// Cache and fetch limits for one key set.
#[derive(Debug, Clone)]
pub struct JwksPolicy {
    /// Lifetime when the response carries no usable `Cache-Control: max-age`.
    pub default_ttl: Duration,
    pub min_ttl: Duration,
    pub max_ttl: Duration,
    /// Smallest gap between two fetch attempts.
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

/// Source of monotonic time. Production uses [`SystemClock`]; tests move time by hand.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

/// The system monotonic clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Clone)]
struct CachedKey {
    key: Arc<DecodingKey>,
    /// The `alg` the key set declares for this key, when it does.
    alg: Option<Algorithm>,
}

#[derive(Default)]
struct CacheState {
    keys: HashMap<String, CachedKey>,
    /// When the current key set was fetched; `None` before the first success.
    fetched_at: Option<Instant>,
    expires_at: Option<Instant>,
    /// The last fetch attempt, successful or not.
    last_attempt: Option<Instant>,
    /// Unknown `kid`s and when to forget them.
    negative: HashMap<String, Instant>,
}

/// What to do for one lookup.
enum Decision {
    Use(CachedKey),
    Unknown,
    Unavailable(&'static str),
    Fetch,
}

/// The signing keys of one identity provider.
pub struct JwksCache {
    uri: Url,
    client: reqwest::Client,
    state: RwLock<CacheState>,
    /// Held while a fetch is in flight; waiters re-check the cache after getting it.
    flight: tokio::sync::Mutex<()>,
    policy: JwksPolicy,
    clock: Arc<dyn Clock>,
}

impl JwksCache {
    #[must_use]
    pub fn new(uri: Url, client: reqwest::Client, policy: JwksPolicy) -> Self {
        Self::with_clock(uri, client, policy, Arc::new(SystemClock))
    }

    #[must_use]
    pub fn with_clock(
        uri: Url,
        client: reqwest::Client,
        policy: JwksPolicy,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            uri,
            client,
            state: RwLock::new(CacheState::default()),
            flight: tokio::sync::Mutex::new(()),
            policy,
            clock,
        }
    }

    /// The verification key for `kid`, usable with `alg`.
    pub async fn key(&self, kid: &str, alg: Algorithm) -> Result<Arc<DecodingKey>, AuthnError> {
        let found = match self.decide(kid) {
            Decision::Fetch => self.fetch_once(kid).await?,
            other => resolve(other)?,
        };
        usable(&found, alg)
    }

    async fn fetch_once(&self, kid: &str) -> Result<CachedKey, AuthnError> {
        let _flight = self.flight.lock().await;
        // Another caller may have fetched while this one waited.
        let decision = self.decide(kid);
        if !matches!(decision, Decision::Fetch) {
            return resolve(decision);
        }
        let attempt = self.clock.now();
        let outcome = self.fetch().await;
        let now = self.clock.now();
        let mut state = self.state.write().unwrap_or_else(PoisonError::into_inner);
        state.last_attempt = Some(attempt);
        match outcome {
            Ok((keys, ttl)) => {
                state.keys = keys;
                state.fetched_at = Some(now);
                state.expires_at = Some(now + ttl);
                state.negative.clear();
                let found = state.keys.get(kid).cloned();
                if found.is_none() {
                    self.remember_unknown(&mut state, kid, now);
                }
                found.ok_or(AuthnError::UnknownKey)
            }
            Err(reason) => {
                // Keep the previous key set while it is young enough.
                let young = state
                    .fetched_at
                    .filter(|at| now.saturating_duration_since(*at) < self.policy.max_stale);
                match young {
                    Some(_) => state.keys.get(kid).cloned().ok_or(AuthnError::UnknownKey),
                    None => Err(AuthnError::JwksUnavailable(reason.to_owned())),
                }
            }
        }
    }

    /// Cache lookup rules; see the module documentation.
    fn decide(&self, kid: &str) -> Decision {
        let now = self.clock.now();
        let state = self.state.read().unwrap_or_else(PoisonError::into_inner);
        let throttled = state
            .last_attempt
            .is_some_and(|at| now.saturating_duration_since(at) < self.policy.min_refresh);
        let Some(fetched_at) = state.fetched_at else {
            // Nothing cached yet. After a failed attempt wait out the window.
            return if throttled {
                Decision::Unavailable("the last key set fetch failed")
            } else {
                Decision::Fetch
            };
        };
        let fresh = state.expires_at.is_some_and(|at| now < at);
        let young = now.saturating_duration_since(fetched_at) < self.policy.max_stale;
        let known = state.keys.get(kid).cloned();
        if fresh {
            return match known {
                Some(key) => Decision::Use(key),
                None if throttled || state.negative.get(kid).is_some_and(|until| now < *until) => {
                    Decision::Unknown
                }
                None => Decision::Fetch,
            };
        }
        // The TTL has passed.
        if !throttled {
            return Decision::Fetch;
        }
        match known {
            Some(key) if young => Decision::Use(key),
            None if young => Decision::Unknown,
            _ => Decision::Unavailable("the cached key set is too old"),
        }
    }

    fn remember_unknown(&self, state: &mut CacheState, kid: &str, now: Instant) {
        if state.negative.len() >= MAX_NEGATIVE_ENTRIES {
            state.negative.retain(|_, until| now < *until);
        }
        if state.negative.len() < MAX_NEGATIVE_ENTRIES {
            state
                .negative
                .insert(kid.to_owned(), now + self.policy.min_refresh);
        }
    }

    /// One fetch with the total timeout of the policy. The error is a fixed text that
    /// names neither the URL nor anything from the response.
    async fn fetch(&self) -> Result<(HashMap<String, CachedKey>, Duration), &'static str> {
        tokio::time::timeout(self.policy.timeout, self.fetch_unbounded())
            .await
            .unwrap_or(Err("the key set request timed out"))
    }

    async fn fetch_unbounded(
        &self,
    ) -> Result<(HashMap<String, CachedKey>, Duration), &'static str> {
        let mut response = self
            .client
            .get(self.uri.clone())
            .send()
            .await
            .map_err(|_| "the key set request failed")?;
        if response.status() != reqwest::StatusCode::OK {
            return Err("the key set endpoint did not answer 200");
        }
        let ttl = self.ttl_of(
            response
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
        );
        if response
            .content_length()
            .is_some_and(|len| len > self.policy.max_body_bytes as u64)
        {
            return Err("the key set response is too large");
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "the key set response could not be read")?
        {
            if body.len() + chunk.len() > self.policy.max_body_bytes {
                return Err("the key set response is too large");
            }
            body.extend_from_slice(&chunk);
        }
        let keys = parse_key_set(&body).ok_or("the key set response is not a JWKS")?;
        Ok((keys, ttl))
    }

    /// The cache lifetime for a `Cache-Control` header value.
    fn ttl_of(&self, header: Option<&str>) -> Duration {
        header
            .and_then(max_age)
            .map_or(self.policy.default_ttl, |ttl| {
                ttl.clamp(self.policy.min_ttl, self.policy.max_ttl)
            })
    }
}

fn resolve(decision: Decision) -> Result<CachedKey, AuthnError> {
    match decision {
        Decision::Use(key) => Ok(key),
        Decision::Unknown | Decision::Fetch => Err(AuthnError::UnknownKey),
        Decision::Unavailable(reason) => Err(AuthnError::JwksUnavailable(reason.to_owned())),
    }
}

/// The `max-age` directive of a `Cache-Control` value, when it parses.
fn max_age(header: &str) -> Option<Duration> {
    header.split(',').find_map(|directive| {
        let (name, value) = directive.trim().split_once('=')?;
        if !name.trim().eq_ignore_ascii_case("max-age") {
            return None;
        }
        value
            .trim()
            .trim_matches('"')
            .parse::<u64>()
            .ok()
            .map(Duration::from_secs)
    })
}

/// A key that cannot verify with `alg` is the same as no key: a `kid` of another key type
/// or a key that declares another algorithm never validates this token.
fn usable(found: &CachedKey, alg: Algorithm) -> Result<Arc<DecodingKey>, AuthnError> {
    if found.key.family() != alg.family() || found.alg.is_some_and(|declared| declared != alg) {
        return Err(AuthnError::UnknownKey);
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
        let cached = CachedKey {
            key: Arc::new(key),
            alg,
        };
        if keys.insert(kid.clone(), cached).is_some() {
            ambiguous.push(kid);
        }
    }
    for kid in ambiguous {
        keys.remove(&kid);
    }
    Some(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_age_is_read_from_cache_control() {
        assert_eq!(max_age("max-age=5"), Some(Duration::from_secs(5)));
        assert_eq!(
            max_age("public, Max-Age=120, must-revalidate"),
            Some(Duration::from_secs(120))
        );
        assert_eq!(max_age("max-age=\"7\""), Some(Duration::from_secs(7)));
        assert_eq!(max_age("no-store"), None);
        assert_eq!(max_age("max-age=abc"), None);
        assert_eq!(max_age("max-age=-1"), None);
        assert_eq!(max_age("s-maxage=9"), None);
    }

    #[test]
    fn ttl_is_clamped_to_the_policy_range() {
        let cache = JwksCache::new(
            Url::parse("http://127.0.0.1:1/jwks").expect("url"),
            reqwest::Client::new(),
            JwksPolicy::default(),
        );
        assert_eq!(cache.ttl_of(Some("max-age=5")), Duration::from_mins(1));
        assert_eq!(cache.ttl_of(Some("max-age=60")), Duration::from_mins(1));
        assert_eq!(cache.ttl_of(Some("max-age=600")), Duration::from_mins(10));
        assert_eq!(
            cache.ttl_of(Some("max-age=86400")),
            Duration::from_hours(24)
        );
        assert_eq!(
            cache.ttl_of(Some("max-age=999999")),
            Duration::from_hours(24)
        );
        assert_eq!(cache.ttl_of(None), Duration::from_hours(1));
        assert_eq!(cache.ttl_of(Some("no-cache")), Duration::from_hours(1));
    }
}
