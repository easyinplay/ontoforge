use std::collections::BTreeMap;

use camino::Utf8Path;
use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::ast::{ObjectSet, PageSpec};
use ontoforge_query::compile::{CompileLimits, ValidatedQuery, compile_source_select};
use ontoforge_query::QueryRequest;
use ontoforge_schema::ir::{ObjectTypeName, PropertyName, ProfileName};
use ontoforge_schema::load_dir;

fn name(s: &str) -> ObjectTypeName { ObjectTypeName::try_from(s.to_owned()).unwrap() }
fn prop(s: &str) -> PropertyName { PropertyName::try_from(s.to_owned()).unwrap() }

#[test]
fn source_select_has_exactly_one_table() {
    let ontology = load_dir(Utf8Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation"))).ontology.unwrap();
    let scope = TypeScope { row: Pred::Const(true), columns: BTreeMap::from([(prop("flightId"), Pred::Const(true)), (prop("status"), Pred::Const(true))]) };
    let binding = &ontology.profile_bindings(&ProfileName::try_from("dev".to_owned()).unwrap()).unwrap().objects[&name("Flight")];
    let query = ValidatedQuery::new(QueryRequest { set: ObjectSet::Type { object_type: name("Flight") }, select: vec![prop("status")] }, vec![], PageSpec { first: 5, after: None }, &ontology, &scope, CompileLimits::default()).unwrap();
    let source = compile_source_select(&query, &ontology, binding, &scope, &Attrs::default()).unwrap();
    assert!(source.select.from.is_some());
    assert_eq!(source.select.limit, Some(5));
    assert_eq!(source.select.projection.len(), 2);
}
