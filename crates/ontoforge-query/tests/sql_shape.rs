use std::collections::BTreeMap;

use camino::Utf8Path;
use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::{Attrs, CmpOp, Operand, Pred, Scalar};
use ontoforge_query::QueryRequest;
use ontoforge_query::ast::{
    Direction, Nulls, ObjectSet, OrderBy, PageSpec, QueryLiteral, TermMode, WhereClause,
};
use ontoforge_query::compile::{CompileLimits, ValidatedQuery, compile_source_select};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::load_dir;
use serde_json::json;

fn name(s: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(s.to_owned()).unwrap()
}
fn prop(s: &str) -> PropertyName {
    PropertyName::try_from(s.to_owned()).unwrap()
}

#[test]
fn source_select_has_exactly_one_table() {
    let ontology = load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap();
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([
            (prop("flightId"), Pred::Const(true)),
            (prop("status"), Pred::Const(true)),
        ]),
    };
    let binding = &ontology
        .profile_bindings(&ProfileName::try_from("dev".to_owned()).unwrap())
        .unwrap()
        .objects[&name("Flight")];
    let query = ValidatedQuery::new(
        QueryRequest {
            set: ObjectSet::Type {
                object_type: name("Flight"),
            },
            select: vec![prop("status")],
        },
        vec![],
        PageSpec {
            first: 5,
            after: None,
        },
        &ontology,
        &scope,
        CompileLimits::default(),
    )
    .unwrap();
    let source =
        compile_source_select(&query, &ontology, binding, &scope, &Attrs::default()).unwrap();
    assert!(source.select.from.is_some());
    assert_eq!(source.select.limit, Some(5));
    assert_eq!(source.select.projection.len(), 2);
}

#[test]
fn collation_forces_recheck_and_disables_source_order() {
    let ontology = load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap();
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([
            (prop("flightId"), Pred::Const(true)),
            (prop("flightNumber"), Pred::Const(true)),
        ]),
    };
    let binding = &ontology
        .profile_bindings(&ProfileName::try_from("dev".to_owned()).unwrap())
        .unwrap()
        .objects[&name("Flight")];
    let query = ValidatedQuery::new(
        QueryRequest {
            set: ObjectSet::Where {
                object_type: name("Flight"),
                predicate: WhereClause::Eq {
                    property: prop("flightNumber"),
                    value: QueryLiteral(json!("LH100")),
                },
            },
            select: vec![prop("flightNumber")],
        },
        vec![OrderBy {
            property: prop("flightNumber"),
            direction: Direction::Asc,
            nulls: Nulls::Last,
        }],
        PageSpec {
            first: 10,
            after: None,
        },
        &ontology,
        &scope,
        CompileLimits::default(),
    )
    .unwrap();
    let source =
        compile_source_select(&query, &ontology, binding, &scope, &Attrs::default()).unwrap();
    assert!(source.final_recheck);
    assert!(source.select.order_by.is_empty());
    assert!(matches!(
        source.select.filter,
        Some(ontoforge_engine::printer::ast::Expr::Literal(_))
    ));
}

#[test]
fn scope_row_and_filter_are_anded_without_role_crossing() {
    let ontology = load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap();
    let binding = &ontology
        .profile_bindings(&ProfileName::try_from("dev".to_owned()).unwrap())
        .unwrap()
        .objects[&name("Flight")];
    let only_first = Pred::Cmp {
        left: Operand::Column(prop("flightId")),
        op: CmpOp::Eq,
        right: Operand::Lit(Scalar::String("F-1001".into())),
    };
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([
            (prop("flightId"), Pred::Const(true)),
            (prop("status"), only_first),
        ]),
    };
    let predicate = WhereClause::Not {
        clause: Box::new(WhereClause::Eq {
            property: prop("status"),
            value: QueryLiteral(json!("scheduled")),
        }),
    };
    let query = ValidatedQuery::new(
        QueryRequest {
            set: ObjectSet::Where {
                object_type: name("Flight"),
                predicate,
            },
            select: vec![prop("status")],
        },
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
    let source =
        compile_source_select(&query, &ontology, binding, &scope, &Attrs::default()).unwrap();
    let printed = ontoforge_engine::printer::print_select(
        &source.select,
        ontoforge_engine::printer::Dialect::Sqlite,
    )
    .unwrap();
    assert!(printed.as_str().contains("CASE WHEN"));
    assert!(printed.as_str().contains("NOT ("));
    assert!(printed.as_str().contains(" AND "));
}

#[test]
fn unbounded_page_is_rejected() {
    let ontology = load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap();
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([(prop("status"), Pred::Const(true))]),
    };
    let query = QueryRequest {
        set: ObjectSet::Type {
            object_type: name("Flight"),
        },
        select: vec![prop("status")],
    };
    assert!(matches!(
        ValidatedQuery::new(
            query,
            vec![],
            PageSpec {
                first: u64::MAX,
                after: None
            },
            &ontology,
            &scope,
            CompileLimits::default()
        ),
        Err(ontoforge_query::QueryError::QueryTooComplex)
    ));
}

#[test]
fn all_source_variants_have_one_table_and_logical_aliases() {
    let ontology = load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap();
    let binding = &ontology
        .profile_bindings(&ProfileName::try_from("dev".to_owned()).unwrap())
        .unwrap()
        .objects[&name("Flight")];
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([
            (prop("flightId"), Pred::Const(true)),
            (prop("status"), Pred::Const(true)),
        ]),
    };
    let sets = [
        ObjectSet::Pk {
            object_type: name("Flight"),
            pk: json!("F-1001"),
        },
        ObjectSet::Type {
            object_type: name("Flight"),
        },
        ObjectSet::Where {
            object_type: name("Flight"),
            predicate: WhereClause::Eq {
                property: prop("status"),
                value: QueryLiteral(json!("delayed")),
            },
        },
        ObjectSet::Search {
            object_type: name("Flight"),
            property: prop("status"),
            query: "late flight".into(),
            mode: TermMode::Any,
        },
    ];
    for set in sets {
        let is_pk = matches!(set, ObjectSet::Pk { .. });
        let query = ValidatedQuery::new(
            QueryRequest {
                set,
                select: vec![prop("status")],
            },
            vec![],
            PageSpec {
                first: 10,
                after: None,
            },
            &ontology,
            &scope,
            CompileLimits::default(),
        )
        .unwrap();
        let source =
            compile_source_select(&query, &ontology, binding, &scope, &Attrs::default()).unwrap();
        assert_eq!(source.select.from.as_ref().unwrap().table, "flights");
        assert_eq!(source.select.limit, Some(if is_pk { 2 } else { 10 }));
        assert!(
            source
                .select
                .projection
                .iter()
                .all(|item| matches!(item.alias.as_deref(), Some("flightId" | "status")))
        );
        assert!(source.required_columns.contains(&prop("flightId")));
        let sql = ontoforge_engine::printer::print_select(
            &source.select,
            ontoforge_engine::printer::Dialect::Trino,
        )
        .unwrap();
        assert_eq!(sql.as_str().matches(" FROM ").count(), 1);
        assert!(!sql.as_str().contains(" JOIN "));
        assert!(!sql.as_str().contains(" UNION "));
    }
}
