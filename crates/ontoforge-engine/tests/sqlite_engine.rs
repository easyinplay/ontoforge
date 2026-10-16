//! Tracer: printer -> Engine trait -> SQLite stand-in -> read back.

use std::time::Duration;

use ontoforge_engine::printer::ast::{Expr, Literal, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS, Dialect, Engine, EngineError,
    EngineStatus, PrintError, QueryContext, TableRef, Value,
};

fn ctx() -> QueryContext {
    QueryContext::new("test".to_owned(), Vec::new())
}

fn select_literal(lit: Literal, alias: Option<&str>) -> Select {
    Select {
        projection: vec![SelectItem {
            expr: Expr::Literal(lit),
            alias: alias.map(str::to_owned),
        }],
        from: None,
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    }
}

fn table(catalog: &str) -> TableRef {
    TableRef {
        catalog: catalog.to_owned(),
        schema: "s".to_owned(),
        table: "t".to_owned(),
    }
}

#[tokio::test]
async fn printed_literal_round_trips_through_sqlite() {
    let select = select_literal(Literal::String("it's \"q\"; --x".to_owned()), Some("a\"b"));
    let sql = print_select(&select, Dialect::Sqlite).unwrap();
    assert_eq!(sql.as_str(), "SELECT 'it''s \"q\"; --x' AS \"a\"\"b\"");

    let engine = SqliteEngine::in_memory().await.unwrap();
    let out = engine.execute(&sql, &ctx()).await.unwrap();
    assert_eq!(out.columns[0].name, "a\"b");
    assert_eq!(
        out.rows,
        vec![vec![Value::String("it's \"q\"; --x".to_owned())]]
    );
    assert!(!out.truncated);
}

#[test]
fn nul_in_string_is_print_error() {
    let select = select_literal(Literal::String("a\0b".to_owned()), None);
    assert_eq!(
        print_select(&select, Dialect::Sqlite).unwrap_err(),
        PrintError::NulInString
    );
}

#[test]
fn empty_identifier_is_print_error() {
    let select = select_literal(Literal::Long(1), Some(""));
    assert_eq!(
        print_select(&select, Dialect::Sqlite).unwrap_err(),
        PrintError::EmptyIdentifier
    );
}

#[tokio::test]
async fn status_is_available() {
    let engine = SqliteEngine::in_memory().await.unwrap();
    assert_eq!(engine.status().await, EngineStatus::Available);
}

#[test]
fn error_codes_are_upper_snake() {
    let re = regex::Regex::new("^[A-Z][A-Z0-9_]+$").unwrap();
    let all = [
        EngineError::Unavailable {
            retryable: true,
            reason: String::new(),
        },
        EngineError::Unauthorized {
            reason: String::new(),
        },
        EngineError::Timeout { after_ms: 1 },
        EngineError::Cancelled,
        EngineError::Query {
            error_name: String::new(),
            message: String::new(),
        },
        EngineError::Protocol(String::new()),
        EngineError::Print(PrintError::EmptyProjection),
        EngineError::Config(String::new()),
        EngineError::Fixture(String::new()),
    ];
    for e in &all {
        assert!(re.is_match(e.code()), "bad code {:?} for {e:?}", e.code());
    }
}

#[test]
fn engine_wide_classification() {
    let wide = [
        EngineError::Unavailable {
            retryable: false,
            reason: "x".to_owned(),
        },
        EngineError::Unauthorized {
            reason: "x".to_owned(),
        },
        EngineError::Cancelled,
        EngineError::Config("x".to_owned()),
    ];
    let local = [
        EngineError::Query {
            error_name: "X".to_owned(),
            message: "x".to_owned(),
        },
        EngineError::Timeout { after_ms: 1 },
        EngineError::Protocol("x".to_owned()),
        EngineError::Print(PrintError::NulInString),
        EngineError::Fixture("x".to_owned()),
    ];
    for e in &wide {
        assert!(e.is_engine_wide(), "{e:?} should be engine wide");
    }
    for e in &local {
        assert!(!e.is_engine_wide(), "{e:?} should be per target");
    }
}

#[test]
fn query_context_defaults() {
    let c = ctx();
    assert_eq!(c.timeout, Duration::from_secs(30));
    assert_eq!(c.max_rows, 10_000);
    assert_eq!(c.max_bytes, 16 * 1024 * 1024);
    assert_eq!(c.timeout, Duration::from_millis(DEFAULT_QUERY_TIMEOUT_MS));
    assert_eq!(c.max_rows, DEFAULT_MAX_ROWS);
    assert_eq!(c.max_bytes, DEFAULT_MAX_BYTES);
}

#[tokio::test]
async fn describe_tables_rejects_mixed_catalogs() {
    let engine = SqliteEngine::in_memory().await.unwrap();
    let err = engine
        .describe_tables(&[table("a"), table("b")], &ctx())
        .await
        .unwrap_err();
    assert!(matches!(err, EngineError::Config(_)), "got {err:?}");
    let empty = engine.describe_tables(&[], &ctx()).await.unwrap();
    assert!(empty.is_empty());
}
