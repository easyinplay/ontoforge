//! Development tokens: HS256 tokens signed with a local key (AUTH-02).
//!
//! A developer issues a token with `ontoforge dev-token` and a server started in dev mode
//! accepts it, so no identity provider is needed on a laptop. The token says who the caller
//! is (`sub`), which groups the caller belongs to (D-02) and the claims behind the principal
//! attributes (D-05). Verification ends in the same claims to [`Principal`] mapping as OIDC,
//! so a dev principal has exactly the shape of a production one, with issuer
//! [`DEV_ISSUER`] (D-04).
//!
//! The signing key is symmetric: whoever holds the key file can mint a token for any subject.
//! For that reason the verifier cannot be switched on by configuration. [`DevVerifier::new`]
//! asks for a [`DevMode`] value, which only code that deliberately opts in constructs (the
//! `serve --dev` command and tests); `Authenticator::from_config` never builds one, even when
//! `[auth.dev]` is present. That section only names the key file.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::time::Duration;

use camino::Utf8Path;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, get_current_timestamp,
};
use ontoforge_engine::config::PrincipalAttrConfig;
use secrecy::{ExposeSecret, SecretBox};
use serde_json::{Map, Value};

use super::AuthnError;
use super::oidc::map_error;
use super::principal::{Principal, principal_from_claims};

/// Issuer and audience of every development token. The principal id of a dev token is
/// `ontoforge-dev|<sub>`.
pub const DEV_ISSUER: &str = "ontoforge-dev";

/// Length of the signing key in bytes.
const KEY_LEN: usize = 32;

/// Clock skew accepted for `exp` and `nbf`, the same default as the OIDC route.
const LEEWAY_SECS: u64 = 30;

/// The claim that carries the group names.
const GROUPS_CLAIM: &str = "groups";

/// Claims a dev token sets itself (`nbf` is never set, but a value would delay the token).
const RESERVED_CLAIMS: [&str; 7] = ["iss", "aud", "sub", "exp", "iat", "nbf", GROUPS_CLAIM];

/// Proof that the caller knowingly enables insecure development tokens.
///
/// Only `serve --dev` (Phase 7) and tests are meant to call [`DevMode::enable_insecure`].
/// A configuration file cannot produce this value: `[auth.dev]` supplies the key file path and
/// nothing else, and by itself it never makes a server accept a dev token.
pub struct DevMode {
    _priv: (),
}

impl DevMode {
    /// Acknowledges that dev tokens, signed with a local symmetric key, will be accepted.
    #[must_use]
    pub fn enable_insecure() -> Self {
        Self { _priv: () }
    }
}

/// The signing key of development tokens: 32 random bytes.
pub struct DevKey(SecretBox<[u8; KEY_LEN]>);

impl std::fmt::Debug for DevKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DevKey(..)")
    }
}

impl DevKey {
    /// Reads the key at `path`, or creates it there when the file does not exist.
    ///
    /// A new key comes from the operating system's random source and is written with
    /// exclusive creation, so an existing file is never overwritten. Missing parent directories
    /// are created first. On Unix the new directories have mode 0700 and the file 0600; an
    /// existing directory is left as it is.
    ///
    /// On Windows nothing restricts the file: it inherits the ACL of its directory, and a
    /// project directory on a data drive usually carries an inherited
    /// `NT AUTHORITY\Authenticated Users` modify entry, so other local accounts can read this
    /// symmetric key and mint tokens for any subject. To keep it private, give an absolute
    /// path below the user's profile (`--key-file`, or `key_file` in `[auth.dev]`), or tighten
    /// the ACL by hand. This code does not change ACLs; see the known-gaps list in
    /// `docs/edit-store.md`.
    ///
    /// An existing file that does not hold exactly 32 bytes is an error and is left untouched:
    /// replacing it would silently invalidate every token issued so far.
    ///
    /// Two processes creating the key at the same moment race on the exclusive creation; the
    /// loser reads the winner's file. Start the first one alone.
    pub fn load_or_create(path: &Utf8Path) -> Result<Self, AuthnError> {
        if let Some(key) = read_existing(path)? {
            return Ok(Self(SecretBox::new(Box::new(key))));
        }
        match create(path) {
            Ok(key) => Ok(Self(SecretBox::new(Box::new(key)))),
            // Someone created it between our read and our create.
            Err(CreateError::Exists) => read_existing(path)?
                .map(|key| Self(SecretBox::new(Box::new(key))))
                .ok_or_else(|| key_error(path, "disappeared while it was being created")),
            Err(CreateError::Random) => Err(AuthnError::Config(
                "the operating system random source failed".to_owned(),
            )),
            Err(CreateError::Dir) => Err(key_error(
                path,
                "cannot be created because its directory cannot be created",
            )),
            Err(CreateError::Io) => Err(key_error(path, "cannot be created")),
        }
    }
}

fn key_error(path: &Utf8Path, what: &str) -> AuthnError {
    AuthnError::Config(format!("dev key file `{path}` {what}"))
}

/// The key stored at `path`, `None` when there is no such file.
fn read_existing(path: &Utf8Path) -> Result<Option<[u8; KEY_LEN]>, AuthnError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(key_error(path, "cannot be read")),
    };
    // One byte more than the key, so a longer file is told apart from a 32 byte one.
    let mut bytes = Vec::with_capacity(KEY_LEN + 1);
    file.take(KEY_LEN as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| key_error(path, "cannot be read"))?;
    let key: [u8; KEY_LEN] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| key_error(path, "must hold exactly 32 bytes"))?;
    Ok(Some(key))
}

enum CreateError {
    Exists,
    Random,
    Dir,
    Io,
}

fn create(path: &Utf8Path) -> Result<[u8; KEY_LEN], CreateError> {
    let mut key = [0_u8; KEY_LEN];
    getrandom::fill(&mut key).map_err(|_| CreateError::Random)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_str().is_empty()) {
        create_key_dir(parent).map_err(|_| CreateError::Dir)?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| match e.kind() {
        ErrorKind::AlreadyExists => CreateError::Exists,
        _ => CreateError::Io,
    })?;
    if file.write_all(&key).and_then(|()| file.sync_all()).is_err() {
        // Do not leave a short key behind: it would be refused on the next start.
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(CreateError::Io);
    }
    Ok(key)
}

/// Creates `dir` and its missing ancestors; on Unix the new ones get mode 0700.
fn create_key_dir(dir: &Utf8Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

/// What a development token says about its subject.
pub struct DevClaims {
    pub sub: String,
    pub groups: Vec<String>,
    /// Claims behind the principal attributes, already keyed by claim name (the `claim` of
    /// `[auth.principal_attrs.<name>]`) with JSON values of the declared type.
    pub attrs: Map<String, Value>,
    /// Lifetime from now.
    pub ttl: Duration,
}

/// Signs a development token: HS256, `iss` and `aud` are [`DEV_ISSUER`], with `iat` and `exp`.
///
/// `claims.attrs` may not use a claim name this function sets itself: an attribute claim
/// named `sub` or `exp` would otherwise decide who the token is for or how long it lives.
pub fn issue_dev_token(key: &DevKey, claims: &DevClaims) -> Result<String, AuthnError> {
    if claims.sub.is_empty() {
        return Err(AuthnError::InvalidClaims(
            "claim `sub` is missing".to_owned(),
        ));
    }
    if let Some(name) = RESERVED_CLAIMS
        .iter()
        .find(|name| claims.attrs.contains_key(**name))
    {
        return Err(AuthnError::InvalidClaims(format!(
            "claim `{name}` is reserved and cannot carry an attribute"
        )));
    }
    let now = get_current_timestamp();
    let mut payload = claims.attrs.clone();
    payload.insert("iss".to_owned(), Value::from(DEV_ISSUER));
    payload.insert("aud".to_owned(), Value::from(DEV_ISSUER));
    payload.insert("sub".to_owned(), Value::from(claims.sub.as_str()));
    payload.insert("iat".to_owned(), Value::from(now));
    payload.insert(
        "exp".to_owned(),
        Value::from(now.saturating_add(claims.ttl.as_secs())),
    );
    payload.insert(GROUPS_CLAIM.to_owned(), Value::from(claims.groups.clone()));
    encode(
        &Header::new(Algorithm::HS256),
        &payload,
        &EncodingKey::from_secret(key.0.expose_secret()),
    )
    .map_err(|_| AuthnError::Config("cannot sign the dev token".to_owned()))
}

/// Verifies development tokens.
pub struct DevVerifier {
    key: DevKey,
    attrs: BTreeMap<String, PrincipalAttrConfig>,
}

impl DevVerifier {
    /// A verifier for tokens signed with `key`. `attrs` is `[auth.principal_attrs]`, which
    /// maps claims to principal attributes exactly as for OIDC tokens.
    #[must_use]
    pub fn new(_mode: DevMode, key: DevKey, attrs: &BTreeMap<String, PrincipalAttrConfig>) -> Self {
        Self {
            key,
            attrs: attrs.clone(),
        }
    }

    /// Verifies an HS256 `token`.
    pub(crate) fn verify(&self, token: &str) -> Result<Principal, AuthnError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = LEEWAY_SECS;
        validation.validate_nbf = true;
        validation.set_issuer(&[DEV_ISSUER]);
        validation.set_audience(&[DEV_ISSUER]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let data = decode::<Map<String, Value>>(
            token,
            &DecodingKey::from_secret(self.key.0.expose_secret()),
            &validation,
        )
        .map_err(|e| map_error(&e))?;
        principal_from_claims(DEV_ISSUER, &data.claims, GROUPS_CLAIM, false, &self.attrs)
    }
}
