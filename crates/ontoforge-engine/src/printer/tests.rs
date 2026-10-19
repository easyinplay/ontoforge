use super::ast::{AggregateFunc, CompareOp, Expr, Literal, OrderItem, Select, SelectItem};
use super::{Dialect, print_select};
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
