//! The D-07 subset of CEL a row filter may use: what is accepted (and as which
//! `Pred`), and what is rejected with a positioned issue.

mod common;

use common::Fixture;
use ontoforge_expr::{
    CmpOp, ListSource, MatchKind, Operand, Pred, PredIssue, Scalar, compile_row_filter,
};
use ontoforge_schema::ir::PropertyName;

fn compile(src: &str) -> Result<Pred, Vec<PredIssue>> {
    let fx = Fixture::new();
    compile_row_filter(src, &fx.cx())
}

fn prop(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property name")
}

/// Compiles `src` and returns its canonical text (`Pred`'s Display).
fn canonical(src: &str) -> String {
    match compile(src) {
        Ok(p) => p.to_string(),
        Err(issues) => panic!("{src}: {issues:#?}"),
    }
}

#[test]
fn accepts_the_six_comparisons() {
    for (src, want) in [
        ("object.n == 1", "(object.n == 1)"),
        ("object.n != 1", "(object.n != 1)"),
        ("object.n < 1", "(object.n < 1)"),
        ("object.n <= 1", "(object.n <= 1)"),
        ("object.n > 1", "(object.n > 1)"),
        ("object.n >= 1", "(object.n >= 1)"),
        ("object.big >= -3", "(object.big >= -3)"),
        ("object.d < 1.5", "(object.d < 1.5)"),
        ("object.code >= 'm'", "(object.code >= 'm')"),
        ("object.flag == true", "(object.flag == true)"),
        (
            "object.flag != principal.attrs.active",
            "(object.flag != principal.attrs.active)",
        ),
        (
            "object.n == principal.attrs.level",
            "(object.n == principal.attrs.level)",
        ),
    ] {
        assert_eq!(canonical(src), want, "{src}");
    }
}

#[test]
fn accepts_logic_with_cel_precedence_and_left_association() {
    assert_eq!(
        canonical("object.a == 'x' && object.b == 'y' || !(object.n < 3)"),
        "(((object.a == 'x') && (object.b == 'y')) || !(object.n < 3))"
    );
    assert_eq!(
        canonical("object.n == 1 || object.n == 2 || object.n == 3"),
        "(((object.n == 1) || (object.n == 2)) || (object.n == 3))"
    );
    assert!(compile("!!(object.n == 1)").is_ok());
}

#[test]
fn null_on_either_side_becomes_is_null() {
    assert_eq!(
        compile("object.region == null"),
        Ok(Pred::IsNull {
            operand: Operand::Column(prop("region")),
            negated: false
        })
    );
    assert_eq!(
        compile("null == object.region"),
        compile("object.region == null")
    );
    assert_eq!(
        compile("object.region != null"),
        Ok(Pred::IsNull {
            operand: Operand::Column(prop("region")),
            negated: true
        })
    );
    assert_eq!(
        compile("principal.attrs.tenantId == null"),
        Ok(Pred::IsNull {
            operand: Operand::Attr("tenantId".to_owned()),
            negated: false
        })
    );
    // Every property type except geopoint may be tested against null.
    for p in ["when", "day", "cost", "tags", "flag", "n", "d"] {
        assert!(compile(&format!("object.{p} == null")).is_ok(), "{p}");
    }
}

#[test]
fn accepts_in_lists_literal_and_attribute() {
    assert_eq!(
        compile("object.region in ['eu', 'us']"),
        Ok(Pred::InList {
            operand: Operand::Column(prop("region")),
            list: ListSource::Literal(vec![
                Scalar::String("eu".to_owned()),
                Scalar::String("us".to_owned())
            ]),
        })
    );
    assert_eq!(
        compile("object.n in [1, 2, -3]"),
        Ok(Pred::InList {
            operand: Operand::Column(prop("n")),
            list: ListSource::Literal(vec![Scalar::Int(1), Scalar::Int(2), Scalar::Int(-3)]),
        })
    );
    assert_eq!(
        compile("object.region in principal.attrs.regions"),
        Ok(Pred::InList {
            operand: Operand::Column(prop("region")),
            list: ListSource::Attr("regions".to_owned()),
        })
    );
    assert_eq!(
        canonical("object.n in principal.attrs.nums"),
        "(object.n in principal.attrs.nums)"
    );
    assert_eq!(canonical("object.region in []"), "(object.region in [])");
    assert_eq!(
        canonical("principal.attrs.tenantId in ['a']"),
        "(principal.attrs.tenantId in ['a'])"
    );
}

#[test]
fn accepts_string_matching() {
    assert_eq!(
        compile("object.code.startsWith('A')"),
        Ok(Pred::StrMatch {
            column: prop("code"),
            needle: Operand::Lit(Scalar::String("A".to_owned())),
            kind: MatchKind::StartsWith,
        })
    );
    assert_eq!(
        canonical("object.code.endsWith('z')"),
        "object.code.endsWith('z')"
    );
    assert_eq!(
        canonical("object.code.contains('mid')"),
        "object.code.contains('mid')"
    );
    assert_eq!(
        compile("object.code.startsWith(principal.attrs.prefix)"),
        Ok(Pred::StrMatch {
            column: prop("code"),
            needle: Operand::Attr("prefix".to_owned()),
            kind: MatchKind::StartsWith,
        })
    );
}

#[test]
fn accepts_two_columns_and_constants() {
    assert_eq!(
        compile("object.a == object.b"),
        Ok(Pred::Cmp {
            left: Operand::Column(prop("a")),
            op: CmpOp::Eq,
            right: Operand::Column(prop("b")),
        })
    );
    assert_eq!(compile("true"), Ok(Pred::Const(true)));
    assert_eq!(compile("false"), Ok(Pred::Const(false)));
}

#[test]
fn string_literals_keep_their_escapes() {
    assert_eq!(
        compile(r"object.a == 'it\'s \\ \n'"),
        Ok(Pred::Cmp {
            left: Operand::Column(prop("a")),
            op: CmpOp::Eq,
            right: Operand::Lit(Scalar::String("it's \\ \n".to_owned())),
        })
    );
}

/// Each source is rejected with exactly one issue positioned inside the expression.
#[test]
fn rejects_everything_outside_the_subset() {
    for src in [
        // macros, has(), matches, arithmetic, size, ternary, bytes, uint
        "object.tags.exists(t, t == 'x')",
        "object.tags.all(t, t == 'x')",
        "has(object.n)",
        "object.code.matches('a.*')",
        "object.n + 1 > 2",
        "size(object.code) > 0",
        "object.a == 'x' ? true : false",
        "object.b == b'x'",
        "object.n == 1u",
        // same-row restriction
        "principal.id == 'x'",
        "principal.groups == 'x'",
        "principal.attrs.undeclared == 'x'",
        "object.missing == 'x'",
        "request.ip == '1'",
        "object == 'x'",
        "object.a.b == 'x'",
        // types
        "object.n == 1.5",
        "object.d == 1",
        "object.n == 'x'",
        "object.when > '2026-01-01'",
        "object.cost == '1.0'",
        "object.day < 'x'",
        "object.location == null",
        "object.region in ['a', null]",
        "object.region in ['a', 1]",
        "object.n in [1, 2.5]",
        "object.region in principal.attrs.tenantId",
        "object.region in principal.attrs.nums",
        "object.n.startsWith('1')",
        "'abc'.startsWith('b')",
        "object.code.startsWith(object.a)",
        "object.code.startsWith(1)",
        "object.flag < true",
        "object.n < null",
        "null == null",
        "principal.attrs.regions == 'eu'",
        "object.tags == 'x'",
        // a bare operand is not a predicate
        "object.flag",
        "object.n",
    ] {
        let issues = compile(src).expect_err(src);
        assert_eq!(issues.len(), 1, "{src}: {issues:#?}");
        assert!(issues[0].pos.is_some(), "{src}: {issues:#?}");
    }
}

#[test]
fn a_literal_target_with_a_column_needle_reports_both() {
    let issues = compile("'abc'.startsWith(object.code)").expect_err("both wrong");
    assert_eq!(issues.len(), 2, "{issues:#?}");
}

#[test]
fn issue_messages_say_what_to_change() {
    let msg = |src: &str| compile(src).expect_err(src).remove(0).message;
    assert!(msg("object.tags.exists(t, t == 'x')").contains("macros"));
    assert!(msg("has(object.n)").contains("has()"));
    assert!(msg("object.missing == 'x'").contains("no property `missing`"));
    assert!(msg("principal.attrs.undeclared == 'x'").contains("not declared"));
    assert!(msg("principal.id == 'x'").contains("principal.attrs"));
    assert!(msg("object.n == 1.5").contains("int"));
    assert!(msg("object.when > '2026-01-01'").contains("timestamp"));
    assert!(msg("object.region in ['a', null]").contains("null"));
}

#[test]
fn positions_point_into_the_expression() {
    let issues = compile("object.a == 'x'\n  && object.missing == 'y'").expect_err("missing");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].pos.map(|(line, _)| line), Some(2));
}

#[test]
fn all_issues_are_collected() {
    let issues =
        compile("object.missing == 'x' && principal.attrs.undeclared == 'y'").expect_err("two");
    assert_eq!(issues.len(), 2, "{issues:#?}");
}

#[test]
fn syntax_and_size_problems_are_issues_too() {
    let issues = compile("object.a ==").expect_err("syntax");
    assert_eq!(issues.len(), 1);
    assert!(issues[0].pos.is_some());
    let long = format!("object.a == '{}'", "x".repeat(5000));
    let issues = compile(&long).expect_err("too long");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].pos, None);
}
