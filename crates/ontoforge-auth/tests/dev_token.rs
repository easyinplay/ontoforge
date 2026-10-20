//! Development token authentication (AUTH-02). Keys are generated per test into a temporary
//! directory; nothing here is a fixed secret.

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::authn::dev::{DevClaims, DevKey, DevMode, DevVerifier, issue_dev_token};
use ontoforge_auth::{Authenticator, AuthnError};
use ontoforge_engine::config::AuthConfig;
use ontoforge_expr::rowpred::{AttrValue, Scalar};
use serde_json::{Map, Value, json};
use std::time::Duration;

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
