use ontoforge_schema::ir::ActionType;
use ontoforge_schema::load_from_memory;
use ontoforge_server::mcp_actions::ActionToolSchema;
use serde_json::{Value, json};

#[path = "support/security_evidence.rs"]
mod evidence;
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
        assert_eq!(
            schema.schema["properties"]["params"]["properties"]["gate"]["type"],
            expected
        );
        assert!(
            schema
                .validate_arguments(
                    json!({"params":{"gate":value,"name":"x"}})
                        .as_object()
                        .unwrap()
                )
                .is_ok()
        );
        assert!(
            schema
                .validate_arguments(
                    json!({"params":{"gate":"42","name":"x"}})
                        .as_object()
                        .unwrap()
                )
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
            schema.schema["properties"]["params"]["properties"]["name"]["items"]["type"],
            "number"
        );
        let valid = json!({"params":{"gate":"A1","name":[1.25, 2.5]}})
            .as_object()
            .unwrap()
            .clone();
        assert!(schema.validate_arguments(&valid).is_ok());
        let invalid = json!({"params":{"gate":"A1","name":["invalid"]}})
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
    assert_eq!(
        schema.schema["properties"]["params"]["properties"]["name"]["type"],
        "string"
    );
    assert_eq!(
        schema.schema["properties"]["validateOnly"]["type"],
        "boolean"
    );
    assert_eq!(schema.schema["properties"]["commandId"]["type"], "string");
    assert!(
        schema.schema["properties"]["params"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "name")
    );
    assert_eq!(schema.schema["required"], json!(["params"]));
    let invalid = json!({"params":{"name":"x"}});
    assert!(
        schema
            .validate_arguments(invalid.as_object().unwrap())
            .is_err()
    );
    let invalid_type = json!({"params":{"gate":42,"name":"x"}});
    assert!(
        schema
            .validate_arguments(invalid_type.as_object().unwrap())
            .is_err()
    );
}

#[test]
fn mcp_validate_only_keeps_fence_cas_guards_and_zero_writes() {
    let schema = ActionToolSchema::from_action(&action(), &ontology());
    let valid = json!({"validateOnly":true,"params":{"gate":"A1","name":"x"}});
    assert!(
        schema
            .validate_arguments(valid.as_object().unwrap())
            .is_ok()
    );
}

#[test]
fn mcp_action_envelope_rejects_wrong_shapes_and_unknown_fields() {
    let schema = ActionToolSchema::from_action(&action(), &ontology());
    assert_eq!(schema.schema["additionalProperties"], false);
    assert_eq!(
        schema.schema["properties"]["params"]["additionalProperties"],
        false
    );
    for value in [
        json!({}),
        json!({"params":null}),
        json!({"params":[]}),
        json!({"params":"wrong"}),
        json!({"gate":"A1","name":"x"}),
        json!({"params":{"gate":"A1","name":"x","extra":true}}),
        json!({"params":{"gate":"A1","name":"x"},"extra":true}),
        json!({"params":{"gate":"A1","name":"x"},"validateOnly":"true"}),
        json!({"params":{"gate":"A1","name":"x"},"commandId":1}),
    ] {
        assert!(
            schema
                .validate_arguments(value.as_object().unwrap())
                .is_err(),
            "{value}"
        );
    }
}

#[test]
fn mcp_action_envelope_optional_params_preserve_business_defaults() {
    let mut definition = action();
    for parameter in definition.parameters.values_mut() {
        parameter.required = false;
    }
    definition.parameters.get_mut("name").unwrap().default = Some(json!("fallback"));
    let schema = ActionToolSchema::from_action(&definition, &ontology());
    assert!(
        !schema
            .schema
            .get("required")
            .and_then(Value::as_array)
            .is_some_and(|required| required.contains(&json!("params")))
    );
    assert_eq!(
        schema.schema["properties"]["params"]["properties"]["name"]["default"],
        "fallback"
    );
    for value in [json!({}), json!({"params":{}})] {
        assert!(
            schema
                .validate_arguments(value.as_object().unwrap())
                .is_ok(),
            "{value}"
        );
    }
    for value in [
        json!({"params":null}),
        json!({"params":[]}),
        json!({"params":false}),
    ] {
        assert!(
            schema
                .validate_arguments(value.as_object().unwrap())
                .is_err(),
            "{value}"
        );
    }
}

#[test]
fn mcp_action_envelope_separates_same_named_business_fields() {
    let mut definition = action();
    let template = definition.parameters["name"].clone();
    definition.parameters.clear();
    for name in ["commandId", "validateOnly", "params"] {
        definition
            .parameters
            .insert(name.to_owned().try_into().unwrap(), template.clone());
    }
    let schema = ActionToolSchema::from_action(&definition, &ontology());
    assert_eq!(schema.schema["properties"].as_object().unwrap().len(), 3);
    let business = &schema.schema["properties"]["params"];
    assert_eq!(business["type"], "object");
    let required: std::collections::BTreeSet<_> = business["required"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        required,
        std::collections::BTreeSet::from(["commandId", "params", "validateOnly"])
    );
    assert_eq!(business["properties"]["validateOnly"]["type"], "string");
    let args = json!({"commandId":"control","validateOnly":true,"params":{"commandId":"business-command","validateOnly":"business-validation","params":"business-payload"}});
    assert!(schema.validate_arguments(args.as_object().unwrap()).is_ok());
}

#[tokio::test]
async fn mcp_rest_same_command_race_and_receipt() {
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    let mut running = support::fixture().await.start().await;
    running.client = evidence::client();
    let session = evidence::session(&running.client, &running.url, &running.token).await;
    let before = evidence::counts(&running.path).await;
    let before_rows = evidence::durable_rows(&running.path).await;
    running.engine.block.store(true, Ordering::SeqCst);
    let client = running.client.clone();
    let url = running.url.clone();
    let token = running.token.clone();
    let rest = tokio::spawn(async move {
        client.post(format!("{url}/api/actions/delayFlight")).bearer_auth(token)
            .json(&json!({"commandId":"cross-transport-race","items":[{"params":{"flight":"F-1001","delayMinutes":5}}]}))
            .send().await.unwrap()
    });
    tokio::time::timeout(Duration::from_secs(5), running.engine.entered.notified())
        .await
        .unwrap();
    let client = running.client.clone();
    let url = running.url.clone();
    let token = running.token.clone();
    let session_copy = session.clone();
    let mcp = tokio::spawn(async move {
        evidence::rpc(
            &client,
            &url,
            &token,
            &session_copy,
            evidence::action_call(2, "cross-transport-race", false),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), running.engine.entered.notified())
        .await
        .unwrap();
    running.engine.block.store(false, Ordering::SeqCst);
    running.engine.release.notify_waiters();
    let rest = tokio::time::timeout(Duration::from_secs(10), rest)
        .await
        .unwrap()
        .unwrap();
    let receipt = support::success(rest).await;
    let mcp = tokio::time::timeout(Duration::from_secs(10), mcp)
        .await
        .unwrap()
        .unwrap();
    assert!(mcp.get("error").is_none(), "{mcp}");
    assert!(receipt["eventId"].is_string());
    assert!(receipt["committedAt"].is_string());
    assert_eq!(mcp["result"]["structuredContent"]["receipt"], receipt);
    let committed_rows = evidence::durable_rows(&running.path).await;
    evidence::assert_one_flight_commit(&before_rows, &committed_rows);
    let replay = evidence::rpc(
        &running.client,
        &running.url,
        &running.token,
        &session,
        evidence::action_call(3, "cross-transport-race", false),
    )
    .await;
    assert_eq!(replay["result"]["structuredContent"]["receipt"], receipt);
    assert_eq!(
        support::success(running.get("/api/commands/cross-transport-race").await).await,
        receipt
    );
    assert_eq!(
        evidence::delta(evidence::counts(&running.path).await, before),
        [1; 5],
        "REST and MCP must share one log, entry, history, command and outbox commit"
    );
    assert_eq!(evidence::durable_rows(&running.path).await, committed_rows);
    assert!(running.finish_actions().await.is_empty());
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "keep schema capture, reload and execution assertions in temporal order"
)]
async fn mcp_action_schema_execution_keeps_admitted_definition() {
    use ontoforge_storage::reload::{ReloadOptions, reload};
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    let mut running = support::fixture().await.start().await;
    running.client = evidence::client();
    let other = ontoforge_storage::store::EditStore::open(support::settings(&running.path))
        .await
        .unwrap();
    let session = evidence::session(&running.client, &running.url, &running.token).await;
    let list = |id| json!({"jsonrpc":"2.0","id":id,"method":"tools/list"});
    let old = evidence::rpc(
        &running.client,
        &running.url,
        &running.token,
        &session,
        list(2),
    )
    .await;
    let schema = |value: &Value| {
        value["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "action_delayFlight")
            .unwrap()["inputSchema"]
            .clone()
    };
    assert!(
        schema(&old)["properties"]["params"]["properties"]
            .get("confirmation")
            .is_none()
    );
    let before = evidence::counts(&running.path).await;
    let before_rows = evidence::durable_rows(&running.path).await;
    running.engine.block.store(true, Ordering::SeqCst);
    let client = running.client.clone();
    let url = running.url.clone();
    let token = running.token.clone();
    let session_copy = session.clone();
    let pending = tokio::spawn(async move {
        evidence::rpc(
            &client,
            &url,
            &token,
            &session_copy,
            evidence::action_call(3, "mcp-old-schema", false),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), running.engine.entered.notified())
        .await
        .unwrap();
    let mut ontology = other.active_definition().await.unwrap().ontology.clone();
    let action = ontology.action_types.get_mut("delayFlight").unwrap();
    let mut parameter = action.parameters.get("delayMinutes").unwrap().clone();
    parameter.required = true;
    action
        .parameters
        .insert("confirmation".to_owned().try_into().unwrap(), parameter);
    tokio::time::timeout(
        Duration::from_secs(5),
        reload(&other, &ontology, &ReloadOptions::default()),
    )
    .await
    .unwrap()
    .unwrap();
    running.engine.block.store(false, Ordering::SeqCst);
    running.engine.release.notify_one();
    let admitted = tokio::time::timeout(Duration::from_secs(10), pending)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        admitted["error"]["data"]["code"], "DEFINITION_STALE",
        "old schema execution must reach its fence: {admitted}"
    );
    let current = evidence::rpc(
        &running.client,
        &running.url,
        &running.token,
        &session,
        list(4),
    )
    .await;
    assert!(
        schema(&current)["properties"]["params"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("confirmation"))
    );
    let calls = running.engine.calls.load(Ordering::SeqCst);
    let invalid = evidence::rpc(
        &running.client,
        &running.url,
        &running.token,
        &session,
        evidence::action_call(5, "mcp-new-schema", false),
    )
    .await;
    assert!(
        invalid["error"].is_object(),
        "new request must validate the new schema: {invalid}"
    );
    assert_eq!(running.engine.calls.load(Ordering::SeqCst), calls);
    assert_eq!(evidence::counts(&running.path).await, before);
    assert_eq!(evidence::durable_rows(&running.path).await, before_rows);
    other.read_audit().shutdown().await.unwrap();
    assert!(running.finish().await.is_empty());
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
    let dryrun = common(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"action_delayFlight","arguments":{"params":{"flight":"F1","delayMinutes":10},"validateOnly":true,"commandId":"mcp-dryrun-1"}}})).await;
    let dryrun_body = dryrun.text().await.unwrap();
    assert!(
        dryrun_body.contains("error") || dryrun_body.contains("NOT_FOUND"),
        "{dryrun_body}"
    );
    let _ = running.finish().await;
}
