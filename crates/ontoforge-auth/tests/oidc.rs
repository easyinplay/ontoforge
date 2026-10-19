//! OIDC authentication against a local mock identity provider (wiremock serves the JWKS).
//!
//! The signing keys under `tests/data/` are test-only keys; see `tests/data/README.md`.

use camino::Utf8Path;
use jsonwebtoken::jwk::Jwk;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, get_current_timestamp};
use ontoforge_auth::{Authenticator, AuthnError, Principal};
use ontoforge_engine::config::AuthConfig;
use ontoforge_expr::rowpred::{AttrValue, Scalar};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const RS256_PEM: &[u8] = include_bytes!("data/oidc_rs256.pem");

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

    fn jwk(&self) -> Value {
        let mut jwk = Jwk::from_encoding_key(&self.encoding, self.alg).expect("jwk");
        jwk.common.key_id = Some(self.kid.clone());
        serde_json::to_value(jwk).expect("jwk json")
    }

    fn sign(&self, claims: &Value) -> String {
        let header = Header {
            kid: Some(self.kid.clone()),
            ..Header::new(self.alg)
        };
        encode(&header, claims, &self.encoding).expect("sign")
    }
}

/// The issuer URL of the mock identity provider on `server`.
fn issuer_of(server: &MockServer) -> String {
    format!("{}/realm", server.uri())
}

async fn mount_jwks(server: &MockServer, keys: &[Value]) {
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": keys })))
        .mount(server)
        .await;
}

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
    let config = auth_config(&[(issuer_of(server), format!("{}/jwks", server.uri()))]);
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

async fn authenticate(authn: &Authenticator, token: &str) -> Result<Principal, AuthnError> {
    authn.authenticate(token).await
}

#[tokio::test]
async fn rs256_token_becomes_principal() {
    let server = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks(&server, &[key.jwk()]).await;
    let authn = authenticator_for(&server);

    let token = key.sign(&base_claims(&issuer_of(&server)));
    let principal = authenticate(&authn, &token).await.expect("authenticated");

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
async fn missing_attr_claim_is_absent() {
    let server = MockServer::start().await;
    let key = TestKey::rs256("k1");
    mount_jwks(&server, &[key.jwk()]).await;
    let authn = authenticator_for(&server);

    let mut claims = base_claims(&issuer_of(&server));
    claims.as_object_mut().expect("object").remove("tenant");
    let principal = authenticate(&authn, &key.sign(&claims))
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
        authenticate(&authn, &token).await.expect("authenticated");
    }

    assert_eq!(jwks_requests(&server).await, 1);
}
