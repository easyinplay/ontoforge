//! `reload` against a real PostgreSQL edit store that a real Trino reads (plan 03-18, STORE-02:
//! "Trino metadata visible before the switch").
//!
//! Every test is `#[ignore]`d: Windows development machines have neither a Trino nor the CI
//! PostgreSQL. The Linux CI job `integration` starts both (the PostgreSQL database `editstore`,
//! mapped by the Trino catalog `editstore`, see `ci/trino/catalog/editstore.properties`) and runs
//! `cargo test -p ontoforge-storage --test reload_trino_it --locked -- --ignored` with
//! `ONTOFORGE_TRINO_URL` and `ONTOFORGE_PG_URL` set; the latter must name the database the
//! catalog maps. Without either variable an ignored test panics instead of passing silently
//! (RESEARCH Pitfall 8).

use std::collections::BTreeMap;
use std::time::Duration;

use camino::Utf8Path;
use futures_util::future::BoxFuture;
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::printer::ast::{Expr, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::{
    AnyEngine, Dialect, Engine, EngineError, PhysicalColumn, QueryContext, QueryOutput, TableRef,
};
use ontoforge_schema::ir::Ontology;
use ontoforge_storage::reload::{ProbeContext, ProbeTarget, ReloadError, ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};

const TRINO_URL_VAR: &str = "ONTOFORGE_TRINO_URL";
const PG_URL_VAR: &str = "ONTOFORGE_PG_URL";

/// The Trino catalog of the CI that maps the `editstore` database.
const CATALOG: &str = "editstore";

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

/// The value of the variable `name`, or a panic that says what to do.
fn required(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => panic!(
            "{name} is not set: this test needs a running Trino and the PostgreSQL it reads \
             (see CONTRIBUTING.md)"
        ),
    }
}

/// The Trino of `ONTOFORGE_TRINO_URL` as the service identity `ontology`, built from a server
/// configuration the way the command line does (the CI Trino has no authentication).
fn trino() -> AnyEngine {
    let url = required(TRINO_URL_VAR);
    let text = format!(
        "[trino]
url = \"{url}\"
connect_timeout_ms = 5000

[trino.identities.ontology]
user = \"ontoforge\"
auth = \"none\"
"
    );
    let config = parse_server_config(&text)
        .unwrap_or_else(|e| panic!("{TRINO_URL_VAR} gives no usable configuration: {e:?}"));
    let trino = config.trino.expect("a [trino] section");
    let engines = build_trino_engines(&trino, Utf8Path::new("."), &|_| None)
        .unwrap_or_else(|e| panic!("{e:?}"));
    AnyEngine::Trino(engines.ontology)
}

/// A PostgreSQL edit store on a schema pair of its own, and the name of the edit schema.
async fn pg_store() -> (EditStore, String) {
    let url = required(PG_URL_VAR);
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let schema_edit = format!("of_it_{unique}_edit");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Postgres {
            url: secrecy::SecretString::from(url),
            schema_edit: schema_edit.clone(),
            schema_sys: format!("of_it_{unique}_sys"),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(60),
        trino_catalog: Some(CATALOG.to_owned()),
    })
    .await
    .expect("open the PostgreSQL edit store");
    (store, schema_edit)
}

fn aviation() -> Ontology {
    let out = ontoforge_schema::load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation loads")
}

/// The aviation definition with one more property on `Flight`.
fn aviation_with_gate() -> Ontology {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).expect("create dir");
        for entry in std::fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let target = to.join(entry.file_name());
            if entry.file_type().expect("file type").is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).expect("copy");
            }
        }
    }
    let dir = tempfile::tempdir().expect("tempdir");
    copy(std::path::Path::new(AVIATION), dir.path());
    let path = dir.path().join("objects.yaml");
    let from = "  delayMinutes: { type: integer, displayName: Delay in minutes }";
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains(from), "objects.yaml lost `{from}`");
    std::fs::write(
        &path,
        text.replace(
            from,
            &format!("{from}\n  gate: {{ type: string, displayName: Gate }}"),
        ),
    )
    .expect("write");
    let root = Utf8Path::from_path(dir.path()).expect("utf-8 path");
    let out = ontoforge_schema::load_dir(root);
    out.ontology
        .unwrap_or_else(|| panic!("{:#?}", out.diagnostics))
}

/// `SELECT <columns> FROM editstore.<schema>.<table>` through Trino, as a person reading the
/// edit store would send it.
async fn select(engine: &AnyEngine, schema: &str, table: &str, columns: &[&str]) -> QueryOutput {
    let query = Select {
        projection: columns
            .iter()
            .map(|name| SelectItem {
                expr: Expr::Column((*name).to_owned()),
                alias: None,
            })
            .collect(),
        from: Some(TableRef {
            catalog: CATALOG.to_owned(),
            schema: schema.to_owned(),
            table: table.to_owned(),
        }),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: Some(10),
    };
    let sql = print_select(&query, Dialect::Trino).expect("printable");
    let mut ctx = QueryContext::new(
        "ontoforge-reload-it".to_owned(),
        vec!["ontoforge".to_owned()],
    );
    ctx.timeout = Duration::from_secs(60);
    engine
        .execute(&sql, &ctx)
        .await
        .unwrap_or_else(|e| panic!("Trino cannot read {schema}.{table}: {e}"))
}

fn probing(target: &dyn ProbeTarget) -> ReloadOptions<'_> {
    ReloadOptions {
        probe: Some(ProbeContext {
            target,
            catalog: CATALOG.to_owned(),
            timeout: Duration::from_secs(60),
        }),
        ..ReloadOptions::default()
    }
}

struct NoFlush<'a>(&'a AnyEngine);

impl ProbeTarget for NoFlush<'_> {
    fn flush_metadata<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<(), EngineError>> {
        Box::pin(async { Ok(()) })
    }

    fn describe<'a>(
        &'a self,
        tables: &'a [TableRef],
    ) -> BoxFuture<'a, Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError>> {
        self.0.describe(tables)
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn reload_activates_once_trino_sees_the_new_tables() {
    let engine = trino();
    let (store, schema) = pg_store().await;

    let out = reload(&store, &aviation(), &probing(&engine))
        .await
        .expect("reload with the probe");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(1));

    // The definition is active, so Trino sees the tables it needs.
    let flights = select(&engine, &schema, "of_obj_flight", &["flight_id", "status"]).await;
    assert!(flights.rows.is_empty(), "a new edit table has no rows");
    select(&engine, &schema, "of_lnk_flight_crew", &["a_pk", "b_pk"]).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn a_new_column_is_visible_when_its_definition_becomes_active() {
    let engine = trino();
    let (store, schema) = pg_store().await;
    reload(&store, &aviation(), &probing(&engine))
        .await
        .expect("v1");
    // Trino has looked at the table, so its metadata is cached.
    select(&engine, &schema, "of_obj_flight", &["flight_id"]).await;

    let out = reload(&store, &aviation_with_gate(), &probing(&engine))
        .await
        .expect("a new column becomes visible within the probe timeout");
    assert_eq!(out.def_version, Some(2));
    let flights = select(&engine, &schema, "of_obj_flight", &["flight_id", "gate"]).await;
    assert!(flights.rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn cached_column_requires_flush_before_definition_switch() {
    let engine = trino();
    let (store, schema) = pg_store().await;
    reload(&store, &aviation(), &probing(&engine))
        .await
        .expect("v1");
    select(&engine, &schema, "of_obj_flight", &["flight_id"]).await;

    let no_flush = NoFlush(&engine);
    let error = reload(
        &store,
        &aviation_with_gate(),
        &ReloadOptions {
            probe: Some(ProbeContext {
                target: &no_flush,
                catalog: CATALOG.to_owned(),
                timeout: Duration::from_secs(2),
            }),
            ..ReloadOptions::default()
        },
    )
    .await
    .expect_err("cached columns should remain stale without a flush");
    assert!(
        matches!(&error, ReloadError::NotVisible { missing } if missing.iter().any(|item| item == "of_obj_flight.gate")),
        "expected a stale gate column: {error:?}"
    );
    assert_eq!(store.active_definition().await.unwrap().def_version, 1);

    let out = reload(&store, &aviation_with_gate(), &probing(&engine))
        .await
        .expect("real flush reveals the pending column");
    assert_eq!(out.def_version, Some(2));
    assert_eq!(store.active_definition().await.unwrap().def_version, 2);
    select(&engine, &schema, "of_obj_flight", &["flight_id", "gate"]).await;
}
