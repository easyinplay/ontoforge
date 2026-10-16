//! Dialect differences and literal validation of the printer.

use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::{Dialect, Engine, PrintError, QueryContext, TableRef, Value};

fn select_expr(expr: Expr) -> Select {
    Select {
        projection: vec![SelectItem { expr, alias: None }],
        from: None,
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    }
}

fn select_literal(lit: Literal) -> Select {
    select_expr(Expr::Literal(lit))
}

fn print(lit: Literal, dialect: Dialect) -> Result<String, PrintError> {
    print_select(&select_literal(lit), dialect).map(|p| p.as_str().to_owned())
}

#[test]
fn trino_literals_carry_type_prefixes() {
    let t = Dialect::Trino;
    assert_eq!(
        print(Literal::Decimal("1.50".into()), t).unwrap(),
        "SELECT DECIMAL '1.50'"
    );
    assert!(
        print(Literal::Double(1.5), t)
            .unwrap()
            .starts_with("SELECT DOUBLE '")
    );
    assert_eq!(
        print(Literal::Date("2026-10-16".into()), t).unwrap(),
        "SELECT DATE '2026-10-16'"
    );
    assert_eq!(
        print(Literal::Timestamp("2026-10-16 08:00:00.000".into()), t).unwrap(),
        "SELECT TIMESTAMP '2026-10-16 08:00:00.000'"
    );
    assert_eq!(print(Literal::Boolean(true), t).unwrap(), "SELECT TRUE");
    assert_eq!(print(Literal::Boolean(false), t).unwrap(), "SELECT FALSE");
    assert_eq!(print(Literal::Null, t).unwrap(), "SELECT NULL");
}

#[test]
fn sqlite_literals_are_plain() {
    let s = Dialect::Sqlite;
    assert_eq!(
        print(Literal::Decimal("1.50".into()), s).unwrap(),
        "SELECT '1.50'"
    );
    assert_eq!(
        print(Literal::Date("2026-10-16".into()), s).unwrap(),
        "SELECT '2026-10-16'"
    );
    assert_eq!(
        print(Literal::Timestamp("2026-10-16 08:00:00".into()), s).unwrap(),
        "SELECT '2026-10-16 08:00:00'"
    );
    assert_eq!(print(Literal::Boolean(true), s).unwrap(), "SELECT 1");
    assert_eq!(print(Literal::Boolean(false), s).unwrap(), "SELECT 0");
    assert_eq!(print(Literal::Double(1.5), s).unwrap(), "SELECT 1.5");
}

#[test]
fn invalid_literals_are_rejected() {
    for d in [Dialect::Trino, Dialect::Sqlite] {
        assert_eq!(
            print(Literal::Double(f64::NAN), d),
            Err(PrintError::NonFiniteDouble)
        );
        assert_eq!(
            print(Literal::Double(f64::INFINITY), d),
            Err(PrintError::NonFiniteDouble)
        );
        assert_eq!(
            print(Literal::Double(f64::NEG_INFINITY), d),
            Err(PrintError::NonFiniteDouble)
        );
        assert_eq!(
            print(Literal::Decimal("1e5".into()), d),
            Err(PrintError::InvalidDecimal("1e5".into()))
        );
        assert_eq!(
            print(Literal::Date("2026-13-01".into()), d),
            Err(PrintError::InvalidDate("2026-13-01".into()))
        );
        assert_eq!(
            print(Literal::Date("2026-1-1".into()), d),
            Err(PrintError::InvalidDate("2026-1-1".into()))
        );
        assert_eq!(
            print(Literal::Timestamp("2026-10-16T08:00:00".into()), d),
            Err(PrintError::InvalidTimestamp("2026-10-16T08:00:00".into()))
        );
        assert_eq!(
            print(Literal::Timestamp("2026-10-16 25:00:00".into()), d),
            Err(PrintError::InvalidTimestamp("2026-10-16 25:00:00".into()))
        );
    }
}

#[test]
fn empty_lists_are_rejected() {
    let empty_proj = Select {
        projection: Vec::new(),
        ..select_literal(Literal::Null)
    };
    assert_eq!(
        print_select(&empty_proj, Dialect::Sqlite).unwrap_err(),
        PrintError::EmptyProjection
    );
    let col = || Box::new(Expr::Column("a".into()));
    let with_filter = |filter: Expr| Select {
        filter: Some(filter),
        from: Some(TableRef {
            catalog: "c".into(),
            schema: "s".into(),
            table: "t".into(),
        }),
        ..select_literal(Literal::Long(1))
    };
    assert_eq!(
        print_select(
            &with_filter(Expr::InList {
                expr: col(),
                list: Vec::new(),
                negated: false
            }),
            Dialect::Trino
        )
        .unwrap_err(),
        PrintError::EmptyInList
    );
    assert_eq!(
        print_select(&with_filter(Expr::And(Vec::new())), Dialect::Trino).unwrap_err(),
        PrintError::EmptyBoolList
    );
    assert_eq!(
        print_select(&with_filter(Expr::Or(Vec::new())), Dialect::Sqlite).unwrap_err(),
        PrintError::EmptyBoolList
    );
}

#[test]
fn order_by_always_spells_out_nulls() {
    for d in [Dialect::Trino, Dialect::Sqlite] {
        let mut s = select_literal(Literal::Long(1));
        s.order_by = vec![
            OrderItem {
                expr: Expr::Column("a".into()),
                descending: false,
                nulls_first: false,
            },
            OrderItem {
                expr: Expr::Column("b".into()),
                descending: true,
                nulls_first: true,
            },
        ];
        let sql = print_select(&s, d).unwrap();
        assert!(
            sql.as_str()
                .ends_with("ORDER BY \"a\" ASC NULLS LAST, \"b\" DESC NULLS FIRST"),
            "{sql}"
        );
    }
}

#[test]
fn i64_min_is_printed_without_overflow() {
    for d in [Dialect::Trino, Dialect::Sqlite] {
        assert_eq!(
            print(Literal::Long(i64::MIN), d).unwrap(),
            "SELECT (-9223372036854775807 - 1)"
        );
    }
}

#[tokio::test]
async fn i64_min_reads_back_as_integer_on_sqlite() {
    let sql = print_select(&select_literal(Literal::Long(i64::MIN)), Dialect::Sqlite).unwrap();
    let engine = SqliteEngine::in_memory().await.unwrap();
    let out = engine
        .execute(&sql, &QueryContext::new("t".into(), Vec::new()))
        .await
        .unwrap();
    assert_eq!(out.rows, vec![vec![Value::Long(i64::MIN)]]);
}

#[test]
fn compare_operands_are_parenthesised() {
    // `a AND b = c` would parse as `a AND (b = c)`; nested boolean operands must keep their grouping.
    let inner = Expr::Or(vec![
        Expr::IsNull {
            expr: Box::new(Expr::Column("a".into())),
            negated: false,
        },
        Expr::IsNull {
            expr: Box::new(Expr::Column("b".into())),
            negated: false,
        },
    ]);
    let cmp = Expr::Compare {
        left: Box::new(inner),
        op: CompareOp::Eq,
        right: Box::new(Expr::Literal(Literal::Boolean(true))),
    };
    let sql = print_select(&select_expr(cmp), Dialect::Trino).unwrap();
    assert_eq!(
        sql.as_str(),
        "SELECT ((\"a\" IS NULL) OR (\"b\" IS NULL)) = TRUE"
    );
}
