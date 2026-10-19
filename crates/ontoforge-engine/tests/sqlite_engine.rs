//! Tracer: printer -> Engine trait -> SQLite stand-in -> read back.

use std::time::Duration;

use camino::Utf8Path;

use ontoforge_engine::printer::ast::{Expr, Literal, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::{
    FixtureColumn, FixtureError, FixturePlan, FixtureTable, LogicalType, plan_fixture_tables,
};
use ontoforge_engine::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS, Dialect, Engine, EngineError,
    EngineStatus, PrintError, QueryContext, TableRef, Value,
};
use ontoforge_schema::ir::ProfileName;
use ontoforge_schema::load_from_memory;
use tempfile::TempDir;

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

// ---------------------------------------------------------------------------------------
// Fixture semantics (ENGINE-04, ADR-0021): typed conversion, NULL, shared tables, limits.
// ---------------------------------------------------------------------------------------

fn fx_ref(name: &str) -> TableRef {
    TableRef {
        catalog: "demo".to_owned(),
        schema: "aviation".to_owned(),
        table: name.to_owned(),
    }
}

fn fx_table(name: &str, file: &str, columns: Vec<(&str, LogicalType)>) -> FixtureTable {
    FixtureTable {
        def_names: vec!["dev/Test".to_owned()],
        table: fx_ref(name),
        file: file.to_owned(),
        columns: columns
            .into_iter()
            .map(|(name, logical)| FixtureColumn {
                name: name.to_owned(),
                logical,
            })
            .collect(),
    }
}

fn fx_root(dir: &TempDir) -> &Utf8Path {
    Utf8Path::from_path(dir.path()).expect("utf-8 temp dir")
}

/// Loads one fixture file `file` (with `text`) as table `t`.
async fn fx_load(
    file: &str,
    text: &str,
    columns: Vec<(&str, LogicalType)>,
) -> Result<SqliteEngine, Vec<FixtureError>> {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join(file), text).expect("write fixture");
    SqliteEngine::from_fixtures(fx_root(&dir), &[fx_table("t", file, columns)]).await
}

async fn fx_rows(engine: &SqliteEngine, names: &[&str]) -> Vec<Vec<Value>> {
    let select = Select {
        projection: names
            .iter()
            .map(|n| SelectItem {
                expr: Expr::Column((*n).to_owned()),
                alias: None,
            })
            .collect(),
        from: Some(fx_ref("t")),
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    let sql = print_select(&select, Dialect::Sqlite).expect("printable");
    engine
        .execute(&sql, &QueryContext::new("test".to_owned(), Vec::new()))
        .await
        .expect("query")
        .rows
}

/// `link` becomes a directory symbolic link to `target`.
#[cfg(unix)]
fn link_dir(target: &std::path::Path, link: &std::path::Path) {
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

/// `link` becomes a directory junction to `target` (no privilege needed, unlike a symlink).
#[cfg(windows)]
fn link_dir(target: &std::path::Path, link: &std::path::Path) {
    let status = std::process::Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(link)
        .arg(target)
        .stdout(std::process::Stdio::null())
        .status()
        .expect("run mklink");
    assert!(status.success(), "mklink /J failed");
}

/// CR-S01: the stand-in never loads rows from behind a directory link (a junction on Windows)
/// below the ontology root, even when the plan names such a file.
#[cfg(any(unix, windows))]
#[tokio::test]
async fn fixture_behind_directory_link_is_refused() {
    let outside = TempDir::new().expect("tempdir");
    std::fs::write(outside.path().join("secret.csv"), "id\nleaked\n").expect("write");
    let dir = TempDir::new().expect("tempdir");
    link_dir(outside.path(), &dir.path().join("fixtures"));
    let table = fx_table(
        "t",
        "fixtures/secret.csv",
        vec![("id", LogicalType::String)],
    );
    let Err(errors) = SqliteEngine::from_fixtures(fx_root(&dir), &[table]).await else {
        panic!("a fixture behind a junction or symlink was loaded");
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "fixtures/secret.csv");
    assert!(errors[0].message.contains("cannot be read"), "{errors:?}");
}

/// WR-S05: a decimal type outside ONT-E13's range is refused, never clamped to
/// `decimal(255, ..)`, even when no cell would be converted.
#[tokio::test]
async fn out_of_range_decimal_type_is_refused() {
    let decimal = |precision, scale| LogicalType::Decimal { precision, scale };
    for logical in [
        decimal(1000, 2),
        decimal(0, 0),
        decimal(12, 13),
        LogicalType::Array(Box::new(decimal(40, 2))),
    ] {
        let Err(errors) = fx_load("rows.csv", "amount\n", vec![("amount", logical.clone())]).await
        else {
            panic!("{logical:?} was accepted");
        };
        assert!(
            errors[0].message.contains("precision must be 1 to 38"),
            "{errors:?}"
        );
    }
    fx_load("rows.csv", "amount\n", vec![("amount", decimal(38, 38))])
        .await
        .expect("decimal(38, 38) is valid");
}

fn fx_csv_cell(cell: &str) -> String {
    format!("\"{}\"", cell.replace('"', "\"\""))
}

/// Loads one `v` column with a row per input and returns what was stored, in order.
async fn fx_accepts(logical: LogicalType, inputs: &[&str]) -> Vec<Value> {
    let mut text = String::from("v\n");
    for input in inputs {
        text.push_str(&fx_csv_cell(input));
        text.push('\n');
    }
    let engine = fx_load("t.csv", &text, vec![("v", logical)])
        .await
        .unwrap_or_else(|e| panic!("{inputs:?} should load: {e:?}"));
    fx_rows(&engine, &["v"])
        .await
        .into_iter()
        .map(|mut row| row.remove(0))
        .collect()
}

/// Loads one `v` cell that must be refused; returns the error.
async fn fx_rejects(logical: LogicalType, input: &str) -> FixtureError {
    let text = format!("v\n{}\n", fx_csv_cell(input));
    let Err(errors) = fx_load("t.csv", &text, vec![("v", logical)]).await else {
        panic!("{input:?} should be refused");
    };
    assert_eq!(errors.len(), 1, "{input:?}: {errors:?}");
    let error = errors.into_iter().next().expect("one error");
    assert_eq!(error.file, "t.csv");
    assert_eq!(error.line, Some(2), "{input:?}: {error:?}");
    error
}

#[tokio::test]
async fn fixture_errors_never_repeat_cell_contents() {
    let secret = "s3cr3t-value";
    let types = [
        fx_int(i64::from(i32::MIN), i64::from(i32::MAX)),
        LogicalType::Float,
        LogicalType::Decimal {
            precision: 12,
            scale: 2,
        },
        LogicalType::Boolean,
        LogicalType::Date,
        LogicalType::Timestamp { aware: false },
        LogicalType::Timestamp { aware: true },
        LogicalType::Array(Box::new(LogicalType::Boolean)),
    ];
    for logical in types {
        let error = fx_rejects(logical.clone(), secret).await;
        assert!(!error.message.contains(secret), "{logical:?}: {error:?}");
    }
    // The same through a JSON fixture and an array element.
    let json = format!(r#"[{{"v": "{secret}"}}]"#);
    let errors = fx_load(
        "t.json",
        &json,
        vec![("v", fx_int(i64::from(i32::MIN), i64::from(i32::MAX)))],
    )
    .await
    .err()
    .expect("a string is not an integer");
    assert!(!errors[0].message.contains(secret), "{errors:?}");
    let element = format!(r#"["{secret}"]"#);
    let error = fx_rejects(LogicalType::Array(Box::new(fx_int(0, 10))), &element).await;
    assert!(!error.message.contains(secret), "{error:?}");
}

fn fx_strings(values: &[&str]) -> Vec<Value> {
    values
        .iter()
        .map(|s| Value::String((*s).to_owned()))
        .collect()
}

fn fx_int(min: i64, max: i64) -> LogicalType {
    LogicalType::Integer { min, max }
}

#[tokio::test]
async fn csv_empty_cell_is_null_for_every_type() {
    let columns = vec![
        ("s", LogicalType::String),
        ("i", fx_int(i64::from(i32::MIN), i64::from(i32::MAX))),
        ("f", LogicalType::Float),
        (
            "d",
            LogicalType::Decimal {
                precision: 12,
                scale: 2,
            },
        ),
        ("b", LogicalType::Boolean),
        ("date", LogicalType::Date),
        ("ts", LogicalType::Timestamp { aware: false }),
        ("tz", LogicalType::Timestamp { aware: true }),
        ("a", LogicalType::Array(Box::new(LogicalType::String))),
    ];
    let names: Vec<&str> = columns.iter().map(|(n, _)| *n).collect();
    let text = "s,i,f,d,b,date,ts,tz,a\n,,,,,,,,\n";
    let engine = fx_load("t.csv", text, columns).await.expect("loads");
    let rows = fx_rows(&engine, &names).await;
    assert_eq!(rows, vec![vec![Value::Null; 9]]);
}

#[tokio::test]
async fn header_only_csv_loads_an_empty_table() {
    let engine = fx_load("t.csv", "v\n", vec![("v", LogicalType::String)])
        .await
        .expect("loads");
    assert!(fx_rows(&engine, &["v"]).await.is_empty());
}

#[tokio::test]
async fn json_fixture_loads() {
    let columns = || {
        vec![
            ("id", LogicalType::String),
            ("n", fx_int(i64::from(i32::MIN), i64::from(i32::MAX))),
            (
                "amount",
                LogicalType::Decimal {
                    precision: 12,
                    scale: 2,
                },
            ),
            ("when", LogicalType::Timestamp { aware: false }),
            ("ok", LogicalType::Boolean),
        ]
    };
    let text = r#"[
        {"id": "a", "n": 1, "amount": "1.5", "when": null, "ok": true},
        {"id": "b"}
    ]"#;
    let engine = fx_load("t.json", text, columns()).await.expect("loads");
    let rows = fx_rows(&engine, &["id", "n", "amount", "when", "ok"]).await;
    assert_eq!(
        rows,
        vec![
            vec![
                Value::String("a".to_owned()),
                Value::Long(1),
                Value::String("1.50".to_owned()),
                Value::Null,
                Value::Long(1),
            ],
            vec![
                Value::String("b".to_owned()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ]
    );

    let number = r#"[{"id": "a", "amount": 1.5}]"#;
    let errors = fx_load("t.json", number, columns())
        .await
        .err()
        .expect("a JSON number is not a decimal");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].message.contains("string"), "{errors:?}");
    assert_eq!(errors[0].line, Some(1), "{errors:?}");

    let object = r#"{"id": "a"}"#;
    assert!(fx_load("t.json", object, columns()).await.is_err());
}

#[tokio::test]
async fn decimal_boundaries_12_2() {
    let d = || LogicalType::Decimal {
        precision: 12,
        scale: 2,
    };
    let stored = fx_accepts(
        d(),
        &[
            "9999999999.99",
            "-9999999999.99",
            "0.01",
            "0",
            "1.5",
            "-0",
            "-0.00",
            "007.5",
        ],
    )
    .await;
    assert_eq!(
        stored,
        fx_strings(&[
            "9999999999.99",
            "-9999999999.99",
            "0.01",
            "0.00",
            "1.50",
            "0.00",
            "0.00",
            "7.50",
        ])
    );
    for bad in [
        "99999999999.99",
        "-99999999999.99",
        "0.001",
        "1e3",
        "+1",
        "1.",
        ".5",
        "abc",
        " 1",
    ] {
        fx_rejects(d(), bad).await;
    }
}

#[tokio::test]
async fn integer_ranges() {
    let int = fx_int(i64::from(i32::MIN), i64::from(i32::MAX));
    assert_eq!(
        fx_accepts(int.clone(), &["2147483647", "-2147483648", "0"]).await,
        vec![
            Value::Long(2_147_483_647),
            Value::Long(-2_147_483_648),
            Value::Long(0)
        ]
    );
    for bad in ["2147483648", "-2147483649", "abc", "1.5", "+5", "1e3"] {
        fx_rejects(int.clone(), bad).await;
    }
    let short = fx_int(i64::from(i16::MIN), i64::from(i16::MAX));
    assert_eq!(
        fx_accepts(short.clone(), &["-32768", "32767"]).await,
        vec![Value::Long(-32768), Value::Long(32767)]
    );
    fx_rejects(short.clone(), "-32769").await;
    fx_rejects(short, "32768").await;
    let byte = fx_int(i64::from(i8::MIN), i64::from(i8::MAX));
    assert_eq!(
        fx_accepts(byte.clone(), &["127", "-128"]).await,
        vec![Value::Long(127), Value::Long(-128)]
    );
    fx_rejects(byte.clone(), "128").await;
    fx_rejects(byte, "-129").await;
    let long = fx_int(i64::MIN, i64::MAX);
    assert_eq!(
        fx_accepts(
            long.clone(),
            &["9223372036854775807", "-9223372036854775808"]
        )
        .await,
        vec![Value::Long(i64::MAX), Value::Long(i64::MIN)]
    );
    fx_rejects(long, "9223372036854775808").await;
}

#[tokio::test]
async fn floats_must_be_finite() {
    assert_eq!(
        fx_accepts(LogicalType::Float, &["1.5", "-2", "1e3"]).await,
        vec![
            Value::Double(1.5),
            Value::Double(-2.0),
            Value::Double(1000.0)
        ]
    );
    for bad in ["inf", "NaN", "abc"] {
        fx_rejects(LogicalType::Float, bad).await;
    }
}

#[tokio::test]
async fn timestamps() {
    let local = || LogicalType::Timestamp { aware: false };
    assert_eq!(
        fx_accepts(
            local(),
            &[
                "2026-03-29 02:30:00",
                "2026-03-29T02:30:00.123",
                "2026-10-25T02:30:00.5",
            ]
        )
        .await,
        fx_strings(&[
            "2026-03-29 02:30:00.000",
            "2026-03-29 02:30:00.123",
            "2026-10-25 02:30:00.500",
        ])
    );
    for bad in [
        "2026-03-29T01:30:00+01:00",
        "2026-03-29T01:30:00Z",
        "2026-03-29",
        "2026-03-29 25:00:00",
        "2026-03-29 02:30:60",
        "2026-03-29 02:30:00.1234",
        "2026-02-30 00:00:00",
        "yesterday",
    ] {
        fx_rejects(local(), bad).await;
    }

    let aware = || LogicalType::Timestamp { aware: true };
    assert_eq!(
        fx_accepts(
            aware(),
            &[
                "2026-03-29 01:30:00+01:00",
                "2026-03-29T01:30:00Z",
                "2026-03-29T01:30:00.250-05:30",
            ]
        )
        .await,
        fx_strings(&[
            "2026-03-29 01:30:00.000+01:00",
            "2026-03-29 01:30:00.000+00:00",
            "2026-03-29 01:30:00.250-05:30",
        ])
    );
    for bad in [
        "2026-03-29 01:30:00",
        "2026-03-29 01:30:00+0100",
        "2026-03-29 01:30:00+25:00",
        "2026-03-29 01:30:00+01:60",
    ] {
        fx_rejects(aware(), bad).await;
    }
}

#[tokio::test]
async fn booleans_and_dates() {
    assert_eq!(
        fx_accepts(LogicalType::Boolean, &["true", "false"]).await,
        vec![Value::Long(1), Value::Long(0)]
    );
    for bad in ["True", "1", "yes", "FALSE"] {
        fx_rejects(LogicalType::Boolean, bad).await;
    }
    assert_eq!(
        fx_accepts(LogicalType::Date, &["2026-03-29", "2024-02-29"]).await,
        fx_strings(&["2026-03-29", "2024-02-29"])
    );
    for bad in [
        "2026-02-30",
        "2026-3-29",
        "29.03.2026",
        "2026-13-01",
        "2025-02-29",
    ] {
        fx_rejects(LogicalType::Date, bad).await;
    }
}

#[tokio::test]
async fn arrays() {
    let strings = || LogicalType::Array(Box::new(LogicalType::String));
    assert_eq!(
        fx_accepts(strings(), &[r#"["a","b"]"#, "[]", r#"[ "a" , null ]"#]).await,
        fx_strings(&[r#"["a","b"]"#, "[]", r#"["a",null]"#])
    );
    for bad in ["a,b", r#"{"a":1}"#, "[1]", r#"["a""#] {
        fx_rejects(strings(), bad).await;
    }
    let ints = || LogicalType::Array(Box::new(fx_int(i64::from(i32::MIN), i64::from(i32::MAX))));
    assert_eq!(fx_accepts(ints(), &["[1,2]"]).await, fx_strings(&["[1,2]"]));
    fx_rejects(ints(), r#"[1,"x"]"#).await;
    fx_rejects(ints(), "[3000000000]").await;
    let decimals = || {
        LogicalType::Array(Box::new(LogicalType::Decimal {
            precision: 12,
            scale: 2,
        }))
    };
    assert_eq!(
        fx_accepts(decimals(), &[r#"["1.5","0"]"#]).await,
        fx_strings(&[r#"["1.50","0.00"]"#])
    );
    fx_rejects(decimals(), "[1.5]").await;
    fx_rejects(decimals(), r#"["0.001"]"#).await;
    let flags = || LogicalType::Array(Box::new(LogicalType::Boolean));
    assert_eq!(
        fx_accepts(flags(), &["[true,false]"]).await,
        fx_strings(&["[true,false]"])
    );
}

#[tokio::test]
async fn fixture_error_has_line() {
    let errors = fx_load(
        "t.csv",
        "id,n\na,1\nb,2\nc,abc\nd,4\n",
        vec![
            ("id", LogicalType::String),
            ("n", fx_int(i64::from(i32::MIN), i64::from(i32::MAX))),
        ],
    )
    .await
    .err()
    .expect("`abc` is not an integer");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "t.csv");
    assert_eq!(errors[0].line, Some(4));
    assert!(errors[0].message.contains('n'), "{errors:?}");
    assert!(!errors[0].message.contains("abc"), "{errors:?}");
}

#[tokio::test]
async fn ragged_records_are_refused_with_their_line() {
    let errors = fx_load(
        "t.csv",
        "a,b\n1,2\n3\n",
        vec![("a", LogicalType::String), ("b", LogicalType::String)],
    )
    .await
    .err()
    .expect("a short record");
    assert_eq!(errors[0].line, Some(3), "{errors:?}");
}

#[tokio::test]
async fn unmapped_columns_are_kept_as_text() {
    let engine = fx_load(
        "t.csv",
        "v,extra\n1,hello\n,\n",
        vec![("v", LogicalType::Float)],
    )
    .await
    .expect("loads");
    assert_eq!(
        fx_rows(&engine, &["extra"]).await,
        vec![vec![Value::String("hello".to_owned())], vec![Value::Null]]
    );
    let described = engine
        .describe_tables(
            &[fx_ref("t")],
            &QueryContext::new("t".to_owned(), Vec::new()),
        )
        .await
        .expect("described");
    let columns = &described[&fx_ref("t")];
    assert_eq!(columns[1].name, "extra");
    assert_eq!(columns[1].physical.render(), "varchar");
}

#[tokio::test]
async fn attach_limit() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("t.csv"), "v\nx\n").expect("write");
    let tables: Vec<FixtureTable> = (0..=10)
        .map(|i| {
            let mut table = fx_table("t", "t.csv", vec![("v", LogicalType::String)]);
            table.table.catalog = format!("c{i}");
            table
        })
        .collect();
    let errors = SqliteEngine::from_fixtures(fx_root(&dir), &tables)
        .await
        .err()
        .expect("11 databases are too many");
    assert!(
        errors.iter().any(|e| e.message.contains("at most 10")),
        "{errors:?}"
    );
    // Exactly 10 is fine.
    SqliteEngine::from_fixtures(fx_root(&dir), &tables[..10])
        .await
        .expect("10 databases load");

    let mut first = fx_table("t", "t.csv", vec![("v", LogicalType::String)]);
    first.table.catalog = "a__b".to_owned();
    first.table.schema = "c".to_owned();
    let mut second = fx_table("t", "t.csv", vec![("v", LogicalType::String)]);
    second.table.catalog = "a".to_owned();
    second.table.schema = "b__c".to_owned();
    let errors = SqliteEngine::from_fixtures(fx_root(&dir), &[first, second])
        .await
        .err()
        .expect("both would be database `a__b__c`");
    assert!(
        errors.iter().any(|e| e.message.contains("a__b__c")),
        "{errors:?}"
    );
}

const FX_MANIFEST: &str = "apiName: demo\ndefaultProfile: dev\n";
const FX_OBJECTS: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
backing: source
---
kind: ObjectType
apiName: FlightNote
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  note: { type: string }
backing: source
";
const FX_FLIGHT: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
fixture: { file: fixtures/flights.csv }
";
const FX_FLIGHTS_CSV: &str = "flightId,status,note\nF1,ok,hello\n";

fn fx_note_binding(table: &str, fixture: &str) -> String {
    format!(
        "kind: SourceBinding\ntarget: {{ objectType: FlightNote }}\nsource: {{ catalog: demo, schema: aviation, table: {table} }}\nfixture: {{ file: {fixture} }}\n"
    )
}

fn fx_plan(
    note_binding: &str,
    objects: &str,
    extra: &[(&str, &str)],
) -> Result<FixturePlan, Vec<FixtureError>> {
    let mut files = vec![
        ("ontology.yaml", FX_MANIFEST),
        ("objects.yaml", objects),
        ("bindings/dev/flight.yaml", FX_FLIGHT),
        ("bindings/dev/note.yaml", note_binding),
        ("bindings/dev/fixtures/flights.csv", FX_FLIGHTS_CSV),
    ];
    files.extend_from_slice(extra);
    let out = load_from_memory(&files);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let dev = ProfileName::try_from("dev".to_owned()).expect("profile");
    plan_fixture_tables(&ontology, &out.sources, &dev)
}

#[tokio::test]
async fn shared_table_loaded_once() {
    let note = fx_note_binding("flights", "fixtures/flights.csv");
    let plan = fx_plan(&note, FX_OBJECTS, &[]).expect("plan");
    assert_eq!(plan.tables.len(), 1, "{:#?}", plan.tables);
    let table = &plan.tables[0];
    assert_eq!(table.def_names, ["dev/Flight", "dev/FlightNote"]);
    assert_eq!(table.file, "bindings/dev/fixtures/flights.csv");
    let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["flightId", "status", "note"]);
    assert!(plan.without_fixture.is_empty());

    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("bindings/dev/fixtures");
    std::fs::create_dir_all(&path).expect("directories");
    std::fs::write(path.join("flights.csv"), FX_FLIGHTS_CSV).expect("write");
    let engine = SqliteEngine::from_fixtures(fx_root(&dir), &plan.tables)
        .await
        .expect("one table is created once");
    let described = engine
        .describe_tables(
            &[fx_ref("flights")],
            &QueryContext::new("t".to_owned(), Vec::new()),
        )
        .await
        .expect("described");
    assert_eq!(described[&fx_ref("flights")].len(), 3);
}

#[test]
fn shared_table_conflicts_are_errors() {
    // Different fixture files for one table.
    let note = fx_note_binding("flights", "fixtures/other.csv");
    let errors = fx_plan(
        &note,
        FX_OBJECTS,
        &[("bindings/dev/fixtures/other.csv", FX_FLIGHTS_CSV)],
    )
    .expect_err("two files for one table");
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message = &errors[0].message;
    assert!(
        message.contains("dev/Flight") && message.contains("dev/FlightNote"),
        "{message}"
    );
    assert!(message.contains("demo.aviation.flights"), "{message}");

    // The same column with different logical types.
    let clash = FX_OBJECTS.replace("note: { type: string }", "status: { type: integer }");
    let note = fx_note_binding("flights", "fixtures/flights.csv");
    let errors = fx_plan(&note, &clash, &[]).expect_err("string against integer");
    assert!(
        errors.iter().any(|e| e.message.contains("status")),
        "{errors:?}"
    );

    // Names that differ only in case are one table.
    let note = fx_note_binding("Flights", "fixtures/flights.csv");
    let plan = fx_plan(&note, FX_OBJECTS, &[]).expect("same table");
    assert_eq!(plan.tables.len(), 1, "{:#?}", plan.tables);
}

#[tokio::test]
async fn float_columns_report_real() {
    let engine = fx_load("t.csv", "v\n1.5\n", vec![("v", LogicalType::Float)])
        .await
        .expect("loads");
    let described = engine
        .describe_tables(
            &[fx_ref("t")],
            &QueryContext::new("t".to_owned(), Vec::new()),
        )
        .await
        .expect("described");
    assert_eq!(described[&fx_ref("t")][0].physical.render(), "real");
}

#[tokio::test]
async fn double_columns_report_double() {
    let engine = fx_load("t.csv", "v\n1.5\n", vec![("v", LogicalType::Double)])
        .await
        .expect("loads");
    let described = engine
        .describe_tables(
            &[fx_ref("t")],
            &QueryContext::new("t".to_owned(), Vec::new()),
        )
        .await
        .expect("described");
    assert_eq!(described[&fx_ref("t")][0].physical.render(), "double");
}
