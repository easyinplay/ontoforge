use std::collections::BTreeSet;

use ontoforge_engine::printer::l2::{L2Error, L2Policy, validate_l2};
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{Engine, QueryContext};
use std::time::Duration;
use wiremock::MockServer;

fn policy() -> L2Policy {
    L2Policy {
        raw_allowed: true,
        catalogs: BTreeSet::from(["hive".to_owned()]),
        schemas: BTreeSet::from(["hive.sales".to_owned()]),
        edit_catalog: "edit".to_owned(),
        max_rows: 100,
    }
}

#[test]
fn l2_gate_accepts_only_validated_display() {
    let sql = validate_l2("SELECT id FROM hive.sales.orders; -- discarded", &policy())
        .expect("an authorized SELECT must be accepted");
    assert_eq!(sql.as_str(), "SELECT id FROM hive.sales.orders LIMIT 101");
    assert!(!sql.as_str().contains("discarded"));
}

#[test]
fn l2_gate_rejects_unauthorized_relation() {
    assert_eq!(
        validate_l2("SELECT * FROM edit.s.secret", &policy()),
        Err(L2Error::Denied)
    );
}

#[test]
fn l2_gate_preserves_offset_when_capping() {
    let sql = validate_l2(
        "SELECT id FROM hive.sales.orders ORDER BY id OFFSET 10",
        &policy(),
    )
    .expect("bounded offset query");
    assert!(sql.as_str().contains("OFFSET 10 ROWS"), "{sql}");
    assert!(sql.as_str().contains("FETCH FIRST 101 ROWS ONLY"), "{sql}");
}

#[test]
fn l2_gate_handles_cte_join_and_subquery() {
    let sql = validate_l2(
        "WITH q AS (SELECT id FROM hive.sales.orders) SELECT q.id FROM q JOIN hive.sales.shipments s ON q.id = s.id WHERE EXISTS (SELECT 1 FROM hive.sales.items i WHERE i.id = q.id)",
        &policy(),
    )
    .expect("all physical relations allowed");
    assert!(
        sql.as_str().contains("FROM q JOIN hive.sales.shipments"),
        "{sql}"
    );
}

#[test]
fn l2_gate_rejects_hidden_relations_and_table_functions() {
    for raw in [
        "SELECT * FROM hive.sales.orders WHERE EXISTS (SELECT 1 FROM edit.s.secret)",
        "WITH q AS (SELECT * FROM edit.s.secret) SELECT * FROM q",
        "SELECT * FROM hive.sales.orders JOIN hive.system.runtime r ON true",
        "SELECT * FROM hive.sales.orders(1)",
        "SELECT * FROM TABLE(hive.system.query('DELETE FROM edit.s.secret'))",
        "SELECT * FROM JSON_TABLE('[1]', '$[*]' COLUMNS(x INTEGER PATH '$')) AS jt",
        "SELECT * FROM orders",
    ] {
        assert!(validate_l2(raw, &policy()).is_err(), "accepted {raw}");
    }
}

#[test]
fn l2_gate_rejects_mutation_multiple_statements_and_scalar_qualified_calls() {
    for raw in [
        "CALL hive.system.flush_metadata_cache()",
        "EXECUTE task",
        "SELECT 1; DELETE FROM hive.sales.orders",
        "SELECT hive.system.query('DELETE FROM edit.s.secret')",
        "SELECT id INTO edit.s.secret FROM hive.sales.orders",
        "SELECT * FROM hive.sales.orders FOR UPDATE",
        "TABLE edit.s.secret",
    ] {
        assert!(validate_l2(raw, &policy()).is_err(), "accepted {raw}");
    }
}

#[test]
fn l2_gate_requires_raw_grant_and_rejects_system_even_if_granted() {
    let mut policy = policy();
    policy.raw_allowed = false;
    assert_eq!(validate_l2("SELECT 1", &policy), Err(L2Error::Denied));
    policy.raw_allowed = true;
    policy.catalogs.insert("system".to_owned());
    policy.schemas.insert("hive.system".to_owned());
    for raw in [
        "SELECT * FROM system.runtime.queries",
        "SELECT * FROM hive.system.runtime",
        "SELECT * FROM hive.sales.\"orders$partitions\"",
    ] {
        assert_eq!(validate_l2(raw, &policy), Err(L2Error::Denied), "{raw}");
    }
}

#[test]
fn l2_gate_bounds_input_and_rejects_malformed_or_dynamic_limits() {
    for raw in [
        "",
        "SELECT id FROM hive.sales.orders LIMIT (SELECT 1)",
        "SELECT id FROM hive.sales.orders LIMIT 1, 2",
        "SELECT id FROM hive.sales.orders FETCH FIRST 10 PERCENT ROWS ONLY",
        "SELECT id FROM hive.sales.orders FETCH FIRST 10 ROWS WITH TIES",
    ] {
        assert_eq!(
            validate_l2(raw, &policy()),
            Err(L2Error::Unsupported),
            "{raw}"
        );
    }
    let long = format!("SELECT '{}'", "a".repeat(32 * 1024));
    assert_eq!(validate_l2(&long, &policy()), Err(L2Error::Unsupported));
}

#[test]
fn l2_gate_bounded_parser_stack_handles_deep_or_long_input() {
    let long = format!(
        "SELECT {} FROM hive.sales.orders",
        "1 + ".repeat(2_500) + "1"
    );
    let result = validate_l2(&long, &policy());
    assert!(result.is_ok() || result == Err(L2Error::Unsupported));
    let deep = format!("SELECT {}1{}", "(".repeat(80), ")".repeat(80));
    assert_eq!(validate_l2(&deep, &policy()), Err(L2Error::Unsupported));
}

#[tokio::test]
async fn l2_gate_rejection_cannot_send_trino_post() {
    let server = MockServer::start().await;
    let engine = TrinoEngine::new(TrinoConnection {
        url: reqwest::Url::parse(&server.uri()).expect("URL"),
        user: "test".to_owned(),
        auth: Auth::None,
        ca_pem: None,
        connect_timeout: Duration::from_secs(1),
        max_page_bytes: 4096,
        extra_origins: Vec::new(),
    })
    .expect("engine");
    let gate = validate_l2("SELECT * FROM edit.s.secret", &policy());
    if let Ok(sql) = gate {
        let _ = engine
            .execute(&sql, &QueryContext::new("probe".to_owned(), vec![]))
            .await;
    }
    assert!(
        server
            .received_requests()
            .await
            .expect("requests")
            .is_empty()
    );
}
