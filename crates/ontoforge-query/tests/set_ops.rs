use std::collections::BTreeSet;

use camino::Utf8Path;
use camino::Utf8PathBuf;
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_auth::scope::TypeScope;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::ast::ObjectSet;
use ontoforge_query::ast::PageSpec;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::set::{SetOperator, evaluate_set_operation};
use ontoforge_query::{ExecutionLimits, QueryService, TokenCodec};
use ontoforge_query::{QueryError, QueryRequest};
use ontoforge_schema::ir::ObjectTypeName;
use ontoforge_schema::ir::{ProfileName, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use proptest::prelude::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn nested_set_wire_accepts_logical_names_only() {
    let wire = json!({
        "kind": "union",
        "object_type": "Flight",
        "sets": [
            {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
            {"kind": "subtract", "object_type": "Flight",
             "left": {"kind": "type", "object_type": "Flight"},
             "right": {"kind": "pk", "object_type": "Flight", "pk": "F-1002"}}
        ]
    });
    let parsed: ObjectSet = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
}

#[test]
fn valid_same_type_set_and_cross_type_rejection() {
    let ontology = load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap();
    let flight = ObjectTypeName::try_from("Flight".to_owned()).unwrap();
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: ontology.object_types[&flight]
            .properties
            .keys()
            .cloned()
            .map(|name| (name, Pred::Const(true)))
            .collect(),
    };
    let wire = json!({
        "kind": "union", "object_type": "Flight", "sets": [
            {"kind": "type", "object_type": "Flight"},
            {"kind": "pk", "object_type": "Flight", "pk": "F-1001"}
        ]
    });
    let validate = |set| {
        ValidatedQuery::new(
            QueryRequest {
                set,
                select: vec![],
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
    };
    assert!(validate(serde_json::from_value(wire.clone()).unwrap()).is_ok());
    let mut cross_type = wire;
    cross_type["sets"][1]["object_type"] = json!("Airport");
    assert!(matches!(
        validate(serde_json::from_value(cross_type).unwrap()),
        Err(QueryError::InvalidQuery)
    ));

    let empty: ObjectSet = serde_json::from_value(json!({
        "kind": "union", "object_type": "Flight", "sets": []
    })).unwrap();
    assert!(matches!(validate(empty), Err(QueryError::InvalidQuery)));

    let nested: ObjectSet = serde_json::from_value(json!({
        "kind": "union", "object_type": "Flight", "sets": [
            {"kind": "intersect", "object_type": "Flight", "sets": [
                {"kind": "type", "object_type": "Flight"}
            ]}
        ]
    })).unwrap();
    let tight = CompileLimits { max_depth: 2, ..CompileLimits::default() };
    assert!(matches!(ValidatedQuery::new(
        QueryRequest { set: nested, select: vec![] }, vec![],
        PageSpec { first: 10, after: None }, &ontology, &scope, tight,
    ), Err(QueryError::QueryTooComplex)));
}

#[tokio::test]
async fn set_result_reuses_normal_hydration_order_and_cursor() {
    let root = Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let fixtures = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = SqliteEngine::from_fixtures(root, &fixtures.tables)
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite {
            path: Utf8PathBuf::from_path_buf(dir.path().join("edits.db")).unwrap(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = store.active_definition().await.unwrap();
    let flight = ObjectTypeName::try_from("Flight".to_owned()).unwrap();
    let property = |raw: &str| PropertyName::try_from(raw.to_owned()).unwrap();
    let scope = PrincipalScope {
        types: std::iter::once((
            flight.clone(),
            TypeScope {
                row: Pred::Const(true),
                columns: ontology.object_types[&flight]
                    .properties
                    .keys()
                    .cloned()
                    .map(|name| (name, Pred::Const(true)))
                    .collect(),
            },
        ))
        .collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "set-reader"),
        issuer: "test".into(),
        subject: "set-reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let tokens = TokenCodec::new(b"set-test".to_vec()).unwrap();
    let service = QueryService {
        principal: &principal,
        scope: &scope,
        active: Arc::clone(&active),
        engine: &engine,
        store: &store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let request = QueryRequest {
        set: serde_json::from_value(json!({
            "kind": "union", "object_type": "Flight", "sets": [
                {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
                {"kind": "pk", "object_type": "Flight", "pk": "F-1002"}
            ]
        }))
        .unwrap(),
        select: vec![property("status")],
    };
    let query = |after| {
        ValidatedQuery::new(
            request.clone(),
            vec![],
            PageSpec { first: 1, after },
            &ontology,
            &scope.types[&flight],
            CompileLimits::default(),
        )
        .unwrap()
    };
    let first = service.execute_list(&query(None)).await.unwrap();
    assert_eq!(first.rows.len(), 1);
    assert_eq!(first.rows[0].pk, json!("F-1001"));
    assert!(first.has_more);
    let second = service.execute_list(&query(first.cursor)).await.unwrap();
    assert_eq!(second.rows.len(), 1);
    assert_eq!(second.rows[0].pk, json!("F-1002"));
    assert!(!second.has_more);

    store.commit(Commit {
        actor: Actor::Principal(principal.id.to_string()),
        action: "setTestTombstone".to_owned(),
        def_version: active.def_version,
        params: json!({}),
        edits: vec![Edit::Object {
            type_name: flight.clone(),
            pk: PropertyValue::String("F-1001".to_owned()),
            op: ObjectOp::Tombstone,
        }],
    }).await.unwrap();
    let after_tombstone = service.execute_list(&query(None)).await.unwrap();
    assert_eq!(after_tombstone.rows.iter().map(|row| &row.pk).collect::<Vec<_>>(), vec![&json!("F-1002")]);

    let hidden_scope = PrincipalScope {
        types: std::iter::once((flight.clone(), TypeScope {
            row: Pred::Const(false),
            columns: scope.types[&flight].columns.clone(),
        })).collect(),
        ..PrincipalScope::default()
    };
    let hidden = QueryService { scope: &hidden_scope, ..service };
    assert!(hidden.execute_list(&query(None)).await.unwrap().rows.is_empty());
}

fn reference(operator: SetOperator, operands: &[Vec<u8>]) -> BTreeSet<String> {
    let sets = operands
        .iter()
        .map(|keys| keys.iter().map(u8::to_string).collect::<BTreeSet<_>>())
        .collect::<Vec<_>>();
    match operator {
        SetOperator::Union => sets.into_iter().flatten().collect(),
        SetOperator::Intersect => sets
            .split_first()
            .map(|(first, rest)| {
                first
                    .iter()
                    .filter(|key| rest.iter().all(|set| set.contains(*key)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
        SetOperator::Subtract => sets
            .split_first()
            .map(|(first, rest)| {
                first
                    .iter()
                    .filter(|key| rest.iter().all(|set| !set.contains(*key)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
    }
}

#[test]
fn set_ops_match_independent_btreeset_model_fixed() {
    let operands = vec![vec![1, 2, 2, 3], vec![2, 3, 4], vec![3, 5]];
    let sets = operands
        .iter()
        .map(|keys| keys.iter().map(u8::to_string).collect())
        .collect::<Vec<_>>();
    for operator in [
        SetOperator::Union,
        SetOperator::Intersect,
        SetOperator::Subtract,
    ] {
        assert_eq!(
            evaluate_set_operation(operator, &sets).unwrap(),
            reference(operator, &operands),
            "{operator:?}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn set_ops_match_independent_btreeset_model(
        operands in prop::collection::vec(prop::collection::vec(0_u8..16, 0..24), 1..5),
    ) {
        let sets = operands.iter().map(|keys| {
            keys.iter().map(u8::to_string).collect::<BTreeSet<_>>()
        }).collect::<Vec<_>>();
        for operator in [SetOperator::Union, SetOperator::Intersect, SetOperator::Subtract] {
            prop_assert_eq!(
                evaluate_set_operation(operator, &sets).unwrap(),
                reference(operator, &operands),
                "{:?}", operator
            );
        }
    }
}
