use std::collections::BTreeSet;

use ontoforge_query::set::{SetOperator, evaluate_set_operation};
use ontoforge_query::ast::ObjectSet;
use proptest::prelude::*;
use serde_json::json;

#[test]
fn nested_set_wire_accepts_logical_names_only() {
    let wire = json!({
        "kind": "union",
        "objectType": "Flight",
        "sets": [
            {"kind": "pk", "objectType": "Flight", "pk": "F-1001"},
            {"kind": "subtract", "objectType": "Flight",
             "left": {"kind": "type", "objectType": "Flight"},
             "right": {"kind": "pk", "objectType": "Flight", "pk": "F-1002"}}
        ]
    });
    let parsed: ObjectSet = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
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
