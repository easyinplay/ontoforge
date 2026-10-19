//! JSON Schema export for editors and LLM generation (SCHEMA-07).
//!
//! The schemas are derived from the IR types, so the format an editor completes
//! against is the format the loader accepts. The committed copies under `schemas/`
//! are checked against this output by a test.

use schemars::{JsonSchema, schema_for};
use serde_json::{Value, json};

use crate::ir::{Definition, Manifest, ScopeDefinition};

/// A definition file holds one document or, for JSON, an array of them. Export-only:
/// the loader never deserializes this type.
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum DefinitionFile {
    One(Box<Definition>),
    Many(Vec<Definition>),
}

/// A scope file holds one document or, for JSON, an array of them. Export-only, like
/// [`DefinitionFile`].
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum ScopeFile {
    One(Box<ScopeDefinition>),
    Many(Vec<ScopeDefinition>),
}

fn with_header(mut schema: Value, title: &str, description: &str) -> Value {
    let root = schema
        .as_object_mut()
        .expect("a derived root schema is an object");
    root.insert("title".to_owned(), json!(title));
    root.insert("description".to_owned(), json!(description));
    schema
}

/// Replace the description of `$defs.<name>`: the IR doc comments of the two tagged
/// enums describe loader internals, which editor users should not see.
fn describe_def(schema: &mut Value, name: &str, description: &str) {
    if let Some(def) = schema
        .pointer_mut(&format!("/$defs/{name}"))
        .and_then(Value::as_object_mut)
    {
        def.insert("description".to_owned(), json!(description));
    }
}

/// Schema of one definition file: a single definition or an array of definitions.
pub fn definition_schema() -> Value {
    let mut schema = with_header(
        serde_json::to_value(schema_for!(DefinitionFile)).expect("schema serializes"),
        "ontoforge definition document",
        "An ObjectType, LinkType, ActionType or SourceBinding document (ADR-0012, ADR-0021), selected by its `kind`; \
         JSON files may hold an array of them.",
    );
    describe_def(
        &mut schema,
        "Definition",
        "One definition document: an ObjectType, LinkType, ActionType or SourceBinding, selected by `kind`.",
    );
    describe_def(
        &mut schema,
        "Rule",
        "One edit of an action, selected by `kind`. Rules run in list order.",
    );
    schema
}

/// Schema of `ontology.yaml`.
pub fn manifest_schema() -> Value {
    with_header(
        serde_json::to_value(schema_for!(Manifest)).expect("schema serializes"),
        "ontoforge ontology manifest",
        "The ontology.yaml manifest naming the ontology and listing definition files.",
    )
}

/// Schema of one scope file: a `Role` or `RoleBinding` document, or an array of them.
pub fn scope_schema() -> Value {
    let mut schema = with_header(
        serde_json::to_value(schema_for!(ScopeFile)).expect("schema serializes"),
        "ontoforge scope document",
        "A Role or RoleBinding document (ADR-0018, ADR-0019), selected by its `kind`. Scope documents live in their own directory, apart from the ontology definition and its SourceBinding files; JSON files may hold an array of them.",
    );
    describe_def(
        &mut schema,
        "ScopeDefinition",
        "One scope document: a Role or a RoleBinding, selected by `kind`.",
    );
    schema
}

/// File name and content of each exported schema; content is pretty JSON ending in a
/// newline and identical on every call.
pub fn rendered_schemas() -> Vec<(&'static str, String)> {
    let render = |schema: &Value| {
        let mut text = serde_json::to_string_pretty(schema).expect("schema serializes");
        text.push('\n');
        text
    };
    vec![
        (
            "ontoforge-definition.schema.json",
            render(&definition_schema()),
        ),
        ("ontoforge-manifest.schema.json", render(&manifest_schema())),
        ("ontoforge-scope.schema.json", render(&scope_schema())),
    ]
}
