//! Row predicates: Rust `eval` against the lowered SQL executed by SQLite (plan 03-11).
//!
//! The tracer cases come first (Task 1); the full differential proptest and the fixed
//! corpus follow in Task 2.

mod rowpred_common;

use ontoforge_auth::scope::lower;
use ontoforge_expr::{AttrValue, Attrs, Pred, Scalar, Tri, compile_row_filter, eval};
use rowpred_common::{Fixture, TestRow, column_name, with_harness};

fn pred(src: &str) -> Pred {
    compile_row_filter(src, &Fixture::new().cx()).unwrap_or_else(|e| panic!("{src}: {e:?}"))
}

fn row(s: Option<&str>) -> TestRow {
    TestRow {
        id: 1,
        b: None,
        i: None,
        d: None,
        s: s.map(str::to_owned),
        t: None,
    }
}

fn attrs(entries: &[(&str, AttrValue)]) -> Attrs {
    let mut out = Attrs::new();
    for (name, value) in entries {
        out.insert(*name, value.clone()).expect("attribute");
    }
    out
}

fn text(s: &str) -> AttrValue {
    AttrValue::Scalar(Scalar::String(s.to_owned()))
}

/// `(sqlite, rust)` truth values of `src` on `row`.
fn both(src: &str, row: &TestRow, attrs: &Attrs) -> (Tri, Tri) {
    let p = pred(src);
    let lowered = lower(&p, attrs, &column_name);
    let sqlite = with_harness(|h| h.tri_of(&lowered, row));
    (sqlite, eval(&p, &row.values(), attrs))
}

fn assert_same(src: &str, row: &TestRow, attrs: &Attrs, expected: Tri) {
    let (sqlite, rust) = both(src, row, attrs);
    assert_eq!(rust, expected, "Rust eval of {src}");
    assert_eq!(sqlite, expected, "SQLite for {src}");
}

#[test]
fn tenant_equality_matches() {
    let src = "object.s == principal.attrs.tenant";
    let r = row(Some("acme"));
    assert_same(src, &r, &attrs(&[("tenant", text("acme"))]), Tri::True);
    assert_same(src, &r, &attrs(&[("tenant", text("other"))]), Tri::False);
    assert_same(src, &r, &attrs(&[]), Tri::Unknown);
}

#[test]
fn null_column_comparison_is_unknown_on_both_sides() {
    assert_same("object.s != 'a'", &row(None), &attrs(&[]), Tri::Unknown);
}

#[test]
fn empty_list_attr_is_false_on_both_sides() {
    let src = "object.s in principal.attrs.regions";
    let empty = attrs(&[("regions", AttrValue::List(Vec::new()))]);
    // `x in []` is False even for a NULL column (ADR-0025 rule 1).
    assert_same(src, &row(None), &empty, Tri::False);
    assert_same(src, &row(Some("eu")), &empty, Tri::False);
    // A missing list attribute is NULL, not empty: Unknown.
    assert_same(src, &row(Some("eu")), &attrs(&[]), Tri::Unknown);
}
