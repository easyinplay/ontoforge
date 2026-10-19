//! Row predicate evaluation: the tracer (plan 03-05 Task 2) and the Kleene laws
//! (Task 3).

mod common;

use std::collections::BTreeMap;

use common::Fixture;
use ontoforge_expr::{
    AttrValue, Attrs, CmpOp, Operand, Pred, Scalar, Tri, compile_row_filter, eval,
};
use ontoforge_schema::ir::PropertyName;

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
