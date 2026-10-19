//! Integration tests against a real Trino (ENGINE-05, D9).
//!
//! Every test that talks to Trino is `#[ignore]`d: Windows development machines have no Trino,
//! and `cargo test --workspace` must stay green there. The Linux CI job `integration` starts
//! PostgreSQL and Trino, seeds the aviation example and runs
//! `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored` with
//! `ONTOFORGE_TRINO_URL` set. Without that variable an ignored test panics instead of
//! passing silently (RESEARCH Pitfall 8).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camino::Utf8Path;
use ontoforge_engine::bind_check::{check_drift, expected_columns};
use ontoforge_engine::printer::ast::{
    AggregateFunc, CompareOp, Expr, Literal, OrderItem, Select, SelectItem,
};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{
    AnyEngine, Dialect, Engine, EngineError, PrintedSql, QueryContext, QueryOutput, TableRef, Value,
};
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
    match value {
        Some(url) if !url.trim().is_empty() => url,
        _ => panic!(
            "{URL_VAR} is not set: the Trino integration tests need a running Trino \
             (see CONTRIBUTING.md)"
        ),
    }
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
    let columns = described
        .get(&flights)
        .expect("demo.aviation.flights exists");
    let actual: BTreeSet<String> = columns.iter().map(|c| c.name.clone()).collect();
    assert_eq!(actual, expected);
}

fn column(name: &str) -> Expr {
    Expr::Column(name.to_owned())
}

/// `SELECT <columns> FROM <table> WHERE <key> IN (<keys>) ORDER BY <key>`.
fn rows_by_key(table: &TableRef, columns: &[&str], key: &str, keys: &[&str]) -> PrintedSql {
    let select = Select {
        projection: columns
            .iter()
            .map(|c| SelectItem {
                expr: column(c),
                alias: None,
            })
            .collect(),
        from: Some(table.clone()),
        filter: Some(Expr::InList {
            expr: Box::new(column(key)),
            list: keys
                .iter()
                .map(|k| Literal::String((*k).to_owned()))
                .collect(),
            negated: false,
        }),
        group_by: Vec::new(),
        having: None,
        order_by: vec![OrderItem {
            expr: column(key),
            descending: false,
            nulls_first: false,
        }],
        limit: None,
    };
    print_select(&select, Dialect::Trino).expect("printable")
}

/// A query result as text: `-- label: name type | ...`, then one `Debug` line per row.
fn render(label: &str, out: &QueryOutput) -> String {
    let header: Vec<String> = out
        .columns
        .iter()
        .map(|c| format!("{} {}", c.name, c.physical.render()))
        .collect();
    let mut text = format!("-- {label}: {}\n", header.join(" | "));
    for row in &out.rows {
        writeln!(text, "{row:?}").expect("write to String");
    }
    text
}

/// The first characters of `text`, for assertion messages about very long strings.
fn preview(text: &str) -> String {
    let head: String = text.chars().take(40).collect();
    format!("{head:?} ({} chars)", text.chars().count())
}

/// The types the PostgreSQL connector reports for the seeded aviation tables, described in
/// one request because all five live in catalog `demo` (E22, RESEARCH A3).
#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn describe_types_golden() {
    let tables: Vec<TableRef> = [
        "aircraft",
        "flight_crew",
        "flights",
        "maintenance_records",
        "pilots",
    ]
    .iter()
    .map(|t| table("demo", "aviation", t))
    .collect();
    let described = trino()
        .describe_tables(&tables, &ctx())
        .await
        .expect("describe");
    assert_eq!(described.len(), tables.len(), "{described:#?}");
    let mut text = String::new();
    for (table, columns) in &described {
        for c in columns {
            let not_null = if c.nullable { "" } else { " not null" };
            writeln!(
                text,
                "{}.{}.{}.{} {}{not_null}",
                table.catalog,
                table.schema,
                table.table,
                c.name,
                c.physical.render()
            )
            .expect("write to String");
        }
    }
    // Confirmed against the first CI run, run 39219827829 (E21)
    insta::assert_snapshot!(text, @r"
    demo.aviation.aircraft.tail_number varchar(16) not null
    demo.aviation.aircraft.model varchar not null
    demo.aviation.aircraft.seat_capacity integer
    demo.aviation.aircraft.in_service_since date
    demo.aviation.flight_crew.flight_id varchar(16) not null
    demo.aviation.flight_crew.pilot_id varchar(16) not null
    demo.aviation.flights.flight_id varchar(16) not null
    demo.aviation.flights.flight_no varchar(16) not null
    demo.aviation.flights.departure_airport varchar(3) not null
    demo.aviation.flights.arrival_airport varchar(3) not null
    demo.aviation.flights.tail_number varchar(16)
    demo.aviation.flights.scheduled_departure timestamp(3)
    demo.aviation.flights.status varchar
    demo.aviation.flights.delay_minutes integer
    demo.aviation.flights.tags array(varchar)
    demo.aviation.maintenance_records.record_id varchar(16) not null
    demo.aviation.maintenance_records.tail_number varchar(16) not null
    demo.aviation.maintenance_records.performed_at timestamp(3) with time zone
    demo.aviation.maintenance_records.performed_by varchar(16)
    demo.aviation.maintenance_records.summary varchar
    demo.aviation.maintenance_records.cost decimal(12,2)
    demo.aviation.pilots.employee_id varchar(16) not null
    demo.aviation.pilots.full_name varchar not null
    demo.aviation.pilots.licensenumber varchar(32)
    demo.aviation.pilots.flight_hours double
    ");
}

/// Values of every modelled type as the client decodes them from the real wire format
/// (RESEARCH A11): integer, double, decimal(12,2), date, timestamp, timestamp with time zone,
/// varchar, boolean and array(varchar), with NULLs.
#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn decode_golden() {
    let engine = trino();
    let ctx = ctx();
    let queries = [
        (
            "flights",
            rows_by_key(
                &table("demo", "aviation", "flights"),
                &["flight_id", "scheduled_departure", "delay_minutes", "tags"],
                "flight_id",
                &["F-1001", "F-1002", "F-1003", "F-2003", "F-2012"],
            ),
        ),
        (
            "aircraft",
            rows_by_key(
                &table("demo", "aviation", "aircraft"),
                &["tail_number", "seat_capacity", "in_service_since"],
                "tail_number",
                &["D-AAAA", "D-AC05", "D-AC07"],
            ),
        ),
        (
            "maintenance_records",
            rows_by_key(
                &table("demo", "aviation", "maintenance_records"),
                &["record_id", "performed_at", "cost"],
                "record_id",
                &["R-001", "R-003", "R-004", "R-006", "R-008", "R-009"],
            ),
        ),
        (
            "pilots",
            rows_by_key(
                &table("demo", "aviation", "pilots"),
                &["employee_id", "full_name", "flight_hours"],
                "employee_id",
                &["E-001", "E-003", "E-006", "E-007"],
            ),
        ),
        (
            "literals",
            select_values(vec![
                SelectItem {
                    expr: Expr::Literal(Literal::Boolean(true)),
                    alias: Some("t".to_owned()),
                },
                SelectItem {
                    expr: Expr::Literal(Literal::Boolean(false)),
                    alias: Some("f".to_owned()),
                },
                SelectItem {
                    expr: Expr::Literal(Literal::Null),
                    alias: Some("n".to_owned()),
                },
            ]),
        ),
    ];
    let mut text = String::new();
    for (label, sql) in &queries {
        let out = engine
            .execute(sql, &ctx)
            .await
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        assert!(!out.truncated, "{label}");
        text.push_str(&render(label, &out));
    }
    // Confirmed against the first CI run, run 39219827829 (E21)
    insta::assert_snapshot!(text, @r#"
    -- flights: flight_id varchar(16) | scheduled_departure timestamp(3) | delay_minutes integer | tags array(varchar)
    [String("F-1001"), Timestamp("2026-03-28 09:15:00.000"), Long(0), Array([String("domestic")])]
    [String("F-1002"), Timestamp("2026-03-29 01:30:00.000"), Long(25), Array([String("charter"), String("night")])]
    [String("F-1003"), Timestamp("2026-04-02 14:40:00.000"), Long(0), Array([])]
    [String("F-2003"), Timestamp("2026-03-29 01:59:59.000"), Long(15), Null]
    [String("F-2012"), Null, Long(60), Array([])]
    -- aircraft: tail_number varchar(16) | seat_capacity integer | in_service_since date
    [String("D-AAAA"), Long(180), Date("2019-05-01")]
    [String("D-AC05"), Null, Date("2015-06-06")]
    [String("D-AC07"), Long(190), Null]
    -- maintenance_records: record_id varchar(16) | performed_at timestamp(3) with time zone | cost decimal(12,2)
    [String("R-001"), TimestampTz("2026-02-01 10:00:00.000 UTC"), Decimal("1200.50")]
    [String("R-003"), TimestampTz("2026-01-15 08:00:00.000 UTC"), Decimal("9999999999.99")]
    [String("R-004"), TimestampTz("2026-06-01 10:00:00.000 UTC"), Decimal("-9999999999.99")]
    [String("R-006"), TimestampTz("2026-03-29 00:30:00.000 UTC"), Decimal("0.00")]
    [String("R-008"), Null, Decimal("728.04")]
    [String("R-009"), TimestampTz("2026-01-15 08:00:00.000 UTC"), Null]
    -- pilots: employee_id varchar(16) | full_name varchar | flight_hours double
    [String("E-001"), String("Alex Example"), Double(1520.5)]
    [String("E-003"), String("Sam Placeholder"), Double(480.0)]
    [String("E-006"), String("Pilot Sample 006"), Null]
    [String("E-007"), String("Pilot Sample 007"), Double(0.0)]
    -- literals: t boolean | f boolean | n unknown
    [Boolean(true), Boolean(false), Null]
    "#);
}

/// The fixed printer corpus run on Trino (D23): quotes, backslashes, comment and statement
/// separators, `U&` text, line breaks, emoji, a non-BMP character and a 10 000 character
/// string. Counter-examples found later are appended to this array.
const CORPUS: &[&str] = &[
    "",
    "'",
    "''",
    "\"",
    "\\",
    "\\'",
    "--x",
    "/*x*/",
    ";DROP TABLE t",
    "U&'\\0041'",
    "\n",
    "\r\n",
    "\t",
    "\u{1F6EB} departure",
    "\u{1D11E}",
    "a'b\"c\\d--e/*f*/;g\nh",
];

/// Length of the long corpus string.
const LONG_LEN: usize = 10_000;

fn corpus() -> Vec<String> {
    let mut all: Vec<String> = CORPUS.iter().map(|s| (*s).to_owned()).collect();
    let long = format!("{}x", "x'\"".repeat((LONG_LEN - 1) / 3));
    assert_eq!(long.chars().count(), LONG_LEN);
    all.push(long);
    all
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn printer_corpus_round_trips() {
    let engine = trino();
    let ctx = ctx();
    for text in corpus() {
        let literal = select_values(vec![SelectItem {
            expr: Expr::Literal(Literal::String(text.clone())),
            alias: Some("v".to_owned()),
        }]);
        let out = engine
            .execute(&literal, &ctx)
            .await
            .unwrap_or_else(|e| panic!("literal {}: {e}", preview(&text)));
        assert_eq!(
            out.rows,
            vec![vec![Value::String(text.clone())]],
            "literal {}",
            preview(&text)
        );

        // An identifier cannot be empty, so the empty string only runs as a literal.
        if text.is_empty() {
            continue;
        }
        let alias = select_values(vec![SelectItem {
            expr: Expr::Literal(Literal::Long(1)),
            alias: Some(text.clone()),
        }]);
        let out = engine
            .execute(&alias, &ctx)
            .await
            .unwrap_or_else(|e| panic!("alias {}: {e}", preview(&text)));
        let name = out.columns.first().map(|c| c.name.as_str());
        assert_eq!(name, Some(text.as_str()), "alias {}", preview(&text));
    }
}

/// A query that outlives its timeout is cancelled on the server, not only abandoned by the
/// client (ENGINE-01, A5).
#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn timeout_cancels_server_query() {
    let engine = trino();
    // The marker makes the query text unique, so the query can be found again below.
    let marker = unique("ontoforge-timeout");
    let slow = Select {
        projection: vec![SelectItem {
            expr: Expr::Aggregate {
                func: AggregateFunc::Sum,
                arg: Box::new(column("extendedprice")),
            },
            alias: Some("total".to_owned()),
        }],
        from: Some(table("tpch", "sf1000", "lineitem")),
        filter: Some(Expr::Compare {
            left: Box::new(column("comment")),
            op: CompareOp::NotEq,
            right: Box::new(Expr::Literal(Literal::String(marker.clone()))),
        }),
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    let slow = print_select(&slow, Dialect::Trino).expect("printable");
    let mut short = ctx();
    short.timeout = Duration::from_secs(1);
    let err = engine
        .execute(&slow, &short)
        .await
        .expect_err("summing tpch.sf1000.lineitem takes longer than 1 s");
    assert!(matches!(err, EngineError::Timeout { .. }), "{err:?}");

    // `system.runtime.queries` has no trace token column, so the query is found by its source
    // (X-Trino-Source: ontoforge) and its unique text.
    let lookup = Select {
        projection: ["query_id", "state", "error_code", "query"]
            .iter()
            .map(|c| SelectItem {
                expr: column(c),
                alias: None,
            })
            .collect(),
        from: Some(table("system", "runtime", "queries")),
        filter: Some(Expr::Compare {
            left: Box::new(column("source")),
            op: CompareOp::Eq,
            right: Box::new(Expr::Literal(Literal::String("ontoforge".to_owned()))),
        }),
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    let lookup = print_select(&lookup, Dialect::Trino).expect("printable");
    // The session limit `query_max_execution_time` (timeout + 5 s) would also end the query;
    // checking within 2 s of the timeout shows that the client's DELETE did it.
    let mut last = String::new();
    for _ in 0..10 {
        let out = engine
            .execute(&lookup, &ctx())
            .await
            .expect("system.runtime.queries");
        let found: Vec<&Vec<Value>> = out
            .rows
            .iter()
            .filter(|row| matches!(row.get(3), Some(Value::String(q)) if q.contains(&marker)))
            .collect();
        assert_eq!(found.len(), 1, "{:#?}", out.rows);
        let row = found[0];
        last = format!("state {:?}, error_code {:?}", row[1], row[2]);
        if row[1] == Value::String("FAILED".to_owned()) {
            assert_eq!(
                row[2],
                Value::String("USER_CANCELED".to_owned()),
                "cancelled by the client, not by the session limit: {last}"
            );
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("the query was not cancelled within 2 s of the timeout: {last}");
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn catalogs_visible() {
    let select = Select {
        projection: vec![SelectItem {
            expr: column("catalog_name"),
            alias: None,
        }],
        from: Some(table("system", "metadata", "catalogs")),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    let sql = print_select(&select, Dialect::Trino).expect("printable");
    let out = trino().execute(&sql, &ctx()).await.expect("catalogs");
    let names: BTreeSet<&str> = out
        .rows
        .iter()
        .filter_map(|row| match row.first() {
            Some(Value::String(name)) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(names.contains("demo"), "{names:?}");
    assert!(names.contains("editstore"), "{names:?}");
}

/// The aviation `dev` bindings match the seeded tables, so drift finds nothing to degrade.
#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn drift_is_healthy_on_ci_trino() {
    let ontology = aviation();
    let report = check_drift(&AnyEngine::Trino(trino()), &ontology, &dev(), &ctx())
        .await
        .expect("the CI Trino answers");
    assert!(report.is_healthy(), "{report:#?}");
}

/// Turns on the Iceberg checks; only the nightly workflow sets it to `1`.
const ICEBERG_VAR: &str = "ONTOFORGE_TRINO_ICEBERG";

/// `SELECT count(*) AS "n" FROM <table>`.
async fn row_count(engine: &TrinoEngine, from: &TableRef) -> Vec<Vec<Value>> {
    let select = Select {
        projection: vec![SelectItem {
            expr: Expr::CountStar,
            alias: Some("n".to_owned()),
        }],
        from: Some(from.clone()),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    let sql = print_select(&select, Dialect::Trino).expect("printable");
    engine
        .execute(&sql, &ctx())
        .await
        .unwrap_or_else(|e| {
            panic!(
                "count of {}.{}.{}: {e}",
                from.catalog, from.schema, from.table
            )
        })
        .rows
}

/// The nightly workflow copies `demo.aviation.flights` into `iceberg.aviation.flights` (JDBC
/// catalog) and sets `ONTOFORGE_TRINO_ICEBERG=1`. The PR job has no Iceberg catalog, so
/// without that variable this test checks nothing beyond `ONTOFORGE_TRINO_URL` being set.
#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn iceberg_table_is_readable() {
    let engine = trino();
    if std::env::var(ICEBERG_VAR).as_deref() != Ok("1") {
        return;
    }
    let iceberg = table("iceberg", "aviation", "flights");
    let described = engine
        .describe_tables(std::slice::from_ref(&iceberg), &ctx())
        .await
        .expect("describe the Iceberg table");
    let columns = described
        .get(&iceberg)
        .expect("iceberg.aviation.flights exists");
    assert!(!columns.is_empty());

    let copied = row_count(&engine, &iceberg).await;
    let source = row_count(&engine, &table("demo", "aviation", "flights")).await;
    assert_ne!(
        source,
        vec![vec![Value::Long(0)]],
        "the source table is seeded"
    );
    assert_eq!(copied, source);
}
