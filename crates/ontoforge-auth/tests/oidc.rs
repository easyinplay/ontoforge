//! OIDC authentication against a local mock identity provider (wiremock serves the JWKS).
//!
//! The signing keys under `tests/data/` are test-only keys; see `tests/data/README.md`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use camino::Utf8Path;
use jsonwebtoken::jwk::Jwk;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, get_current_timestamp};
use ontoforge_auth::authn::jwks::{Clock, JwksCache, JwksPolicy};
use ontoforge_auth::{Authenticator, AuthnError, Principal};
use ontoforge_engine::config::AuthConfig;
use ontoforge_expr::rowpred::{AttrValue, Scalar};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const RS256_PEM: &[u8] = include_bytes!("data/oidc_rs256.pem");
const ES256_PEM: &[u8] = include_bytes!("data/oidc_es256.pem");

const AUDIENCE: &str = "ontoforge";

/// A signing key plus the JWK that publishes its public half.
struct TestKey {
    kid: String,
    alg: Algorithm,
    encoding: EncodingKey,
}

impl TestKey {
    fn rs256(kid: &str) -> Self {
        Self {
            kid: kid.to_owned(),
            alg: Algorithm::RS256,
            encoding: EncodingKey::from_rsa_pem(RS256_PEM).expect("test rsa key"),
        }
    }

    fn es256(kid: &str) -> Self {
        Self {
            kid: kid.to_owned(),
            alg: Algorithm::ES256,
            encoding: EncodingKey::from_ec_pem(ES256_PEM).expect("test ec key"),
        }
    }

    fn jwk_typed(&self) -> Jwk {
        let mut jwk = Jwk::from_encoding_key(&self.encoding, self.alg).expect("jwk");
        jwk.common.key_id = Some(self.kid.clone());
        jwk
    }

    fn jwk(&self) -> Value {
        serde_json::to_value(self.jwk_typed()).expect("jwk json")
    }

    fn header(&self) -> Header {
        Header {
            kid: Some(self.kid.clone()),
            ..Header::new(self.alg)
        }
    }

    fn sign(&self, claims: &Value) -> String {
        encode(&self.header(), claims, &self.encoding).expect("sign")
    }
}

/// A clock the test moves by hand.
struct TestClock {
    base: Instant,
    offset: Mutex<Duration>,
}

impl TestClock {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            base: Instant::now(),
            offset: Mutex::new(Duration::ZERO),
        })
    }

    fn advance(&self, by: Duration) {
        *self.offset.lock().expect("clock lock") += by;
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().expect("clock lock")
    }
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// The issuer URL of the mock identity provider on `server`.
fn issuer_of(server: &MockServer) -> String {
    format!("{}/realm", server.uri())
}

fn jwks_url(server: &MockServer) -> String {
    format!("{}/jwks", server.uri())
}

async fn mount_jwks(server: &MockServer, keys: &[Value]) {
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": keys })))
        .mount(server)
        .await;
}

async fn mount_jwks_with(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(response)
        .mount(server)
        .await;
}

/// Requests for the key set since the last `reset`.
async fn jwks_requests(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .expect("request recording is on")
        .iter()
        .filter(|request| request.url.path() == "/jwks")
        .count()
}

/// `[auth]` with one entry per `(issuer, jwks_uri)` and the two attributes of the tracer.
fn auth_config(issuers: &[(String, String)]) -> AuthConfig {
    let oidc: Vec<Value> = issuers
        .iter()
        .map(|(issuer, jwks_uri)| {
            json!({ "issuer": issuer, "audience": AUDIENCE, "jwks_uri": jwks_uri })
        })
        .collect();
    serde_json::from_value(json!({
        "oidc": oidc,
        "principal_attrs": {
            "tenantId": { "claim": "tenant", "type": "string" },
            "homeAirports": { "claim": "airports", "type": "string_list" },
        },
    }))
    .expect("auth config")
}

fn authenticator_for(server: &MockServer) -> Authenticator {
    let config = auth_config(&[(issuer_of(server), jwks_url(server))]);
    Authenticator::from_config(&config, Utf8Path::new(".")).expect("authenticator")
}

fn base_claims(issuer: &str) -> Value {
    json!({
        "iss": issuer,
        "aud": AUDIENCE,
        "sub": "alice",
        "exp": get_current_timestamp() + 300,
        "groups": ["dispatchers"],
        "tenant": "acme",
        "airports": ["FRA", "MUC"],
    })
}

fn without(mut claims: Value, name: &str) -> Value {
    claims.as_object_mut().expect("object").remove(name);
    claims
}

fn cache_for(server: &MockServer, clock: &Arc<TestClock>) -> JwksCache {
    JwksCache::with_clock(
        reqwest::Url::parse(&jwks_url(server)).expect("url"),
        reqwest::Client::new(),
        JwksPolicy::default(),
        Arc::clone(clock) as Arc<dyn Clock>,
    )
}

/// An error text names the kind of failure only: no token, no claim value.
fn assert_clean(error: &AuthnError, token: &str, secrets: &[&str]) {
    let text = format!("{error} / {error:?}");
    assert!(
        token.is_empty() || !text.contains(token),
        "token leaked into: {text}"
    );
    for secret in secrets {
        assert!(!text.contains(secret), "`{secret}` leaked into: {text}");
    }
}

/// Authenticates `token` and asserts the rejection satisfies `check`.
async fn assert_rejected(
    authn: &Authenticator,
    token: &str,
    check: impl FnOnce(&AuthnError) -> bool,
) {
    let error = authn
        .authenticate(token)
        .await
        .expect_err("the token must be rejected");
    assert!(check(&error), "unexpected rejection: {error:?}");
    assert_clean(&error, token, &["acme", "alice", "FRA", "dispatchers"]);
}

fn b64url(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let group = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..=chunk.len() {
            let index = usize::try_from((group >> (18 - 6 * i)) & 63).expect("six bits");
            out.push(char::from(TABLE[index]));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// Tracer
// ---------------------------------------------------------------------------------------

#[tokio::test]
async fn rs256_token_becomes_principal() {
    let server = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks(&server, &[key.jwk()]).await;
    let authn = authenticator_for(&server);

    let token = key.sign(&base_claims(&issuer_of(&server)));
    let principal: Principal = authn.authenticate(&token).await.expect("authenticated");

    assert_eq!(
        principal.id.as_str(),
        format!("{}|alice", issuer_of(&server))
    );
    assert_eq!(principal.issuer, issuer_of(&server));
    assert_eq!(principal.subject, "alice");
    assert_eq!(
        principal
            .groups
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["dispatchers"]
    );
    assert_eq!(
        principal.attrs.get("tenantId"),
        &AttrValue::Scalar(Scalar::String("acme".to_owned()))
    );
    assert_eq!(
        principal.attrs.get("homeAirports"),
        &AttrValue::List(vec![
            Scalar::String("FRA".to_owned()),
            Scalar::String("MUC".to_owned()),
        ])
    );
}

#[tokio::test]
async fn es256_token_becomes_principal() {
    let server = MockServer::start().await;
    let key = TestKey::es256("ec1");
    mount_jwks(&server, &[key.jwk()]).await;
    let authn = authenticator_for(&server);

    let token = key.sign(&base_claims(&issuer_of(&server)));
    let principal = authn.authenticate(&token).await.expect("authenticated");

    assert_eq!(principal.subject, "alice");
}

#[tokio::test]
async fn missing_attr_claim_is_absent() {
    let server = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks(&server, &[key.jwk()]).await;
    let authn = authenticator_for(&server);

    let claims = without(base_claims(&issuer_of(&server)), "tenant");
    let principal = authn
        .authenticate(&key.sign(&claims))
        .await
        .expect("authenticated");

    assert_eq!(principal.attrs.get("tenantId"), &AttrValue::Null);
}

#[tokio::test]
async fn null_attr_claim_is_absent() {
    let server = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks(&server, &[key.jwk()]).await;
    let authn = authenticator_for(&server);

    let mut claims = base_claims(&issuer_of(&server));
    claims["tenant"] = Value::Null;
    let principal = authn
        .authenticate(&key.sign(&claims))
        .await
        .expect("authenticated");

    assert_eq!(principal.attrs.get("tenantId"), &AttrValue::Null);
}

#[tokio::test]
async fn jwks_is_fetched_once_for_repeated_tokens() {
    let server = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks(&server, &[key.jwk()]).await;
    let authn = authenticator_for(&server);

    for _ in 0..3 {
        let token = key.sign(&base_claims(&issuer_of(&server)));
        authn.authenticate(&token).await.expect("authenticated");
    }

    assert_eq!(jwks_requests(&server).await, 1);
}

// ---------------------------------------------------------------------------------------
// JWKS cache policy
// ---------------------------------------------------------------------------------------

#[tokio::test]
async fn cache_control_max_age_is_clamped() {
    let key = TestKey::rs256("k1");

    // max-age=5 is raised to the 60 s floor.
    let server = MockServer::start().await;
    mount_jwks_with(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(json!({ "keys": [key.jwk()] }))
            .insert_header("Cache-Control", "max-age=5"),
    )
    .await;
    let clock = TestClock::new();
    let cache = cache_for(&server, &clock);
    cache.key("k1", Algorithm::RS256).await.expect("key");
    clock.advance(secs(59));
    cache.key("k1", Algorithm::RS256).await.expect("key");
    assert_eq!(jwks_requests(&server).await, 1, "still cached at 59 s");
    clock.advance(secs(2));
    cache.key("k1", Algorithm::RS256).await.expect("key");
    assert_eq!(jwks_requests(&server).await, 2, "refetched after 60 s");

    // max-age=999999 is lowered to the 24 h ceiling.
    let server = MockServer::start().await;
    mount_jwks_with(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(json!({ "keys": [key.jwk()] }))
            .insert_header("Cache-Control", "public, max-age=999999"),
    )
    .await;
    let clock = TestClock::new();
    let cache = cache_for(&server, &clock);
    cache.key("k1", Algorithm::RS256).await.expect("key");
    clock.advance(secs(24 * 3600 - 1));
    cache.key("k1", Algorithm::RS256).await.expect("key");
    assert_eq!(jwks_requests(&server).await, 1, "still cached before 24 h");
    clock.advance(secs(2));
    cache.key("k1", Algorithm::RS256).await.expect("key");
    assert_eq!(jwks_requests(&server).await, 2, "refetched after 24 h");
}

#[tokio::test]
async fn kid_miss_refetches_once_then_throttles() {
    let old = TestKey::rs256("k1");
    let rotated = TestKey::es256("k2");
    let server = MockServer::start().await;
    mount_jwks(&server, &[old.jwk()]).await;
    let clock = TestClock::new();
    let cache = cache_for(&server, &clock);
    cache.key("k1", Algorithm::RS256).await.expect("k1");

    // The identity provider rotates: k2 appears.
    server.reset().await;
    mount_jwks(&server, &[old.jwk(), rotated.jwk()]).await;
    clock.advance(secs(31));
    cache
        .key("k2", Algorithm::ES256)
        .await
        .expect("k2 after refetch");
    assert_eq!(jwks_requests(&server).await, 1);

    // A flood of random kids does not reach the identity provider.
    for i in 0..20 {
        let result = cache.key(&format!("random-{i}"), Algorithm::RS256).await;
        assert!(matches!(result, Err(AuthnError::UnknownKey)));
    }
    assert_eq!(
        jwks_requests(&server).await,
        1,
        "throttled inside the window"
    );

    // Throttled, not blocked: one more fetch is allowed after the window.
    clock.advance(secs(31));
    let result = cache.key("random-late", Algorithm::RS256).await;
    assert!(matches!(result, Err(AuthnError::UnknownKey)));
    assert_eq!(jwks_requests(&server).await, 2);
    // The same unknown kid is remembered.
    let result = cache.key("random-late", Algorithm::RS256).await;
    assert!(matches!(result, Err(AuthnError::UnknownKey)));
    assert_eq!(jwks_requests(&server).await, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_kid_miss_is_single_flight() {
    let server = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks_with(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(json!({ "keys": [key.jwk()] }))
            .set_delay(Duration::from_millis(250)),
    )
    .await;
    let authn = Arc::new(authenticator_for(&server));
    let token = key.sign(&base_claims(&issuer_of(&server)));

    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..10 {
        let authn = Arc::clone(&authn);
        let token = token.clone();
        tasks.spawn(async move { authn.authenticate(&token).await });
    }
    while let Some(joined) = tasks.join_next().await {
        joined.expect("task").expect("authenticated");
    }

    assert_eq!(jwks_requests(&server).await, 1);
}

#[tokio::test]
async fn stale_keys_survive_idp_outage() {
    let key = TestKey::rs256("k1");
    let server = MockServer::start().await;
    mount_jwks(&server, &[key.jwk()]).await;
    let clock = TestClock::new();
    let cache = cache_for(&server, &clock);
    cache.key("k1", Algorithm::RS256).await.expect("key");

    // The identity provider goes down once the default 1 h TTL has passed.
    server.reset().await;
    mount_jwks_with(&server, ResponseTemplate::new(503)).await;
    clock.advance(secs(3601));
    cache
        .key("k1", Algorithm::RS256)
        .await
        .expect("served from the stale set");
    assert_eq!(jwks_requests(&server).await, 1, "one attempt was made");
    // The failing endpoint is not hammered.
    cache.key("k1", Algorithm::RS256).await.expect("stale");
    assert_eq!(jwks_requests(&server).await, 1);

    // Past jwks_max_stale (24 h since the last good fetch) the cache fails closed.
    clock.advance(secs(24 * 3600));
    let result = cache.key("k1", Algorithm::RS256).await;
    assert!(matches!(result, Err(AuthnError::JwksUnavailable(_))));

    // Recovery: a good answer is used again once the throttle window has passed.
    server.reset().await;
    mount_jwks(&server, &[key.jwk()]).await;
    clock.advance(secs(31));
    cache.key("k1", Algorithm::RS256).await.expect("recovered");
}

/// A key set document of exactly `len` bytes.
fn jwks_body_of_len(keys: &[Value], len: usize) -> Vec<u8> {
    let bare = serde_json::to_string(&json!({ "keys": keys, "pad": "" })).expect("json");
    let mut body = serde_json::to_string(&json!({
        "keys": keys,
        "pad": "a".repeat(len - bare.len()),
    }))
    .expect("json")
    .into_bytes();
    body.truncate(len);
    assert_eq!(body.len(), len);
    body
}

#[tokio::test]
async fn oversized_jwks_rejected() {
    let key = TestKey::rs256("k1");
    let server = MockServer::start().await;
    mount_jwks_with(
        &server,
        ResponseTemplate::new(200)
            .set_body_bytes(jwks_body_of_len(&[key.jwk()], 1024 * 1024 + 1))
            .insert_header("Content-Type", "application/json"),
    )
    .await;
    let clock = TestClock::new();
    let cache = cache_for(&server, &clock);

    let result = cache.key("k1", Algorithm::RS256).await;
    assert!(matches!(result, Err(AuthnError::JwksUnavailable(_))));

    // Nothing was cached: once the endpoint serves a sane document the key is found.
    server.reset().await;
    mount_jwks(&server, &[key.jwk()]).await;
    clock.advance(secs(31));
    cache.key("k1", Algorithm::RS256).await.expect("key");
}

#[tokio::test]
async fn jwks_of_exactly_one_mebibyte_is_accepted() {
    let key = TestKey::rs256("k1");
    let server = MockServer::start().await;
    mount_jwks_with(
        &server,
        ResponseTemplate::new(200)
            .set_body_bytes(jwks_body_of_len(&[key.jwk()], 1024 * 1024))
            .insert_header("Content-Type", "application/json"),
    )
    .await;
    let cache = cache_for(&server, &TestClock::new());

    cache.key("k1", Algorithm::RS256).await.expect("key");
}

#[tokio::test]
async fn ambiguous_and_non_signature_keys_are_not_used() {
    let key = TestKey::rs256("dup");
    let mut encryption = TestKey::rs256("enc").jwk();
    encryption["use"] = json!("enc");
    let server = MockServer::start().await;
    mount_jwks(&server, &[key.jwk(), key.jwk(), encryption]).await;
    let cache = cache_for(&server, &TestClock::new());

    let duplicate = cache.key("dup", Algorithm::RS256).await;
    assert!(matches!(duplicate, Err(AuthnError::UnknownKey)));
    let encryption = cache.key("enc", Algorithm::RS256).await;
    assert!(matches!(encryption, Err(AuthnError::UnknownKey)));
}

// ---------------------------------------------------------------------------------------
// Rejection matrix
// ---------------------------------------------------------------------------------------

/// An authenticator whose only issuer publishes `keys`.
async fn authn_with(keys: &[Value]) -> (MockServer, Authenticator) {
    let server = MockServer::start().await;
    mount_jwks(&server, keys).await;
    let authn = authenticator_for(&server);
    (server, authn)
}

#[tokio::test]
async fn expired_token_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let mut claims = base_claims(&issuer_of(&server));
    claims["exp"] = json!(get_current_timestamp() - 300);

    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::Expired)
    })
    .await;
}

#[tokio::test]
async fn leeway_is_honoured_and_then_used_up() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let mut claims = base_claims(&issuer_of(&server));

    // The default leeway is 30 s.
    claims["exp"] = json!(get_current_timestamp() - 20);
    authn
        .authenticate(&key.sign(&claims))
        .await
        .expect("inside the leeway");
    claims["exp"] = json!(get_current_timestamp() - 40);
    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::Expired)
    })
    .await;
}

#[tokio::test]
async fn not_yet_valid_token_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let mut claims = base_claims(&issuer_of(&server));
    claims["nbf"] = json!(get_current_timestamp() + 600);

    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::InvalidClaims(_))
    })
    .await;
}

#[tokio::test]
async fn wrong_issuer_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let mut claims = base_claims(&issuer_of(&server));
    claims["iss"] = json!("https://evil.example/realm");

    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::UnknownIssuer)
    })
    .await;
}

#[tokio::test]
async fn wrong_audience_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let mut claims = base_claims(&issuer_of(&server));
    claims["aud"] = json!("some-other-service");

    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::InvalidClaims(_))
    })
    .await;
}

#[tokio::test]
async fn missing_required_claims_are_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    for name in ["sub", "exp", "aud", "iss"] {
        let claims = without(base_claims(&issuer_of(&server)), name);
        assert_rejected(&authn, &key.sign(&claims), |e| {
            matches!(e, AuthnError::InvalidClaims(_) | AuthnError::UnknownIssuer)
        })
        .await;
    }
    let mut claims = base_claims(&issuer_of(&server));
    claims["sub"] = json!("");
    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::InvalidClaims(_))
    })
    .await;
}

#[tokio::test]
async fn alg_none_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let header = b64url(br#"{"alg":"none","typ":"JWT","kid":"k1"}"#);
    let payload = b64url(base_claims(&issuer_of(&server)).to_string().as_bytes());

    for token in [
        format!("{header}.{payload}."),
        format!("{header}.{payload}"),
    ] {
        assert_rejected(&authn, &token, |e| {
            matches!(e, AuthnError::Malformed | AuthnError::UnsupportedAlgorithm)
        })
        .await;
    }
    assert_eq!(jwks_requests(&server).await, 0, "no key set was fetched");
}

#[tokio::test]
async fn hs256_with_the_public_key_as_secret_is_rejected() {
    let key = TestKey::rs256("k1");
    let jwk = key.jwk();
    let (server, authn) = authn_with(&[jwk.clone()]).await;
    // The classic algorithm confusion: sign with HMAC, using public key material as secret.
    let secret = jwk["n"].as_str().expect("modulus").as_bytes().to_vec();
    let header = Header {
        kid: Some("k1".to_owned()),
        ..Header::new(Algorithm::HS256)
    };
    let token = encode(
        &header,
        &base_claims(&issuer_of(&server)),
        &EncodingKey::from_secret(&secret),
    )
    .expect("sign");

    assert_rejected(&authn, &token, |e| {
        matches!(e, AuthnError::UnsupportedAlgorithm)
    })
    .await;
    assert_eq!(jwks_requests(&server).await, 0, "no key set was fetched");
}

#[tokio::test]
async fn algorithm_outside_the_issuer_allow_list_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    // Same RSA key, PS256 is not configured for the issuer.
    let header = Header {
        kid: Some("k1".to_owned()),
        ..Header::new(Algorithm::PS256)
    };
    let token = encode(
        &header,
        &base_claims(&issuer_of(&server)),
        &EncodingKey::from_rsa_pem(RS256_PEM).expect("key"),
    )
    .expect("sign");

    assert_rejected(&authn, &token, |e| {
        matches!(e, AuthnError::UnsupportedAlgorithm)
    })
    .await;
    assert_eq!(jwks_requests(&server).await, 0, "no key set was fetched");
}

#[tokio::test]
async fn tampered_token_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let issuer = issuer_of(&server);
    let genuine = key.sign(&base_claims(&issuer));
    let mut forged_claims = base_claims(&issuer);
    forged_claims["sub"] = json!("mallory");
    let forged = key.sign(&forged_claims);

    // Header and payload of the forged token, signature of the genuine one.
    let parts: Vec<&str> = forged.split('.').collect();
    let signature = genuine.rsplit('.').next().expect("signature");
    let token = format!("{}.{}.{}", parts[0], parts[1], signature);

    assert_rejected(&authn, &token, |e| {
        matches!(e, AuthnError::InvalidSignature)
    })
    .await;
}

#[tokio::test]
async fn header_key_urls_are_never_followed() {
    // The attacker publishes a key set and points the token header at it.
    let attacker_key = TestKey::es256("evil");
    let attacker = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "keys": [attacker_key.jwk()] })),
        )
        .mount(&attacker)
        .await;

    let trusted_key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[trusted_key.jwk()]).await;

    let header = Header {
        jku: Some(format!("{}/jwks", attacker.uri())),
        x5u: Some(format!("{}/cert", attacker.uri())),
        jwk: Some(attacker_key.jwk_typed()),
        ..attacker_key.header()
    };
    let token = encode(
        &header,
        &base_claims(&issuer_of(&server)),
        &attacker_key.encoding,
    )
    .expect("sign");

    assert_rejected(&authn, &token, |e| matches!(e, AuthnError::UnknownKey)).await;
    assert_eq!(
        attacker.received_requests().await.expect("recording").len(),
        0,
        "the attacker endpoint must never be contacted"
    );
}

#[tokio::test]
async fn token_without_kid_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    let token = encode(
        &Header::new(Algorithm::RS256),
        &base_claims(&issuer_of(&server)),
        &key.encoding,
    )
    .expect("sign");

    assert_rejected(&authn, &token, |e| matches!(e, AuthnError::Malformed)).await;
}

#[tokio::test]
async fn garbage_is_rejected() {
    let key = TestKey::rs256("k1");
    let (_server, authn) = authn_with(&[key.jwk()]).await;
    for token in ["", "abc", "a.b", "a.b.c", "....", "Bearer x.y.z"] {
        assert_rejected(&authn, token, |e| matches!(e, AuthnError::Malformed)).await;
    }
}

#[tokio::test]
async fn groups_claim_must_be_an_array_of_strings() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    for groups in [
        json!("dispatchers"),
        json!(["ok", 7]),
        json!({"a": 1}),
        Value::Null,
    ] {
        let mut claims = base_claims(&issuer_of(&server));
        claims["groups"] = groups;
        assert_rejected(&authn, &key.sign(&claims), |e| {
            matches!(e, AuthnError::InvalidClaims(_))
        })
        .await;
    }
    // A missing groups claim is an empty group set, not an error.
    let claims = without(base_claims(&issuer_of(&server)), "groups");
    let principal = authn
        .authenticate(&key.sign(&claims))
        .await
        .expect("authenticated");
    assert!(principal.groups.is_empty());
}

#[tokio::test]
async fn list_claim_with_null_or_foreign_element_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    for airports in [
        json!(["FRA", null]),
        json!(["FRA", 3]),
        json!("FRA"),
        json!([["FRA"]]),
    ] {
        let mut claims = base_claims(&issuer_of(&server));
        claims["airports"] = airports;
        assert_rejected(&authn, &key.sign(&claims), |e| {
            matches!(e, AuthnError::InvalidClaims(_))
        })
        .await;
    }
    // The empty list is a value, not an error.
    let mut claims = base_claims(&issuer_of(&server));
    claims["airports"] = json!([]);
    let principal = authn
        .authenticate(&key.sign(&claims))
        .await
        .expect("authenticated");
    assert_eq!(
        principal.attrs.get("homeAirports"),
        &AttrValue::List(vec![])
    );
}

#[tokio::test]
async fn scalar_claim_of_the_wrong_type_is_rejected() {
    let key = TestKey::rs256("k1");
    let (server, authn) = authn_with(&[key.jwk()]).await;
    for tenant in [
        json!(7),
        json!(true),
        json!(["acme"]),
        json!({"id": "acme"}),
    ] {
        let mut claims = base_claims(&issuer_of(&server));
        claims["tenant"] = tenant;
        assert_rejected(&authn, &key.sign(&claims), |e| {
            matches!(e, AuthnError::InvalidClaims(_))
        })
        .await;
    }
}

#[tokio::test]
async fn integer_and_boolean_attrs_are_typed_strictly() {
    let key = TestKey::rs256("k1");
    let server = MockServer::start().await;
    mount_jwks(&server, &[key.jwk()]).await;
    let config: AuthConfig = serde_json::from_value(json!({
        "oidc": [{ "issuer": issuer_of(&server), "audience": AUDIENCE, "jwks_uri": jwks_url(&server) }],
        "principal_attrs": {
            "level": { "claim": "level", "type": "integer" },
            "vip": { "claim": "vip", "type": "boolean" },
            "shifts": { "claim": "shifts", "type": "integer_list" },
        },
    }))
    .expect("config");
    let authn = Authenticator::from_config(&config, Utf8Path::new(".")).expect("authenticator");

    let mut claims = base_claims(&issuer_of(&server));
    claims["level"] = json!(3);
    claims["vip"] = json!(true);
    claims["shifts"] = json!([1, 2]);
    let principal = authn
        .authenticate(&key.sign(&claims))
        .await
        .expect("authenticated");
    assert_eq!(
        principal.attrs.get("level"),
        &AttrValue::Scalar(Scalar::Int(3))
    );
    assert_eq!(
        principal.attrs.get("vip"),
        &AttrValue::Scalar(Scalar::Bool(true))
    );
    assert_eq!(
        principal.attrs.get("shifts"),
        &AttrValue::List(vec![Scalar::Int(1), Scalar::Int(2)])
    );

    for (claim, bad) in [
        ("level", json!(3.5)),
        ("level", json!("3")),
        ("level", json!(18_446_744_073_709_551_615_u64)),
        ("vip", json!("true")),
        ("shifts", json!([1, null])),
        ("shifts", json!([1.5])),
    ] {
        let mut claims = base_claims(&issuer_of(&server));
        claims[claim] = bad;
        assert_rejected(&authn, &key.sign(&claims), |e| {
            matches!(e, AuthnError::InvalidClaims(_))
        })
        .await;
    }
}

// ---------------------------------------------------------------------------------------
// Several issuers
// ---------------------------------------------------------------------------------------

#[tokio::test]
async fn two_issuers_namespace_groups() {
    let server_a = MockServer::start().await;
    let server_b = MockServer::start().await;
    let key_a = TestKey::rs256("k1");
    // Both identity providers use the kid `k1`; B's key is a different (EC) key.
    let key_b = TestKey::es256("k1");
    mount_jwks(&server_a, &[key_a.jwk()]).await;
    mount_jwks(&server_b, &[key_b.jwk()]).await;
    let config = auth_config(&[
        (issuer_of(&server_a), jwks_url(&server_a)),
        (issuer_of(&server_b), jwks_url(&server_b)),
    ]);
    let authn = Authenticator::from_config(&config, Utf8Path::new(".")).expect("authenticator");

    let mut claims_a = base_claims(&issuer_of(&server_a));
    claims_a["groups"] = json!(["ops"]);
    let principal = authn
        .authenticate(&key_a.sign(&claims_a))
        .await
        .expect("issuer A");
    assert_eq!(
        principal.groups.iter().cloned().collect::<Vec<_>>(),
        [format!("{}|ops", issuer_of(&server_a))]
    );

    let mut claims_b = base_claims(&issuer_of(&server_b));
    claims_b["groups"] = json!(["ops"]);
    let principal = authn
        .authenticate(&key_b.sign(&claims_b))
        .await
        .expect("issuer B");
    assert_eq!(
        principal.groups.iter().cloned().collect::<Vec<_>>(),
        [format!("{}|ops", issuer_of(&server_b))]
    );
    assert_eq!(
        principal.id.as_str(),
        format!("{}|alice", issuer_of(&server_b))
    );

    // A token that claims issuer B but is signed with A's key must not pass.
    let forged = key_a.sign(&claims_b);
    assert_rejected(&authn, &forged, |e| {
        matches!(e, AuthnError::InvalidSignature | AuthnError::UnknownKey)
    })
    .await;
}

#[tokio::test]
async fn unknown_issuer_is_rejected_when_several_are_trusted() {
    let server_a = MockServer::start().await;
    let server_b = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks(&server_a, &[key.jwk()]).await;
    mount_jwks(&server_b, &[key.jwk()]).await;
    let config = auth_config(&[
        (issuer_of(&server_a), jwks_url(&server_a)),
        (issuer_of(&server_b), jwks_url(&server_b)),
    ]);
    let authn = Authenticator::from_config(&config, Utf8Path::new(".")).expect("authenticator");

    let claims = base_claims("https://third.example/realm");
    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::UnknownIssuer)
    })
    .await;
    let claims = without(base_claims(&issuer_of(&server_a)), "iss");
    assert_rejected(&authn, &key.sign(&claims), |e| {
        matches!(e, AuthnError::UnknownIssuer)
    })
    .await;
    assert_eq!(jwks_requests(&server_a).await, 0);
    assert_eq!(jwks_requests(&server_b).await, 0);
}
