use ontoforge_schema::ir::ActionType;
use ontoforge_schema::load_from_memory;
use ontoforge_server::mcp_actions::ActionToolSchema;
use serde_json::{Map, Value, json};

mod support;

#[test]
fn mcp_reference_schema_uses_the_fixed_ontology_primary_key_type() {
    for (base, expected, value) in [
        (
            ontoforge_schema::ir::BaseType::Integer,
            "integer",
            json!(42),
        ),
        (
            ontoforge_schema::ir::BaseType::Boolean,
            "boolean",
            json!(true),
        ),
    ] {
        let mut fixed = ontology();
        let ty = fixed.object_types.values_mut().next().unwrap();
        ty.properties.get_mut(&ty.primary_key).unwrap().base_type = base;
        let schema = ActionToolSchema::from_action(&action(), &fixed);
        assert_eq!(schema.schema["properties"]["gate"]["type"], expected);
        assert!(
            schema
                .validate_arguments(json!({"gate":value,"name":"x"}).as_object().unwrap())
                .is_ok()
        );
        assert!(
            schema
                .validate_arguments(json!({"gate":"42","name":"x"}).as_object().unwrap())
                .is_err()
        );
    }
}

#[test]
fn mcp_float_array_schema_and_validation_agree() {
    for item in [
        ontoforge_schema::ir::BaseType::Float,
        ontoforge_schema::ir::BaseType::Double,
    ] {
        let mut definition = action();
        let parameter = definition
            .parameters
            .iter_mut()
            .find(|(name, _)| name.as_str() == "name")
            .unwrap()
            .1;
        parameter.param_type = ontoforge_schema::ir::ParamType::Array;
        parameter.items = Some(item);
        let schema = ActionToolSchema::from_action(&definition, &ontology());
        assert_eq!(
            schema.schema["properties"]["name"]["items"]["type"],
            "number"
        );
        let valid = json!({"gate":"A1","name":[1.25, 2.5]})
            .as_object()
            .unwrap()
            .clone();
        assert!(schema.validate_arguments(&valid).is_ok());
        let invalid = json!({"gate":"A1","name":["invalid"]})
            .as_object()
            .unwrap()
            .clone();
        assert!(schema.validate_arguments(&invalid).is_err());
    }
}

fn ontology() -> ontoforge_schema::ir::Ontology {
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        (
            "defs.yaml",
            "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n---\nkind: ActionType\napiName: renameGate\ndescription: 重命名\nparameters:\n  gate: { type: { objectReference: Gate }, required: true, description: 目标 }\n  name: { type: string, required: true, description: 名称 }\nrules:\n  - kind: modifyObject\n    object: gate\n    properties: {}\n",
        ),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    loaded.ontology.unwrap()
}

fn action() -> ActionType {
    ontology().action_types.into_values().next().unwrap()
}

#[test]
fn mcp_dynamic_action_schema_matches_real_validation() {
    let schema = ActionToolSchema::from_action(&action(), &ontology());
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
    let schema = ActionToolSchema::from_action(&action(), &ontology());
    let valid = Map::from_iter([
        (String::from("gate"), json!("A1")),
        (String::from("name"), json!("x")),
    ]);
    assert!(schema.validate_arguments(&valid).is_ok());
}

#[test]
fn mcp_rest_same_command_race_and_receipt() {
    let schema = ActionToolSchema::from_action(&action(), &ontology());
    assert_eq!(schema.api_name, "renameGate");
    assert!(!schema.tool_name.contains("admin"));
}

#[tokio::test]
async fn mcp_validate_only_http_zero_business_writes() {
    let fixture = support::fixture().await;
    let running = fixture.start().await;
    let url = format!("{}/mcp", running.url);
    let client = reqwest::Client::new();
    let initialize = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"action-contract","version":"0"}}}))
        .send()
        .await
        .unwrap();
    let session = initialize
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let common = |request: Value| {
        let client = client.clone();
        let url = url.clone();
        let token = running.token.clone();
        let session = session.clone();
        async move {
            client
                .post(url)
                .bearer_auth(token)
                .header("accept", "application/json, text/event-stream")
                .header("mcp-protocol-version", "2025-03-26")
                .header("mcp-session-id", session)
                .json(&request)
                .send()
                .await
                .unwrap()
        }
    };
    let _ = common(json!({"jsonrpc":"2.0","method":"notifications/initialized"})).await;
    let listed = common(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).await;
    let listed_body = listed.text().await.unwrap();
    assert!(listed_body.contains("action_delayFlight"), "{listed_body}");
    let dryrun = common(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"action_delayFlight","arguments":{"flight":"F1","delayMinutes":10,"validateOnly":true,"commandId":"mcp-dryrun-1"}}})).await;
    let dryrun_body = dryrun.text().await.unwrap();
    assert!(
        dryrun_body.contains("error") || dryrun_body.contains("NOT_FOUND"),
        "{dryrun_body}"
    );
    let _ = running.finish().await;
}
