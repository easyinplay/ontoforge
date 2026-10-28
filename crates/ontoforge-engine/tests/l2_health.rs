use std::collections::BTreeSet;

use camino::Utf8Path;
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::printer::l2::{L2Policy, validate_l2};
use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
use ontoforge_engine::{Engine, EngineError, QueryContext};
use serde_json::json;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn config(url: &str, l2_user: &str) -> String {
    format!(
        "[trino]\nurl = \"{url}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"{l2_user}\"\n"
    )
}

#[tokio::test]
async fn l2_health_unverified_identity_never_submits_a_statement() {
    let server = MockServer::start().await;
    let cfg = parse_server_config(&config(&server.uri(), "l2"))
        .expect("valid configuration")
        .trino
        .expect("Trino configuration");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("identities build");
    let policy = L2Policy {
        raw_allowed: true,
        catalogs: BTreeSet::from(["demo".to_owned()]),
        schemas: BTreeSet::from(["demo.tiny".to_owned()]),
        edit_catalog: "editstore".to_owned(),
        max_rows: 10,
    };
    let sql = validate_l2("SELECT * FROM demo.tiny.nation", &policy).expect("authorized syntax");
    let result = engines
        .l2
        .expect("L2 identity")
        .execute(&sql, &QueryContext::new("l2-health-red".to_owned(), vec![]))
        .await;
    assert!(
        matches!(result, Err(EngineError::Unavailable { .. })),
        "unverified L2 must be disabled, got {result:?}"
    );
    assert!(
        server
            .received_requests()
            .await
            .expect("requests")
            .is_empty(),
        "a disabled L2 identity must never submit SQL"
    );
}

#[test]
fn l2_health_rejects_reused_ontology_identity() {
    let result = parse_server_config(&config("http://127.0.0.1:8080", "ontology"));
    assert!(
        result.is_err(),
        "ontology and L2 must have different identities"
    );
}

#[test]
fn l2_health_rejects_probe_for_wrong_edit_catalog() {
    let text = format!(
        "{}\n[store]\nkind = \"postgres\"\nurl_env = \"EDIT_URL\"\n[store.trino]\ncatalog = \"real_edit\"\n",
        attested_config("http://127.0.0.1:8080")
    );
    assert!(
        parse_server_config(&text).is_err(),
        "the ACL probe must cover the real edit catalog"
    );
}

fn attested_config(url: &str) -> String {
    format!(
        "{}\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n",
        config(url, "l2")
    )
}

async fn answer(server: &MockServer, user: &str, fragment: &str, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", user))
        .and(body_string_contains(fragment))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

fn rows(rows: &serde_json::Value, value_type: &str) -> serde_json::Value {
    json!({"id":"q1", "columns":[{"name":"_col0","type":value_type}], "data":rows, "stats":{"state":"FINISHED"}})
}

fn denied(name: &str) -> serde_json::Value {
    json!({"id":"q1", "error":{"errorName":name,"message":"sensitive SQL and credentials"}, "stats":{"state":"FAILED"}})
}

async fn setup_probes(server: &MockServer, negative_error: Option<&str>, leak: bool) {
    for relation in ["demo", "outside", "editstore"] {
        answer(
            server,
            "ontology",
            &format!("\"{relation}\"."),
            rows(&json!([[1]]), "integer"),
        )
        .await;
    }
    answer(server, "l2", "\"demo\".", rows(&json!([[1]]), "integer")).await;
    for relation in ["outside", "editstore"] {
        let response = negative_error.map_or_else(|| rows(&json!([[1]]), "integer"), denied);
        answer(server, "l2", &format!("\"{relation}\"."), response).await;
    }
    answer(
        server,
        "ontology",
        "other_1",
        json!({"columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}],"data":[["other_1","RUNNING"]]}),
    )
    .await;
    answer(
        server,
        "l2",
        "other_1",
        rows(
            &if leak {
                json!([["other_1"]])
            } else {
                json!([])
            },
            "varchar",
        ),
    )
    .await;
}

#[tokio::test]
async fn l2_health_requires_exact_denial_and_known_query_isolation() {
    let server = MockServer::start().await;
    setup_probes(&server, Some("PERMISSION_DENIED"), false).await;
    let cfg = parse_server_config(&attested_config(&server.uri()))
        .expect("config")
        .trino
        .expect("trino");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
    let l2 = engines.l2.expect("L2");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::NotVerified
        })
    );
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "other_1").await,
        L2HealthStatus::Enabled
    );
    assert_eq!(l2.l2_status(), Some(L2HealthStatus::Enabled));
}

#[tokio::test]
async fn l2_health_finished_query_cannot_attest_active_isolation() {
    let server = MockServer::start().await;
    setup_probes(&server, Some("PERMISSION_DENIED"), false).await;
    Mock::given(method("POST"))
        .and(header("x-trino-user", "ontology"))
        .and(body_string_contains("state"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}],
            "data":[["other_1","FINISHED"]]
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    let cfg = parse_server_config(&attested_config(&server.uri()))
        .expect("config")
        .trino
        .expect("trino");
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &|_: &str| None).expect("engines");
    let l2 = engines.l2.expect("L2");
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "other_1").await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::OtherQueryUnconfirmed
        },
        "a retained finished query ID is not active-query evidence"
    );
}

#[tokio::test]
async fn l2_health_rejects_other_coordinator_without_network() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    let build = |url: &str| {
        let cfg = parse_server_config(&attested_config(url))
            .expect("config")
            .trino
            .expect("trino");
        build_trino_engines(&cfg, Utf8Path::new("."), &|_: &str| None).expect("engines")
    };
    let left = build(&first.uri());
    let right = build(&second.uri());
    assert_eq!(
        left.l2
            .expect("l2")
            .recheck_l2_acl(&right.ontology, "other_1")
            .await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::InvalidProbe
        }
    );
    assert!(
        first
            .received_requests()
            .await
            .expect("requests")
            .is_empty()
    );
    assert!(
        second
            .received_requests()
            .await
            .expect("requests")
            .is_empty()
    );
}

#[tokio::test]
async fn l2_health_loose_acl_and_non_permission_error_fail_closed() {
    for name in [None, Some("SYNTAX_ERROR"), Some("CATALOG_NOT_FOUND")] {
        let server = MockServer::start().await;
        setup_probes(&server, name, false).await;
        let cfg = parse_server_config(&attested_config(&server.uri()))
            .expect("config")
            .trino
            .expect("trino");
        let env = |_: &str| None;
        let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
        let l2 = engines.l2.expect("L2");
        assert_eq!(
            l2.recheck_l2_acl(&engines.ontology, "other_1").await,
            L2HealthStatus::Disabled {
                reason: L2DisableReason::NegativeProbeFailed
            }
        );
    }
}

#[tokio::test]
async fn l2_health_other_query_leak_disables_l2() {
    let server = MockServer::start().await;
    setup_probes(&server, Some("PERMISSION_DENIED"), true).await;
    let cfg = parse_server_config(&attested_config(&server.uri()))
        .expect("config")
        .trino
        .expect("trino");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
    let l2 = engines.l2.expect("L2");
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "other_1").await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::QueryVisibilityFailed
        }
    );
}

#[tokio::test]
async fn l2_health_recheck_revocation_disables_public_execution() {
    let server = MockServer::start().await;
    setup_probes(&server, Some("PERMISSION_DENIED"), false).await;
    let cfg = parse_server_config(&attested_config(&server.uri()))
        .expect("config")
        .trino
        .expect("trino");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
    let l2 = engines.l2.expect("L2");
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "other_1").await,
        L2HealthStatus::Enabled
    );
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", "l2"))
        .and(body_string_contains("\"outside\"."))
        .respond_with(ResponseTemplate::new(200).set_body_json(rows(&json!([[1]]), "integer")))
        .with_priority(1)
        .mount(&server)
        .await;
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "other_1").await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::NegativeProbeFailed
        }
    );
    let policy = L2Policy {
        raw_allowed: true,
        catalogs: BTreeSet::from(["demo".to_owned()]),
        schemas: BTreeSet::from(["demo.tiny".to_owned()]),
        edit_catalog: "editstore".to_owned(),
        max_rows: 10,
    };
    let sql = validate_l2("SELECT * FROM demo.tiny.nation", &policy).expect("SQL");
    let before = server.received_requests().await.expect("requests").len();
    assert!(matches!(
        l2.execute(&sql, &QueryContext::new("revoked".to_owned(), vec![]))
            .await,
        Err(EngineError::Unavailable { .. })
    ));
    assert_eq!(
        server.received_requests().await.expect("requests").len(),
        before
    );
}

#[tokio::test]
async fn l2_health_missing_negative_object_and_unsafe_query_id_disable() {
    let server = MockServer::start().await;
    setup_probes(&server, Some("PERMISSION_DENIED"), false).await;
    let cfg = parse_server_config(&attested_config(&server.uri()))
        .expect("config")
        .trino
        .expect("trino");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
    let l2 = engines.l2.expect("L2");
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "other_1' OR 1=1")
            .await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::InvalidProbe
        }
    );
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", "ontology"))
        .and(body_string_contains("\"outside\"."))
        .respond_with(ResponseTemplate::new(200).set_body_json(denied("CATALOG_NOT_FOUND")))
        .with_priority(1)
        .mount(&server)
        .await;
    assert_eq!(
        l2.recheck_l2_acl(&engines.ontology, "other_1").await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::ObjectUnconfirmed
        }
    );
}
