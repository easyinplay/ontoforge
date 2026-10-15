//! The exported JSON Schemas stay in step with the IR and accept the examples.

use ontoforge_schema::schema_export::{definition_schema, manifest_schema, rendered_schemas};
use serde_json::Value;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn example_dir(name: &str) -> std::path::PathBuf {
    std::path::Path::new(ROOT)
        .join("examples/ontologies")
        .join(name)
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every YAML document of a file, parsed as loosely typed JSON with the loader's
/// boolean strictness.
fn yaml_documents(path: &std::path::Path) -> Vec<Value> {
    let mut options = serde_saphyr::Options::default();
    options.strict_booleans = true;
    serde_saphyr::from_multiple_with_options::<Value>(&read(path), options)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn json_document(path: &std::path::Path) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn validator(schema: &Value) -> jsonschema::Validator {
    jsonschema::validator_for(schema).expect("the exported schema is itself valid")
}

/// Collect all `const` values anywhere inside `value`.
fn consts(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if key == "const"
                    && let Some(s) = inner.as_str()
                {
                    out.push(s.to_owned());
                }
                consts(inner, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| consts(i, out)),
        _ => {}
    }
}

#[test]
fn definition_schema_is_draft_2020_12() {
    for schema in [definition_schema(), manifest_schema()] {
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }
}

#[test]
fn definition_variants_are_discriminated_by_kind() {
    let schema = definition_schema();
    let mut found = Vec::new();
    for key in ["$defs", "definitions"] {
        if let Some(defs) = schema.get(key).and_then(Value::as_object) {
            let one_of = defs
                .get("Definition")
                .and_then(|d| d.get("oneOf"))
                .and_then(Value::as_array);
            if let Some(branches) = one_of {
                assert_eq!(branches.len(), 3, "Definition has three branches");
                for branch in branches {
                    let kind = branch["properties"]["kind"]["const"]
                        .as_str()
                        .expect("every branch pins `kind` with a const");
                    found.push(kind.to_owned());
                    assert_eq!(
                        branch["additionalProperties"],
                        Value::Bool(false),
                        "branch {kind} rejects unknown fields"
                    );
                }
            }
        }
    }
    found.sort();
    assert_eq!(found, ["ActionType", "LinkType", "ObjectType"]);
}

#[test]
fn rule_variants_are_discriminated_by_kind() {
    let mut all = Vec::new();
    consts(&definition_schema(), &mut all);
    for rule in [
        "createObject",
        "modifyObject",
        "deleteObject",
        "addLink",
        "removeLink",
    ] {
        assert!(all.iter().any(|c| c == rule), "missing const `{rule}`");
    }
}

#[test]
fn committed_schemas_are_up_to_date() {
    let update = std::env::var("ONTOFORGE_UPDATE_SCHEMAS").as_deref() == Ok("1");
    let dir = std::path::Path::new(ROOT).join("schemas");
    for (name, rendered) in rendered_schemas() {
        let path = dir.join(name);
        if update {
            std::fs::create_dir_all(&dir).expect("create schemas dir");
            std::fs::write(&path, &rendered).expect("write schema");
            continue;
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == rendered,
            "schemas/{name} is out of date with the IR; run \
             `cargo run -p ontoforge -- schema export` and commit the result"
        );
    }
}

#[test]
fn rendering_is_deterministic() {
    assert_eq!(rendered_schemas(), rendered_schemas());
}

#[test]
fn aviation_documents_validate() {
    let definition = validator(&definition_schema());
    let manifest = validator(&manifest_schema());

    for name in ["objects.yaml", "links.yaml", "actions.yaml"] {
        let path = example_dir("aviation").join(name);
        for (i, doc) in yaml_documents(&path).iter().enumerate() {
            let errors: Vec<String> = definition
                .iter_errors(doc)
                .map(|e| format!("{e} at {}", e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{name} document {i}: {errors:#?}");
        }
    }
    for name in ["objects.json", "links.json", "actions.json"] {
        let path = example_dir("aviation-json").join(name);
        let doc = json_document(&path);
        let errors: Vec<String> = definition
            .iter_errors(&doc)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:#?}");
    }
    for dir in ["aviation", "aviation-json"] {
        let path = example_dir(dir).join("ontology.yaml");
        for doc in yaml_documents(&path) {
            let errors: Vec<String> = manifest
                .iter_errors(&doc)
                .map(|e| format!("{e} at {}", e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{dir}/ontology.yaml: {errors:#?}");
        }
    }
}

#[test]
fn invalid_examples_fail_schema() {
    let definition = validator(&definition_schema());
    for name in [
        "v04-unknown-field",
        "v06-unknown-base-type",
        "v07-bad-object-type-name",
    ] {
        let path = example_dir("invalid").join(name).join("objects.yaml");
        let rejected = yaml_documents(&path)
            .iter()
            .any(|d| !definition.is_valid(d));
        assert!(rejected, "{name} unexpectedly passes the definition schema");
    }
}
