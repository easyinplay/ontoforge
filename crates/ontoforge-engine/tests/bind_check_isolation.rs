//! `describe_bindings` asks once per catalog and keeps one catalog's failure away from the
//! others (E3, E22). Trino is a wiremock server.

use std::time::Duration;

use ontoforge_engine::bind_check::{DescribedBinding, describe_bindings};
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{AnyEngine, EngineError, QueryContext};
use ontoforge_schema::ir::{BindingTarget, Ontology, ProfileName};
use ontoforge_schema::load_from_memory;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const OBJECTS: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
backing: source
---
kind: ObjectType
apiName: Pilot
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
backing: source
---
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
backing: source
";

const FLIGHT: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
";
const PILOT: &str = "\
kind: SourceBinding
target: { objectType: Pilot }
source: { catalog: demo, schema: aviation, table: pilots }
";
const AIRPORT: &str = "\
kind: SourceBinding
target: { objectType: Airport }
source: { catalog: other, schema: aviation, table: airports }
";

fn ontology() -> (Ontology, ProfileName) {
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", OBJECTS),
        ("bindings/dev/flight.yaml", FLIGHT),
        ("bindings/dev/pilot.yaml", PILOT),
        ("bindings/dev/airport.yaml", AIRPORT),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    (
        out.ontology.expect("ontology"),
        ProfileName::try_from("dev".to_owned()).expect("profile"),
    )
}

fn engine_for(server: &MockServer) -> AnyEngine {
    AnyEngine::Trino(
        TrinoEngine::new(TrinoConnection {
            url: reqwest::Url::parse(&server.uri()).expect("url"),
            user: "ontoforge".to_owned(),
            auth: Auth::None,
            ca_pem: None,
            connect_timeout: Duration::from_secs(5),
            max_page_bytes: 64 * 1024 * 1024,
            extra_origins: Vec::new(),
        })
        .expect("engine"),
    )
}

fn ctx() -> QueryContext {
    QueryContext::new(
        "trace-1".to_owned(),
        vec!["ontoforge".to_owned(), "validate-online".to_owned()],
    )
}

fn info_page(rows: &serde_json::Value) -> serde_json::Value {
    let columns: Vec<serde_json::Value> = [
        "table_schema",
        "table_name",
        "column_name",
        "data_type",
        "is_nullable",
    ]
    .iter()
    .map(|name| json!({ "name": name, "type": "varchar" }))
    .collect();
    json!({ "id": "q1", "columns": columns, "data": rows, "stats": { "state": "FINISHED" } })
}

fn described<'a>(all: &'a [DescribedBinding], name: &str) -> &'a DescribedBinding {
    all.iter()
        .find(|d| matches!(&d.key.target, BindingTarget::ObjectType(n) if n.as_str() == name))
        .unwrap_or_else(|| panic!("no binding for {name}"))
}

#[tokio::test]
async fn query_error_isolated_to_its_catalog() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("\"demo\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(info_page(&json!([
            ["aviation", "flights", "flightId", "varchar", "NO"],
            ["aviation", "pilots", "employeeId", "varchar", "NO"]
        ]))))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(body_string_contains("\"other\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "q2",
            "error": { "message": "denied", "errorName": "PERMISSION_DENIED" },
            "stats": { "state": "FAILED" }
        })))
        .mount(&server)
        .await;

    let (ontology, profile) = ontology();
    let all = describe_bindings(&engine_for(&server), &ontology, &profile, &ctx())
        .await
        .expect("a query error does not abort the round");

    assert_eq!(all.len(), 3);
    let flight = described(&all, "Flight");
    let columns = flight
        .physical
        .as_ref()
        .expect("described")
        .as_ref()
        .expect("table exists");
    assert_eq!(columns[0].name, "flightId");
    assert!(
        described(&all, "Pilot")
            .physical
            .as_ref()
            .is_ok_and(Option::is_some)
    );
    match &described(&all, "Airport").physical {
        Err(EngineError::Query { error_name, .. }) => assert_eq!(error_name, "PERMISSION_DENIED"),
        other => panic!("expected a Query error for catalog `other`, got {other:?}"),
    }

    let requests = server.received_requests().await.expect("recorded");
    assert_eq!(requests.len(), 2, "one request per catalog");
}

#[tokio::test]
async fn engine_wide_error_aborts_round() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let (ontology, profile) = ontology();
    let err = describe_bindings(&engine_for(&server), &ontology, &profile, &ctx())
        .await
        .expect_err("authentication failure aborts the round");
    assert!(matches!(err, EngineError::Unauthorized { .. }), "{err:?}");
}
