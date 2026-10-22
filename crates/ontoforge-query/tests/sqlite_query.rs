use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use ontoforge_auth::scope::{RowVisibility, TypeScope};
use ontoforge_expr::{Pred, Tri};
use ontoforge_query::ast::{ObjectSet, PageSpec, QueryLiteral, TermMode, WhereClause};
use ontoforge_query::compile::{CompileLimits, ValidatedQuery, evaluate_where};
use ontoforge_query::{QueryError, QueryRequest};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_schema::load_dir;
use serde_json::{Value, json};

fn name(s: &str) -> ObjectTypeName { ObjectTypeName::try_from(s.to_owned()).unwrap() }
fn prop(s: &str) -> PropertyName { PropertyName::try_from(s.to_owned()).unwrap() }
fn lit(v: Value) -> QueryLiteral { QueryLiteral(v) }
fn row() -> BTreeMap<PropertyName, Value> {
    BTreeMap::from([(prop("status"), json!("HELLO foo baz BAR")), (prop("delayMinutes"), json!(12))])
}
fn visibility() -> RowVisibility {
    RowVisibility { visible: true, columns: BTreeSet::from([prop("status"), prop("delayMinutes")]) }
}
fn ontology() -> ontoforge_schema::ir::Ontology {
    load_dir(Utf8Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation"))).ontology.unwrap()
}
fn scope() -> TypeScope {
    TypeScope { row: Pred::Const(true), columns: BTreeMap::from([(prop("status"), Pred::Const(true)), (prop("delayMinutes"), Pred::Const(true))]) }
}
fn request(predicate: WhereClause) -> QueryRequest {
    QueryRequest { set: ObjectSet::Where { object_type: name("Flight"), predicate }, select: vec![prop("status")] }
}
fn page() -> PageSpec { PageSpec { first: 10, after: None } }

#[test]
fn every_where_operator_validates_and_evaluates() {
    let cases = vec![
        (WhereClause::Eq { property: prop("delayMinutes"), value: lit(json!(12)) }, true),
        (WhereClause::Gt { property: prop("delayMinutes"), value: lit(json!(10)) }, true),
        (WhereClause::Gte { property: prop("delayMinutes"), value: lit(json!(12)) }, true),
        (WhereClause::Lt { property: prop("delayMinutes"), value: lit(json!(15)) }, true),
        (WhereClause::Lte { property: prop("delayMinutes"), value: lit(json!(12)) }, true),
        (WhereClause::IsNull { property: prop("delayMinutes") }, false),
        (WhereClause::In { property: prop("delayMinutes"), values: vec![lit(json!(9)), lit(json!(12))] }, true),
        (WhereClause::StartsWith { property: prop("status"), value: "HELLO".into() }, true),
        (WhereClause::ContainsAnyTerm { property: prop("status"), value: "lost foo".into() }, true),
        (WhereClause::ContainsAllTerms { property: prop("status"), value: "BAR foo".into() }, true),
        (WhereClause::And { clauses: vec![WhereClause::Eq { property: prop("delayMinutes"), value: lit(json!(12)) }] }, true),
        (WhereClause::Or { clauses: vec![WhereClause::Eq { property: prop("delayMinutes"), value: lit(json!(12)) }] }, true),
        (WhereClause::Not { clause: Box::new(WhereClause::IsNull { property: prop("status") }) }, true),
    ];
    for (predicate, expected) in cases {
        ValidatedQuery::new(request(predicate.clone()), vec![], page(), &ontology(), &scope(), CompileLimits::default()).unwrap();
        assert_eq!(evaluate_where(&predicate, &row(), &visibility()).is_true(), expected, "{predicate:?}");
    }
}

#[test]
fn hidden_column_is_rejected_before_execution() {
    let hidden = WhereClause::Eq { property: prop("flightNumber"), value: lit(json!("LH100")) };
    assert!(matches!(ValidatedQuery::new(request(hidden), vec![], page(), &ontology(), &scope(), CompileLimits::default()), Err(QueryError::InvalidQuery)));
}

#[test]
fn multi_role_grant_is_bound_to_its_row() {
    let predicate = WhereClause::Not { clause: Box::new(WhereClause::Eq { property: prop("status"), value: lit(json!("secret")) }) };
    let hidden = RowVisibility { visible: true, columns: BTreeSet::from([prop("delayMinutes")]) };
    assert_eq!(evaluate_where(&predicate, &row(), &hidden), Tri::Unknown);
}

#[test]
fn term_search_is_whitespace_any_or_all() {
    for mode in [TermMode::Any, TermMode::All] {
        let query = QueryRequest { set: ObjectSet::Search { object_type: name("Flight"), property: prop("status"), query: "foo bar".into(), mode }, select: vec![prop("status")] };
        ValidatedQuery::new(query, vec![], page(), &ontology(), &scope(), CompileLimits::default()).unwrap();
    }
    let empty = QueryRequest { set: ObjectSet::Search { object_type: name("Flight"), property: prop("status"), query: "  \t".into(), mode: TermMode::All }, select: vec![prop("status")] };
    assert!(matches!(ValidatedQuery::new(empty, vec![], page(), &ontology(), &scope(), CompileLimits::default()), Err(QueryError::InvalidQuery)));
}

#[test]
fn limits_reject_before_recursion() {
    let mut nested = WhereClause::IsNull { property: prop("status") };
    for _ in 0..65 { nested = WhereClause::Not { clause: Box::new(nested) }; }
    assert!(matches!(ValidatedQuery::new(request(nested), vec![], page(), &ontology(), &scope(), CompileLimits::default()), Err(QueryError::QueryTooComplex)));
}
