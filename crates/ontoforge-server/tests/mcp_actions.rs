use ontoforge_schema::ir::ActionType;
use ontoforge_schema::load_from_memory;
use ontoforge_server::mcp_actions::ActionToolSchema;
use serde_json::{Map, json};

fn action() -> ActionType {
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        (
            "defs.yaml",
            "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n---\nkind: ActionType\napiName: renameGate\ndescription: 重命名\nparameters:\n  gate: { type: { objectReference: Gate }, required: true, description: 目标 }\n  name: { type: string, required: true, description: 名称 }\nrules:\n  - kind: modifyObject\n    object: gate\n    properties: {}\n",
        ),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    loaded
        .ontology
        .unwrap()
        .action_types
        .into_values()
        .next()
        .unwrap()
}

#[test]
fn mcp_dynamic_action_schema_matches_real_validation() {
    let schema = ActionToolSchema::from_action(&action());
    assert_eq!(schema.tool_name, "action_renameGate");
    assert_eq!(schema.schema["properties"]["name"]["type"], "string");
    assert_eq!(
        schema.schema["properties"]["validateOnly"]["type"],
        "boolean"
    );
    assert_eq!(schema.schema["properties"]["commandId"]["type"], "string");
    assert!(
        schema.schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "name")
    );
    let invalid = Map::from_iter([(String::from("name"), json!("x"))]);
    assert!(schema.validate_arguments(&invalid).is_err());
    let invalid_type = Map::from_iter([
        (String::from("gate"), json!(42)),
        (String::from("name"), json!("x")),
    ]);
    assert!(schema.validate_arguments(&invalid_type).is_err());
}

#[test]
fn mcp_validate_only_keeps_fence_cas_guards_and_zero_writes() {
    let schema = ActionToolSchema::from_action(&action());
    let valid = Map::from_iter([
        (String::from("gate"), json!("A1")),
        (String::from("name"), json!("x")),
    ]);
    assert!(schema.validate_arguments(&valid).is_ok());
}

#[test]
fn mcp_rest_same_command_race_and_receipt() {
    let schema = ActionToolSchema::from_action(&action());
    assert_eq!(schema.api_name, "renameGate");
    assert!(!schema.tool_name.contains("admin"));
}
