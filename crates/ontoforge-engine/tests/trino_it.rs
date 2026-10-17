//! Integration tests against a real Trino (ENGINE-05, D9).
//!
//! Every test that talks to Trino is `#[ignore]`d: Windows development machines have no Trino,
//! and `cargo test --workspace` must stay green there. The Linux CI job `integration` starts
//! PostgreSQL and Trino, seeds the aviation example and runs
//! `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored` with
//! `ONTOFORGE_TRINO_URL` set. Without that variable an ignored test panics instead of
//! passing silently (RESEARCH Pitfall 8).

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camino::Utf8Path;
use ontoforge_engine::bind_check::expected_columns;
use ontoforge_engine::printer::ast::{Expr, Literal, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{Dialect, Engine, PrintedSql, QueryContext, TableRef, Value};
use ontoforge_schema::ir::{ObjectTypeName, Ontology, ProfileName};

/// The variable naming the Trino coordinator, e.g. `http://localhost:8080`.
const URL_VAR: &str = "ONTOFORGE_TRINO_URL";

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

/// The Trino URL taken from `value`.
///
/// Panics when the value is absent, so that `--ignored` without the environment fails loudly.
fn trino_url(value: Option<String>) -> String {
    value.unwrap_or_default()
}

/// A Trino engine for the URL in `ONTOFORGE_TRINO_URL`, as user `ontoforge` without
/// authentication (the CI Trino has none).
fn trino() -> TrinoEngine {
    let url = trino_url(std::env::var(URL_VAR).ok());
    TrinoEngine::new(TrinoConnection {
        url: reqwest::Url::parse(&url).unwrap_or_else(|e| panic!("{URL_VAR} is not a URL: {e}")),
        user: "ontoforge".to_owned(),
        auth: Auth::None,
        ca_pem: None,
        connect_timeout: Duration::from_secs(5),
        max_page_bytes: 64 * 1024 * 1024,
        extra_origins: Vec::new(),
    })
    .expect("engine")
}

/// A trace token that differs between runs.
fn unique(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

fn ctx() -> QueryContext {
    let mut ctx = QueryContext::new(
        unique("ontoforge-it"),
        vec!["ontoforge".to_owned(), "integration-test".to_owned()],
    );
    ctx.timeout = Duration::from_secs(60);
    ctx
}

fn dev() -> ProfileName {
    ProfileName::try_from("dev".to_owned()).expect("profile")
}

fn aviation() -> Ontology {
    let out = ontoforge_schema::load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation loads")
}

fn table(catalog: &str, schema: &str, name: &str) -> TableRef {
    TableRef {
        catalog: catalog.to_owned(),
        schema: schema.to_owned(),
        table: name.to_owned(),
    }
}

/// `SELECT <items>` without a table.
fn select_values(items: Vec<SelectItem>) -> PrintedSql {
    let select = Select {
        projection: items,
        from: None,
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    print_select(&select, Dialect::Trino).expect("printable")
}

#[test]
fn missing_env_panics() {
    let outcome = std::panic::catch_unwind(|| trino_url(None));
    let payload = outcome.expect_err("a missing ONTOFORGE_TRINO_URL must panic");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_default();
    assert!(message.contains(URL_VAR), "{message}");
}

#[test]
fn present_env_is_used() {
    assert_eq!(
        trino_url(Some("http://localhost:8080".to_owned())),
        "http://localhost:8080"
    );
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn select_one() {
    let sql = select_values(vec![SelectItem {
        expr: Expr::Literal(Literal::Long(1)),
        alias: None,
    }]);
    let out = trino().execute(&sql, &ctx()).await.expect("SELECT 1");
    assert_eq!(out.rows, vec![vec![Value::Long(1)]]);
    assert!(!out.truncated);
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn describe_demo_flights() {
    let ontology = aviation();
    let flight = ObjectTypeName::try_from("Flight".to_owned()).expect("name");
    let binding = ontology
        .profile_bindings(&dev())
        .and_then(|b| b.objects.get(&flight))
        .expect("Flight is bound in dev");
    let expected: BTreeSet<String> = expected_columns(&ontology, binding)
        .into_iter()
        .map(|c| c.column.to_lowercase())
        .collect();

    let flights = table("demo", "aviation", "flights");
    let described = trino()
        .describe_tables(std::slice::from_ref(&flights), &ctx())
        .await
        .expect("describe");
    let columns = described.get(&flights).expect("demo.aviation.flights exists");
    let actual: BTreeSet<String> = columns.iter().map(|c| c.name.clone()).collect();
    assert_eq!(actual, expected);
}
