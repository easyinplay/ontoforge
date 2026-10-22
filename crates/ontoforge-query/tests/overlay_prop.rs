use std::collections::BTreeMap;

use camino::Utf8Path;
use ontoforge_auth::scope::{RoleTypeGrant, merge};
use ontoforge_engine::sqlite::fixture::normalize_decimal;
use ontoforge_expr::{Attrs, CmpOp, Operand, Pred, Scalar};
use ontoforge_query::ast::{Direction, Nulls, ObjectSet, OrderBy, QueryLiteral, WhereClause};
use ontoforge_query::execute::WorkBudget;
use ontoforge_query::overlay::{
    NormalizedOrder, compare_rows, compare_rows_normalized, eligible_row, overlay_candidates,
    overlay_object,
};
use ontoforge_query::{ExecutionLimits, QueryError};
use ontoforge_schema::ir::{BaseType, Collation, ObjectTypeName, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::query_read::ObjectEdit;
use ontoforge_storage::state::EditRow;
use ontoforge_storage::value::PropertyValue;
use proptest::prelude::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}

fn flight() -> ontoforge_schema::ir::ObjectType {
    load_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    )))
    .ontology
    .unwrap()
    .object_types[&ObjectTypeName::try_from("Flight".to_owned()).unwrap()]
        .clone()
}

// This model deliberately does not call the production merge, filter, or sort helpers.
fn reference(
    source: Option<BTreeMap<PropertyName, Value>>,
    edit: Option<&ObjectEdit>,
    pk: i64,
) -> Option<BTreeMap<PropertyName, Value>> {
    if edit.is_some_and(|edit| edit.row.deleted) {
        return None;
    }
    let mut row = match source {
        Some(row) => row,
        None if edit.is_some_and(|edit| !edit.row.released) => BTreeMap::new(),
        None => return None,
    };
    row.insert(prop("flightId"), json!(pk));
    if let Some(edit) = edit.filter(|edit| !edit.row.released) {
        for (property, value) in &edit.row.set {
            row.insert(property.clone(), value.to_json());
        }
    }
    Some(row)
}

#[test]
fn tombstone_release_and_duplicate_candidates_are_idempotent() {
    let source = BTreeMap::from([
        (prop("flightId"), json!(7)),
        (prop("status"), json!("source")),
    ]);
    let edit = ObjectEdit {
        pk: PropertyValue::Int(7),
        row: EditRow {
            set: BTreeMap::from([(prop("status"), PropertyValue::String("patched".into()))]),
            deleted: false,
            released: false,
            version: 2,
        },
    };
    let rows = overlay_candidates(
        vec![
            (PropertyValue::Int(7), source.clone()),
            (PropertyValue::Int(7), source.clone()),
        ],
        vec![edit.clone(), edit.clone()],
        &prop("flightId"),
    )
    .unwrap();
    assert_eq!(
        rows,
        vec![BTreeMap::from([
            (prop("flightId"), json!(7)),
            (prop("status"), json!("patched"))
        ])]
    );
    let mut released = edit.clone();
    released.row.released = true;
    assert_eq!(
        overlay_object(
            Some(&source),
            Some(&released),
            &prop("flightId"),
            &released.pk
        )
        .unwrap(),
        Some(source.clone())
    );
    released.row.released = false;
    released.row.deleted = true;
    assert_eq!(
        overlay_object(
            Some(&source),
            Some(&released),
            &prop("flightId"),
            &released.pk
        )
        .unwrap(),
        None
    );
    let edit_only = overlay_object(None, Some(&edit), &prop("flightId"), &edit.pk)
        .unwrap()
        .unwrap();
    assert_eq!(edit_only[&prop("status")], json!("patched"));
}

#[test]
fn overlay_partial_patch_and_explicit_null() {
    let source = BTreeMap::from([
        (prop("flightId"), json!(7)),
        (prop("status"), json!("old")),
        (prop("delayMinutes"), json!(12)),
    ]);
    let edit = ObjectEdit {
        pk: PropertyValue::Int(7),
        row: EditRow {
            set: BTreeMap::from([
                (prop("status"), PropertyValue::String("new".into())),
                (prop("delayMinutes"), PropertyValue::Null),
            ]),
            deleted: false,
            released: false,
            version: 1,
        },
    };
    let actual = overlay_object(Some(&source), Some(&edit), &prop("flightId"), &edit.pk).unwrap();
    let expected = reference(Some(source), Some(&edit), 7);
    assert_eq!(actual, expected);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn overlay_matches_independent_reference(
        source_value in -100_i64..100,
        patch in prop::option::of(-100_i64..100),
        deleted in any::<bool>(),
        released in any::<bool>(),
    ) {
        prop_assume!(!deleted || !released);
        let source = BTreeMap::from([
            (prop("flightId"), json!(7)),
            (prop("delayMinutes"), json!(source_value)),
        ]);
        let edit = ObjectEdit {
            pk: PropertyValue::Int(7),
            row: EditRow {
                set: patch.map_or_else(BTreeMap::new, |value| {
                    BTreeMap::from([(prop("delayMinutes"), PropertyValue::Int(value))])
                }),
                deleted,
                released,
                version: 1,
            },
        };
        let actual = overlay_object(Some(&source), Some(&edit), &prop("flightId"), &edit.pk).unwrap();
        let expected = reference(Some(source), Some(&edit), 7);
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn overlay_multikey_order_matches_reference(
        delays in prop::collection::vec(0_i64..4, 3),
        statuses in prop::collection::vec(any::<bool>(), 3),
        reverse_input in any::<bool>(),
    ) {
        let ty = flight();
        let mut rows = (0..3).map(|index| BTreeMap::from([
            (prop("flightId"), json!(i64::try_from(index + 1).unwrap())),
            (prop("delayMinutes"), json!(delays[index])),
            (prop("status"), json!(if statuses[index] { "z" } else { "a" })),
        ])).collect::<Vec<_>>();
        if reverse_input {
            rows.reverse();
        }
        let order = [
            OrderBy { property: prop("delayMinutes"), direction: Direction::Asc, nulls: Nulls::Last },
            OrderBy { property: prop("status"), direction: Direction::Desc, nulls: Nulls::First },
        ];
        rows.sort_by(|a, b| compare_rows(a, b, &order, &ty.primary_key, &ty));
        let actual = rows.iter().map(|row| row[&ty.primary_key].as_i64().unwrap()).collect::<Vec<_>>();
        let mut expected = (0..3).map(|index| (delays[index], statuses[index], i64::try_from(index + 1).unwrap())).collect::<Vec<_>>();
        expected.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)).then_with(|| a.2.cmp(&b.2)));
        prop_assert_eq!(actual, expected.into_iter().map(|item| item.2).collect::<Vec<_>>());
    }
}

#[test]
fn scope_and_predicate_run_on_merged_row() {
    let ty = flight();
    let source = BTreeMap::from([
        (prop("flightId"), json!(7)),
        (prop("status"), json!("old")),
        (prop("delayMinutes"), json!(12)),
    ]);
    let edit = ObjectEdit {
        pk: PropertyValue::Int(7),
        row: EditRow {
            set: BTreeMap::from([(prop("status"), PropertyValue::String("new".into()))]),
            deleted: false,
            released: false,
            version: 1,
        },
    };
    let merged = overlay_object(Some(&source), Some(&edit), &ty.primary_key, &edit.pk)
        .unwrap()
        .unwrap();
    let scope = merge(&[RoleTypeGrant {
        properties: BTreeSet::from([prop("flightId"), prop("status")]),
        row: Some(Pred::Cmp {
            left: Operand::Column(prop("status")),
            op: CmpOp::Eq,
            right: Operand::Lit(Scalar::String("new".into())),
        }),
    }])
    .unwrap();
    let set = ObjectSet::Where {
        object_type: ObjectTypeName::try_from("Flight".to_owned()).unwrap(),
        predicate: WhereClause::Eq {
            property: prop("status"),
            value: QueryLiteral(json!("new")),
        },
    };
    assert!(eligible_row(&source, &set, &ty, &scope, &Attrs::default()).is_none());
    assert_eq!(
        eligible_row(&merged, &set, &ty, &scope, &Attrs::default()).unwrap()[&prop("status")],
        json!("new")
    );
}

#[test]
fn hidden_columns_never_reach_sort_filter_or_projection() {
    let ty = flight();
    let scope = merge(&[
        RoleTypeGrant {
            properties: BTreeSet::from([prop("flightId"), prop("status")]),
            row: Some(Pred::Cmp {
                left: Operand::Column(prop("status")),
                op: CmpOp::Eq,
                right: Operand::Lit(Scalar::String("public".into())),
            }),
        },
        RoleTypeGrant {
            properties: BTreeSet::from([prop("flightId"), prop("delayMinutes")]),
            row: None,
        },
    ])
    .unwrap();
    let one = BTreeMap::from([
        (prop("flightId"), json!(1)),
        (prop("status"), json!("secret-a")),
        (prop("delayMinutes"), json!(4)),
    ]);
    let two = BTreeMap::from([
        (prop("flightId"), json!(2)),
        (prop("status"), json!("secret-z")),
        (prop("delayMinutes"), json!(4)),
    ]);
    let set = ObjectSet::Where {
        object_type: ObjectTypeName::try_from("Flight".to_owned()).unwrap(),
        predicate: WhereClause::Eq {
            property: prop("status"),
            value: QueryLiteral(json!("secret-a")),
        },
    };
    assert!(eligible_row(&one, &set, &ty, &scope, &Attrs::default()).is_none());
    let all = ObjectSet::Type {
        object_type: ObjectTypeName::try_from("Flight".to_owned()).unwrap(),
    };
    let masked_one = eligible_row(&one, &all, &ty, &scope, &Attrs::default()).unwrap();
    let masked_two = eligible_row(&two, &all, &ty, &scope, &Attrs::default()).unwrap();
    assert!(!masked_one.contains_key(&prop("status")));
    assert!(!masked_two.contains_key(&prop("status")));
    let order = [OrderBy {
        property: prop("status"),
        direction: Direction::Asc,
        nulls: Nulls::Last,
    }];
    assert_eq!(
        compare_rows(&masked_one, &masked_two, &order, &ty.primary_key, &ty),
        std::cmp::Ordering::Less
    );
}

#[test]
fn ordering_is_total_and_deterministic() {
    let ty = flight();
    let rows = [
        BTreeMap::from([
            (prop("flightId"), json!(3)),
            (prop("delayMinutes"), json!(null)),
        ]),
        BTreeMap::from([
            (prop("flightId"), json!(1)),
            (prop("delayMinutes"), json!(5)),
        ]),
        BTreeMap::from([
            (prop("flightId"), json!(2)),
            (prop("delayMinutes"), json!(5)),
        ]),
    ];
    for direction in [Direction::Asc, Direction::Desc] {
        for nulls in [Nulls::First, Nulls::Last] {
            let order = [OrderBy {
                property: prop("delayMinutes"),
                direction,
                nulls,
            }];
            let mut sorted = rows.to_vec();
            sorted.sort_by(|a, b| compare_rows(a, b, &order, &ty.primary_key, &ty));
            let actual = sorted
                .iter()
                .map(|row| row[&ty.primary_key].as_i64().unwrap())
                .collect::<Vec<_>>();
            let expected = if nulls == Nulls::First {
                vec![3, 1, 2]
            } else {
                vec![1, 2, 3]
            };
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn ordering_case_insensitive_unicode_uses_final_rust_recheck() {
    let ty = flight();
    let first = BTreeMap::from([
        (prop("flightId"), json!(2)),
        (prop("status"), json!("Äpfel")),
    ]);
    let second = BTreeMap::from([
        (prop("flightId"), json!(1)),
        (prop("status"), json!("äPFEL")),
    ]);
    let key = NormalizedOrder {
        order: OrderBy {
            property: prop("status"),
            direction: Direction::Asc,
            nulls: Nulls::Last,
        },
        collation: Collation::CaseInsensitive,
        final_recheck: true,
    };
    assert_eq!(
        compare_rows_normalized(&first, &second, &[key], &ty.primary_key, &ty),
        std::cmp::Ordering::Greater
    );
}

#[test]
fn ordering_decimal_uses_numeric_magnitude() {
    let mut ty = flight();
    let def = ty.properties.get_mut(&prop("delayMinutes")).unwrap();
    def.base_type = BaseType::Decimal;
    def.precision = Some(38);
    def.scale = Some(4);
    let order = [OrderBy {
        property: prop("delayMinutes"),
        direction: Direction::Asc,
        nulls: Nulls::Last,
    }];
    let rows = [
        ("F-01", "2.0"),
        ("F-02", "10.00"),
        ("F-03", "-2.000"),
        ("F-04", "-10.00"),
        ("F-05", "0"),
        ("F-06", "-0.00"),
        ("F-07", "2.00"),
        ("F-08", "9999999999999999999999999999999999.9999"),
        ("F-09", "-9999999999999999999999999999999999.9999"),
        ("F-10", "0.0001"),
        ("F-11", "-0.0001"),
    ];
    let mut reference = rows
        .iter()
        .map(|(pk, value)| {
            let canonical = normalize_decimal(value, 38, 4).unwrap();
            (*pk, canonical.replace('.', "").parse::<i128>().unwrap())
        })
        .collect::<Vec<_>>();
    reference.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(right.0)));
    let mut rows = rows
        .into_iter()
        .map(|(pk, value)| {
            BTreeMap::from([
                (prop("flightId"), json!(pk)),
                (prop("delayMinutes"), json!(value)),
            ])
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| compare_rows(a, b, &order, &ty.primary_key, &ty));
    assert_eq!(
        rows.iter()
            .map(|row| row[&ty.primary_key].as_str().unwrap())
            .collect::<Vec<_>>(),
        reference.iter().map(|(pk, _)| *pk).collect::<Vec<_>>()
    );
}

#[test]
fn pagination_execution_limit_boundaries_are_inclusive() {
    let defaults = ExecutionLimits::phase4_default();
    assert_eq!(
        (
            defaults.max_backend_calls,
            defaults.max_raw_candidates,
            defaults.max_backfill_keys,
            defaults.max_decoded_bytes
        ),
        (32, 10_000, 2_048, 64 * 1024 * 1024)
    );
    let mut budget = WorkBudget::new(defaults);
    budget.charge_backend_calls(32).unwrap();
    budget.charge_raw_candidates(10_000).unwrap();
    budget.charge_backfill_keys(2_048).unwrap();
    budget.charge_decoded_bytes(64 * 1024 * 1024).unwrap();
    assert!(matches!(
        budget.charge_backend_calls(1),
        Err(QueryError::WorkBudgetExceeded)
    ));
    assert!(matches!(
        budget.charge_raw_candidates(1),
        Err(QueryError::WorkBudgetExceeded)
    ));
    assert!(matches!(
        budget.charge_backfill_keys(1),
        Err(QueryError::WorkBudgetExceeded)
    ));
    assert!(matches!(
        budget.charge_decoded_bytes(1),
        Err(QueryError::WorkBudgetExceeded)
    ));
}

#[test]
fn pagination_budget_rejects_overflow_and_untrusted_limits() {
    for invalid in [
        ExecutionLimits::try_new(0, 1, 1, 1),
        ExecutionLimits::try_new(1, 0, 1, 1),
        ExecutionLimits::try_new(1, 1, 0, 1),
        ExecutionLimits::try_new(1, 1, 1, 0),
        ExecutionLimits::try_new(usize::MAX, 1, 1, 1),
    ] {
        assert!(matches!(invalid, Err(QueryError::InvalidQuery)));
    }
    let mut budget = WorkBudget::new(ExecutionLimits::phase4_default());
    assert!(matches!(
        budget.charge_raw_candidates(usize::MAX),
        Err(QueryError::WorkBudgetExceeded)
    ));
}
