//! Injection properties of the printer (ENGINE-03).
//!
//! Strings and identifiers come from strategies biased towards quote characters, comment
//! openers, statement separators and non-BMP characters. The printed SQL is checked three
//! ways: sqlparser sees one statement of unchanged shape, and SQLite reads the value or
//! identifier back unchanged. The Trino side of the same inputs is covered by the fixed
//! corpus that runs against a real server (plan 02-10).

use ontoforge_engine::printer::ast::{CompareOp, Expr, Literal, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::{Dialect, Engine, PrintError, QueryContext, TableRef, Value};
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;
use sqlparser::ast::{
    Expr as SqlExpr, ObjectNamePart, SetExpr, Statement, TableFactor, Value as SqlValue,
};
use sqlparser::dialect::AnsiDialect;
use sqlparser::parser::Parser;

fn config() -> ProptestConfig {
    ProptestConfig {
        failure_persistence: Some(Box::new(FileFailurePersistence::WithSource(
            "proptest-regressions",
        ))),
        ..ProptestConfig::default()
    }
}

/// Arbitrary strings mixed with fragments that matter to SQL lexers.
fn tricky_string() -> impl Strategy<Value = String> {
    let fragments = prop::sample::select(vec![
        "'",
        "''",
        "\"",
        "\\",
        "--",
        "/*",
        "*/",
        ";",
        "\n",
        "\r",
        "\t",
        "U&",
        "E'",
        "x",
        "a b",
        " ",
        "%",
        "_",
        "\u{1F600}",
        "\u{10FFFF}",
        "\u{0}",
        "\u{FEFF}",
        "\u{202E}",
    ]);
    prop_oneof![
        4 => any::<String>(),
        4 => prop::collection::vec(fragments, 0..24).prop_map(|v| v.concat()),
        1 => (1usize..4, prop::sample::select(vec!["x", "'", "\"", "\u{1F600}"]))
            .prop_map(|(n, unit)| unit.repeat(2500 * n)),
    ]
}

/// Non-empty identifiers without NUL.
fn identifier() -> impl Strategy<Value = String> {
    tricky_string()
        .prop_map(|s| s.replace('\0', ""))
        .prop_filter("identifier must not be empty", |s| !s.is_empty())
}

fn ctx() -> QueryContext {
    QueryContext::new("prop".to_owned(), Vec::new())
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn table() -> TableRef {
    TableRef {
        catalog: "c".to_owned(),
        schema: "s".to_owned(),
        table: "t".to_owned(),
    }
}

fn where_eq(value: &str) -> Select {
    Select {
        projection: vec![SelectItem {
            expr: Expr::Column("a".to_owned()),
            alias: None,
        }],
        from: Some(table()),
        filter: Some(Expr::Compare {
            left: Box::new(Expr::Column("b".to_owned())),
            op: CompareOp::Eq,
            right: Box::new(Expr::Literal(Literal::String(value.to_owned()))),
        }),
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    }
}

fn select_alias(alias: &str) -> Select {
    Select {
        projection: vec![SelectItem {
            expr: Expr::Literal(Literal::Long(1)),
            alias: Some(alias.to_owned()),
        }],
        from: None,
        filter: None,
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    }
}

fn parse_one(sql: &str) -> Statement {
    let mut statements = Parser::parse_sql(&AnsiDialect {}, sql)
        .unwrap_or_else(|e| panic!("sqlparser rejected {sql:?}: {e}"));
    assert_eq!(
        statements.len(),
        1,
        "expected exactly one statement in {sql:?}"
    );
    statements.remove(0)
}

fn query_select(stmt: &mut Statement) -> &mut sqlparser::ast::Select {
    let Statement::Query(query) = stmt else {
        panic!("not a query: {stmt:?}");
    };
    let SetExpr::Select(select) = query.body.as_mut() else {
        panic!("not a plain select");
    };
    select
}

/// The right operand of the `WHERE b = <literal>` comparison.
fn where_rhs(stmt: &mut Statement) -> &mut SqlExpr {
    let select = query_select(stmt);
    let Some(SqlExpr::BinaryOp { right, .. }) = select.selection.as_mut() else {
        panic!("no comparison in WHERE");
    };
    right.as_mut()
}

fn single_quoted(expr: &SqlExpr) -> &str {
    let SqlExpr::Value(v) = expr else {
        panic!("right operand is not a literal: {expr:?}");
    };
    let SqlValue::SingleQuotedString(s) = &v.value else {
        panic!("literal is not a single quoted string: {v:?}");
    };
    s
}

#[test]
fn ansi_dialect_has_no_backslash_escape() {
    // Trino and SQLite treat a backslash inside '...' as an ordinary character. If sqlparser's
    // ANSI dialect did not, the round trip properties below would test the wrong semantics.
    let mut stmt = parse_one(r"SELECT a FROM t WHERE b = 'a\b'");
    assert_eq!(single_quoted(where_rhs(&mut stmt)), r"a\b");
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn string_literal_keeps_statement_structure(s in tricky_string()) {
        for dialect in [Dialect::Trino, Dialect::Sqlite] {
            let printed = print_select(&where_eq(&s), dialect);
            if s.contains('\0') {
                prop_assert_eq!(printed.unwrap_err(), PrintError::NulInString);
                continue;
            }
            let sql = printed.unwrap();
            let mut with_s = parse_one(sql.as_str());
            prop_assert_eq!(single_quoted(where_rhs(&mut with_s)), s.as_str());

            let fixed = print_select(&where_eq("x"), dialect).unwrap();
            let mut with_x = parse_one(fixed.as_str());
            // Same shape: after swapping the right operand the two trees are identical.
            *where_rhs(&mut with_s) = where_rhs(&mut with_x).clone();
            prop_assert_eq!(with_s, with_x);
        }
    }

    #[test]
    fn string_literal_round_trips_on_sqlite(s in tricky_string()) {
        prop_assume!(!s.contains('\0'));
        let select = Select {
            projection: vec![SelectItem {
                expr: Expr::Literal(Literal::String(s.clone())),
                alias: Some("v".to_owned()),
            }],
            from: None,
            filter: None,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
        };
        let sql = print_select(&select, Dialect::Sqlite).unwrap();
        let rt = runtime();
        let engine = rt.block_on(SqliteEngine::in_memory()).unwrap();
        let out = rt.block_on(engine.execute(&sql, &ctx())).unwrap();
        prop_assert_eq!(out.rows, vec![vec![Value::String(s)]]);
    }

    #[test]
    fn identifier_round_trips_on_sqlite(s in identifier()) {
        let sql = print_select(&select_alias(&s), Dialect::Sqlite).unwrap();
        let rt = runtime();
        let engine = rt.block_on(SqliteEngine::in_memory()).unwrap();
        let out = rt.block_on(engine.execute(&sql, &ctx())).unwrap();
        prop_assert_eq!(out.columns[0].name.as_str(), s.as_str());

        // The same identifier as a table name: one statement, one quoted identifier.
        let from_table = Select {
            from: Some(TableRef { catalog: "c".to_owned(), schema: "s".to_owned(), table: s.clone() }),
            ..select_alias("a")
        };
        let sql = print_select(&from_table, Dialect::Trino).unwrap();
        let mut stmt = parse_one(sql.as_str());
        let select = query_select(&mut stmt);
        let TableFactor::Table { name, .. } = &select.from[0].relation else {
            panic!("FROM is not a table");
        };
        let Some(ObjectNamePart::Identifier(last)) = name.0.last() else {
            panic!("table name has no identifier");
        };
        prop_assert_eq!(last.value.as_str(), s.as_str());
        prop_assert_eq!(last.quote_style, Some('"'));
    }

    #[test]
    fn nul_in_identifier_is_rejected(prefix in "[a-z]{0,4}", suffix in "[a-z]{0,4}") {
        let s = format!("{prefix}\0{suffix}");
        prop_assert_eq!(
            print_select(&select_alias(&s), Dialect::Sqlite).unwrap_err(),
            PrintError::NulInIdentifier
        );
    }
}
