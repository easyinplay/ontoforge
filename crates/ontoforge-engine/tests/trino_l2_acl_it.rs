//! Real Trino 483 ACL acceptance. Start `bash ci/query-services.sh trino-l2` first.

use camino::Utf8Path;
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::printer::l2::{L2Error, L2Policy, validate_l2};
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use ontoforge_engine::{Engine, QueryContext};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

#[derive(Deserialize)]
struct CorpusCase {
    id: String,
    sql: String,
    expected: String,
}

async fn assert_corpus_on_verified_l2(l2: &TrinoEngine) {
    let policy = L2Policy {
        raw_allowed: true,
        catalogs: BTreeSet::from(["demo".to_owned(), "hive".to_owned()]),
        schemas: BTreeSet::from(["demo.tiny".to_owned(), "hive.sales".to_owned()]),
        edit_catalog: "editstore".to_owned(),
        max_rows: 100,
    };
    let cases: Vec<CorpusCase> =
        serde_json::from_str(include_str!("data/l2_corpus.json")).expect("104-ID corpus");
    assert_eq!(cases.len(), 104);
    let rejected = cases.iter().find(|case| case.id == "G11").expect("G11");
    assert_eq!(rejected.expected, "unsupported");
    assert_eq!(
        validate_l2(&rejected.sql, &policy),
        Err(L2Error::Unsupported)
    );

    let allowed: Vec<_> = cases
        .iter()
        .filter(|case| case.expected == "allow")
        .collect();
    assert_eq!(allowed.len(), 32);
    let mut failures = Vec::new();
    for case in allowed {
        let id = case.id.as_str();
        let sql = validate_l2(&case.sql, &policy).expect("validated Display SQL");
        if id == "B53" {
            assert_eq!(sql.as_str(), "SELECT 1 FROM hive.sales.orders LIMIT 101");
        }
        let output = match l2
            .execute(&sql, &QueryContext::new(id.to_owned(), vec![]))
            .await
        {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{id}: {error}"));
                continue;
            }
        };
        let expected_rows = match id {
            "G04" => 3,
            "G16" | "G22" => 0,
            "G18" => 2,
            _ => 1,
        };
        println!("{id}: FINISHED, {} row(s)", output.rows.len());
        if output.rows.len() != expected_rows {
            failures.push(format!(
                "{id}: expected {expected_rows} rows, got {}",
                output.rows.len()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Trino 483 corpus failures: {failures:#?}"
    );
}

fn endpoint() -> String {
    std::env::var("ONTOFORGE_TRINO_L2_URL").unwrap_or_else(|_| "http://127.0.0.1:58081".to_owned())
}

fn acl_config(url: &str) -> String {
    format!(
        "[trino]\nurl = \"{url}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    )
}

async fn submit(client: &Client, url: &str, user: &str, sql: &str) -> Value {
    let bytes = client
        .post(format!("{url}/v1/statement"))
        .header("X-Trino-User", user)
        .body(sql.to_owned())
        .send()
        .await
        .expect("Trino statement POST")
        .error_for_status()
        .expect("statement response")
        .bytes()
        .await
        .expect("Trino page");
    serde_json::from_slice(&bytes).expect("Trino JSON")
}

async fn finish(client: &Client, mut page: Value) -> Value {
    let mut data = Vec::new();
    for _ in 0..200 {
        if let Some(rows) = page.get_mut("data").and_then(Value::as_array_mut) {
            data.append(rows);
        }
        let Some(next) = page["nextUri"].as_str() else {
            if !data.is_empty() {
                page["data"] = Value::Array(data);
            }
            return page;
        };
        let bytes = client
            .get(next)
            .send()
            .await
            .expect("Trino next page")
            .error_for_status()
            .expect("next page response")
            .bytes()
            .await
            .expect("next page body");
        page = serde_json::from_slice(&bytes).expect("next page JSON");
    }
    panic!("Trino query did not finish within 200 pages");
}

async fn query(client: &Client, url: &str, user: &str, sql: &str) -> Value {
    finish(client, submit(client, url, user, sql).await).await
}

async fn assert_version(client: &Client, url: &str) {
    let bytes = client
        .get(format!("{url}/v1/info"))
        .send()
        .await
        .expect("independent L2 container")
        .bytes()
        .await
        .expect("info body");
    let info: Value = serde_json::from_slice(&bytes).expect("info JSON");
    assert_eq!(info["nodeVersion"]["version"], "483");
    assert_eq!(info["starting"], false);
}

#[tokio::test]
#[ignore = "requires independent Trino 483"]
async fn trino_l2_acl_rejects_existing_outside_and_edit_tables() {
    let url = endpoint();
    let client = Client::new();
    assert_version(&client, &url).await;
    for catalog in ["demo", "outside", "editstore"] {
        let sql = format!("SELECT 1 FROM {catalog}.tiny.nation LIMIT 1");
        let exists = query(&client, &url, "ontology", &sql).await;
        assert!(
            exists.get("error").is_none(),
            "{catalog} must really exist: {exists}"
        );
        assert!(
            exists["data"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty()),
            "{catalog} must contain a readable row: {exists}"
        );
        let result = query(&client, &url, "l2", &sql).await;
        if catalog == "demo" {
            assert!(result.get("error").is_none(), "allowed table: {result}");
            assert!(
                result["data"]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty()),
                "L2 allowed table must yield a row: {result}"
            );
        } else {
            assert_eq!(
                result["error"]["errorName"], "PERMISSION_DENIED",
                "{catalog}: {result}"
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires independent Trino 483"]
async fn trino_l2_acl_hides_known_active_other_query_and_attests_engine() {
    let url = endpoint();
    let client = Client::new();
    assert_version(&client, &url).await;
    let mut running = submit(
        &client,
        &url,
        "ontology",
        "SELECT count(*) FROM demo.sf100000.orders",
    )
    .await;
    let query_id = running["id"]
        .as_str()
        .expect("Trino assigned query ID")
        .to_owned();
    assert!(
        running.get("error").is_none(),
        "long-running query failed: {running}"
    );
    for _ in 0..10 {
        if running["stats"]["state"] == "RUNNING" {
            break;
        }
        let next = running["nextUri"].as_str().expect("query is still active");
        let bytes = client
            .get(next)
            .send()
            .await
            .expect("query next page")
            .bytes()
            .await
            .expect("query page");
        running = serde_json::from_slice(&bytes).expect("query page JSON");
        assert!(
            running.get("error").is_none(),
            "long-running query failed: {running}"
        );
    }
    assert_eq!(
        running["stats"]["state"], "RUNNING",
        "query must stay active"
    );
    let cancel_url = running["nextUri"]
        .as_str()
        .expect("active query next URI")
        .to_owned();
    let visibility_sql =
        format!("SELECT query_id FROM system.runtime.queries WHERE query_id = '{query_id}'");
    let mut confirmed = false;
    let mut last_observation = Value::Null;
    for _ in 0..20 {
        let observation = query(&client, &url, "ontology", &visibility_sql).await;
        if observation["data"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row[0] == query_id))
        {
            confirmed = true;
            break;
        }
        last_observation = observation;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        confirmed,
        "ontology did not confirm active query {query_id}; last={last_observation}"
    );

    let cfg_text = acl_config(&url);
    let cfg = parse_server_config(&cfg_text)
        .expect("ACL probe config")
        .trino
        .expect("trino");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
    let l2 = engines.l2.expect("L2 identity");
    let status = l2.recheck_l2_acl(&engines.ontology, &query_id).await;
    let _ = client.delete(cancel_url).send().await;
    assert_eq!(
        status,
        L2HealthStatus::Enabled,
        "known other query {query_id} must be invisible"
    );

    assert_corpus_on_verified_l2(&l2).await;
}

#[tokio::test]
#[ignore = "requires independent Trino 483 loose ACL control"]
async fn trino_l2_acl_loosened_rule_fails_closed() {
    let url = std::env::var("ONTOFORGE_TRINO_L2_LOOSE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:58082".to_owned());
    let client = Client::new();
    assert_version(&client, &url).await;
    let readable = query(
        &client,
        &url,
        "l2",
        "SELECT 1 FROM outside.tiny.nation LIMIT 1",
    )
    .await;
    assert!(
        readable.get("error").is_none(),
        "loose rule must read actual table: {readable}"
    );
    let cfg = parse_server_config(&acl_config(&url))
        .expect("ACL probe config")
        .trino
        .expect("trino");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
    let status = engines
        .l2
        .expect("L2 identity")
        .recheck_l2_acl(&engines.ontology, "dummy_1")
        .await;
    assert_eq!(
        status,
        L2HealthStatus::Disabled {
            reason: ontoforge_engine::trino::l2_health::L2DisableReason::NegativeProbeFailed,
        }
    );
}

fn monitor_engines(url: &str, sql: &str) -> (Arc<TrinoEngine>, Arc<TrinoEngine>) {
    let mut cfg = parse_server_config(&acl_config(url))
        .expect("config")
        .trino
        .expect("trino");
    cfg.l2_acl.as_mut().expect("ACL").active_query_sql = Some(sql.to_owned());
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &|_: &str| None).expect("engines");
    (
        Arc::new(engines.l2.expect("l2")),
        Arc::new(engines.ontology),
    )
}

#[tokio::test]
#[ignore = "requires independent Trino 483"]
async fn trino_l2_monitor_startup_cancels_real_active_probe() {
    let url = endpoint();
    let client = Client::new();
    assert_version(&client, &url).await;
    let (l2, ontology) = monitor_engines(&url, "SELECT count(*) FROM demo.sf100000.orders");
    let monitor = l2
        .clone()
        .start_l2_acl_monitor(ontology)
        .await
        .expect("monitor");
    assert_eq!(l2.l2_status(), Some(L2HealthStatus::Enabled));
    let active_sql = "SELECT query_id FROM system.runtime.queries WHERE state = 'RUNNING' AND query LIKE 'SELECT count(*) FROM demo.sf100000.orders%'";
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let active = query(&client, &url, "ontology", active_sql).await;
            assert!(
                active.get("error").is_none(),
                "runtime observation: {active}"
            );
            if active["data"].as_array().is_none_or(|rows| rows.is_empty()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("active probe cancelled before monitor admission");
    monitor.shutdown().await.expect("shutdown");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: ontoforge_engine::trino::l2_health::L2DisableReason::Stopped
        })
    );
}

#[tokio::test]
#[ignore = "requires independent Trino 483"]
async fn trino_l2_monitor_short_query_does_not_prove_active_isolation() {
    let url = endpoint();
    let (l2, ontology) = monitor_engines(&url, "SELECT 1");
    let monitor = l2
        .clone()
        .start_l2_acl_monitor(ontology)
        .await
        .expect("monitor");
    assert!(
        matches!(l2.l2_status(), Some(L2HealthStatus::Disabled { .. })),
        "a finished short query cannot attest isolation"
    );
    monitor.shutdown().await.expect("shutdown");
}

#[tokio::test]
#[ignore = "requires independent Trino 483 loose ACL control"]
async fn trino_l2_monitor_loose_acl_fails_closed() {
    let url = std::env::var("ONTOFORGE_TRINO_L2_LOOSE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:58082".to_owned());
    let (l2, ontology) = monitor_engines(&url, "SELECT count(*) FROM demo.sf100000.orders");
    let monitor = l2
        .clone()
        .start_l2_acl_monitor(ontology)
        .await
        .expect("monitor");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: ontoforge_engine::trino::l2_health::L2DisableReason::NegativeProbeFailed
        })
    );
    monitor.shutdown().await.expect("shutdown");
}
