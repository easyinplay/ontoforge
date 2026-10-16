//! Trino client: the statement protocol over `reqwest` (ENGINE-01, ADR-0017).
//!
//! `POST {url}/v1/statement` with the SQL text, then `GET nextUri` until a page has no
//! `nextUri`. The client keeps no session state: every request carries its own headers and
//! `X-Trino-Set-*` response headers are ignored. Not implemented on purpose: spooling,
//! transactions, prepared statements, Kerberos.

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder, Url};
use secrecy::SecretString;

use crate::{
    Column, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext,
    QueryOutput, TableRef, Value,
};

mod client;
mod decode;
mod headers;

/// How a service identity authenticates.
pub enum Auth {
    None,
    Basic { password: SecretString },
    Bearer { token: SecretString },
}

/// Everything needed to talk to one Trino coordinator as one service identity.
pub struct TrinoConnection {
    pub url: Url,
    pub user: String,
    pub auth: Auth,
    pub ca_pem: Option<Vec<u8>>,
    pub connect_timeout: Duration,
    /// Largest accepted response page, in bytes.
    pub max_page_bytes: usize,
    /// Origins besides `url` that `nextUri` may point to.
    pub extra_origins: Vec<Url>,
}

/// Trino implementation of [`Engine`]. Each instance owns its own `reqwest::Client`, so two
/// service identities never share a connection pool or credentials.
pub struct TrinoEngine {
    http: Client,
    conn: Box<TrinoConnection>,
    statement_url: Url,
}

impl TrinoEngine {
    /// Checks the transport rules (E2) and builds the HTTP client. Every construction path
    /// goes through here, so credentials cannot be sent in the clear and an extra origin
    /// cannot downgrade the scheme, however the connection was assembled.
    pub fn new(conn: TrinoConnection) -> Result<Self, EngineError> {
        check_transport(&conn)?;
        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(conn.connect_timeout)
            .user_agent(concat!("ontoforge/", env!("CARGO_PKG_VERSION")));
        if let Some(pem) = &conn.ca_pem {
            let certs = reqwest::Certificate::from_pem_bundle(pem)
                .map_err(|_| EngineError::Config("the CA bundle is not valid PEM".to_owned()))?;
            if certs.is_empty() {
                return Err(EngineError::Config(
                    "the CA bundle holds no certificate".to_owned(),
                ));
            }
            // Extra roots are added to the platform roots; certificate checks stay on.
            builder = builder.tls_certs_merge(certs);
        }
        let http = builder
            .build()
            .map_err(|e| EngineError::Config(format!("cannot build the HTTP client: {e}")))?;
        let statement_url = statement_url(&conn.url)?;
        Ok(Self {
            http,
            conn: Box::new(conn),
            statement_url,
        })
    }

    fn request(
        &self,
        method: Method,
        url: Url,
        ctx: &QueryContext,
        with_session: bool,
    ) -> Result<RequestBuilder, EngineError> {
        let headers = headers::statement_headers(&self.conn.user, ctx, with_session)?;
        let request = self.http.request(method, url).headers(headers);
        Ok(headers::apply_auth(
            request,
            &self.conn.user,
            &self.conn.auth,
        ))
    }

    /// Parses `next_uri` and checks it points at a configured origin.
    fn checked_next_uri(&self, next_uri: &str) -> Result<Url, EngineError> {
        let url = Url::parse(next_uri)
            .map_err(|_| EngineError::Protocol("nextUri is not a valid URL".to_owned()))?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err(EngineError::Protocol(
                "nextUri carries user information".to_owned(),
            ));
        }
        let allowed = std::iter::once(&self.conn.url)
            .chain(&self.conn.extra_origins)
            .any(|origin| same_origin(origin, &url));
        if allowed {
            Ok(url)
        } else {
            Err(EngineError::Protocol(format!(
                "nextUri origin {} is not the configured Trino origin",
                url.origin().ascii_serialization()
            )))
        }
    }
}

impl std::fmt::Debug for TrinoEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let auth = match self.conn.auth {
            Auth::None => "none",
            Auth::Basic { .. } => "basic",
            Auth::Bearer { .. } => "bearer",
        };
        f.debug_struct("TrinoEngine")
            .field("origin", &self.conn.url.origin().ascii_serialization())
            .field("user", &self.conn.user)
            .field("auth", &auth)
            .finish_non_exhaustive()
    }
}

/// Whether `tag` may be used as a client tag.
pub(crate) fn is_valid_client_tag(tag: &str) -> bool {
    headers::is_client_tag(tag)
}

/// The transport rules a connection must satisfy before a client is built. Messages carry
/// schemes and hosts only, never the URL or a credential.
fn check_transport(conn: &TrinoConnection) -> Result<(), EngineError> {
    let config = |message: String| Err(EngineError::Config(message));
    let url = &conn.url;
    if url.host_str().is_none() {
        return config("the Trino url has no host".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return config("the Trino url must not carry user information".to_owned());
    }
    if conn.user.is_empty() {
        return config("the Trino user is empty".to_owned());
    }
    match (url.scheme(), &conn.auth) {
        ("https", _) | ("http", Auth::None) => {}
        ("http", _) => {
            return config("basic and bearer authentication need an https url".to_owned());
        }
        (other, _) => return config(format!("unsupported url scheme `{other}`")),
    }
    for origin in &conn.extra_origins {
        let host = origin.host_str().unwrap_or("<none>");
        if origin.host_str().is_none() {
            return config("an extra origin has no host".to_owned());
        }
        if origin.scheme() != url.scheme() {
            return config(format!(
                "extra origin {host} uses scheme `{}` but the Trino url uses `{}`",
                origin.scheme(),
                url.scheme()
            ));
        }
    }
    Ok(())
}

/// `{url}/v1/statement`, keeping any path prefix of `url` (a gateway may mount Trino below
/// a path).
fn statement_url(base: &Url) -> Result<Url, EngineError> {
    let mut base = base.clone();
    base.set_query(None);
    base.set_fragment(None);
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    base.join("v1/statement")
        .map_err(|_| EngineError::Config("the Trino url cannot take a path".to_owned()))
}

fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

/// Rows and columns gathered over all pages.
struct Collector {
    columns: Option<Vec<Column>>,
    rows: Vec<Vec<Value>>,
    max_rows: usize,
    truncated: bool,
}

impl Collector {
    fn new(max_rows: usize) -> Self {
        Self {
            columns: None,
            rows: Vec::new(),
            max_rows,
            truncated: false,
        }
    }

    fn absorb(&mut self, page: decode::QueryResults) -> Result<(), EngineError> {
        if self.columns.is_none()
            && let Some(wire) = &page.columns
        {
            self.columns = Some(decode::columns(wire));
        }
        for row in page.data.unwrap_or_default() {
            let Some(columns) = &self.columns else {
                return Err(EngineError::Protocol(
                    "a data page arrived before any column description".to_owned(),
                ));
            };
            if row.len() != columns.len() {
                return Err(EngineError::Protocol(format!(
                    "a row has {} cells but the result has {} columns",
                    row.len(),
                    columns.len()
                )));
            }
            if self.rows.len() >= self.max_rows {
                self.truncated = true;
                break;
            }
            self.rows.push(decode::decode_row(columns, &row));
        }
        Ok(())
    }

    fn finish(self) -> QueryOutput {
        QueryOutput {
            columns: self.columns.unwrap_or_default(),
            rows: self.rows,
            truncated: self.truncated,
        }
    }
}

impl Engine for TrinoEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Trino
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        let max_page = self.conn.max_page_bytes;
        let post = self
            .request(Method::POST, self.statement_url.clone(), ctx, true)?
            .body(sql.as_str().to_owned());
        let mut page = client::send_once(post, max_page).await?;
        let mut collector = Collector::new(ctx.max_rows);
        loop {
            if let Some(error) = page.error.take() {
                return Err(error.into_engine_error());
            }
            let next_url = page
                .next_uri
                .take()
                .map(|uri| self.checked_next_uri(&uri))
                .transpose()?;
            collector.absorb(page)?;
            let Some(next_url) = next_url else { break };
            if collector.truncated {
                break;
            }
            let get = self.request(Method::GET, next_url, ctx, false)?;
            page = client::send_once(get, max_page).await?;
        }
        Ok(collector.finish())
    }

    // Placeholder until the information_schema query lands in the next task of this plan.
    #[allow(clippy::unused_async_trait_impl)]
    async fn describe_tables(
        &self,
        _tables: &[TableRef],
        _ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        Err(EngineError::Config(
            "describe_tables is not implemented yet".to_owned(),
        ))
    }

    // Placeholder, see `describe_tables`.
    #[allow(clippy::unused_async_trait_impl)]
    async fn status(&self) -> EngineStatus {
        EngineStatus::Unavailable {
            reason: "status is not implemented yet".to_owned(),
        }
    }
}
