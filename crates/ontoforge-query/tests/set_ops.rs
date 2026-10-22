use std::collections::BTreeSet;

use ontoforge_query::set::{SetOperator, evaluate_set_operation};
use ontoforge_query::ast::ObjectSet;
use ontoforge_query::ast::PageSpec;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{QueryError, QueryRequest};
use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::Pred;
use ontoforge_schema::ir::ObjectTypeName;
use ontoforge_schema::load_dir;
use camino::Utf8Path;
use proptest::prelude::*;
use serde_json::json;

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
            QueryRequest { set, select: vec![] }, vec![],
            PageSpec { first: 10, after: None }, &ontology, &scope,
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
