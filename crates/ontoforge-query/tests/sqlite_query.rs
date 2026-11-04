use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use ontoforge_auth::scope::{RowVisibility, TypeScope};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::{Engine, QueryContext};
use ontoforge_expr::{CmpOp, Operand, Pred, Scalar, Tri};
use ontoforge_query::ast::{ObjectSet, PageSpec, QueryLiteral, TermMode, WhereClause};
use ontoforge_query::compile::{
    CompileLimits, ValidatedQuery, compile_source_select, evaluate_search, evaluate_where,
};
use ontoforge_query::{QueryError, QueryRequest};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::load_dir;
use serde_json::{Value, json};

fn name(s: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(s.to_owned()).unwrap()
}
fn prop(s: &str) -> PropertyName {
    PropertyName::try_from(s.to_owned()).unwrap()
}
fn lit(v: Value) -> QueryLiteral {
    QueryLiteral(v)
}
fn row() -> BTreeMap<PropertyName, Value> {
    BTreeMap::from([
        (prop("status"), json!("HELLO foo baz BAR")),
        (prop("delayMinutes"), json!(12)),
    ])
}
fn visibility() -> RowVisibility {
    RowVisibility {
        visible: true,
        columns: BTreeSet::from([prop("status"), prop("delayMinutes")]),
    }
}
fn ontology() -> ontoforge_schema::ir::Ontology {
    load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap()
}
fn scope() -> TypeScope {
    TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([
            (prop("flightId"), Pred::Const(true)),
            (prop("status"), Pred::Const(true)),
            (prop("delayMinutes"), Pred::Const(true)),
        ]),
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn sqlite_matches_rust_fixed_corpus() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixture = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = SqliteEngine::from_fixtures(root, &fixture.tables)
        .await
        .unwrap();
    let binding = &ontology.profile_bindings(&profile).unwrap().objects[&name("Flight")];
    let scope = scope();
    let all = QueryRequest {
        set: ObjectSet::Type {
            object_type: name("Flight"),
        },
        select: vec![prop("status"), prop("delayMinutes")],
    };
    let all = ValidatedQuery::new(
        all,
        vec![],
        PageSpec {
            first: 100,
            after: None,
        },
        &ontology,
        &scope,
        CompileLimits::default(),
    )
    .unwrap();
    let all_source = compile_source_select(
        &all,
        &ontology,
        binding,
        &scope,
        &ontoforge_expr::Attrs::default(),
    )
    .unwrap();
    let all_sql = print_select(&all_source.select, engine.dialect()).unwrap();
    let all_output = engine
        .execute(&all_sql, &QueryContext::new("all".into(), vec![]))
        .await
        .unwrap();
    let rows = all_output
        .rows
        .iter()
        .map(|values| {
            all_source
                .required_columns
                .iter()
                .zip(values)
                .map(|(p, v)| (p.clone(), simple_json(v)))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let visible = RowVisibility {
        visible: true,
        columns: BTreeSet::from([prop("flightId"), prop("status"), prop("delayMinutes")]),
    };
    let ty = &ontology.object_types[&name("Flight")];
    let cases = [
        WhereClause::Eq {
            property: prop("status"),
            value: lit(json!("delayed")),
        },
        WhereClause::Gt {
            property: prop("delayMinutes"),
            value: lit(json!(10)),
        },
        WhereClause::Gte {
            property: prop("delayMinutes"),
            value: lit(json!(10)),
        },
        WhereClause::Lt {
            property: prop("delayMinutes"),
            value: lit(json!(10)),
        },
        WhereClause::Lte {
            property: prop("delayMinutes"),
            value: lit(json!(10)),
        },
        WhereClause::IsNull {
            property: prop("status"),
        },
        WhereClause::In {
            property: prop("delayMinutes"),
            values: vec![lit(json!(5)), lit(json!(25))],
        },
        WhereClause::StartsWith {
            property: prop("status"),
            value: "delay".into(),
        },
        WhereClause::ContainsAnyTerm {
            property: prop("status"),
            value: "boarding delayed".into(),
        },
        WhereClause::ContainsAllTerms {
            property: prop("status"),
            value: "delayed DEL".into(),
        },
        WhereClause::And {
            clauses: vec![
                WhereClause::Gt {
                    property: prop("delayMinutes"),
                    value: lit(json!(10)),
                },
                WhereClause::Eq {
                    property: prop("status"),
                    value: lit(json!("delayed")),
                },
            ],
        },
        WhereClause::Or {
            clauses: vec![
                WhereClause::Eq {
                    property: prop("status"),
                    value: lit(json!("boarding")),
                },
                WhereClause::Eq {
                    property: prop("status"),
                    value: lit(json!("delayed")),
                },
            ],
        },
        WhereClause::Not {
            clause: Box::new(WhereClause::Eq {
                property: prop("status"),
                value: lit(json!("scheduled")),
            }),
        },
    ];
    for predicate in cases {
        let expected = rows
            .iter()
            .filter(|row| evaluate_where(&predicate, row, &visible, ty).is_true())
            .map(|row| row[&prop("flightId")].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>();
        let query = ValidatedQuery::new(
            request(predicate.clone()),
            vec![],
            PageSpec {
                first: 100,
                after: None,
            },
            &ontology,
            &scope,
            CompileLimits::default(),
        )
        .unwrap();
        let source = compile_source_select(
            &query,
            &ontology,
            binding,
            &scope,
            &ontoforge_expr::Attrs::default(),
        )
        .unwrap();
        let sql = print_select(&source.select, engine.dialect()).unwrap();
        let output = engine
            .execute(&sql, &QueryContext::new("candidate".into(), vec![]))
            .await
            .unwrap();
        let actual = output
            .rows
            .iter()
            .map(|values| {
                source
                    .required_columns
                    .iter()
                    .zip(values)
                    .map(|(p, v)| (p.clone(), simple_json(v)))
                    .collect::<BTreeMap<_, _>>()
            })
            .filter(|row| evaluate_where(&predicate, row, &visible, ty).is_true())
            .map(|row| row[&prop("flightId")].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{predicate:?}; SQL: {}", sql.as_str());
    }
}

fn simple_json(value: &ontoforge_engine::Value) -> Value {
    match value {
        ontoforge_engine::Value::Null => Value::Null,
        ontoforge_engine::Value::Long(v) => json!(v),
        ontoforge_engine::Value::String(v) => json!(v),
        other => panic!("unexpected fixed-corpus value: {other:?}"),
    }
}

#[tokio::test]
async fn sqlite_multi_role_column_grant_does_not_cross_rows() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixture = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = SqliteEngine::from_fixtures(root, &fixture.tables)
        .await
        .unwrap();
    let binding = &ontology.profile_bindings(&profile).unwrap().objects[&name("Flight")];
    let row_a = Pred::Cmp {
        left: Operand::Column(prop("flightId")),
        op: CmpOp::Eq,
        right: Operand::Lit(Scalar::String("F-1001".into())),
    };
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([
            (prop("flightId"), Pred::Const(true)),
            (prop("status"), row_a),
        ]),
    };
    for predicate in [
        WhereClause::Eq {
            property: prop("status"),
            value: lit(json!("delayed")),
        },
        WhereClause::Not {
            clause: Box::new(WhereClause::Eq {
                property: prop("status"),
                value: lit(json!("scheduled")),
            }),
        },
    ] {
        let query = ValidatedQuery::new(
            request(predicate),
            vec![],
            PageSpec {
                first: 100,
                after: None,
            },
            &ontology,
            &scope,
            CompileLimits::default(),
        )
        .unwrap();
        let source = compile_source_select(
            &query,
            &ontology,
            binding,
            &scope,
            &ontoforge_expr::Attrs::default(),
        )
        .unwrap();
        let sql = print_select(&source.select, engine.dialect()).unwrap();
        let output = engine
            .execute(&sql, &QueryContext::new("cross-role".into(), vec![]))
            .await
            .unwrap();
        assert!(
            output.rows.is_empty(),
            "hidden status affected membership: {}",
            sql.as_str()
        );
    }
}
fn request(predicate: WhereClause) -> QueryRequest {
    QueryRequest {
        set: ObjectSet::Where {
            object_type: name("Flight"),
            predicate,
        },
        select: vec![prop("status")],
    }
}
fn page() -> PageSpec {
    PageSpec {
        first: 10,
        after: None,
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_where_operator_validates_and_evaluates() {
    let cases = vec![
        (
            WhereClause::Eq {
                property: prop("delayMinutes"),
                value: lit(json!(12)),
            },
            true,
        ),
        (
            WhereClause::Gt {
                property: prop("delayMinutes"),
                value: lit(json!(10)),
            },
            true,
        ),
        (
            WhereClause::Gte {
                property: prop("delayMinutes"),
                value: lit(json!(12)),
            },
            true,
        ),
        (
            WhereClause::Lt {
                property: prop("delayMinutes"),
                value: lit(json!(15)),
            },
            true,
        ),
        (
            WhereClause::Lte {
                property: prop("delayMinutes"),
                value: lit(json!(12)),
            },
            true,
        ),
        (
            WhereClause::IsNull {
                property: prop("delayMinutes"),
            },
            false,
        ),
        (
            WhereClause::In {
                property: prop("delayMinutes"),
                values: vec![lit(json!(9)), lit(json!(12))],
            },
            true,
        ),
        (
            WhereClause::StartsWith {
                property: prop("status"),
                value: "HELLO".into(),
            },
            true,
        ),
        (
            WhereClause::ContainsAnyTerm {
                property: prop("status"),
                value: "lost foo".into(),
            },
            true,
        ),
        (
            WhereClause::ContainsAllTerms {
                property: prop("status"),
                value: "BAR foo".into(),
            },
            true,
        ),
        (
            WhereClause::And {
                clauses: vec![WhereClause::Eq {
                    property: prop("delayMinutes"),
                    value: lit(json!(12)),
                }],
            },
            true,
        ),
        (
            WhereClause::Or {
                clauses: vec![WhereClause::Eq {
                    property: prop("delayMinutes"),
                    value: lit(json!(12)),
                }],
            },
            true,
        ),
        (
            WhereClause::Not {
                clause: Box::new(WhereClause::IsNull {
                    property: prop("status"),
                }),
            },
            true,
        ),
    ];
    for (predicate, expected) in cases {
        ValidatedQuery::new(
            request(predicate.clone()),
            vec![],
            page(),
            &ontology(),
            &scope(),
            CompileLimits::default(),
        )
        .unwrap();
        assert_eq!(
            evaluate_where(
                &predicate,
                &row(),
                &visibility(),
                &ontology().object_types[&name("Flight")]
            )
            .is_true(),
            expected,
            "{predicate:?}"
        );
    }
}

#[test]
fn hidden_column_is_rejected_before_execution() {
    let hidden = WhereClause::Eq {
        property: prop("flightNumber"),
        value: lit(json!("LH100")),
    };
    assert!(matches!(
        ValidatedQuery::new(
            request(hidden),
            vec![],
            page(),
            &ontology(),
            &scope(),
            CompileLimits::default()
        ),
        Err(QueryError::InvalidQuery)
    ));
}

#[test]
fn multi_role_grant_is_bound_to_its_row() {
    let predicate = WhereClause::Not {
        clause: Box::new(WhereClause::Eq {
            property: prop("status"),
            value: lit(json!("secret")),
        }),
    };
    let hidden = RowVisibility {
        visible: true,
        columns: BTreeSet::from([prop("delayMinutes")]),
    };
    assert_eq!(
        evaluate_where(
            &predicate,
            &row(),
            &hidden,
            &ontology().object_types[&name("Flight")]
        ),
        Tri::Unknown
    );
}

#[test]
fn term_search_is_whitespace_any_or_all() {
    for mode in [TermMode::Any, TermMode::All] {
        let query = QueryRequest {
            set: ObjectSet::Search {
                object_type: name("Flight"),
                property: prop("status"),
                query: "foo bar".into(),
                mode,
            },
            select: vec![prop("status")],
        };
        ValidatedQuery::new(
            query,
            vec![],
            page(),
            &ontology(),
            &scope(),
            CompileLimits::default(),
        )
        .unwrap();
        assert_eq!(
            evaluate_search(&prop("status"), "foo bar", mode, &row(), &visibility()),
            Tri::True
        );
    }
    assert_eq!(
        evaluate_search(
            &prop("status"),
            "absent foo",
            TermMode::Any,
            &row(),
            &visibility()
        ),
        Tri::True
    );
    assert_eq!(
        evaluate_search(
            &prop("status"),
            "absent foo",
            TermMode::All,
            &row(),
            &visibility()
        ),
        Tri::False
    );
    let empty = QueryRequest {
        set: ObjectSet::Search {
            object_type: name("Flight"),
            property: prop("status"),
            query: "  \t".into(),
            mode: TermMode::All,
        },
        select: vec![prop("status")],
    };
    assert!(matches!(
        ValidatedQuery::new(
            empty,
            vec![],
            page(),
            &ontology(),
            &scope(),
            CompileLimits::default()
        ),
        Err(QueryError::InvalidQuery)
    ));
}

#[test]
fn null_unknown_and_wrong_types_are_checked() {
    let ontology = ontology();
    let ty = &ontology.object_types[&name("Flight")];
    let mut null_row = row();
    null_row.insert(prop("delayMinutes"), Value::Null);
    for predicate in [
        WhereClause::Eq {
            property: prop("delayMinutes"),
            value: lit(json!(12)),
        },
        WhereClause::Gt {
            property: prop("delayMinutes"),
            value: lit(json!(10)),
        },
        WhereClause::In {
            property: prop("delayMinutes"),
            values: vec![lit(json!(12))],
        },
    ] {
        assert_eq!(
            evaluate_where(&predicate, &null_row, &visibility(), ty),
            Tri::Unknown
        );
    }
    assert_eq!(
        evaluate_where(
            &WhereClause::IsNull {
                property: prop("delayMinutes")
            },
            &null_row,
            &visibility(),
            ty
        ),
        Tri::True
    );
    assert_eq!(
        evaluate_where(
            &WhereClause::In {
                property: prop("delayMinutes"),
                values: vec![]
            },
            &null_row,
            &visibility(),
            ty
        ),
        Tri::False
    );

    for predicate in [
        WhereClause::Eq {
            property: prop("delayMinutes"),
            value: lit(json!("twelve")),
        },
        WhereClause::Gt {
            property: prop("delayMinutes"),
            value: lit(json!("twelve")),
        },
        WhereClause::Gte {
            property: prop("delayMinutes"),
            value: lit(json!("twelve")),
        },
        WhereClause::Lt {
            property: prop("delayMinutes"),
            value: lit(json!("twelve")),
        },
        WhereClause::Lte {
            property: prop("delayMinutes"),
            value: lit(json!("twelve")),
        },
        WhereClause::In {
            property: prop("delayMinutes"),
            values: vec![lit(json!("twelve"))],
        },
        WhereClause::StartsWith {
            property: prop("delayMinutes"),
            value: "1".into(),
        },
        WhereClause::ContainsAnyTerm {
            property: prop("delayMinutes"),
            value: "1".into(),
        },
        WhereClause::ContainsAllTerms {
            property: prop("delayMinutes"),
            value: "1".into(),
        },
    ] {
        assert!(matches!(
            ValidatedQuery::new(
                request(predicate),
                vec![],
                page(),
                &ontology,
                &scope(),
                CompileLimits::default()
            ),
            Err(QueryError::InvalidQuery)
        ));
    }
}

#[test]
fn canonical_decimal_and_string_order_differ() {
    let ontology = ontology();
    let flight = &ontology.object_types[&name("Flight")];
    let string_row = BTreeMap::from([(prop("status"), json!("10"))]);
    let visible = RowVisibility {
        visible: true,
        columns: BTreeSet::from([prop("status")]),
    };
    let compare = WhereClause::Lt {
        property: prop("status"),
        value: lit(json!("2")),
    };
    assert_eq!(
        evaluate_where(&compare, &string_row, &visible, flight),
        Tri::True
    );

    let maintenance = &ontology.object_types[&name("MaintenanceRecord")];
    let decimal_row = BTreeMap::from([(prop("cost"), json!("9.00"))]);
    let visible = RowVisibility {
        visible: true,
        columns: BTreeSet::from([prop("cost")]),
    };
    let compare = WhereClause::Lt {
        property: prop("cost"),
        value: lit(json!("10.00")),
    };
    assert_eq!(
        evaluate_where(&compare, &decimal_row, &visible, maintenance),
        Tri::True
    );
}

#[test]
fn every_compile_limit_is_enforced() {
    let ontology = ontology();
    let scope = scope();
    let low = CompileLimits {
        max_depth: 2,
        max_nodes: 2,
        max_in_values: 1,
        max_selected: 1,
        max_order: 1,
        max_page: 10,
    };
    let over_nodes = WhereClause::And {
        clauses: vec![
            WhereClause::IsNull {
                property: prop("status"),
            },
            WhereClause::IsNull {
                property: prop("status"),
            },
        ],
    };
    let over_in = WhereClause::In {
        property: prop("delayMinutes"),
        values: vec![lit(json!(1)), lit(json!(2))],
    };
    for predicate in [over_nodes, over_in] {
        assert!(matches!(
            ValidatedQuery::new(request(predicate), vec![], page(), &ontology, &scope, low),
            Err(QueryError::QueryTooComplex)
        ));
    }
    let mut selected = request(WhereClause::IsNull {
        property: prop("status"),
    });
    selected.select.push(prop("delayMinutes"));
    assert!(matches!(
        ValidatedQuery::new(selected, vec![], page(), &ontology, &scope, low),
        Err(QueryError::QueryTooComplex)
    ));
}

#[test]
fn limits_reject_before_recursion() {
    let mut nested = WhereClause::IsNull {
        property: prop("status"),
    };
    for _ in 0..65 {
        nested = WhereClause::Not {
            clause: Box::new(nested),
        };
    }
    assert!(matches!(
        ValidatedQuery::new(
            request(nested),
            vec![],
            page(),
            &ontology(),
            &scope(),
            CompileLimits::default()
        ),
        Err(QueryError::QueryTooComplex)
    ));
}
