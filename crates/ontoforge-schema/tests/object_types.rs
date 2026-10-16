//! Object type fields, base types and structural validation through the public
//! in-memory API.

use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::basetype::{PkSuitability, is_title_suitable, pk_suitability};
use ontoforge_schema::ir::{BaseType, ObjectType};
use ontoforge_schema::{LoadOutput, load_from_memory};

const MANIFEST: &str = "apiName: demo\n";

fn load(objects: &str) -> LoadOutput {
    load_from_memory(&[("ontology.yaml", MANIFEST), ("objects.yaml", objects)])
}

fn only(out: &LoadOutput) -> &Diagnostic {
    assert_eq!(
        out.diagnostics.len(),
        1,
        "diagnostics: {:#?}",
        out.diagnostics
    );
    &out.diagnostics[0]
}

fn object<'a>(out: &'a LoadOutput, name: &str) -> &'a ObjectType {
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology
        .as_ref()
        .expect("ontology")
        .object_types
        .get(name)
        .expect("object type")
}

#[test]
fn all_v1_base_types_parse() {
    let names = [
        "byte",
        "short",
        "integer",
        "long",
        "float",
        "double",
        "decimal",
        "string",
        "boolean",
        "date",
        "timestamp",
        "geopoint",
    ];
    let mut doc = String::from(
        "kind: ObjectType\napiName: Sample\nprimaryKey: id\nproperties:\n  id: { type: string, required: true }\n",
    );
    for n in names {
        doc += &["  p", n, ": { type: ", n, " }\n"].concat();
    }
    doc.push_str("  list: { type: array, items: string }\nbacking: editOnly\n");
    let out = load(&doc);
    let sample = object(&out, "Sample");
    assert_eq!(sample.properties.len(), 1 + names.len() + 1);
    assert_eq!(sample.properties["pdecimal"].base_type, BaseType::Decimal);
    assert_eq!(sample.properties["pgeopoint"].base_type, BaseType::Geopoint);
    assert_eq!(sample.properties["list"].base_type, BaseType::Array);
    assert_eq!(sample.properties["list"].items, Some(BaseType::String));
    assert!(sample.properties["id"].required);
    assert!(!sample.properties["pbyte"].required);
}

#[test]
fn object_type_optional_fields_default() {
    let out = load(
        "kind: ObjectType\napiName: Airport\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\nbacking: editOnly\n",
    );
    let airport = object(&out, "Airport");
    assert!(airport.aliases.is_empty());
    assert!(!airport.hidden);
    assert!(airport.title_key.is_none());
    assert_eq!(airport.effective_title_key().as_str(), "code");
}

#[test]
fn full_object_type_round_trips() {
    let out = load(
        "kind: ObjectType
apiName: Airport
displayName: Airport
description: A place aircraft depart from.
aliases: [aerodrome, airfield]
hidden: true
primaryKey: code
titleKey: name
properties:
  code: { type: string, required: true, displayName: Code, description: IATA code }
  name: { type: string }
  tags: { type: array, items: string }
backing: editOnly
",
    );
    let airport = object(&out, "Airport");
    assert_eq!(airport.display_name.as_deref(), Some("Airport"));
    assert_eq!(
        airport.description.as_deref(),
        Some("A place aircraft depart from.")
    );
    assert_eq!(airport.aliases, ["aerodrome", "airfield"]);
    assert!(airport.hidden);
    assert_eq!(airport.effective_title_key().as_str(), "name");
    let code = &airport.properties["code"];
    assert_eq!(code.display_name.as_deref(), Some("Code"));
    assert_eq!(code.description.as_deref(), Some("IATA code"));
    assert_eq!(airport.properties["tags"].items, Some(BaseType::String));
}

#[test]
fn unknown_base_type_suggests_nearest() {
    let out = load(
        "kind: ObjectType\napiName: Airport\nprimaryKey: code\nproperties:\n  code: { type: strng, required: true }\nbacking: editOnly\n",
    );
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert!(d.help.as_deref().unwrap().contains("`string`"), "{d:?}");
}

#[test]
fn string_field_given_number_suggests_quotes() {
    let out = load(
        "kind: ObjectType\napiName: Airport\ndisplayName: 123\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\nbacking: editOnly\n",
    );
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!((d.line, d.column), (3, 14));
    assert!(d.help.as_deref().unwrap().contains("quote"), "{d:?}");
}

#[test]
fn pk_suitability_table() {
    use BaseType::{
        Array, Boolean, Byte, Date, Decimal, Double, Float, Geopoint, Integer, Long, Short, String,
        Timestamp,
    };
    for t in [String, Integer, Short] {
        assert_eq!(pk_suitability(t), PkSuitability::Valid, "{t:?}");
    }
    for t in [Date, Timestamp, Boolean, Byte, Long] {
        assert_eq!(pk_suitability(t), PkSuitability::Discouraged, "{t:?}");
    }
    for t in [Float, Double, Decimal, Geopoint, Array] {
        assert_eq!(pk_suitability(t), PkSuitability::Invalid, "{t:?}");
    }
    assert!(is_title_suitable(String, None));
    assert!(is_title_suitable(Array, Some(String)));
}

const GOOD: &str = "kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string }
backing: editOnly
";

/// `GOOD` with extra property lines added after its last property.
fn good_with(properties: &str) -> String {
    GOOD.replace(
        "backing: editOnly\n",
        &format!("{properties}backing: editOnly\n"),
    )
}

#[test]
fn primary_key_must_be_required() {
    let out = load(&GOOD.replace("required: true", "required: false"));
    let d = only(&out);
    assert_eq!(d.code, codes::V10);
    assert_eq!((d.line, d.column), (5, 3));
    assert!(
        d.help.as_deref().unwrap().contains("required: true"),
        "{d:?}"
    );
}

#[test]
fn invalid_primary_key_types_are_v10() {
    for t in [
        "float",
        "double",
        "decimal",
        "geopoint",
        "array, items: string",
    ] {
        let out = load(&GOOD.replace("type: string, required", &format!("type: {t}, required")));
        let d = only(&out);
        assert_eq!(d.code, codes::V10, "{t}");
        assert_eq!((d.line, d.column), (3, 13), "{t}");
        let name = t.split(',').next().unwrap();
        assert!(d.message.contains(name), "{t}: {}", d.message);
    }
}

#[test]
fn discouraged_primary_key_is_not_a_validate_error() {
    let out = load(&GOOD.replace("type: string, required", "type: timestamp, required"));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert!(out.ontology.is_some());
}

#[test]
fn title_key_must_exist() {
    let out = load(&GOOD.replace("primaryKey: code\n", "primaryKey: code\ntitleKey: nme\n"));
    let d = only(&out);
    assert_eq!(d.code, codes::V10);
    assert_eq!((d.line, d.column), (4, 11));
    assert!(d.help.as_deref().unwrap().contains("`name`"), "{d:?}");
}

#[test]
fn array_requires_items() {
    let out = load(&good_with("  tags: { type: array }\n"));
    let d = only(&out);
    assert_eq!(d.code, codes::V05);
    assert_eq!((d.line, d.column), (7, 17));
    assert!(d.help.as_deref().unwrap().contains("items"), "{d:?}");
}

#[test]
fn items_only_for_arrays() {
    let out = load(&GOOD.replace(
        "name: { type: string }",
        "name: { type: string, items: string }",
    ));
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!((d.line, d.column), (6, 25));
}

#[test]
fn nested_array_is_not_a_validate_error() {
    let out = load(&good_with("  grid: { type: array, items: array }\n"));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}

#[test]
fn duplicate_object_type_reports_first_location() {
    let out = load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("a.yaml", GOOD),
        ("b.yaml", GOOD),
    ]);
    let d = only(&out);
    assert_eq!(d.code, codes::V08);
    assert_eq!(d.file, "b.yaml");
    assert!(d.message.contains("a.yaml:2:10"), "{}", d.message);
}

// ---- backing (D-02) --------------------------------------------------------

#[test]
fn object_without_backing_is_v05() {
    let out = load(&GOOD.replace("backing: editOnly\n", ""));
    let d = only(&out);
    assert_eq!(d.code, codes::V05);
    assert_eq!(d.message, "missing required field `backing`");
    assert_eq!((d.file.as_str(), d.line, d.column), ("objects.yaml", 1, 7));
}

#[test]
fn unknown_backing_value_is_v06() {
    let out = load(&GOOD.replace("editOnly", "sourced"));
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!(d.help.as_deref(), Some("did you mean `source`?"), "{d:?}");
}

#[test]
fn both_backing_values_load() {
    for (value, json) in [("source", "source"), ("editOnly", "editOnly")] {
        let out = load(&GOOD.replace("editOnly", value));
        assert!(
            out.diagnostics.is_empty(),
            "{value}: {:#?}",
            out.diagnostics
        );
        let json_ir = serde_json::to_value(object(&out, "Airport")).expect("serializable IR");
        assert_eq!(json_ir["backing"], json, "{value}");
    }
}

#[test]
fn aviation_backing_values() {
    let out = ontoforge_schema::load_dir(&camino::Utf8PathBuf::from(format!(
        "{}/../../examples/ontologies/aviation",
        env!("CARGO_MANIFEST_DIR")
    )));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ir = serde_json::to_value(out.ontology.expect("ontology")).expect("serializable IR");
    for (name, backing) in [
        ("Airport", "editOnly"),
        ("Aircraft", "source"),
        ("Flight", "source"),
        ("Pilot", "source"),
        ("MaintenanceRecord", "source"),
    ] {
        assert_eq!(ir["objectTypes"][name]["backing"], backing, "{name}");
    }
    let links = ir["linkTypes"].as_object().expect("linkTypes map");
    for (name, link) in links {
        if name == "flightCrew" {
            assert_eq!(link["backing"], "source", "{name}");
        } else {
            assert!(
                link.get("backing").is_none_or(serde_json::Value::is_null),
                "{name}: {link}"
            );
        }
    }
}
