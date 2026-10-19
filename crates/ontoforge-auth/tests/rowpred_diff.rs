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

// ---------------------------------------------------------------------------------------
// Task 2: the differential proptest and the fixed corpus.
// ---------------------------------------------------------------------------------------

use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;
use rowpred_common::{
    attrs_strategy, fixed_seed_cases, load_corpus, pred as pred_strategy, row_strategy, rust_all,
};

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

proptest! {
    #![proptest_config(config())]

    /// ADR-0025: the lowered SQL executed by SQLite and `eval` give the same truth value for
    /// every predicate, row and set of principal attributes.
    #[test]
    fn rust_eval_equals_sqlite(p in pred_strategy(), r in row_strategy(), a in attrs_strategy()) {
        let lowered = lower(&p, &a, &column_name);
        let sqlite = with_harness(|h| h.tri_of(&lowered, &r));
        prop_assert_eq!(sqlite, eval(&p, &r.values(), &a), "predicate: {}", p);
    }
}

#[test]
fn corpus_cases_match_on_sqlite() {
    let corpus = load_corpus();
    assert!(corpus.rows.len() >= 20 && corpus.cases.len() >= 30);
    for case in &corpus.cases {
        let p = case.pred();
        let lowered = lower(&p, &case.attrs, &column_name);
        let sqlite = with_harness(|h| {
            h.load_rows(&corpus.rows);
            h.tri_all(&lowered)
        });
        assert_eq!(
            sqlite,
            rust_all(&p, &case.attrs, &corpus.rows),
            "corpus case {}",
            case.src
        );
    }
}

/// The 256 seeded predicates the Trino test runs, on the corpus rows, against SQLite.
#[test]
fn fixed_seed_cases_match_on_sqlite() {
    let corpus = load_corpus();
    for (p, a) in fixed_seed_cases(256) {
        let lowered = lower(&p, &a, &column_name);
        let sqlite = with_harness(|h| {
            h.load_rows(&corpus.rows);
            h.tri_all(&lowered)
        });
        assert_eq!(sqlite, rust_all(&p, &a, &corpus.rows), "predicate: {p}");
    }
}
