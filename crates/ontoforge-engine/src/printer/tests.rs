use super::ast::{
    AggregateFunc, CompareOp, Expr, Literal, MatchKind, OrderItem, Select, SelectItem,
};
use super::{Dialect, print_explain_distributed, print_flush_metadata_cache, print_select};
use crate::TableRef;

fn col(name: &str) -> Expr {
    Expr::Column(name.to_owned())
}

fn sample() -> Select {
    Select {
        projection: vec![
            SelectItem {
                expr: col("id"),
                alias: None,
            },
            SelectItem {
                expr: Expr::CountStar,
                alias: Some("n".to_owned()),
            },
            SelectItem {
                expr: Expr::Aggregate {
                    func: AggregateFunc::Max,
                    arg: Box::new(col("x")),
                },
                alias: None,
            },
        ],
        from: Some(TableRef {
            catalog: "demo".to_owned(),
            schema: "aviation".to_owned(),
            table: "flights".to_owned(),
        }),
        filter: Some(Expr::And(vec![
            Expr::Compare {
                left: Box::new(col("status")),
                op: CompareOp::Eq,
                right: Box::new(Expr::Literal(Literal::String("ok".to_owned()))),
            },
            Expr::InList {
                expr: Box::new(col("id")),
                list: vec![Literal::Long(1), Literal::Long(2)],
                negated: false,
            },
            Expr::IsNull {
                expr: Box::new(col("x")),
                negated: true,
            },
            Expr::Not(Box::new(Expr::Compare {
                left: Box::new(col("d")),
                op: CompareOp::Gt,
                right: Box::new(Expr::Literal(Literal::Decimal("1.50".to_owned()))),
            })),
        ])),
        group_by: vec![col("id")],
        having: Some(Expr::Compare {
            left: Box::new(Expr::CountStar),
            op: CompareOp::Gt,
            right: Box::new(Expr::Literal(Literal::Long(1))),
        }),
        order_by: vec![
            OrderItem {
                expr: col("n"),
                descending: true,
                nulls_first: false,
            },
            OrderItem {
                expr: col("id"),
                descending: false,
                nulls_first: true,
            },
        ],
        limit: Some(21),
    }
}

#[test]
fn print_trino_and_sqlite() {
    let select = sample();
    let trino = print_select(&select, Dialect::Trino).unwrap();
    let sqlite = print_select(&select, Dialect::Sqlite).unwrap();
    insta::assert_snapshot!("print_trino", trino.as_str());
    insta::assert_snapshot!("print_sqlite", sqlite.as_str());
}

#[test]
fn case_when_masks_only_projection() {
    let mut select = sample();
    select.projection = vec![SelectItem {
        expr: Expr::CaseWhen {
            when: Box::new(Expr::Compare {
                left: Box::new(col("owner\"id")),
                op: CompareOp::Eq,
                right: Box::new(Expr::Literal(Literal::String("o'hara".into()))),
            }),
            then: Box::new(col("secret")),
            otherwise: Box::new(Expr::Literal(Literal::Null)),
        },
        alias: Some("masked".into()),
    }];
    for dialect in [Dialect::Trino, Dialect::Sqlite] {
        let sql = print_select(&select, dialect).unwrap();
        assert!(sql.as_str().starts_with(
            "SELECT CASE WHEN (\"owner\"\"id\" = 'o''hara') THEN (\"secret\") ELSE (NULL) END AS \"masked\""
        ));
    }
}

#[test]
fn distributed_explain_wraps_typed_select() {
    let select = sample();
    let plain = print_select(&select, Dialect::Trino).unwrap();
    let explain = print_explain_distributed(&select, Dialect::Trino).unwrap();
    assert_eq!(explain.as_str(), format!("EXPLAIN (TYPE DISTRIBUTED) {}", plain.as_str()));
    assert_eq!(
        print_explain_distributed(&select, Dialect::Sqlite).unwrap_err(),
        crate::error::PrintError::UnsupportedDialect
    );
}

/// The text after `WHERE` of `SELECT 1 FROM .. WHERE <e>`.
fn where_text(e: Expr, dialect: Dialect) -> Result<String, crate::error::PrintError> {
    let select = Select {
        projection: vec![SelectItem {
            expr: Expr::Literal(Literal::Long(1)),
            alias: None,
        }],
        from: Some(TableRef {
            catalog: "demo".to_owned(),
            schema: "s".to_owned(),
            table: "t".to_owned(),
        }),
        filter: Some(e),
        group_by: Vec::new(),
        having: None,
        order_by: Vec::new(),
        limit: None,
    };
    let sql = print_select(&select, dialect)?;
    Ok(sql
        .as_str()
        .split_once(" WHERE ")
        .expect("a WHERE clause")
        .1
        .to_owned())
}

fn matches(needle: &str, kind: MatchKind) -> Expr {
    Expr::StringMatch {
        expr: Box::new(col("s")),
        needle: needle.to_owned(),
        kind,
    }
}

#[test]
fn string_match_escapes_wildcards_in_both_dialects() {
    let e = matches("a%_\\b", MatchKind::StartsWith);
    assert_eq!(
        where_text(e.clone(), Dialect::Trino).unwrap(),
        r#""s" LIKE 'a\%\_\\b%' ESCAPE '\'"#
    );
    assert_eq!(
        where_text(e, Dialect::Sqlite).unwrap(),
        r#""s" GLOB 'a%_\b*'"#
    );
}

#[test]
fn string_match_kinds_place_the_wildcards() {
    for (kind, trino, sqlite) in [
        (MatchKind::StartsWith, "'ab%'", "'ab*'"),
        (MatchKind::EndsWith, "'%ab'", "'*ab'"),
        (MatchKind::Contains, "'%ab%'", "'*ab*'"),
    ] {
        let e = matches("ab", kind);
        assert_eq!(
            where_text(e.clone(), Dialect::Trino).unwrap(),
            format!(r#""s" LIKE {trino} ESCAPE '\'"#)
        );
        assert_eq!(
            where_text(e, Dialect::Sqlite).unwrap(),
            format!(r#""s" GLOB {sqlite}"#)
        );
    }
}

#[test]
fn glob_pattern_escapes_the_glob_metacharacters() {
    assert_eq!(
        where_text(matches("[*?", MatchKind::Contains), Dialect::Sqlite).unwrap(),
        r#""s" GLOB '*[[][*][?]*'"#
    );
}

#[test]
fn string_match_quotes_the_needle_and_rejects_nul() {
    assert_eq!(
        where_text(matches("it's", MatchKind::Contains), Dialect::Sqlite).unwrap(),
        r#""s" GLOB '*it''s*'"#
    );
    for dialect in [Dialect::Trino, Dialect::Sqlite] {
        assert_eq!(
            where_text(matches("a\0b", MatchKind::Contains), dialect).unwrap_err(),
            crate::error::PrintError::NulInString
        );
    }
}

#[test]
fn string_match_is_parenthesized_inside_connectives() {
    let e = Expr::Not(Box::new(Expr::And(vec![
        matches("a", MatchKind::StartsWith),
        Expr::IsNull {
            expr: Box::new(col("t")),
            negated: false,
        },
    ])));
    assert_eq!(
        where_text(e, Dialect::Sqlite).unwrap(),
        r#"NOT (("s" GLOB 'a*') AND ("t" IS NULL))"#
    );
}

#[test]
fn flush_metadata_cache_is_a_call_of_the_catalog_procedure() {
    let sql = print_flush_metadata_cache("editstore").expect("printable");
    assert_eq!(
        sql.as_str(),
        r#"CALL "editstore".system.flush_metadata_cache()"#
    );
}

#[test]
fn flush_metadata_cache_quotes_the_catalog() {
    let sql = print_flush_metadata_cache("we\"ird; DROP").expect("printable");
    assert_eq!(
        sql.as_str(),
        r#"CALL "we""ird; DROP".system.flush_metadata_cache()"#
    );
}

#[test]
fn flush_metadata_cache_rejects_an_empty_catalog() {
    assert_eq!(
        print_flush_metadata_cache("").unwrap_err(),
        crate::error::PrintError::EmptyIdentifier
    );
    assert_eq!(
        print_flush_metadata_cache("a\0b").unwrap_err(),
        crate::error::PrintError::NulInIdentifier
    );
}
