//! Merging roles into a `TypeScope` (plan 03-11 Task 3, SCOPE-02 / 03 / 05, ADR-0018).

mod rowpred_common;

use std::collections::BTreeSet;

use ontoforge_auth::scope::{RoleTypeGrant, RowVisibility, TypeScope, merge};
use ontoforge_expr::{Attrs, Pred, Tri, compile_row_filter, eval};
use ontoforge_schema::ir::PropertyName;
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;
use rowpred_common::{
    Fixture, TestRow, attrs_strategy, column_name, pred as pred_strategy, prop, row_strategy,
    with_harness,
};

fn pred(src: &str) -> Pred {
    compile_row_filter(src, &Fixture::new().cx()).unwrap_or_else(|e| panic!("{src}: {e:?}"))
}

fn props(names: &[&str]) -> BTreeSet<PropertyName> {
    names.iter().map(|n| prop(n)).collect()
}

fn grant(names: &[&str], row: Option<&str>) -> RoleTypeGrant {
    RoleTypeGrant {
        properties: props(names),
        row: row.map(pred),
    }
}

fn row(s: Option<&str>, i: Option<i64>) -> TestRow {
    TestRow {
        id: 1,
        b: None,
        i,
        d: None,
        s: s.map(str::to_owned),
        t: None,
    }
}

fn seen(scope: &TypeScope, row: &TestRow) -> RowVisibility {
    scope.evaluate(&row.values(), &Attrs::new())
}

#[test]
fn no_grant_is_invisible() {
    assert_eq!(merge(&[]), None);
}

#[test]
fn role_without_row_filter_sees_all_rows() {
    let scope = merge(&[grant(&["id", "s"], None)]).expect("scope");
    assert_eq!(scope.row, Pred::Const(true));
    assert_eq!(scope.columns.len(), 2);
    let all = RowVisibility {
        visible: true,
        columns: props(&["id", "s"]),
    };
    assert_eq!(seen(&scope, &row(None, None)), all);
    assert_eq!(seen(&scope, &row(Some("x"), Some(1))), all);
}

#[test]
fn a_columns_times_b_rows_do_not_leak() {
    let a = grant(&["id", "s", "t"], Some("object.s == 'eu'"));
    let b = grant(&["id", "i", "b"], Some("object.i > 100"));
    let scope = merge(&[a, b]).expect("scope");

    // Only role A's predicate holds: role B's columns stay hidden.
    assert_eq!(
        seen(&scope, &row(Some("eu"), Some(5))),
        RowVisibility {
            visible: true,
            columns: props(&["id", "s", "t"]),
        }
    );
    // Only role B's predicate holds: role A's columns stay hidden.
    assert_eq!(
        seen(&scope, &row(Some("us"), Some(500))),
        RowVisibility {
            visible: true,
            columns: props(&["id", "i", "b"]),
        }
    );
    // Neither holds.
    assert_eq!(
        seen(&scope, &row(Some("us"), Some(5))),
        RowVisibility {
            visible: false,
            columns: BTreeSet::new(),
        }
    );
    // Both hold: the union.
    assert_eq!(
        seen(&scope, &row(Some("eu"), Some(500))),
        RowVisibility {
            visible: true,
            columns: props(&["id", "s", "t", "i", "b"]),
        }
    );
    // An Unknown predicate (NULL column) grants nothing.
    assert_eq!(
        seen(&scope, &row(None, None)),
        RowVisibility {
            visible: false,
            columns: BTreeSet::new(),
        }
    );
}

#[test]
fn same_property_from_two_roles_is_or() {
    let a = grant(&["id", "s"], Some("object.s == 'eu'"));
    let b = grant(&["id", "i"], Some("object.i > 100"));
    let scope = merge(&[a, b]).expect("scope");
    let (pa, pb) = (pred("object.s == 'eu'"), pred("object.i > 100"));
    assert_eq!(
        scope.columns[&prop("id")],
        Pred::Or(Box::new(pa.clone()), Box::new(pb.clone()))
    );
    assert_eq!(scope.columns[&prop("s")], pa);
    assert_eq!(scope.columns[&prop("i")], pb);
    assert_eq!(scope.row, Pred::Or(Box::new(pa), Box::new(pb)));
}

#[test]
fn identical_role_predicates_appear_once() {
    let a = grant(&["id", "s"], Some("object.s == 'eu'"));
    let b = grant(&["id", "t"], Some("object.s == 'eu'"));
    let scope = merge(&[a, b]).expect("scope");
    let p = pred("object.s == 'eu'");
    assert_eq!(scope.columns[&prop("id")], p);
    assert_eq!(scope.row, p);
}

#[test]
fn a_role_without_filter_makes_the_row_predicate_true() {
    let a = grant(&["id"], Some("object.s == 'eu'"));
    let b = grant(&["id", "s"], None);
    let scope = merge(&[a, b]).expect("scope");
    assert_eq!(scope.row, Pred::Const(true));
    assert_eq!(scope.columns[&prop("id")], Pred::Const(true));
    assert_eq!(scope.columns[&prop("s")], Pred::Const(true));
}

#[test]
fn a_role_with_only_the_key_column_still_makes_rows_visible() {
    let scope = merge(&[grant(&["id"], Some("object.s == 'eu'"))]).expect("scope");
    assert_eq!(
        seen(&scope, &row(Some("eu"), None)),
        RowVisibility {
            visible: true,
            columns: props(&["id"]),
        }
    );
}

fn config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(64);
    ProptestConfig {
        cases,
        failure_persistence: Some(Box::new(FileFailurePersistence::WithSource(
            "proptest-regressions",
        ))),
        ..ProptestConfig::default()
    }
}

fn grants_strategy() -> BoxedStrategy<Vec<RoleTypeGrant>> {
    let one = (
        prop::collection::btree_set(prop::sample::select(vec!["b", "i", "d", "s", "t"]), 0..5),
        prop::option::of(pred_strategy()),
    )
        .prop_map(|(names, row)| RoleTypeGrant {
            properties: names.into_iter().map(prop).collect(),
            row,
        });
    prop::collection::vec(one, 1..5).boxed()
}

/// The visibility the definition gives: the union of the column sets of the roles whose
/// predicate is True.
fn expected(grants: &[RoleTypeGrant], row: &TestRow, attrs: &Attrs) -> RowVisibility {
    let mut visible = false;
    let mut columns = BTreeSet::new();
    for g in grants {
        let holds = g
            .row
            .as_ref()
            .is_none_or(|p| eval(p, &row.values(), attrs) == Tri::True);
        if holds {
            visible = true;
            columns.extend(g.properties.iter().cloned());
        }
    }
    RowVisibility { visible, columns }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn visible_columns_are_union_of_true_grants(
        grants in grants_strategy(),
        r in row_strategy(),
        a in attrs_strategy(),
    ) {
        let scope = merge(&grants).expect("at least one grant");
        prop_assert_eq!(scope.evaluate(&r.values(), &a), expected(&grants, &r, &a));
    }

    #[test]
    fn lowered_scope_matches_evaluate_on_sqlite(
        grants in grants_strategy(),
        r in row_strategy(),
        a in attrs_strategy(),
    ) {
        let scope = merge(&grants).expect("at least one grant");
        let lowered = scope.lower(&a, &column_name);
        prop_assert_eq!(
            lowered.columns.keys().collect::<Vec<_>>(),
            scope.columns.keys().collect::<Vec<_>>()
        );

        let row_tri = with_harness(|h| h.tri_of(&lowered.row, &r));
        let visible = row_tri == Tri::True;
        let columns: BTreeSet<PropertyName> = lowered
            .columns
            .iter()
            .filter(|(_, e)| visible && with_harness(|h| h.tri_of(e, &r)) == Tri::True)
            .map(|(c, _)| c.clone())
            .collect();
        prop_assert_eq!(
            RowVisibility { visible, columns },
            scope.evaluate(&r.values(), &a)
        );
    }
}
