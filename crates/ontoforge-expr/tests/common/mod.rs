//! Shared fixture of the row predicate tests: one object type and the declared
//! principal attributes.

use std::collections::BTreeMap;

use ontoforge_expr::PredContext;
use ontoforge_schema::ir::{AttrType, ObjectType};
use ontoforge_schema::load_from_memory;

const MANIFEST: &str = "apiName: demo\n";

const OBJECTS: &str = "kind: ObjectType
apiName: Doc
primaryKey: id
properties:
  id: { type: string, required: true }
  tenantId: { type: string }
  region: { type: string }
  code: { type: string }
  a: { type: string }
  b: { type: string }
  n: { type: integer }
  big: { type: long }
  d: { type: double }
  flag: { type: boolean }
  when: { type: timestamp }
  day: { type: date }
  cost: { type: decimal }
  tags: { type: array, items: string }
  location: { type: geopoint }
backing: editOnly
";

pub struct Fixture {
    object_type: ObjectType,
    attrs: BTreeMap<String, AttrType>,
}

impl Fixture {
    pub fn new() -> Self {
        let out = load_from_memory(&[("ontology.yaml", MANIFEST), ("objects.yaml", OBJECTS)]);
        assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
        let ontology = out.ontology.expect("ontology");
        let object_type = ontology
            .object_types
            .get("Doc")
            .expect("Doc object type")
            .clone();
        let attrs = [
            ("tenantId", AttrType::String),
            ("regions", AttrType::StringList),
            ("prefix", AttrType::String),
            ("level", AttrType::Integer),
            ("nums", AttrType::IntegerList),
            ("active", AttrType::Boolean),
        ]
        .into_iter()
        .map(|(name, ty)| (name.to_owned(), ty))
        .collect();
        Self { object_type, attrs }
    }

    pub fn cx(&self) -> PredContext<'_> {
        PredContext {
            object_type: &self.object_type,
            attrs: &self.attrs,
        }
    }
}
