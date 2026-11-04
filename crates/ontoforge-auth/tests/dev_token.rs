//! Development token authentication (AUTH-02). Keys are generated per test into a temporary
//! directory; nothing here is a fixed secret.

use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use jsonwebtoken::jwk::Jwk;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, get_current_timestamp};
use ontoforge_auth::authn::dev::{
    DEV_ISSUER, DevClaims, DevKey, DevMode, DevVerifier, issue_dev_token,
};
use ontoforge_auth::{Authenticator, AuthnError};
use ontoforge_engine::config::AuthConfig;
use ontoforge_expr::rowpred::{AttrValue, Scalar};
use serde_json::{Map, Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test-only RS256 key shared with the OIDC tests, see `tests/data/README.md`.
const RS256_PEM: &[u8] = include_bytes!("data/oidc_rs256.pem");

fn auth_config() -> AuthConfig {
    serde_json::from_value(json!({
        "oidc": [{
            "issuer": "https://idp.example.com/realm",
            "audience": "ontoforge",
            "jwks_uri": "https://idp.example.com/realm/jwks",
        }],
        "principal_attrs": {
            "homeAirports": { "claim": "airports", "type": "string_list" },
        },
    }))
    .expect("auth config")
}

fn key_path(dir: &tempfile::TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(dir.path().join("dev.key")).expect("utf-8 path")
}

fn airports() -> Map<String, Value> {
    let mut attrs = Map::new();
    attrs.insert("airports".to_owned(), json!(["FRA", "MUC"]));
    attrs
}

fn alice() -> DevClaims {
    DevClaims {
        sub: "alice".to_owned(),
        groups: vec!["dispatchers".to_owned()],
        attrs: airports(),
        ttl: Duration::from_secs(3600),
    }
}

fn authenticator(key: DevKey) -> Authenticator {
    let config = auth_config();
    let verifier = DevVerifier::new(DevMode::enable_insecure(), key, &config.principal_attrs);
    Authenticator::from_config(&config, Utf8Path::new("."))
        .expect("authenticator")
        .with_dev(verifier)
}

#[tokio::test]
async fn dev_token_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = DevKey::load_or_create(&key_path(&dir)).expect("key");
    let token = issue_dev_token(&key, &alice()).expect("token");
    let authn = authenticator(key);

    let principal = authn.authenticate(&token).await.expect("authenticated");

    assert_eq!(principal.id.as_str(), "ontoforge-dev|alice");
    assert_eq!(principal.issuer, "ontoforge-dev");
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
        principal.attrs.get("homeAirports"),
        &AttrValue::List(vec![
            Scalar::String("FRA".to_owned()),
            Scalar::String("MUC".to_owned()),
        ])
    );
}

#[tokio::test]
async fn key_file_is_created_once_and_reused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = key_path(&dir);
    let first = DevKey::load_or_create(&path).expect("created");
    assert_eq!(std::fs::metadata(&path).expect("key file").len(), 32);
    let token = issue_dev_token(&first, &alice()).expect("token");

    let second = DevKey::load_or_create(&path).expect("reused");

    assert_eq!(std::fs::metadata(&path).expect("key file").len(), 32);
    authenticator(second)
        .authenticate(&token)
        .await
        .expect("a token of the first key verifies under the reloaded key");
}

#[test]
fn missing_key_directory_is_created() {
    let dir = tempfile::tempdir().expect("tempdir");
    let nested = dir.path().join("nested");
    let deeper = nested.join("deeper");
    let path = Utf8PathBuf::from_path_buf(deeper.join("dev.key")).expect("utf-8 path");

    DevKey::load_or_create(&path).expect("created below missing directories");

    let bytes = std::fs::read(&path).expect("key file");
    assert_eq!(bytes.len(), 32);
    DevKey::load_or_create(&path).expect("reused");
    assert_eq!(std::fs::read(&path).expect("key file"), bytes, "same key");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for created in [&nested, &deeper] {
            let mode = std::fs::metadata(created)
                .expect("dir")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o700, "{}", created.display());
        }
    }
}

#[test]
fn key_file_of_wrong_length_is_a_config_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = key_path(&dir);
    std::fs::write(&path, [7_u8; 31]).expect("write");

    let error = DevKey::load_or_create(&path).expect_err("a truncated key is refused");

    assert!(matches!(error, AuthnError::Config(_)), "{error:?}");
    // The file is neither replaced nor extended: silently changing the key would invalidate
    // every token issued so far.
    assert_eq!(std::fs::metadata(&path).expect("key file").len(), 31);
}

// ---------------------------------------------------------------------------------------
// Task 2: configuration cannot enable dev tokens, algorithm routing, token and key limits
// ---------------------------------------------------------------------------------------

const OIDC_ISSUER: &str = "https://idp.example.com/realm";

/// The 32 key bytes of a key file the library created.
fn secret_of(path: &Utf8Path) -> Vec<u8> {
    std::fs::read(path).expect("key file")
}

/// A token with arbitrary `claims`, signed like a dev token but under the caller's control.
fn sign(alg: Algorithm, secret: &[u8], claims: &Value) -> String {
    encode(&Header::new(alg), claims, &EncodingKey::from_secret(secret)).expect("sign")
}

fn dev_claims() -> Value {
    json!({
        "iss": DEV_ISSUER,
        "aud": DEV_ISSUER,
        "sub": "alice",
        "exp": get_current_timestamp() + 300,
        "groups": ["dispatchers"],
        "airports": ["FRA", "MUC"],
    })
}

fn with(mut claims: Value, name: &str, value: Value) -> Value {
    claims[name] = value;
    claims
}

fn without(mut claims: Value, name: &str) -> Value {
    claims.as_object_mut().expect("object").remove(name);
    claims
}

/// An authenticator that trusts `OIDC_ISSUER` and accepts dev tokens of the key at `path`.
fn dev_enabled(path: &Utf8Path) -> Authenticator {
    authenticator(DevKey::load_or_create(path).expect("existing key"))
}

fn assert_clean(error: &AuthnError, token: &str) {
    let text = format!("{error} / {error:?}");
    assert!(!text.contains(token), "token leaked into: {text}");
    for secret in ["alice", "FRA", "dispatchers"] {
        assert!(!text.contains(secret), "`{secret}` leaked into: {text}");
    }
}

/// The dev-enabled authenticator accepts `good` (positive control) and rejects `bad` with the
/// error `check` accepts.
async fn assert_accepts_then_rejects(
    path: &Utf8Path,
    good: &str,
    bad: &str,
    check: impl FnOnce(&AuthnError) -> bool,
) {
    let authn = dev_enabled(path);
    authn.authenticate(good).await.expect("the control token");
    let error = authn.authenticate(bad).await.expect_err("must be rejected");
    assert!(check(&error), "unexpected rejection: {error:?}");
    assert_clean(&error, bad);
}

fn key_in(dir: &tempfile::TempDir) -> (Utf8PathBuf, Vec<u8>) {
    let path = key_path(dir);
    DevKey::load_or_create(&path).expect("key");
    let secret = secret_of(&path);
    (path, secret)
}

/// A mock identity provider that publishes the test RS256 key as `k1`, the `[auth]` that trusts
/// it, and a signer for its tokens.
struct Idp {
    _server: MockServer,
    issuer: String,
    config: AuthConfig,
    encoding: EncodingKey,
}

impl Idp {
    async fn start() -> Self {
        let server = MockServer::start().await;
        let issuer = format!("{}/realm", server.uri());
        let encoding = EncodingKey::from_rsa_pem(RS256_PEM).expect("test rsa key");
        let mut jwk = Jwk::from_encoding_key(&encoding, Algorithm::RS256).expect("jwk");
        jwk.common.key_id = Some("k1".to_owned());
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": [jwk] })))
            .mount(&server)
            .await;
        let config: AuthConfig = serde_json::from_value(json!({
            "oidc": [{
                "issuer": issuer,
                "audience": "ontoforge",
                "jwks_uri": format!("{}/jwks", server.uri()),
            }],
            "principal_attrs": {
                "homeAirports": { "claim": "airports", "type": "string_list" },
            },
        }))
        .expect("auth config");
        Self {
            _server: server,
            issuer,
            config,
            encoding,
        }
    }

    fn token(&self, iss: &str, sub: &str) -> String {
        let header = Header {
            kid: Some("k1".to_owned()),
            ..Header::new(Algorithm::RS256)
        };
        let claims = json!({
            "iss": iss,
            "aud": "ontoforge",
            "sub": sub,
            "exp": get_current_timestamp() + 300,
        });
        encode(&header, &claims, &self.encoding).expect("sign")
    }

    fn authenticator(&self, key: DevKey) -> Authenticator {
        let verifier = DevVerifier::new(
            DevMode::enable_insecure(),
            key,
            &self.config.principal_attrs,
        );
        Authenticator::from_config(&self.config, Utf8Path::new("."))
            .expect("authenticator")
            .with_dev(verifier)
    }
}

#[tokio::test]
async fn config_alone_never_enables_dev_tokens() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = key_path(&dir);
    let key = DevKey::load_or_create(&path).expect("key");
    let token = issue_dev_token(&key, &alice()).expect("token");
    // The control: the very same token is accepted once the caller opts in.
    dev_enabled(&path)
        .authenticate(&token)
        .await
        .expect("accepted after an explicit opt in");

    let config: AuthConfig = serde_json::from_value(json!({
        "oidc": [{
            "issuer": OIDC_ISSUER,
            "audience": "ontoforge",
            "jwks_uri": "https://idp.example.com/realm/jwks",
        }],
        "principal_attrs": {
            "homeAirports": { "claim": "airports", "type": "string_list" },
        },
        "dev": { "key_file": path.as_str() },
    }))
    .expect("auth config with [auth.dev]");
    assert!(config.dev.is_some());
    let authn = Authenticator::from_config(&config, Utf8Path::new(".")).expect("authenticator");

    let error = authn
        .authenticate(&token)
        .await
        .expect_err("a configuration alone must not turn dev tokens on");

    assert!(
        matches!(error, AuthnError::UnsupportedAlgorithm),
        "{error:?}"
    );
}

#[tokio::test]
async fn rs256_token_with_dev_issuer_is_rejected() {
    let idp = Idp::start().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let authn = idp.authenticator(DevKey::load_or_create(&key_path(&dir)).expect("key"));
    authn
        .authenticate(&idp.token(&idp.issuer, "alice"))
        .await
        .expect("the control: the real issuer is accepted");

    let token = idp.token(DEV_ISSUER, "alice");
    let error = authn
        .authenticate(&token)
        .await
        .expect_err("an asymmetric token never takes the dev route");

    assert!(matches!(error, AuthnError::UnknownIssuer), "{error:?}");
    assert_clean(&error, &token);
}

#[tokio::test]
async fn oidc_still_works_with_dev_enabled() {
    let idp = Idp::start().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let key = DevKey::load_or_create(&key_path(&dir)).expect("key");
    let dev_token = issue_dev_token(&key, &alice()).expect("token");
    let authn = idp.authenticator(key);

    let oidc = authn
        .authenticate(&idp.token(&idp.issuer, "bob"))
        .await
        .expect("oidc token");
    let dev = authn.authenticate(&dev_token).await.expect("dev token");

    assert_eq!(oidc.id.as_str(), format!("{}|bob", idp.issuer));
    assert_eq!(dev.id.as_str(), "ontoforge-dev|alice");
}

#[tokio::test]
async fn hs256_with_other_issuer_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, secret) = key_in(&dir);
    let good = sign(Algorithm::HS256, &secret, &dev_claims());
    let bad = sign(
        Algorithm::HS256,
        &secret,
        &with(dev_claims(), "iss", json!("https://idp.example.com")),
    );

    assert_accepts_then_rejects(&path, &good, &bad, |e| {
        matches!(e, AuthnError::UnknownIssuer)
    })
    .await;
}

#[tokio::test]
async fn hs256_with_other_audience_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, secret) = key_in(&dir);
    let good = sign(Algorithm::HS256, &secret, &dev_claims());
    let bad = sign(
        Algorithm::HS256,
        &secret,
        &with(dev_claims(), "aud", json!("ontoforge")),
    );

    assert_accepts_then_rejects(&path, &good, &bad, |e| {
        matches!(e, AuthnError::InvalidClaims(_))
    })
    .await;
}

#[tokio::test]
async fn wrong_key_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, secret) = key_in(&dir);
    let good = sign(Algorithm::HS256, &secret, &dev_claims());
    let bad = sign(Algorithm::HS256, &[9_u8; 32], &dev_claims());

    assert_accepts_then_rejects(&path, &good, &bad, |e| {
        matches!(e, AuthnError::InvalidSignature)
    })
    .await;
}

#[tokio::test]
async fn expired_dev_token_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, secret) = key_in(&dir);
    let now = get_current_timestamp();
    // 10 s past exp is inside the 30 s leeway, 120 s is outside.
    let good = sign(
        Algorithm::HS256,
        &secret,
        &with(dev_claims(), "exp", json!(now - 10)),
    );
    let bad = sign(
        Algorithm::HS256,
        &secret,
        &with(dev_claims(), "exp", json!(now - 120)),
    );

    assert_accepts_then_rejects(&path, &good, &bad, |e| matches!(e, AuthnError::Expired)).await;
}

#[tokio::test]
async fn missing_sub_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, secret) = key_in(&dir);
    let good = sign(Algorithm::HS256, &secret, &dev_claims());
    let bad = sign(Algorithm::HS256, &secret, &without(dev_claims(), "sub"));

    assert_accepts_then_rejects(&path, &good, &bad, |e| {
        matches!(e, AuthnError::InvalidClaims(_))
    })
    .await;
}

#[tokio::test]
async fn other_hmac_algorithms_are_not_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, secret) = key_in(&dir);
    let good = sign(Algorithm::HS256, &secret, &dev_claims());
    let bad = sign(Algorithm::HS512, &secret, &dev_claims());

    assert_accepts_then_rejects(&path, &good, &bad, |e| {
        matches!(e, AuthnError::UnsupportedAlgorithm)
    })
    .await;
}

#[tokio::test]
async fn list_attr_with_a_null_element_is_rejected() {
    // ADR-0025 rule 2 holds for dev tokens as for OIDC tokens.
    let dir = tempfile::tempdir().expect("tempdir");
    let (path, secret) = key_in(&dir);
    let good = sign(Algorithm::HS256, &secret, &dev_claims());
    let bad = sign(
        Algorithm::HS256,
        &secret,
        &with(dev_claims(), "airports", json!(["FRA", null])),
    );

    assert_accepts_then_rejects(&path, &good, &bad, |e| {
        matches!(e, AuthnError::InvalidClaims(_))
    })
    .await;
}

#[test]
fn issuing_refuses_a_claim_that_would_override_a_reserved_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = DevKey::load_or_create(&key_path(&dir)).expect("key");
    for reserved in ["iss", "aud", "sub", "exp", "iat", "nbf", "groups"] {
        let mut claims = alice();
        claims.attrs.insert(reserved.to_owned(), json!("x"));

        let error = issue_dev_token(&key, &claims)
            .expect_err(&format!("attribute claim `{reserved}` must be refused"));

        assert!(matches!(error, AuthnError::InvalidClaims(_)), "{error:?}");
    }
}

#[test]
fn issuing_refuses_an_empty_subject() {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = DevKey::load_or_create(&key_path(&dir)).expect("key");
    let mut claims = alice();
    claims.sub = String::new();

    let error = issue_dev_token(&key, &claims).expect_err("an empty sub is refused");

    assert!(matches!(error, AuthnError::InvalidClaims(_)), "{error:?}");
}

#[cfg(unix)]
#[test]
fn key_file_permissions_unix() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = key_path(&dir);
    DevKey::load_or_create(&path).expect("key");

    let mode = std::fs::metadata(&path)
        .expect("key file")
        .permissions()
        .mode();

    assert_eq!(mode & 0o777, 0o600);
}
