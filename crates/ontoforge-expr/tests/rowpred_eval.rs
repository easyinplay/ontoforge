//! Row predicate evaluation: the tracer (plan 03-05 Task 2) and the Kleene laws
//! (Task 3).

mod common;

use std::collections::BTreeMap;

use common::Fixture;
use ontoforge_expr::{
    AttrValue, Attrs, CmpOp, ListSource, MatchKind, Operand, Pred, Scalar, Tri, compile_row_filter,
    eval,
};
use ontoforge_schema::ir::PropertyName;
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

fn prop(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property name")
}

fn row(values: &[(&str, Scalar)]) -> BTreeMap<PropertyName, Scalar> {
    values
        .iter()
        .map(|(name, v)| (prop(name), v.clone()))
        .collect()
}

fn text(s: &str) -> Scalar {
    Scalar::String(s.to_owned())
}

fn attrs(values: &[(&str, AttrValue)]) -> Attrs {
    let mut out = Attrs::new();
    for (name, v) in values {
        out.insert(*name, v.clone()).expect("attr");
    }
    out
}

/// Compiles `src` against the fixture and evaluates it on one row.
fn visible(src: &str, row: &BTreeMap<PropertyName, Scalar>, attrs: &Attrs) -> Tri {
    let fx = Fixture::new();
    let pred = compile_row_filter(src, &fx.cx()).unwrap_or_else(|e| panic!("{src}: {e:?}"));
    eval(&pred, row, attrs)
}

fn tenant(value: &str) -> Attrs {
    attrs(&[("tenantId", AttrValue::Scalar(text(value)))])
}

#[test]
fn tenant_equality_visible() {
    let fx = Fixture::new();
    let pred = compile_row_filter("object.tenantId == principal.attrs.tenantId", &fx.cx());
    assert_eq!(
        pred,
        Ok(Pred::Cmp {
            left: Operand::Column(prop("tenantId")),
            op: CmpOp::Eq,
            right: Operand::Attr("tenantId".to_owned()),
        })
    );
    let r = row(&[("tenantId", text("acme"))]);
    assert_eq!(
        visible(
            "object.tenantId == principal.attrs.tenantId",
            &r,
            &tenant("acme")
        ),
        Tri::True
    );
}

#[test]
fn tenant_mismatch_false() {
    let r = row(&[("tenantId", text("acme"))]);
    assert_eq!(
        visible(
            "object.tenantId == principal.attrs.tenantId",
            &r,
            &tenant("other")
        ),
        Tri::False
    );
}

#[test]
fn missing_attr_is_unknown_and_invisible() {
    let r = row(&[("tenantId", text("acme"))]);
    let t = visible(
        "object.tenantId == principal.attrs.tenantId",
        &r,
        &Attrs::new(),
    );
    assert_eq!(t, Tri::Unknown);
    assert!(!t.is_true());
}

#[test]
fn null_column_not_equal_is_unknown() {
    let r = row(&[("tenantId", Scalar::Null)]);
    assert_eq!(
        visible("object.tenantId != 'a'", &r, &Attrs::new()),
        Tri::Unknown
    );
    // An absent column is Null too.
    assert_eq!(
        visible("object.tenantId != 'a'", &row(&[]), &Attrs::new()),
        Tri::Unknown
    );
}

#[test]
fn explicit_null_branch_visible() {
    let src = "object.tenantId == null || object.tenantId == principal.attrs.tenantId";
    let r = row(&[("tenantId", Scalar::Null)]);
    assert_eq!(visible(src, &r, &tenant("acme")), Tri::True);
    // Pitfall 4 (accepted, ADR-0025): the explicit null branch also lets a principal
    // without the attribute see unassigned rows.
    assert_eq!(visible(src, &r, &Attrs::new()), Tri::True);
}

#[test]
fn unknown_root_rejected() {
    let fx = Fixture::new();
    let issues = compile_row_filter("request.ip == '1'", &fx.cx()).expect_err("unknown root");
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].pos, Some((1, 1)));
}

// ---------------------------------------------------------------------------
// Task 3: `in`, string matching, ordering
// ---------------------------------------------------------------------------

fn regions(items: &[&str]) -> AttrValue {
    AttrValue::List(items.iter().map(|s| text(s)).collect())
}

#[test]
fn in_literal_list_is_three_valued() {
    let src = "object.region in ['eu', 'us']";
    let none = Attrs::new();
    assert_eq!(
        visible(src, &row(&[("region", text("eu"))]), &none),
        Tri::True
    );
    assert_eq!(
        visible(src, &row(&[("region", text("ap"))]), &none),
        Tri::False
    );
    // A Null left side against a non-empty list is unknown.
    assert_eq!(
        visible(src, &row(&[("region", Scalar::Null)]), &none),
        Tri::Unknown
    );
}

#[test]
fn in_empty_list_is_false_even_for_null() {
    let none = Attrs::new();
    let src = "object.region in []";
    assert_eq!(
        visible(src, &row(&[("region", text("eu"))]), &none),
        Tri::False
    );
    assert_eq!(
        visible(src, &row(&[("region", Scalar::Null)]), &none),
        Tri::False
    );
}

#[test]
fn in_attribute_list_follows_the_same_rules() {
    let src = "object.region in principal.attrs.regions";
    let eu = row(&[("region", text("eu"))]);
    let null = row(&[("region", Scalar::Null)]);
    assert_eq!(
        visible(src, &eu, &attrs(&[("regions", regions(&["eu", "us"]))])),
        Tri::True
    );
    assert_eq!(
        visible(src, &eu, &attrs(&[("regions", regions(&["us"]))])),
        Tri::False
    );
    // A missing list attribute is unknown (D-05), an empty one is false.
    assert_eq!(visible(src, &eu, &Attrs::new()), Tri::Unknown);
    assert_eq!(
        visible(src, &eu, &attrs(&[("regions", regions(&[]))])),
        Tri::False
    );
    assert_eq!(
        visible(src, &null, &attrs(&[("regions", regions(&[]))])),
        Tri::False
    );
    assert_eq!(
        visible(src, &null, &attrs(&[("regions", regions(&["eu"]))])),
        Tri::Unknown
    );
    assert_eq!(visible(src, &null, &Attrs::new()), Tri::Unknown);
}

#[test]
fn in_integer_lists() {
    let src = "object.n in [1, 2, -3]";
    let none = Attrs::new();
    assert_eq!(
        visible(src, &row(&[("n", Scalar::Int(-3))]), &none),
        Tri::True
    );
    assert_eq!(
        visible(src, &row(&[("n", Scalar::Int(5))]), &none),
        Tri::False
    );
    let nums = attrs(&[("nums", AttrValue::List(vec![Scalar::Int(5)]))]);
    assert_eq!(
        visible(
            "object.n in principal.attrs.nums",
            &row(&[("n", Scalar::Int(5))]),
            &nums
        ),
        Tri::True
    );
}

#[test]
fn ordering_follows_value_families() {
    let none = Attrs::new();
    let r = row(&[
        ("n", Scalar::Int(2)),
        ("d", Scalar::Double(1.5)),
        ("code", text("b")),
    ]);
    assert_eq!(visible("object.n < 3", &r, &none), Tri::True);
    assert_eq!(visible("object.n >= 3", &r, &none), Tri::False);
    assert_eq!(visible("object.d > 1.25", &r, &none), Tri::True);
    assert_eq!(visible("object.d <= 1.5", &r, &none), Tri::True);
    // String order is byte order: uppercase sorts before lowercase.
    assert_eq!(visible("object.code > 'B'", &r, &none), Tri::True);
    assert_eq!(visible("object.code < 'c'", &r, &none), Tri::True);
}

#[test]
fn string_match_is_case_sensitive_and_literal() {
    let none = Attrs::new();
    let abc = row(&[("code", text("abc"))]);
    assert_eq!(
        visible("object.code.startsWith('A')", &abc, &none),
        Tri::False
    );
    assert_eq!(
        visible("object.code.startsWith('a')", &abc, &none),
        Tri::True
    );
    assert_eq!(
        visible("object.code.endsWith('bc')", &abc, &none),
        Tri::True
    );
    assert_eq!(visible("object.code.contains('b')", &abc, &none), Tri::True);
    assert_eq!(
        visible("object.code.contains('B')", &abc, &none),
        Tri::False
    );

    // Wildcards of LIKE and GLOB are ordinary characters.
    let pct = row(&[("code", text("a%b"))]);
    for needle in ["%", "_", "*", "?", "[a]"] {
        let src = format!("object.code.contains('{needle}')");
        assert_eq!(visible(&src, &abc, &none), Tri::False, "{src}");
    }
    assert_eq!(visible("object.code.contains('%')", &pct, &none), Tri::True);
    assert_eq!(
        visible("object.code.startsWith('a%')", &pct, &none),
        Tri::True
    );
    assert_eq!(
        visible(
            "object.code.contains('_')",
            &row(&[("code", text("a_b"))]),
            &none
        ),
        Tri::True
    );

    // The empty needle matches every non-null string.
    assert_eq!(
        visible("object.code.startsWith('')", &abc, &none),
        Tri::True
    );
    assert_eq!(
        visible(
            "object.code.contains('')",
            &row(&[("code", text(""))]),
            &none
        ),
        Tri::True
    );
    // Null on either side is unknown.
    assert_eq!(
        visible(
            "object.code.startsWith('a')",
            &row(&[("code", Scalar::Null)]),
            &none
        ),
        Tri::Unknown
    );
    assert_eq!(
        visible(
            "object.code.startsWith(principal.attrs.prefix)",
            &abc,
            &none
        ),
        Tri::Unknown
    );
    let prefix = attrs(&[("prefix", AttrValue::Scalar(text("ab")))]);
    assert_eq!(
        visible(
            "object.code.startsWith(principal.attrs.prefix)",
            &abc,
            &prefix
        ),
        Tri::True
    );
}

// ---------------------------------------------------------------------------
// proptests: Display round trip and the Kleene laws
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Fam {
    Str,
    Int,
    Dbl,
    Bool,
}

const STR_COLUMNS: [&str; 5] = ["tenantId", "region", "code", "a", "b"];
const INT_COLUMNS: [&str; 2] = ["n", "big"];
const OPAQUE_COLUMNS: [&str; 5] = ["when", "day", "cost", "tags", "flag"];

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        failure_persistence: Some(Box::new(FileFailurePersistence::WithSource(
            "proptest-regressions",
        ))),
        ..ProptestConfig::default()
    }
}

fn pick<T: Clone + std::fmt::Debug + 'static>(items: &[T]) -> BoxedStrategy<T> {
    prop::sample::select(items.to_vec()).boxed()
}

fn column(names: &[&'static str]) -> BoxedStrategy<Operand> {
    pick(names)
        .prop_map(|name| Operand::Column(prop(name)))
        .boxed()
}

/// Strings biased towards characters that matter to quoting and to LIKE / GLOB, and
/// towards values that occur on both sides so equality is actually hit.
fn string_value() -> BoxedStrategy<String> {
    let fragments = pick(&[
        "",
        "a",
        "A",
        "b",
        "eu",
        "us",
        "acme",
        "%",
        "_",
        "*",
        "?",
        "[",
        "'",
        "\\",
        "\n",
        "\t",
        "\u{1F600}",
        "\u{0}",
        "\"",
        "é",
    ]);
    prop_oneof![
        4 => fragments.clone().prop_map(str::to_owned),
        3 => prop::collection::vec(fragments, 0..4).prop_map(|v| v.concat()),
        2 => any::<String>(),
    ]
    .boxed()
}

fn int_value() -> BoxedStrategy<i64> {
    prop_oneof![8 => -3i64..4, 1 => any::<i64>().prop_filter("i64::MIN", |i| *i != i64::MIN)]
        .boxed()
}

fn double_value() -> BoxedStrategy<f64> {
    prop_oneof![
        4 => pick(&[-1.5f64, 0.0, 1.5, 2.0, 1e21, 1e-7]),
        2 => prop::num::f64::POSITIVE
            | prop::num::f64::NEGATIVE
            | prop::num::f64::NORMAL
            | prop::num::f64::SUBNORMAL
            | prop::num::f64::ZERO,
    ]
    .boxed()
}

fn literal(f: Fam) -> BoxedStrategy<Scalar> {
    match f {
        Fam::Str => string_value().prop_map(Scalar::String).boxed(),
        Fam::Int => int_value().prop_map(Scalar::Int).boxed(),
        Fam::Dbl => double_value().prop_map(Scalar::Double).boxed(),
        Fam::Bool => any::<bool>().prop_map(Scalar::Bool).boxed(),
    }
}

fn operand(f: Fam) -> BoxedStrategy<Operand> {
    let lit = literal(f).prop_map(Operand::Lit);
    match f {
        Fam::Str => prop_oneof![
            3 => column(&STR_COLUMNS),
            2 => lit,
            2 => pick(&["tenantId", "prefix"]).prop_map(|a| Operand::Attr(a.to_owned())),
        ]
        .boxed(),
        Fam::Int => prop_oneof![
            3 => column(&INT_COLUMNS),
            2 => lit,
            1 => Just(Operand::Attr("level".to_owned())),
        ]
        .boxed(),
        Fam::Dbl => prop_oneof![3 => column(&["d"]), 2 => lit].boxed(),
        Fam::Bool => prop_oneof![
            2 => column(&["flag"]),
            1 => lit,
            1 => Just(Operand::Attr("active".to_owned())),
        ]
        .boxed(),
    }
}

fn family() -> BoxedStrategy<Fam> {
    pick(&[Fam::Str, Fam::Int, Fam::Dbl, Fam::Bool])
}

fn cmp_pred() -> BoxedStrategy<Pred> {
    family()
        .prop_flat_map(|f| {
            let ops = match f {
                Fam::Bool => vec![CmpOp::Eq, CmpOp::Ne],
                _ => vec![
                    CmpOp::Eq,
                    CmpOp::Ne,
                    CmpOp::Lt,
                    CmpOp::Le,
                    CmpOp::Gt,
                    CmpOp::Ge,
                ],
            };
            (operand(f), pick(&ops), operand(f))
        })
        .prop_map(|(left, op, right)| Pred::Cmp { left, op, right })
        .boxed()
}

fn is_null_pred() -> BoxedStrategy<Pred> {
    let target = prop_oneof![
        column(&STR_COLUMNS),
        column(&INT_COLUMNS),
        column(&["d"]),
        column(&OPAQUE_COLUMNS),
        pick(&["tenantId", "prefix", "level", "active"]).prop_map(|a| Operand::Attr(a.to_owned())),
    ];
    (target, any::<bool>())
        .prop_map(|(operand, negated)| Pred::IsNull { operand, negated })
        .boxed()
}

fn in_pred() -> BoxedStrategy<Pred> {
    let literal_list = family().prop_flat_map(|f| {
        (operand(f), prop::collection::vec(literal(f), 0..4)).prop_map(|(operand, items)| {
            Pred::InList {
                operand,
                list: ListSource::Literal(items),
            }
        })
    });
    let attr_list = prop_oneof![
        (operand(Fam::Str), Just("regions")),
        (operand(Fam::Int), Just("nums")),
    ]
    .prop_map(|(operand, name)| Pred::InList {
        operand,
        list: ListSource::Attr(name.to_owned()),
    });
    prop_oneof![3 => literal_list, 1 => attr_list].boxed()
}

fn match_pred() -> BoxedStrategy<Pred> {
    (
        pick(&STR_COLUMNS),
        operand(Fam::Str).prop_filter("needle is a literal or an attribute", |o| {
            !matches!(o, Operand::Column(_))
        }),
        pick(&[
            MatchKind::StartsWith,
            MatchKind::EndsWith,
            MatchKind::Contains,
        ]),
    )
        .prop_map(|(name, needle, kind)| Pred::StrMatch {
            column: prop(name),
            needle,
            kind,
        })
        .boxed()
}

fn leaf() -> BoxedStrategy<Pred> {
    prop_oneof![
        1 => any::<bool>().prop_map(Pred::Const),
        4 => cmp_pred(),
        2 => is_null_pred(),
        2 => in_pred(),
        2 => match_pred(),
    ]
    .boxed()
}

fn pred() -> BoxedStrategy<Pred> {
    leaf()
        .prop_recursive(4, 32, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Pred::And(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Pred::Or(Box::new(a), Box::new(b))),
                inner.prop_map(|a| Pred::Not(Box::new(a))),
            ]
        })
        .boxed()
}

fn maybe<T: std::fmt::Debug + 'static>(
    s: impl Strategy<Value = T> + 'static,
) -> BoxedStrategy<Option<T>> {
    prop::option::weighted(0.75, s).boxed()
}

fn row_strategy() -> BoxedStrategy<BTreeMap<PropertyName, Scalar>> {
    let strings = prop::collection::vec(maybe(string_value()), STR_COLUMNS.len());
    let ints = prop::collection::vec(maybe(int_value()), INT_COLUMNS.len());
    (strings, ints, maybe(double_value()), maybe(any::<bool>()))
        .prop_map(|(strings, ints, d, flag)| {
            let mut out = BTreeMap::new();
            for (name, v) in STR_COLUMNS.iter().zip(strings) {
                out.insert(prop(name), v.map_or(Scalar::Null, Scalar::String));
            }
            for (name, v) in INT_COLUMNS.iter().zip(ints) {
                out.insert(prop(name), v.map_or(Scalar::Null, Scalar::Int));
            }
            out.insert(prop("d"), d.map_or(Scalar::Null, Scalar::Double));
            out.insert(prop("flag"), flag.map_or(Scalar::Null, Scalar::Bool));
            out
        })
        .boxed()
}

fn attrs_strategy() -> BoxedStrategy<Attrs> {
    let str_list = maybe(prop::collection::vec(
        string_value().prop_map(Scalar::String),
        0..4,
    ));
    let int_list = maybe(prop::collection::vec(
        int_value().prop_map(Scalar::Int),
        0..4,
    ));
    (
        maybe(string_value()),
        maybe(string_value()),
        maybe(int_value()),
        maybe(any::<bool>()),
        str_list,
        int_list,
    )
        .prop_map(|(tenant, prefix, level, active, regions, nums)| {
            let mut out = Attrs::new();
            let mut put = |name: &str, v: Option<AttrValue>| {
                if let Some(v) = v {
                    out.insert(name, v).expect("attribute");
                }
            };
            put(
                "tenantId",
                tenant.map(|s| AttrValue::Scalar(Scalar::String(s))),
            );
            put(
                "prefix",
                prefix.map(|s| AttrValue::Scalar(Scalar::String(s))),
            );
            put("level", level.map(|i| AttrValue::Scalar(Scalar::Int(i))));
            put("active", active.map(|b| AttrValue::Scalar(Scalar::Bool(b))));
            put("regions", regions.map(AttrValue::List));
            put("nums", nums.map(AttrValue::List));
            out
        })
        .boxed()
}

proptest! {
    #![proptest_config(config())]

    /// `compile(display(p)) == p` for every predicate the type rules allow.
    #[test]
    fn display_round_trip(p in pred()) {
        let fx = Fixture::new();
        let text = p.to_string();
        let compiled = compile_row_filter(&text, &fx.cx());
        prop_assert_eq!(compiled, Ok(p), "text: {}", text);
    }

    /// The Kleene laws, on random predicates, rows and attributes.
    #[test]
    fn kleene_laws(
        a in pred(),
        b in pred(),
        null_test in is_null_pred(),
        (operand, items) in family().prop_flat_map(|f| (operand(f), prop::collection::vec(literal(f), 0..4))),
        row in row_strategy(),
        attrs in attrs_strategy(),
    ) {
        let ev = |p: &Pred| eval(p, &row, &attrs);
        let not = |p: &Pred| Pred::Not(Box::new(p.clone()));

        // De Morgan, both directions.
        let and = Pred::And(Box::new(a.clone()), Box::new(b.clone()));
        let or = Pred::Or(Box::new(a.clone()), Box::new(b.clone()));
        prop_assert_eq!(
            ev(&not(&and)),
            ev(&Pred::Or(Box::new(not(&a)), Box::new(not(&b))))
        );
        prop_assert_eq!(
            ev(&not(&or)),
            ev(&Pred::And(Box::new(not(&a)), Box::new(not(&b))))
        );
        // Double negation, commutativity, and Unknown propagation.
        prop_assert_eq!(ev(&not(&not(&a))), ev(&a));
        prop_assert_eq!(
            ev(&and),
            ev(&Pred::And(Box::new(b.clone()), Box::new(a.clone())))
        );
        prop_assert_eq!(ev(&not(&a)), ev(&a).not());

        // IS NULL / IS NOT NULL are two-valued and complementary.
        let t = ev(&null_test);
        prop_assert!(t != Tri::Unknown);
        if let Pred::IsNull { operand, negated } = &null_test {
            let flipped = Pred::IsNull { operand: operand.clone(), negated: !negated };
            prop_assert_eq!(ev(&flipped), t.not());
        }

        // `x in [e1 .. en]` is `x == e1 || .. || x == en`; `x in []` is false.
        let in_list = Pred::InList {
            operand: operand.clone(),
            list: ListSource::Literal(items.clone()),
        };
        let expanded = items
            .iter()
            .map(|e| Pred::Cmp {
                left: operand.clone(),
                op: CmpOp::Eq,
                right: Operand::Lit(e.clone()),
            })
            .reduce(|l, r| Pred::Or(Box::new(l), Box::new(r)));
        if let Some(expanded) = expanded {
            prop_assert_eq!(ev(&in_list), ev(&expanded));
        } else {
            prop_assert_eq!(ev(&in_list), Tri::False);
        }
    }
}
