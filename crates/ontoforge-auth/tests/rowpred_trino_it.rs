//! Row predicates on a real Trino: the fixed corpus, historical counterexamples and 256
//! seeded predicates, row by row against `ontoforge_expr::eval` (plan 03-11, D-09).
//!
//! Every test is `#[ignore]`d: Windows development machines have no Trino, and
//! `cargo test --workspace` must stay green there. The Linux CI job that starts PostgreSQL
//! and Trino (wired in plan 03-20) seeds `ci/pg/rowpred_corpus.sql` and runs
//! `cargo test -p ontoforge-auth --test rowpred_trino_it --locked -- --ignored` with
//! `ONTOFORGE_TRINO_URL` set. Without that variable an ignored test panics instead of
//! passing silently (RESEARCH Pitfall 8).

mod rowpred_common;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ontoforge_auth::scope::lower;
use ontoforge_engine::printer::ast::{Expr, OrderItem, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{Dialect, Engine, QueryContext, TableRef, Value};
use ontoforge_expr::{Attrs, Pred, Tri};
use rowpred_common::{TestRow, column_name, fixed_seed_cases, load_corpus, rust_all};

/// The variable naming the Trino coordinator, e.g. `http://localhost:8080`.
const URL_VAR: &str = "ONTOFORGE_TRINO_URL";

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

fn ctx() -> QueryContext {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_nanos();
    let mut ctx = QueryContext::new(
        format!("ontoforge-rowpred-{nanos}"),
        vec!["ontoforge".to_owned(), "integration-test".to_owned()],
    );
    ctx.timeout = Duration::from_secs(60);
    ctx
}

fn rows_table() -> TableRef {
    TableRef {
        catalog: "demo".to_owned(),
        schema: "rowpred".to_owned(),
        table: "rows".to_owned(),
    }
}

fn id_column() -> Expr {
    Expr::Column("id".to_owned())
}

fn item(expr: Expr, alias: Option<&str>) -> SelectItem {
    SelectItem {
        expr,
        alias: alias.map(str::to_owned),
    }
}

fn select_rows(projection: Vec<SelectItem>) -> Select {
    Select {
        projection,
        from: Some(rows_table()),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: vec![OrderItem {
            expr: id_column(),
            descending: false,
            nulls_first: false,
        }],
        limit: None,
    }
}

/// Reads the table through Trino, as `TestRow`s.
async fn read_rows(engine: &TrinoEngine) -> Vec<TestRow> {
    let select = select_rows(
        ["id", "b", "i", "d", "s", "t"]
            .into_iter()
            .map(|c| item(Expr::Column(c.to_owned()), None))
            .collect(),
    );
    let sql = print_select(&select, Dialect::Trino).expect("printable");
    let out = engine.execute(&sql, &ctx()).await.expect("read rows");
    out.rows
        .into_iter()
        .map(|r| {
            let [id, flag, int, dbl, first, second] =
                <[Value; 6]>::try_from(r).expect("six columns");
            TestRow {
                id: match id {
                    Value::Long(n) => n,
                    other => panic!("id: {other:?}"),
                },
                b: match flag {
                    Value::Null => None,
                    Value::Boolean(v) => Some(v),
                    other => panic!("b: {other:?}"),
                },
                i: match int {
                    Value::Null => None,
                    Value::Long(v) => Some(v),
                    other => panic!("i: {other:?}"),
                },
                d: match dbl {
                    Value::Null => None,
                    Value::Double(v) => Some(v),
                    other => panic!("d: {other:?}"),
                },
                s: text(first),
                t: text(second),
            }
        })
        .collect()
}

fn text(v: Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(v) => Some(v),
        other => panic!("not text: {other:?}"),
    }
}

/// Trino's truth value of `pred` on every row, in `id` order.
async fn trino_all(engine: &TrinoEngine, pred: &Pred, attrs: &Attrs) -> Vec<(i64, Tri)> {
    let lowered = lower(pred, attrs, &column_name);
    let select = select_rows(vec![item(id_column(), None), item(lowered, Some("v"))]);
    let sql = print_select(&select, Dialect::Trino).expect("printable");
    let out = engine
        .execute(&sql, &ctx())
        .await
        .unwrap_or_else(|e| panic!("Trino rejected {sql}: {e}"));
    out.rows
        .into_iter()
        .map(|r| {
            let [id, v] = <[Value; 2]>::try_from(r).expect("two columns");
            let id = match id {
                Value::Long(n) => n,
                other => panic!("id: {other:?}"),
            };
            let tri = match v {
                Value::Null => Tri::Unknown,
                Value::Boolean(true) => Tri::True,
                Value::Boolean(false) => Tri::False,
                other => panic!("not a truth value: {other:?} from {sql}"),
            };
            (id, tri)
        })
        .collect()
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

/// The seeded table must equal the JSON corpus; a drift between `ci/pg/rowpred_corpus.sql`
/// and `tests/data/rowpred_corpus.json` fails here first.
#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn trino_table_equals_the_corpus_rows() {
    let engine = trino();
    let corpus = load_corpus();
    assert_eq!(read_rows(&engine).await, corpus.rows);
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn trino_matches_rust_on_the_corpus_cases() {
    let engine = trino();
    let corpus = load_corpus();
    for case in &corpus.cases {
        let p = case.pred();
        assert_eq!(
            trino_all(&engine, &p, &case.attrs).await,
            rust_all(&p, &case.attrs, &corpus.rows),
            "corpus case {}",
            case.src
        );
    }
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
async fn trino_matches_rust_on_seeded_predicates() {
    let engine = trino();
    let corpus = load_corpus();
    for (p, a) in fixed_seed_cases(256) {
        assert_eq!(
            trino_all(&engine, &p, &a).await,
            rust_all(&p, &a, &corpus.rows),
            "predicate: {p}"
        );
    }
}
