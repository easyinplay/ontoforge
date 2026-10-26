//! Trino client: the statement protocol over `reqwest` (ENGINE-01, ADR-0017).
//!
//! `POST {url}/v1/statement` with the SQL text, then `GET nextUri` until a page has no
//! `nextUri`. The client keeps no session state: every request carries its own headers and
//! `X-Trino-Set-*` response headers are ignored. Not implemented on purpose: spooling,
//! transactions, prepared statements, Kerberos.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use reqwest::{Client, Method, RequestBuilder, Url};
use secrecy::SecretString;

use self::cancel::QueryGuard;
use crate::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use crate::printer::print_select;
use crate::{
    Column, DEFAULT_MAX_BYTES, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn,
    PhysicalType, PrintedSql, QueryContext, QueryOutput, TableRef, Value,
};

mod cancel;
mod classify;
mod client;
mod decode;
mod headers;
pub mod l2_health;

use self::l2_health::{L2AclConfig, L2Health, L2HealthStatus};

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
    l2_health: Option<Box<L2Health>>,
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
            l2_health: None,
        })
    }

    pub(crate) fn into_l2(mut self, config: Option<L2AclConfig>) -> Self {
        self.l2_health = Some(Box::new(L2Health::new(config)));
        self
    }

    /// None for ordinary Trino engines; L2 is disabled until an ACL recheck passes.
    pub fn l2_status(&self) -> Option<L2HealthStatus> {
        self.l2_health.as_ref().map(|health| health.status())
    }

    /// Attest the configured ACL against a known, still-active query of another identity.
    pub async fn recheck_l2_acl(
        &self,
        ontology: &TrinoEngine,
        other_query_id: &str,
    ) -> L2HealthStatus {
        l2_health::recheck(self, ontology, other_query_id).await
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

impl TrinoEngine {
    async fn execute_unchecked(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        // Keep cancellation active for internal ACL probes as well as public requests.
        let mut guard = QueryGuard::new();
        match tokio::time::timeout(ctx.timeout, self.run(sql, ctx, &mut guard)).await {
            Err(_elapsed) => {
                guard.cancel_now().await;
                Err(EngineError::Timeout {
                    after_ms: u64::try_from(ctx.timeout.as_millis()).unwrap_or(u64::MAX),
                })
            }
            Ok(Ok(output)) => {
                if output.truncated {
                    guard.cancel_now().await;
                } else {
                    guard.finish();
                }
                Ok(output)
            }
            Ok(Err(error @ EngineError::Query { .. })) => {
                guard.finish();
                Err(error)
            }
            Ok(Err(error)) => {
                guard.cancel_now().await;
                Err(error)
            }
        }
    }

    /// The statement protocol: submit, then follow `nextUri` until the last page, a limit or
    /// an error. `guard` always knows the `DELETE` for the newest `nextUri`.
    async fn run(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
        guard: &mut QueryGuard,
    ) -> Result<QueryOutput, EngineError> {
        let max_page = self.conn.max_page_bytes;
        let mut page = client::send(
            || {
                Ok(self
                    .request(Method::POST, self.statement_url.clone(), ctx, true)?
                    .body(sql.as_str().to_owned()))
            },
            true,
            max_page,
        )
        .await?;
        let mut collector = Collector::new(ctx.max_rows, ctx.max_bytes);
        loop {
            let next_url = page
                .next_uri
                .take()
                .map(|uri| self.checked_next_uri(&uri))
                .transpose()?;
            guard.advance(
                next_url
                    .as_ref()
                    .and_then(|url| self.request(Method::DELETE, url.clone(), ctx, false).ok()),
            );
            collector.absorb(page)?;
            let Some(next_url) = next_url else { break };
            if collector.truncated {
                break;
            }
            page = client::send(
                || self.request(Method::GET, next_url.clone(), ctx, false),
                false,
                max_page,
            )
            .await?;
        }
        Ok(collector.finish())
    }
}

/// Rows and columns gathered over all pages.
struct Collector {
    columns: Option<Vec<Column>>,
    rows: Vec<Vec<Value>>,
    max_rows: usize,
    max_bytes: usize,
    /// JSON bytes of the cells kept so far.
    bytes: usize,
    truncated: bool,
}

impl Collector {
    fn new(max_rows: usize, max_bytes: usize) -> Self {
        Self {
            columns: None,
            rows: Vec::new(),
            max_rows,
            max_bytes,
            bytes: 0,
            truncated: false,
        }
    }

    fn absorb(&mut self, page: decode::QueryResults) -> Result<(), EngineError> {
        if self.columns.is_none()
            && let Some(wire) = &page.columns
        {
            self.columns = Some(decode::columns(wire));
        }
        for raw in page.data.unwrap_or_default() {
            let Some(columns) = &self.columns else {
                return Err(EngineError::Protocol(
                    "a data page arrived before any column description".to_owned(),
                ));
            };
            // Rows past the row limit are never parsed.
            if self.rows.len() >= self.max_rows {
                self.truncated = true;
                break;
            }
            let row: Vec<serde_json::Value> = serde_json::from_str(raw.get()).map_err(|e| {
                // The serde message can quote cell values; keep to the position.
                EngineError::Protocol(format!(
                    "a row is not a JSON array ({:?}) at column {}",
                    e.classify(),
                    e.column()
                ))
            })?;
            if row.len() != columns.len() {
                return Err(EngineError::Protocol(format!(
                    "a row has {} cells but the result has {} columns",
                    row.len(),
                    columns.len()
                )));
            }
            let row_bytes = json_len(&row);
            if self.bytes.saturating_add(row_bytes) > self.max_bytes {
                self.truncated = true;
                break;
            }
            self.bytes += row_bytes;
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

/// Size of `row` as compact JSON, the unit `max_bytes` is counted in.
fn json_len(row: &[serde_json::Value]) -> usize {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 += buf.len();
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    // Writing JSON into a counter cannot fail.
    let _ = serde_json::to_writer(&mut counter, row);
    counter.0
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
        if self
            .l2_health
            .as_ref()
            .is_some_and(|health| health.status() != L2HealthStatus::Enabled)
        {
            return Err(EngineError::Unavailable {
                retryable: false,
                reason: "L2 ACL is not verified".to_owned(),
            });
        }
        self.execute_unchecked(sql, ctx).await
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        let Some(first) = tables.first() else {
            return Ok(BTreeMap::new());
        };
        if let Some(other) = tables.iter().find(|t| t.catalog != first.catalog) {
            return Err(EngineError::Config(format!(
                "describe_tables needs one catalog per call, got `{}` and `{}`",
                first.catalog, other.catalog
            )));
        }
        let sql = print_select(
            &columns_query(first.catalog.clone(), tables),
            Dialect::Trino,
        )?;
        let output = match self.execute(&sql, ctx).await {
            Ok(output) => output,
            // A catalog or schema that does not exist has no tables.
            Err(EngineError::Query { error_name, .. })
                if error_name == "CATALOG_NOT_FOUND" || error_name == "SCHEMA_NOT_FOUND" =>
            {
                return Ok(BTreeMap::new());
            }
            Err(e) => return Err(e),
        };
        if output.truncated {
            // An incomplete column list would read as missing columns.
            return Err(EngineError::Protocol(format!(
                "column metadata of catalog `{}` was truncated by max_rows / max_bytes",
                first.catalog
            )));
        }
        group_columns(tables, &output.rows)
    }

    async fn status(&self) -> EngineStatus {
        let ctx = QueryContext {
            trace_token: "ontoforge-status".to_owned(),
            client_tags: vec!["ontoforge".to_owned()],
            timeout: Duration::from_secs(5),
            max_rows: 1,
            max_bytes: DEFAULT_MAX_BYTES,
        };
        let probe = Select {
            projection: vec![SelectItem {
                expr: Expr::Literal(Literal::Long(1)),
                alias: None,
            }],
            from: None,
            filter: None,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
        };
        let result = match print_select(&probe, Dialect::Trino) {
            Ok(sql) => self.execute(&sql, &ctx).await,
            Err(e) => Err(EngineError::Print(e)),
        };
        match result {
            Ok(_) => EngineStatus::Available,
            Err(e) => EngineStatus::Unavailable {
                reason: e.to_string(),
            },
        }
    }
}

/// `SELECT .. FROM <catalog>.information_schema.columns WHERE <one condition per table>`.
/// Trino exposes identifiers in lower case, so the filter values are lower-cased.
fn columns_query(catalog: String, tables: &[TableRef]) -> Select {
    let equals = |column: &str, value: String| Expr::Compare {
        left: Box::new(Expr::Column(column.to_owned())),
        op: CompareOp::Eq,
        right: Box::new(Expr::Literal(Literal::String(value))),
    };
    let mut conditions: Vec<Expr> = tables
        .iter()
        .map(|t| {
            Expr::And(vec![
                equals("table_schema", t.schema.to_lowercase()),
                equals("table_name", t.table.to_lowercase()),
            ])
        })
        .collect();
    let filter = if conditions.len() == 1 {
        conditions.remove(0)
    } else {
        Expr::Or(conditions)
    };
    let item = |name: &str| SelectItem {
        expr: Expr::Column(name.to_owned()),
        alias: None,
    };
    let ascending = |name: &str| OrderItem {
        expr: Expr::Column(name.to_owned()),
        descending: false,
        nulls_first: false,
    };
    Select {
        projection: vec![
            item("table_schema"),
            item("table_name"),
            item("column_name"),
            item("data_type"),
            item("is_nullable"),
        ],
        from: Some(TableRef {
            catalog,
            schema: "information_schema".to_owned(),
            table: "columns".to_owned(),
        }),
        filter: Some(filter),
        group_by: Vec::new(),
        having: None,
        order_by: vec![
            ascending("table_schema"),
            ascending("table_name"),
            ascending("ordinal_position"),
        ],
        limit: None,
    }
}

/// Files the rows of `columns_query` under the `TableRef` that was asked for.
fn group_columns(
    tables: &[TableRef],
    rows: &[Vec<Value>],
) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
    let mut asked: HashMap<(String, String), Vec<&TableRef>> = HashMap::new();
    for t in tables {
        asked
            .entry((t.schema.to_lowercase(), t.table.to_lowercase()))
            .or_default()
            .push(t);
    }
    let mut described: BTreeMap<TableRef, Vec<PhysicalColumn>> = BTreeMap::new();
    for row in rows {
        let [
            Value::String(schema),
            Value::String(table),
            Value::String(column),
            Value::String(data_type),
            Value::String(nullable),
        ] = row.as_slice()
        else {
            return Err(EngineError::Protocol(
                "an information_schema.columns row has an unexpected shape".to_owned(),
            ));
        };
        let Some(targets) = asked.get(&(schema.to_lowercase(), table.to_lowercase())) else {
            continue;
        };
        for target in targets {
            described
                .entry((*target).clone())
                .or_default()
                .push(PhysicalColumn {
                    name: column.clone(),
                    physical: PhysicalType::parse(data_type),
                    nullable: nullable.eq_ignore_ascii_case("YES"),
                });
        }
    }
    Ok(described)
}
