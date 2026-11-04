#![allow(
    dead_code,
    reason = "shared security evidence helpers have independent consumers"
)]

use camino::Utf8Path;
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_schema::ir::ScopeSet;
use ontoforge_storage::store::{EditStore, sqlite_connect_options};
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::BTreeMap;
use std::time::Duration;

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
}

pub async fn durable_rows(path: &Utf8Path) -> BTreeMap<String, Value> {
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(path))
        .await
        .unwrap();
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND (name GLOB 'of_obj_*' OR name GLOB 'of_lnk_*' OR name IN ('of_edit_log','of_edit_log_entry','of_edit_history','of_action_command','of_webhook_outbox')) ORDER BY name"
    ).fetch_all(&pool).await.unwrap();
    let mut state = BTreeMap::new();
    for table in tables {
        let escaped = table.replace('"', "\"\"");
        let columns = sqlx::query(sqlx::AssertSqlSafe(format!(
            "PRAGMA table_info(\"{escaped}\")"
        )))
        .fetch_all(&pool)
        .await
        .unwrap();
        let fields = columns
            .iter()
            .map(|column| {
                let name: String = column.get("name");
                format!(
                    "'{}', CAST(\"{}\" AS TEXT)",
                    name.replace('\'', "''"),
                    name.replace('"', "\"\"")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let encoded: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT json_group_array(json(row_json)) FROM (SELECT json_object({fields}) AS row_json FROM \"{escaped}\" ORDER BY rowid)"
        ))).fetch_one(&pool).await.unwrap();
        state.insert(table, serde_json::from_str(&encoded).unwrap());
    }
    pool.close().await;
    state
}

pub fn assert_one_flight_commit(before: &BTreeMap<String, Value>, after: &BTreeMap<String, Value>) {
    assert_eq!(before["of_obj_flight"], json!([]));
    let rows = after["of_obj_flight"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["of_version"], "1");
    assert_eq!(rows[0]["status"], "DELAYED");
    assert_eq!(rows[0]["delay_minutes"], "5");
    for (table, rows) in before {
        if (table.starts_with("of_obj_") || table.starts_with("of_lnk_"))
            && table != "of_obj_flight"
        {
            assert_eq!(&after[table], rows, "unrelated overlay {table}");
        }
    }
}

pub async fn ready(client: &reqwest::Client, url: &str) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let health: Value = client
                .get(format!("{url}/health"))
                .timeout(Duration::from_secs(2))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if health["status"] == "healthy" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("host must observe the current source definition before accepting source work");
}

pub async fn counts(path: &Utf8Path) -> [i64; 5] {
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(path))
        .await
        .unwrap();
    let mut result = [0; 5];
    for (index, sql) in [
        "SELECT COUNT(*) FROM of_edit_log",
        "SELECT COUNT(*) FROM of_edit_log_entry",
        "SELECT COUNT(*) FROM of_edit_history",
        "SELECT COUNT(*) FROM of_action_command",
        "SELECT COUNT(*) FROM of_webhook_outbox",
    ]
    .into_iter()
    .enumerate()
    {
        result[index] = sqlx::query_scalar(sql).fetch_one(&pool).await.unwrap();
    }
    pool.close().await;
    result
}

pub fn delta(after: [i64; 5], before: [i64; 5]) -> [i64; 5] {
    std::array::from_fn(|index| after[index] - before[index])
}

pub async fn apply_scope(store: &EditStore, scope: &ScopeSet) {
    let active = store.active_definition().await.unwrap();
    let config = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    }))
    .unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &active.ontology,
        profile: None,
        attrs: &attr_types(Some(&config)),
        scope,
        sources: None,
    })
    .unwrap();
    let props = compiled.row_filter_properties();
    let plan = store.scope_plan(scope, &props).await.unwrap();
    store
        .scope_apply(scope, &props, &plan.hash, "security-evidence")
        .await
        .unwrap();
}

pub async fn session(client: &reqwest::Client, url: &str, token: &str) -> String {
    let response = client.post(format!("{url}/mcp")).bearer_auth(token)
        .timeout(Duration::from_secs(15))
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"security-evidence","version":"0"}}}))
        .send().await.unwrap();
    assert!(response.status().is_success());
    let session = response.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let initialized = client
        .post(format!("{url}/mcp"))
        .timeout(Duration::from_secs(15))
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(initialized.status().is_success());
    session
}

pub async fn rpc(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    session: &str,
    request: Value,
) -> Value {
    let response = client
        .post(format!("{url}/mcp"))
        .timeout(Duration::from_secs(30))
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
        .json(&request)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body = response.text().await.unwrap();
    body.lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|data| serde_json::from_str(data).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .expect("MCP response JSON")
}

pub fn action_call(id: u64, command: &str, validate_only: bool) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{
        "name":"action_delayFlight","arguments":{"commandId":command,
        "validateOnly":validate_only,"params":{"flight":"F-1001","delayMinutes":5}}
    }})
}
