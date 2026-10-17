use sqlx::AssertSqlSafe;

use super::SqliteEngine;
use crate::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use crate::printer::sqlite_ddl::{SqliteStorage, attach_memory, create_table, insert, query_only};
use crate::printer::{Dialect, PrintedSql, print_select};
use crate::{Engine, QueryContext, TableRef, Value};

fn table() -> TableRef {
    TableRef {
        catalog: "demo".to_owned(),
        schema: "aviation".to_owned(),
        table: "flights".to_owned(),
    }
}

fn ctx() -> QueryContext {
    QueryContext::new("test".to_owned(), Vec::new())
}

async fn run(engine: &SqliteEngine, sql: &PrintedSql, binds: &[&str]) -> Result<(), sqlx::Error> {
    let mut query = sqlx::query(AssertSqlSafe(sql.as_str().to_owned()));
    for b in binds {
        query = query.bind((*b).to_owned());
    }
    let mut conn = engine.conn.lock().await;
    query.execute(&mut *conn).await.map(|_| ())
}

fn select_all(columns: &[&str]) -> Select {
    Select {
        projection: columns
            .iter()
            .map(|c| SelectItem {
                expr: Expr::Column((*c).to_owned()),
                alias: None,
            })
            .collect(),
        from: Some(table()),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    }
}

async fn engine_with_rows(texts: &[String]) -> SqliteEngine {
    let engine = SqliteEngine::in_memory().await.unwrap();
    run(&engine, &attach_memory("demo__aviation").unwrap(), &[])
        .await
        .unwrap();
    let columns = vec![("v".to_owned(), SqliteStorage::Text)];
    run(&engine, &create_table(&table(), &columns).unwrap(), &[])
        .await
        .unwrap();
    let ins = insert(&table(), &["v".to_owned()]).unwrap();
    for t in texts {
        run(&engine, &ins, &[t.as_str()]).await.unwrap();
    }
    engine
}

#[tokio::test]
async fn sqlite_ddl_round_trip() {
    let engine = SqliteEngine::in_memory().await.unwrap();
    run(&engine, &attach_memory("demo__aviation").unwrap(), &[])
        .await
        .unwrap();
    let columns = vec![
        ("we\"ird name".to_owned(), SqliteStorage::Text),
        ("n".to_owned(), SqliteStorage::Integer),
        ("r".to_owned(), SqliteStorage::Real),
    ];
    run(&engine, &create_table(&table(), &columns).unwrap(), &[])
        .await
        .unwrap();
    let names: Vec<String> = columns.iter().map(|(n, _)| n.clone()).collect();
    let ins = insert(&table(), &names).unwrap();
    assert!(ins.as_str().contains("?1"));
    assert!(ins.as_str().contains("?3"));
    {
        let q = sqlx::query(AssertSqlSafe(ins.as_str().to_owned()))
            .bind("a'b".to_owned())
            .bind(7_i64)
            .bind(1.5_f64);
        let mut conn = engine.conn.lock().await;
        q.execute(&mut *conn).await.unwrap();
    }
    let sql = print_select(&select_all(&["we\"ird name", "n", "r"]), Dialect::Sqlite).unwrap();
    let out = engine.execute(&sql, &ctx()).await.unwrap();
    assert_eq!(out.columns[0].name, "we\"ird name");
    assert_eq!(
        out.rows,
        vec![vec![
            Value::String("a'b".to_owned()),
            Value::Long(7),
            Value::Double(1.5)
        ]]
    );

    run(&engine, &query_only(), &[]).await.unwrap();
    let err = run(&engine, &ins, &["x", "1", "1.0"]).await;
    assert!(err.is_err(), "insert must fail under query_only");
}

#[tokio::test]
async fn max_rows_truncates() {
    let engine = engine_with_rows(&["a".to_owned(), "b".to_owned(), "c".to_owned()]).await;
    let sql = print_select(&select_all(&["v"]), Dialect::Sqlite).unwrap();
    let mut c = ctx();
    c.max_rows = 2;
    let out = engine.execute(&sql, &c).await.unwrap();
    assert_eq!(out.rows.len(), 2);
    assert!(out.truncated);
    c.max_rows = 3;
    let out = engine.execute(&sql, &c).await.unwrap();
    assert_eq!(out.rows.len(), 3);
    assert!(!out.truncated);
}

#[tokio::test]
async fn max_bytes_truncates() {
    let big = "x".repeat(1000);
    let engine = engine_with_rows(&[big.clone(), big.clone(), big]).await;
    let sql = print_select(&select_all(&["v"]), Dialect::Sqlite).unwrap();
    let mut c = ctx();
    c.max_bytes = 2500;
    let out = engine.execute(&sql, &c).await.unwrap();
    assert_eq!(out.rows.len(), 2);
    assert!(out.truncated);
}

fn fixture_table(file: &str) -> super::fixture::FixtureTable {
    super::fixture::FixtureTable {
        def_names: vec!["dev/Test".to_owned()],
        table: table(),
        file: file.to_owned(),
        columns: vec![super::fixture::FixtureColumn {
            name: "v".to_owned(),
            logical: super::fixture::LogicalType::String,
        }],
    }
}

fn root(dir: &tempfile::TempDir) -> &camino::Utf8Path {
    camino::Utf8Path::from_path(dir.path()).expect("utf-8 temp dir")
}

#[tokio::test]
async fn query_only_after_load() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("t.csv"), "v\nx\n").unwrap();
    let engine = SqliteEngine::from_fixtures(root(&dir), &[fixture_table("t.csv")])
        .await
        .unwrap();
    let ins = insert(&table(), &["v".to_owned()]).unwrap();
    let err = run(&engine, &ins, &["y"]).await;
    assert!(
        err.is_err(),
        "insert must fail once the fixtures are loaded"
    );
    let sql = print_select(&select_all(&["v"]), Dialect::Sqlite).unwrap();
    let out = engine.execute(&sql, &ctx()).await.unwrap();
    assert_eq!(out.rows, vec![vec![Value::String("x".to_owned())]]);
}

/// The `id`s of the rows matching `filter`, in `order_by` order (then by `id`).
async fn ids(
    engine: &SqliteEngine,
    filter: impl Into<Option<Expr>>,
    order_by: &str,
) -> Vec<String> {
    let mut select = select_all(&["id"]);
    select.filter = filter.into();
    select.order_by = [order_by, "id"]
        .iter()
        .map(|c| OrderItem {
            expr: Expr::Column((*c).to_owned()),
            descending: false,
            nulls_first: false,
        })
        .collect();
    let sql = print_select(&select, Dialect::Sqlite).unwrap();
    let out = engine.execute(&sql, &ctx()).await.unwrap();
    out.rows
        .into_iter()
        .map(|row| match &row[0] {
            Value::String(s) => s.clone(),
            other => panic!("{other:?}"),
        })
        .collect()
}

fn compare(column: &str, op: CompareOp, literal: Literal) -> Expr {
    Expr::Compare {
        left: Box::new(Expr::Column(column.to_owned())),
        op,
        right: Box::new(Expr::Literal(literal)),
    }
}

#[tokio::test]
async fn decimal_and_timestamp_compare_by_value() {
    // WR-E03: decimals are stored as fixed-scale text and timestamps as millisecond text; the
    // stand-in must still compare them the way Trino compares DECIMAL and TIMESTAMP values.
    use super::fixture::{FixtureColumn, FixtureTable, LogicalType};
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("t.csv"),
        "id,amount,ts,tsz\n\
         a,1.5,2026-01-01 00:00:00,2026-01-01 08:00:00+08:00\n\
         b,9,2026-01-01 00:00:00.5,2026-01-01 00:00:00Z\n\
         c,10,2026-01-02 00:00:00,2026-01-01 01:00:00+00:00\n\
         d,-2,2025-12-31 23:59:59.999,2025-12-31 18:00:00-05:00\n\
         e,-10,2025-06-01 00:00:00,2025-06-01 00:00:00Z\n",
    )
    .unwrap();
    let column = |name: &str, logical| FixtureColumn {
        name: name.to_owned(),
        logical,
    };
    let fixture = FixtureTable {
        def_names: vec!["dev/Test".to_owned()],
        table: table(),
        file: "t.csv".to_owned(),
        columns: vec![
            column("id", LogicalType::String),
            column(
                "amount",
                LogicalType::Decimal {
                    precision: 12,
                    scale: 2,
                },
            ),
            column("ts", LogicalType::Timestamp { aware: false }),
            column("tsz", LogicalType::Timestamp { aware: true }),
        ],
    };
    let engine = SqliteEngine::from_fixtures(root(&dir), &[fixture])
        .await
        .unwrap();
    let decimal = |t: &str| Literal::Decimal(t.to_owned());
    let timestamp = |t: &str| Literal::Timestamp(t.to_owned());

    // `1.5` is stored as `1.50`.
    let found = ids(
        &engine,
        compare("amount", CompareOp::Eq, decimal("1.5")),
        "amount",
    )
    .await;
    assert_eq!(found, ["a"]);
    // By value, not by text: `10.00 > 9`, and negative numbers in numeric order.
    let found = ids(
        &engine,
        compare("amount", CompareOp::Gt, decimal("9")),
        "amount",
    )
    .await;
    assert_eq!(found, ["c"]);
    let found = ids(
        &engine,
        compare("amount", CompareOp::Lt, decimal("-2.0")),
        "amount",
    )
    .await;
    assert_eq!(found, ["e"]);
    assert_eq!(
        ids(&engine, None, "amount").await,
        ["e", "d", "a", "b", "c"]
    );

    // `2026-01-01 00:00:00` is stored as `2026-01-01 00:00:00.000`.
    let midnight = "2026-01-01 00:00:00";
    let found = ids(
        &engine,
        compare("ts", CompareOp::Eq, timestamp(midnight)),
        "ts",
    )
    .await;
    assert_eq!(found, ["a"]);
    let found = ids(
        &engine,
        compare("ts", CompareOp::Gt, timestamp(midnight)),
        "ts",
    )
    .await;
    assert_eq!(found, ["b", "c"]);
    let found = ids(
        &engine,
        compare("ts", CompareOp::Gt, timestamp("2026-01-01 00:00:00.4999")),
        "ts",
    )
    .await;
    assert_eq!(found, ["b", "c"]);

    // Aware values compare by instant; a literal without an offset is UTC.
    let found = ids(
        &engine,
        compare("tsz", CompareOp::Eq, timestamp(midnight)),
        "tsz",
    )
    .await;
    assert_eq!(found, ["a", "b"]);
    assert_eq!(ids(&engine, None, "tsz").await, ["e", "d", "a", "b", "c"]);
}

#[test]
fn collations_are_total_orders() {
    use std::cmp::Ordering::{Equal, Greater, Less};
    for (a, b, want) in [
        ("1.50", "1.5", Equal),
        ("-0.00", "0", Equal),
        ("-10.00", "9.00", Less),
        ("10.00", "9.00", Greater),
        ("-1.25", "-1.2", Less),
        ("0.05", "0.5", Less),
        ("1.00", "not a number", Less),
        ("x", "y", Less),
    ] {
        assert_eq!(super::compare_decimal(a, b), want, "{a} vs {b}");
        assert_eq!(super::compare_decimal(b, a), want.reverse(), "{b} vs {a}");
    }
    for (a, b, want) in [
        ("2026-01-01 00:00:00.000", "2026-01-01 00:00:00", Equal),
        (
            "2026-01-01 08:00:00.000+08:00",
            "2026-01-01 00:00:00.000Z",
            Equal,
        ),
        (
            "2026-01-01 00:00:00.000-01:00",
            "2026-01-01 00:30:00.000+00:00",
            Greater,
        ),
        ("2026-01-01 00:00:00.123", "2026-01-01 00:00:00.1234", Less),
        ("2026-01-01 00:00:00.000", "garbage", Less),
    ] {
        assert_eq!(super::compare_timestamp(a, b), want, "{a} vs {b}");
        assert_eq!(super::compare_timestamp(b, a), want.reverse(), "{b} vs {a}");
    }
}

#[tokio::test]
async fn total_fixture_bytes_capped() {
    let dir = tempfile::TempDir::new().unwrap();
    let body = format!("v\n{}\n", "x".repeat(596));
    assert_eq!(body.len(), 599);
    std::fs::write(dir.path().join("a.csv"), &body).unwrap();
    std::fs::write(dir.path().join("b.csv"), &body).unwrap();
    let mut second = fixture_table("b.csv");
    second.table.table = "other".to_owned();
    let errors = SqliteEngine::from_fixtures_with_budget(
        root(&dir),
        &[fixture_table("a.csv"), second],
        1024,
    )
    .await
    .err()
    .expect("the second fixture exceeds the budget");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "b.csv");
    assert!(errors[0].message.contains("byte budget"), "{errors:?}");
}
