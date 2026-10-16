//! Loader behavior through the public in-memory API: error mapping and locations.

use ontoforge_schema::diag::{Diagnostic, codes};
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

const GOOD: &str = "kind: ObjectType\napiName: Airport\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\nbacking: editOnly\n";

#[test]
fn valid_document_loads_into_the_ontology() {
    let out = load(GOOD);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert!(out.ontology.is_some());
}

#[test]
fn unknown_field_is_v04_at_the_key_with_a_suggestion() {
    let out = load(&GOOD.replace("primaryKey", "primaryKy"));
    let d = only(&out);
    assert_eq!(d.code, codes::V04);
    assert_eq!((d.file.as_str(), d.line, d.column), ("objects.yaml", 3, 1));
    assert!(d.help.as_deref().unwrap().contains("primaryKey"));
}

#[test]
fn missing_required_field_is_v05_at_the_kind_value() {
    let out = load(
        "kind: ObjectType\napiName: Airport\nproperties:\n  code: { type: string }\nbacking: editOnly\n",
    );
    let d = only(&out);
    assert_eq!(d.code, codes::V05);
    assert!(d.message.contains("primaryKey"), "{}", d.message);
    assert_eq!((d.line, d.column), (1, 7));
}

#[test]
fn unknown_kind_is_v03_with_a_close_name() {
    let out = load(&GOOD.replace("kind: ObjectType", "kind: ObjectTyp"));
    let d = only(&out);
    assert_eq!(d.code, codes::V03);
    assert_eq!((d.line, d.column), (1, 7));
    assert!(d.help.as_deref().unwrap().contains("ObjectType"));
}

#[test]
fn reserved_kind_is_v03_not_supported_in_v1() {
    let out = load("kind: Interface\napiName: Thing\n");
    let d = only(&out);
    assert_eq!(d.code, codes::V03);
    assert!(d.message.contains("not supported in v1"), "{}", d.message);
}

/// `GOOD` with extra property lines added after its last property.
fn good_with(properties: &str) -> String {
    GOOD.replace(
        "backing: editOnly\n",
        &format!("{properties}backing: editOnly\n"),
    )
}

#[test]
fn lowercase_object_type_name_is_v07_with_canonical_help() {
    let out = load(&GOOD.replace("apiName: Airport", "apiName: airport"));
    let d = only(&out);
    assert_eq!(d.code, codes::V07);
    assert_eq!((d.line, d.column), (2, 10));
    assert!(d.help.as_deref().unwrap().contains("Airport"));
}

#[test]
fn reserved_word_is_v07_case_insensitively() {
    let out = load(&GOOD.replace("apiName: Airport", "apiName: Object"));
    let d = only(&out);
    assert_eq!(d.code, codes::V07);
    assert!(d.message.contains("reserved"), "{}", d.message);
}

/// WR-03: a CEL keyword cannot be referenced in a condition (`params.in` does not
/// parse), so parameter and property names may not be one.
#[test]
fn cel_keyword_parameter_or_property_name_is_v07() {
    let out = load(&good_with("  for: { type: string }\n"));
    let d = only(&out);
    assert_eq!(d.code, codes::V07);
    assert_eq!(d.line, 6);
    assert!(d.message.contains("CEL"), "{}", d.message);

    let action =
        "kind: ActionType\napiName: bump\nparameters:\n  in: { type: integer }\nrules: []\n";
    let out = load(&format!("{GOOD}---\n{action}"));
    let d = only(&out);
    assert_eq!(d.code, codes::V07);
    assert!(d.message.contains("`in`"), "{}", d.message);

    // Only exact keywords are refused.
    let out = load(&good_with(
        "  inbound: { type: string }\n  nullable: { type: string }\n",
    ));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}

#[test]
fn duplicate_api_name_is_v08_and_cites_the_first_definition() {
    let doc = format!("{GOOD}---\n{GOOD}");
    let out = load(&doc);
    let d = only(&out);
    assert_eq!(d.code, codes::V08);
    assert_eq!((d.line, d.column), (9, 10));
    assert!(d.message.contains("objects.yaml:2:10"), "{}", d.message);
}

#[test]
fn primary_key_must_be_a_declared_property() {
    let out = load(&GOOD.replace("primaryKey: code", "primaryKey: id"));
    let d = only(&out);
    assert_eq!(d.code, codes::V10);
    assert_eq!((d.line, d.column), (3, 13));
}

#[test]
fn missing_manifest_is_v14_without_a_location() {
    let out = load_from_memory(&[("objects.yaml", GOOD)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V14);
    assert_eq!((d.file.as_str(), d.line, d.column), ("ontology.yaml", 0, 0));
    assert!(out.ontology.is_none());
}

#[test]
fn duplicate_yaml_key_is_v01() {
    let out = load("kind: ObjectType\napiName: A\napiName: B\n");
    assert_eq!(only(&out).code, codes::V01);
}

#[test]
fn json_trailing_comma_is_v01_even_though_yaml_would_accept_it() {
    let out = load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        (
            "objects.json",
            "{\"kind\": \"ObjectType\",\n \"apiName\": \"Airport\",}\n",
        ),
    ]);
    let d = only(&out);
    assert_eq!(d.code, codes::V01);
    assert_eq!(d.file, "objects.json");
    assert_eq!(d.line, 2);
}

#[test]
fn yaml_and_json_load_to_the_same_ontology() {
    let json = r#"{"kind":"ObjectType","apiName":"Airport","primaryKey":"code",
        "properties":{"code":{"type":"string","required":true}},"backing":"editOnly"}"#;
    let from_yaml = load(GOOD);
    let from_json = load_from_memory(&[("ontology.yaml", MANIFEST), ("objects.json", json)]);
    assert!(
        from_json.diagnostics.is_empty(),
        "{:#?}",
        from_json.diagnostics
    );
    assert!(from_yaml.ontology.is_some());
    assert_eq!(from_yaml.ontology, from_json.ontology);
}

#[test]
fn deeply_nested_yaml_is_rejected_not_crashing() {
    let mut doc = String::from(
        "kind: ObjectType\napiName: Airport\nprimaryKey: code\nproperties:\n  code:\n",
    );
    for depth in 0..200 {
        doc.push_str(&"  ".repeat(depth + 2));
        doc.push_str("a:\n");
    }
    let out = load(&doc);
    assert!(!out.diagnostics.is_empty());
}

#[test]
fn unknown_base_type_is_v06_at_the_value_with_a_suggestion() {
    let out = load(&GOOD.replace("type: string", "type: strng"));
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!((d.line, d.column), (5, 17));
    assert!(d.help.as_deref().unwrap().contains("`string`"));
}
