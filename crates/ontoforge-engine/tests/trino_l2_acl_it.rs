//! Real Trino 483 ACL acceptance. Start `bash ci/query-services.sh trino-l2` first.

use camino::Utf8Path;
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use reqwest::Client;
use serde_json::Value;

fn endpoint() -> String {
    std::env::var("ONTOFORGE_TRINO_L2_URL").unwrap_or_else(|_| "http://127.0.0.1:58081".to_owned())
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
    for _ in 0..200 {
        let Some(next) = page["nextUri"].as_str() else {
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
        let result = query(&client, &url, "l2", &sql).await;
        if catalog == "demo" {
            assert!(result.get("error").is_none(), "allowed table: {result}");
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
    let running = submit(
        &client,
        &url,
        "ontology",
        "SELECT count(*) FROM demo.sf1000.orders",
    )
    .await;
    let query_id = running["id"]
        .as_str()
        .expect("Trino assigned query ID")
        .to_owned();
    let cancel_url = running["nextUri"]
        .as_str()
        .expect("active query next URI")
        .to_owned();

    let cfg_text = format!(
        "[trino]\nurl = \"{url}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    );
    let cfg = parse_server_config(&cfg_text)
        .expect("ACL probe config")
        .trino
        .expect("trino");
    let env = |_: &str| None;
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env).expect("engines");
    let status = engines
        .l2
        .expect("L2 identity")
        .recheck_l2_acl(&engines.ontology, &query_id)
        .await;
    let _ = client.delete(cancel_url).send().await;
    assert_eq!(
        status,
        L2HealthStatus::Enabled,
        "known other query {query_id} must be invisible"
    );
}
