use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::Toolset;
use ontoforge_server::mcp_catalog::{AGGREGATE_TOOL, DESCRIBE_TOOL, HISTORY_TOOL, ToolCatalog};
use serde_json::{Value, json};

mod support;

async fn initialize(running: &support::Running) -> String {
    let response = running.client.post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"review-contract","version":"0"}}}))
        .send().await.unwrap();
    let session = response.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let response = running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    session
}

#[tokio::test]
async fn mcp_describe_links_and_engine_status_match_scoped_rest_metadata() {
    let running = support::fixture().await.start().await;
    let session = initialize(&running).await;
    for unavailable in [false, true] {
        running
            .engine
            .unavailable
            .store(unavailable, std::sync::atomic::Ordering::SeqCst);
        let rest = support::success(running.get("/api/ontology").await).await;
        let response = call_read_tool(&running, &session, 2, "describe", json!({})).await;
        let metadata = &response["result"]["structuredContent"];
        assert_eq!(metadata["engineStatus"], rest["engineStatus"]);
        assert_eq!(metadata["links"], rest["links"]);
        assert!(!metadata.to_string().contains("Aircraft"));
        assert!(!metadata.to_string().contains("secret_token"));
    }
    assert!(running.finish().await.is_empty());
}

#[tokio::test]
async fn mcp_get_object_budget_error_preserves_durable_audit_id() {
    let mut limits = ontoforge_server::ServerLimits::default();
    limits.execution.max_decoded_bytes = 1;
    let running = support::fixture().await.start_with_limits(limits).await;
    let session = initialize(&running).await;
    let response = call_read_tool(
        &running,
        &session,
        2,
        "get_object",
        json!({"objectType":"Airport","pk":"BER"}),
    )
    .await;
    assert_eq!(
        response["error"]["data"]["code"], "COST_EXCEEDED",
        "{response}"
    );
    assert_eq!(response["error"]["data"]["apiName"], "get_object");
    let audit = running.finish().await;
    assert_eq!(audit.len(), 1);
    assert_eq!(
        response["error"]["data"]["request_id"],
        audit[0]["request_id"]
    );
}

#[tokio::test]
async fn mcp_get_object_missing_is_distinct_from_budget_exhaustion() {
    let running = support::fixture().await.start().await;
    let session = initialize(&running).await;
    let response = call_read_tool(
        &running,
        &session,
        2,
        "get_object",
        json!({"objectType":"Airport","pk":"ABSENT"}),
    )
    .await;
    assert_eq!(response["error"]["data"]["code"], "NOT_FOUND", "{response}");
    let audit = running.finish().await;
    assert_eq!(audit.len(), 1);
    assert_eq!(
        response["error"]["data"]["request_id"],
        audit[0]["request_id"]
    );
}

#[tokio::test]
async fn mcp_store_failures_are_safe_and_keep_audit_correlation() {
    for (tool, table) in [
        ("get_object", "of_obj_airport"),
        ("history", "of_edit_history"),
    ] {
        let running = full_read_fixture().await.start().await;
        let session = initialize(&running).await;
        let pool = sqlx::SqlitePool::connect_with(
            ontoforge_storage::store::sqlite_connect_options(&running.path),
        )
        .await
        .unwrap();
        let sql = if table == "of_obj_airport" {
            "DROP TABLE of_obj_airport"
        } else {
            "DROP TABLE of_edit_history"
        };
        sqlx::query(sql).execute(&pool).await.unwrap();
        pool.close().await;
        let response = call_read_tool(
            &running,
            &session,
            2,
            tool,
            json!({"objectType":"Airport","pk":"BER"}),
        )
        .await;
        assert_eq!(
            response["error"]["data"]["code"], "STORE_UNAVAILABLE",
            "{response}"
        );
        assert_eq!(response["error"]["data"]["apiName"], tool);
        assert!(!response.to_string().contains(table), "{response}");
        assert!(!response.to_string().contains("SQL"), "{response}");
        let audit = running.finish().await;
        assert_eq!(audit.len(), 1);
        assert_eq!(
            response["error"]["data"]["request_id"],
            audit[0]["request_id"]
        );
    }
}

async fn typed_reference_fixture(
    kind: &str,
    stored_pk: ontoforge_storage::value::PropertyValue,
) -> support::Fixture {
    use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
    use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
    use ontoforge_storage::reload::{ReloadOptions, reload};
    use ontoforge_storage::value::PropertyValue;
    let fixture = support::fixture().await;
    let definition = format!(
        "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: {{ type: {kind}, required: true }}\n  name: {{ type: string }}\n---\nkind: ActionType\napiName: renameGate\nparameters:\n  gate: {{ type: {{ objectReference: Gate }}, required: true }}\n  samples: {{ type: array, items: float, required: true }}\nrules:\n  - kind: modifyObject\n    object: gate\n    properties:\n      name: {{ static: updated }}\n"
    );
    let loaded = ontoforge_schema::load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        ("defs.yaml", &definition),
    ]);
    let addition = loaded.ontology.expect("typed action ontology");
    let mut ontology = fixture
        .store
        .active_definition()
        .await
        .unwrap()
        .ontology
        .clone();
    ontology.object_types.extend(addition.object_types);
    ontology.action_types.extend(addition.action_types);
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let scope = ontoforge_schema::load_scope_from_memory(&[("scope.yaml", "kind: Role\napiName: gateEditor\nobjectTypes:\n  Gate:\n    properties: [code, name]\nactions: [renameGate]\ntoolsets: [core]\n---\nkind: RoleBinding\nrole: gateEditor\nsubjects:\n  - group: dispatchers\n")]).scope.unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&fixture.host.auth_config)),
        scope: &scope,
        sources: None,
    })
    .unwrap();
    let props = compiled.row_filter_properties();
    let plan = fixture.store.scope_plan(&scope, &props).await.unwrap();
    fixture
        .store
        .scope_apply(&scope, &props, &plan.hash, "typed-action-test")
        .await
        .unwrap();
    let active = fixture.store.active_definition().await.unwrap();
    fixture
        .store
        .commit(Commit {
            actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
            action: "seedGate".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().unwrap(),
                pk: stored_pk,
                op: ObjectOp::Set(std::collections::BTreeMap::from([(
                    "name".to_owned().try_into().unwrap(),
                    PropertyValue::String("before".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    fixture
}

#[tokio::test]
async fn mcp_action_numeric_and_boolean_references_accept_real_pk_values() {
    use ontoforge_storage::value::PropertyValue;
    for (kind, pk, stored_pk) in [
        ("integer", json!(42), PropertyValue::Int(42)),
        ("boolean", json!(true), PropertyValue::Bool(true)),
    ] {
        let running = typed_reference_fixture(kind, stored_pk).await.start().await;
        let session = initialize(&running).await;
        let result = call_read_tool(&running, &session, 2, "action_renameGate", json!({"params":{"gate":pk,"samples":[1.25,2.5]},"validateOnly":true,"commandId":"typed-reference"})).await;
        assert_eq!(
            result["result"]["structuredContent"]["validated"], true,
            "{result}"
        );
        let invalid = call_read_tool(
            &running,
            &session,
            3,
            "action_renameGate",
            json!({"params":{"gate":pk,"samples":["bad"]},"validateOnly":true,"commandId":"invalid-array"}),
        )
        .await;
        assert!(invalid["error"].is_object(), "{invalid}");
        let pool = sqlx::SqlitePool::connect_with(
            ontoforge_storage::store::sqlite_connect_options(&running.path),
        )
        .await
        .unwrap();
        sqlx::query("DROP TABLE of_obj_gate")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let failed = call_read_tool(
            &running,
            &session,
            4,
            "action_renameGate",
            json!({"params":{"gate":pk,"samples":[1.25]},"validateOnly":true,"commandId":"failed-reference"}),
        )
        .await;
        assert_eq!(
            failed["error"]["data"]["code"], "STORE_UNAVAILABLE",
            "{failed}"
        );
        assert_eq!(failed["error"]["data"]["apiName"], "action_renameGate");
        assert!(
            failed["error"]["data"]["request_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
        );
        assert!(!failed.to_string().contains("of_obj_gate"), "{failed}");
        running.finish_actions().await;
    }
}

async fn call_read_tool(
    running: &support::Running,
    session: &str,
    id: u64,
    name: &str,
    arguments: Value,
) -> Value {
    let body = running.client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
        .json(&json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}}))
        .send().await.expect("MCP read request")
        .text().await.expect("MCP read response");
    body.lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|data| serde_json::from_str(data).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .expect("MCP JSON response")
}

async fn full_read_fixture() -> support::Fixture {
    use camino::Utf8PathBuf;
    use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
    let mut fixture = support::fixture().await;
    let key_path = Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap();
    let key = DevKey::load_or_create(&key_path).unwrap();
    fixture.token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "alice".to_owned(),
            groups: vec!["dispatchers".to_owned(), "crew-schedulers".to_owned()],
            attrs: serde_json::Map::from_iter([("airports".to_owned(), json!(["BER"]))]),
            ttl: std::time::Duration::from_secs(600),
        },
    )
    .unwrap();
    fixture
}

async fn append_history_group(fixture: &support::Fixture, names: &[String]) {
    use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
    use ontoforge_storage::value::PropertyValue;
    let active = fixture.store.active_definition().await.unwrap();
    fixture
        .store
        .commit(Commit {
            actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
            action: "history-contract".to_owned(),
            def_version: active.def_version,
            params: json!({"private":"must-not-leak"}),
            edits: names
                .iter()
                .map(|name| Edit::Object {
                    type_name: "Airport".to_owned().try_into().unwrap(),
                    pk: PropertyValue::String("BER".to_owned()),
                    op: ObjectOp::Set(std::collections::BTreeMap::from([(
                        "name".to_owned().try_into().unwrap(),
                        PropertyValue::String(name.clone()),
                    )])),
                })
                .collect(),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn mcp_history_pages_match_rest_and_preserve_complete_records() {
    let fixture = full_read_fixture().await;
    append_history_group(&fixture, &["first".to_owned(), "second".to_owned()]).await;
    let running = fixture.start().await;
    let session = initialize(&running).await;
    let rest = support::success(
        running
            .get("/api/objects/Airport/BER/history?limit=1")
            .await,
    )
    .await;
    let response = call_read_tool(
        &running,
        &session,
        2,
        "history",
        json!({"objectType":"Airport","pk":"BER","limit":1}),
    )
    .await;
    let first = &response["result"]["structuredContent"];
    assert_eq!(first, &rest);
    assert_eq!(first["entries"].as_array().unwrap().len(), 2);
    assert_eq!(first["truncated"], true);
    let next = first["nextBeforeSeq"].as_i64().unwrap();
    assert!(
        first["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["seq"] == next)
    );
    let rest = support::success(
        running
            .get(&format!(
                "/api/objects/Airport/BER/history?limit=1&beforeSeq={next}"
            ))
            .await,
    )
    .await;
    let response = call_read_tool(
        &running,
        &session,
        3,
        "history",
        json!({"objectType":"Airport","pk":"BER","limit":1,"beforeSeq":next}),
    )
    .await;
    let last = &response["result"]["structuredContent"];
    assert_eq!(last, &rest);
    assert_eq!(last["entries"].as_array().unwrap().len(), 1);
    assert_eq!(last["truncated"], false);
    assert!(last["nextBeforeSeq"].is_null());
    assert!(!response.to_string().contains("must-not-leak"));
    assert_eq!(running.finish_actions().await.len(), 4);
}

#[tokio::test]
async fn mcp_history_byte_budget_matches_rest_without_partial_records() {
    for budget in [1024, 10_000] {
        let fixture = full_read_fixture().await;
        append_history_group(
            &fixture,
            &["secret-history-payload北京".repeat(100), "short".to_owned()],
        )
        .await;
        let mut limits = ontoforge_server::ServerLimits::default();
        limits.execution.max_decoded_bytes = budget;
        let running = fixture.start_with_limits(limits).await;
        let session = initialize(&running).await;
        let rest_response = running
            .get("/api/objects/Airport/BER/history?limit=100")
            .await;
        let status = rest_response.status();
        let rest: Value = rest_response.json().await.unwrap();
        let response = call_read_tool(
            &running,
            &session,
            2,
            "history",
            json!({"objectType":"Airport","pk":"BER","limit":100}),
        )
        .await;
        if budget == 1024 {
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            assert_eq!(rest["code"], "COST_EXCEEDED");
            assert_eq!(
                response["error"]["data"]["code"], rest["code"],
                "{response}"
            );
            assert!(response["result"].is_null());
            assert!(!response.to_string().contains("secret-history-payload"));
        } else {
            assert_eq!(status, reqwest::StatusCode::OK);
            assert_eq!(response["result"]["structuredContent"], rest);
            assert_eq!(rest["entries"].as_array().unwrap().len(), 3);
            assert!(serde_json::to_vec(&rest).unwrap().len() <= budget);
        }
        let audit = running.finish_actions().await;
        assert_eq!(audit.len(), 2);
        if budget == 1024 {
            assert!(
                audit
                    .iter()
                    .any(|row| row["request_id"] == response["error"]["data"]["request_id"])
            );
        }
    }
}

#[tokio::test]
async fn mcp_action_criteria_failure_retains_safe_domain_diagnostics() {
    let running = support::fixture().await.start().await;
    let session = initialize(&running).await;
    let response = call_read_tool(
        &running,
        &session,
        2,
        "action_delayFlight",
        json!({
            "params":{"flight":"F-1001", "delayMinutes":0}, "validateOnly":true, "commandId":"criteria-error"
        }),
    )
    .await;
    assert_eq!(
        response["error"]["data"]["code"], "CRITERIA_FAILED",
        "{response}"
    );
    assert_eq!(
        response["error"]["data"]["message"],
        "Delay must be positive and cancelled flights cannot be delayed"
    );
    assert!(response["error"]["data"]["parameterErrors"].is_null());
    running.finish().await;
}

async fn envelope_fixture() -> support::Fixture {
    use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
    use ontoforge_storage::reload::{ReloadOptions, reload};
    let fixture = support::fixture().await;
    let loaded = ontoforge_schema::load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        (
            "envelope.yaml",
            r"
kind: ObjectType
apiName: EnvelopeRecord
backing: editOnly
primaryKey: code
properties:
  code: {type: string, required: true}
  commandValue: {type: string}
  validateValue: {type: string}
  payload: {type: string}
---
kind: ActionType
apiName: echoEnvelope
parameters:
  commandId: {type: string, required: true}
  validateOnly: {type: string, required: true}
  params: {type: string, required: true}
rules:
  - kind: createObject
    objectType: EnvelopeRecord
    properties:
      code: {static: envelope}
      commandValue: {param: commandId}
      validateValue: {param: validateOnly}
      payload: {param: params}
---
kind: ActionType
apiName: defaultEnvelope
parameters:
  label: {type: string, default: fallback}
rules:
  - kind: createObject
    objectType: EnvelopeRecord
    properties:
      code: {static: default}
      payload: {param: label}
",
        ),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let addition = loaded.ontology.unwrap();
    let mut ontology = fixture
        .store
        .active_definition()
        .await
        .unwrap()
        .ontology
        .clone();
    ontology.object_types.extend(addition.object_types);
    ontology.action_types.extend(addition.action_types);
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let scope = ontoforge_schema::load_scope_from_memory(&[("scope.yaml", "kind: Role\napiName: envelopeEditor\nobjectTypes:\n  EnvelopeRecord:\n    properties: [code, commandValue, validateValue, payload]\nactions: [echoEnvelope, defaultEnvelope]\ntoolsets: [core]\n---\nkind: RoleBinding\nrole: envelopeEditor\nsubjects:\n  - group: dispatchers\n")]).scope.unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&fixture.host.auth_config)),
        scope: &scope,
        sources: None,
    })
    .unwrap();
    let props = compiled.row_filter_properties();
    let plan = fixture.store.scope_plan(&scope, &props).await.unwrap();
    fixture
        .store
        .scope_apply(&scope, &props, &plan.hash, "envelope-test")
        .await
        .unwrap();
    fixture
}

#[tokio::test]
async fn mcp_action_envelope_preserves_colliding_business_values_and_controls() {
    let running = envelope_fixture().await.start().await;
    let session = initialize(&running).await;
    let params = json!({"commandId":"business-command","validateOnly":"business-validation","params":"business-payload"});
    let dryrun = call_read_tool(
        &running,
        &session,
        2,
        "action_echoEnvelope",
        json!({"validateOnly":true,"params":params}),
    )
    .await;
    assert_eq!(
        dryrun["result"]["structuredContent"]["validated"], true,
        "{dryrun}"
    );
    assert_eq!(
        running
            .get("/api/objects/EnvelopeRecord/envelope")
            .await
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    let missing_control = call_read_tool(
        &running,
        &session,
        3,
        "action_echoEnvelope",
        json!({"params":params}),
    )
    .await;
    assert_eq!(
        missing_control["error"]["data"]["code"], "MISSING_COMMAND_ID",
        "{missing_control}"
    );
    let args = json!({"commandId":"control-receipt","params":params});
    let committed =
        call_read_tool(&running, &session, 4, "action_echoEnvelope", args.clone()).await;
    let receipt = &committed["result"]["structuredContent"]["receipt"];
    assert_eq!(receipt["commandId"], "control-receipt", "{committed}");
    let row = support::success(running.get("/api/objects/EnvelopeRecord/envelope").await).await;
    assert_eq!(row["properties"]["commandValue"], "business-command");
    assert_eq!(row["properties"]["validateValue"], "business-validation");
    assert_eq!(row["properties"]["payload"], "business-payload");
    let replay = call_read_tool(&running, &session, 5, "action_echoEnvelope", args).await;
    assert_eq!(&replay["result"]["structuredContent"]["receipt"], receipt);
    running.finish_actions().await;
}

#[tokio::test]
async fn mcp_action_envelope_optional_params_apply_domain_defaults() {
    let running = envelope_fixture().await.start().await;
    let session = initialize(&running).await;
    for args in [
        json!({"validateOnly":true}),
        json!({"validateOnly":true,"params":{}}),
    ] {
        let dryrun = call_read_tool(&running, &session, 2, "action_defaultEnvelope", args).await;
        assert_eq!(
            dryrun["result"]["structuredContent"]["validated"], true,
            "{dryrun}"
        );
    }
    for params in [Value::Null, json!([]), json!(false), json!("wrong")] {
        let rejected = call_read_tool(
            &running,
            &session,
            3,
            "action_defaultEnvelope",
            json!({"validateOnly":true,"params":params}),
        )
        .await;
        assert_eq!(
            rejected["error"]["data"]["code"], "INVALID_PARAMETERS",
            "{rejected}"
        );
    }
    let committed = call_read_tool(
        &running,
        &session,
        4,
        "action_defaultEnvelope",
        json!({"commandId":"default-receipt"}),
    )
    .await;
    assert_eq!(
        committed["result"]["structuredContent"]["receipt"]["commandId"], "default-receipt",
        "{committed}"
    );
    let row = support::success(running.get("/api/objects/EnvelopeRecord/default").await).await;
    assert_eq!(row["properties"]["payload"], "fallback");
    running.finish_actions().await;
}

#[tokio::test]
async fn mcp_action_envelope_rejects_flat_and_invalid_nested_requests() {
    let running = support::fixture().await.start().await;
    let session = initialize(&running).await;
    for args in [
        json!({"validateOnly":true,"flight":"F-1001","delayMinutes":1}),
        json!({"validateOnly":true}),
        json!({"validateOnly":true,"params":{"flight":"F-1001"}}),
        json!({"validateOnly":true,"params":{"flight":"F-1001","delayMinutes":1,"extra":true}}),
        json!({"validateOnly":true,"params":{"flight":"F-1001","delayMinutes":1},"extra":true}),
        json!({"validateOnly":true,"params":{"flight":"F-1001","delayMinutes":"wrong"}}),
        json!({"validateOnly":"true","params":{"flight":"F-1001","delayMinutes":1}}),
        json!({"commandId":false,"params":{"flight":"F-1001","delayMinutes":1}}),
    ] {
        let rejected =
            call_read_tool(&running, &session, 2, "action_delayFlight", args.clone()).await;
        assert_eq!(
            rejected["error"]["data"]["code"], "INVALID_PARAMETERS",
            "{args}: {rejected}"
        );
    }
    running.finish().await;
}

#[tokio::test]
async fn mcp_read_tools_use_declared_argument_shapes() {
    let fixture = full_read_fixture().await;
    let running = fixture.start().await;
    let initialized = running.client.post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"read-contract","version":"0"}}}))
        .send().await.unwrap();
    let session = initialized.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let notification = running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(notification.status().is_success());
    let described = call_read_tool(&running, &session, 2, "describe", json!({})).await;
    assert!(
        described["error"].is_null(),
        "describe accepts its empty argument schema: {described}"
    );
    let types = described["result"]["structuredContent"]["objectTypes"]
        .as_array()
        .expect("describe object types");
    assert!(types.iter().any(|ty| ty["apiName"] == "Airport"));
    assert!(!types.iter().any(|ty| ty["apiName"] == "Aircraft"));
    for (id, name) in [(3, "query"), (4, "object_set"), (5, "searchAround")] {
        let payload = call_read_tool(
            &running,
            &session,
            id,
            name,
            json!({"request":{
                "set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":3}
            }}),
        )
        .await;
        assert!(
            payload["error"].is_null(),
            "{name} accepts request-only arguments: {payload}"
        );
        assert_eq!(
            payload["result"]["structuredContent"]["rows"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }
    let aggregated = call_read_tool(
        &running,
        &session,
        6,
        "aggregate",
        json!({"request":{
            "set":{"kind":"type","object_type":"Airport"},"measures":[{"op":"count"}]
        }}),
    )
    .await;
    assert!(
        aggregated["error"].is_null(),
        "aggregate accepts request-only arguments: {aggregated}"
    );
    assert_eq!(
        aggregated["result"]["structuredContent"]["buckets"][0]["values"][0],
        3
    );
    let denied = call_read_tool(&running, &session, 7, "query", json!({"request":{
        "set":{"kind":"type","object_type":"Aircraft"},"select":["tailNumber"],"page":{"first":1}
    }})).await;
    assert!(
        denied["error"].is_object(),
        "ungranted types remain hidden: {denied}"
    );
    let missing = call_read_tool(&running, &session, 8, "get_object", json!({"pk":"BER"})).await;
    assert!(
        missing["error"].is_object(),
        "get_object still requires objectType: {missing}"
    );
    let audit = running.finish().await;
    assert_eq!(
        audit.len(),
        4,
        "three list calls and one aggregate must be audited once each"
    );
    assert!(audit.iter().all(|row| row["status"] == "success"));
}

fn scope(toolsets: &[Toolset]) -> PrincipalScope {
    PrincipalScope {
        toolsets: toolsets.iter().copied().collect(),
        ..PrincipalScope::default()
    }
}

#[test]
fn mcp_core_full_describe_query_history_equal_rest() {
    let core = ToolCatalog::for_scope(&scope(&[Toolset::Core]));
    assert!(core.all().iter().any(|tool| tool.name == DESCRIBE_TOOL));
    assert!(core.all().iter().any(|tool| tool.name == "object_set"));
    assert!(core.all().iter().any(|tool| tool.name == "searchAround"));
    assert!(!core.all().iter().any(|tool| tool.name == AGGREGATE_TOOL));
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Core, Toolset::Full]));
    assert!(full.all().iter().any(|tool| tool.name == AGGREGATE_TOOL));
    assert!(full.all().iter().any(|tool| tool.name == HISTORY_TOOL));
}

#[test]
fn mcp_list_filters_before_keyset_and_direct_call_denies() {
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Full]));
    let first = full.page(None, 1);
    assert_eq!(first.tools.len(), 1);
    assert!(first.has_more);
    let second = full.page(first.next.as_deref(), 16);
    assert!(!second.tools.is_empty());
    assert!(!full.allows("admin_reload"));
    assert!(!full.allows("scope_apply"));
}

#[test]
fn mcp_inflight_snapshot_and_revoke_on_next_call() {
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Full]));
    assert!(full.allows(DESCRIBE_TOOL));
    let revoked = ToolCatalog::for_scope(&scope(&[]));
    assert!(!revoked.allows(DESCRIBE_TOOL));
}

#[tokio::test]
async fn mcp_real_http_client_lists_core_tools_and_rejects_hidden_call() {
    let fixture = support::fixture().await;
    let running = fixture.start().await;
    let url = format!("{}/mcp", running.url);
    let client = reqwest::Client::new();
    let initialize = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"contract","version":"0"}}}))
        .send()
        .await
        .unwrap();
    assert!(initialize.status().is_success());
    let session = initialize
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let initialized = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(initialized.status().is_success());
    let listed = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    let body = listed.text().await.unwrap();
    let payload: Value = body
        .lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|value| serde_json::from_str(value).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .unwrap_or(Value::Null);
    assert!(
        payload["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == DESCRIBE_TOOL)
    );
    let denied = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"admin_reload","arguments":{}}}))
        .send()
        .await
        .unwrap();
    let denied_body = denied.text().await.unwrap();
    assert!(denied_body.contains("error") || denied_body.contains("tool unavailable"));
    let _ = running.finish().await;
}
