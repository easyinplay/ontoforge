#[path = "support/security_evidence.rs"]
mod evidence;
#[path = "support/pg_real.rs"]
mod support;

use std::sync::atomic::Ordering;
use std::time::Duration;

use ontoforge_storage::reload::{ReloadOptions, reload};
use serde_json::json;

async fn pg_durable_rows(
    running: &support::Running,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let edit_schema = format!("{}_edit", running.schema_sys.strip_suffix("_sys").unwrap());
    let tables: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_schema, table_name FROM information_schema.tables WHERE table_schema IN ($1, $2) AND (left(table_name, 7) IN ('of_obj_', 'of_lnk_') OR table_name IN ('of_edit_log','of_edit_log_entry','of_edit_history','of_action_command','of_webhook_outbox')) ORDER BY table_name"
    ).bind(&edit_schema).bind(&running.schema_sys).fetch_all(&running.side).await.unwrap();
    let mut state = std::collections::BTreeMap::new();
    for (schema, table) in tables {
        let sql = format!(
            "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]'::jsonb) FROM \"{}\".\"{}\" t",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\"")
        );
        let rows: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_one(&running.side)
            .await
            .unwrap();
        state.insert(table, rows);
    }
    state
}

#[tokio::test]
#[ignore = "requires ONTOFORGE_PG_URL and ONTOFORGE_TRINO_URL with aviation source fixtures"]
async fn pg_http_definition_stale_barrier_has_zero_business_writes() {
    support::run_case(|mut running| async move {
        running.client = evidence::client();
        for validate_only in [false, true] {
            evidence::ready(&running.client, &running.url).await;
            let old = running.other.active_definition().await.unwrap();
            let before = running.counts().await;
            let before_rows = pg_durable_rows(&running).await;
            running.engine.pause.store(true, Ordering::SeqCst);
            let client = running.client.clone();
            let url = running.url.clone();
            let token = running.token.clone();
            let pending = tokio::spawn(async move {
                client.post(format!("{url}/api/actions/delayFlight")).bearer_auth(token)
                    .json(&json!({"commandId":format!("pg-definition-stale-{validate_only}"),"validateOnly":validate_only,
                        "items":[{"params":{"flight":"F-1001","delayMinutes":5}}]}))
                    .send().await.unwrap()
            });
            tokio::time::timeout(Duration::from_secs(5), running.engine.entered.notified()).await.unwrap();
            let mut changed = old.ontology.clone();
            changed.description = Some(format!("pg-definition-barrier-{validate_only}"));
            tokio::time::timeout(Duration::from_secs(5), reload(&running.other, &changed, &ReloadOptions::default()))
                .await.unwrap().unwrap();
            assert_ne!(running.other.active_definition().await.unwrap().def_version, old.def_version);
            running.engine.release.notify_one();
            let response = tokio::time::timeout(Duration::from_secs(30), pending).await.unwrap().unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
            let error: serde_json::Value = response.json().await.unwrap();
            assert_eq!(error["code"], "DEFINITION_STALE", "{error}");
            assert_eq!(running.counts().await, before, "definition fence cannot retry or mutate business state");
            assert_eq!(pg_durable_rows(&running).await, before_rows);
        }
        running.finish().await;
    }).await;
}

#[tokio::test]
#[ignore = "requires ONTOFORGE_PG_URL and ONTOFORGE_TRINO_URL with aviation source fixtures"]
async fn pg_mcp_rest_same_command_race_one_persistent_receipt() {
    support::run_case(|mut running| async move {
        running.client = evidence::client();
        let session = evidence::session(&running.client, &running.url, &running.token).await;
        assert_eq!(running.counts().await, [0; 5]);
        let before_rows = pg_durable_rows(&running).await;
        running.engine.pause.store(true, Ordering::SeqCst);
        let client = running.client.clone();
        let url = running.url.clone();
        let token = running.token.clone();
        let pending = tokio::spawn(async move {
            client.post(format!("{url}/api/actions/delayFlight")).bearer_auth(token)
                .json(&json!({"commandId":"pg-cross-transport","items":[{"params":{"flight":"F-1001","delayMinutes":5}}]}))
                .send().await.unwrap()
        });
        tokio::time::timeout(Duration::from_secs(5), running.engine.entered.notified()).await.unwrap();
        let mcp = tokio::time::timeout(Duration::from_secs(30), evidence::rpc(
            &running.client, &running.url, &running.token, &session,
            evidence::action_call(2, "pg-cross-transport", false))).await.unwrap();
        assert!(mcp.get("error").is_none(), "{mcp}");
        assert_eq!(running.counts().await, [1; 5], "MCP wins once while REST is in flight");
        let committed_rows = pg_durable_rows(&running).await;
        assert_eq!(before_rows["of_obj_flight"], json!([]));
        let flight = committed_rows["of_obj_flight"].as_array().unwrap();
        assert_eq!(flight.len(), 1);
        assert_eq!(flight[0]["of_version"], 1);
        assert_eq!(flight[0]["status"], "DELAYED");
        assert_eq!(flight[0]["delay_minutes"], 5);
        for (table, rows) in &before_rows {
            if (table.starts_with("of_obj_") || table.starts_with("of_lnk_")) && table != "of_obj_flight" {
                assert_eq!(&committed_rows[table], rows, "unrelated overlay {table}");
            }
        }
        running.engine.release.notify_one();
        let response = tokio::time::timeout(Duration::from_secs(30), pending).await.unwrap().unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let receipt: serde_json::Value = response.json().await.unwrap();
        assert!(receipt["eventId"].is_string());
        assert!(receipt["committedAt"].is_string());
        assert_eq!(mcp["result"]["structuredContent"]["receipt"], receipt);
        let replay = evidence::rpc(&running.client, &running.url, &running.token, &session,
            evidence::action_call(3, "pg-cross-transport", false)).await;
        assert_eq!(replay["result"]["structuredContent"]["receipt"], receipt);
        assert_eq!(running.counts().await, [1; 5]);
        assert_eq!(pg_durable_rows(&running).await, committed_rows);
        running.finish().await;
    }).await;
}

#[tokio::test]
#[ignore = "requires real PostgreSQL and an authenticated ontoforge HTTP server"]
#[allow(
    clippy::too_many_lines,
    reason = "Keep the snapshot and revoke scenario in execution order"
)]
async fn pg_http_mixed_snapshot_reload_revoke_and_history() {
    support::run_case(|running| async move {
        let bearer = running.token.clone();
        let (status, metadata) = running.get_json("/api/ontology", &bearer).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{metadata}");
        assert_eq!(metadata["engineStatus"], "available", "{metadata}");
        assert!(
            metadata["objectTypes"]
                .as_array()
                .is_some_and(|types| !types.is_empty())
        );

        let query = serde_json::json!({
            "set": {"kind": "type", "object_type": "Flight"},
            "select": ["flightId", "status"],
            "page": {"first": 3}
        });
        let (status, page) = running.post_json("/api/query", &bearer, &query).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{page}");
        assert!(page["rows"].as_array().is_some_and(|rows| !rows.is_empty()));

        let (status, object) = running
            .get_json("/api/objects/Flight/F-1001", &bearer)
            .await;
        assert_eq!(status, reqwest::StatusCode::OK, "{object}");
        assert_eq!(object["pk"], "F-1001");

        let denied_token = running.no_grant_token.clone();
        let (status, denied) = running
            .get_json("/api/objects/Flight/F-1001", &denied_token)
            .await;
        assert!(
            matches!(
                status,
                reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::NOT_FOUND
            ),
            "{status}: {denied}"
        );
        let old = running
            .other
            .request_snapshot()
            .await
            .expect("old PostgreSQL snapshot");
        running.engine.pause.store(true, Ordering::SeqCst);
        let client = running.client.clone();
        let url = running.url.clone();
        let token = bearer.clone();
        let pending = tokio::spawn(async move {
            client
                .get(format!("{url}/api/objects/Flight/F-1001"))
                .bearer_auth(token)
                .send()
                .await
                .expect("in-flight object")
        });
        tokio::time::timeout(Duration::from_secs(5), running.engine.entered.notified())
            .await
            .expect("request reached real Trino boundary");
        let mut next = old.active.ontology.clone();
        next.description = Some("pg-http-reloaded-definition".to_owned());
        tokio::time::timeout(
            Duration::from_secs(5),
            reload(&running.other, &next, &ReloadOptions::default()),
        )
        .await
        .expect("in-flight source request must release PG metadata lock")
        .expect("external reload");
        let mut revoked = old.scope.clone();
        revoked.bindings.clear();
        tokio::time::timeout(Duration::from_secs(5), running.apply_scope(&revoked))
            .await
            .expect("external revoke must not wait for in-flight source");
        running.engine.release.notify_one();
        let response = tokio::time::timeout(Duration::from_secs(30), pending)
            .await
            .expect("old admitted request finishes")
            .expect("request task");
        let request_id = response.headers()["x-request-id"]
            .to_str()
            .expect("request id")
            .to_owned();
        let status = response.status();
        let object: serde_json::Value = response.json().await.expect("old object JSON");
        assert_eq!(status, reqwest::StatusCode::OK, "{object}");
        assert_eq!(object["properties"]["flightId"], "F-1001");
        let (status, _) = running
            .get_json("/api/objects/Flight/F-1001", &bearer)
            .await;
        assert_eq!(
            status,
            reqwest::StatusCode::NOT_FOUND,
            "new request must observe external revoke"
        );
        let (_, metadata) = running.get_json("/api/ontology", &bearer).await;
        assert_eq!(metadata["description"], "pg-http-reloaded-definition");
        assert_eq!(metadata["objectTypes"], json!([]));
        assert_eq!(
            running.counts().await,
            [0; 5],
            "reads/reload/revoke must never write Action tables"
        );
        let side = running.side.clone();
        let schema_sys = running.schema_sys.clone();
        running.finish().await;
        let audit_sql = format!(
            "SELECT def_version, status FROM \"{schema_sys}\".of_read_audit WHERE request_id = $1"
        );
        let (version, outcome): (i64, String) = sqlx::query_as(sqlx::AssertSqlSafe(audit_sql))
            .bind(request_id)
            .fetch_one(&side)
            .await
            .expect("durable admitted read audit");
        assert_eq!(
            version, old.active.def_version,
            "audit must retain admitted definition"
        );
        assert_eq!(outcome, "success");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires real PostgreSQL and an authenticated ontoforge HTTP server"]
#[allow(
    clippy::too_many_lines,
    reason = "Keep the Action replay and atomicity scenario in execution order"
)]
async fn pg_http_action_validate_stale_replay_atomicity() {
    support::run_case(|running| async move {
    let bearer = running.token.clone();
    assert_eq!(running.counts().await, [0; 5]);
    let (_, initial) = running
        .get_json("/api/objects/Flight/F-1001", &bearer)
        .await;
    let initial_version = initial["_version"]
        .as_str()
        .expect("version token")
        .to_owned();
    let command_id = format!(
        "pg-http-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    );
    let validate = serde_json::json!({
        "validateOnly": true,
        "items": [{"params": {"flight": "F-1001", "delayMinutes": 7}}]
    });
    let (status, validated) = running
        .post_json("/api/actions/delayFlight", &bearer, &validate)
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{validated}");
    assert_eq!(validated["validated"], true, "dry-run must not commit");
    assert_eq!(
        running.counts().await,
        [0; 5],
        "validate-only must not write any Action table"
    );

    let request = serde_json::json!({
        "commandId": command_id,
        "items": [{"params": {"flight": "F-1001", "delayMinutes": 7}, "expectedVersions": {"flight": initial_version}}]
    });
    let (status, first) = running
        .post_json("/api/actions/delayFlight", &bearer, &request)
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{first}");
    assert_eq!(first["commandId"], command_id);
    let committed_counts = running.counts().await;
    assert_eq!(
        committed_counts, [1; 5],
        "one atomic Action across log, entries, history, claim and outbox"
    );

    let (status, replay) = running
        .post_json("/api/actions/delayFlight", &bearer, &request)
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{replay}");
    assert_eq!(
        replay, first,
        "explicit replay must return the original receipt"
    );

    let changed = serde_json::json!({
        "commandId": command_id,
        "items": [{"params": {"flight": "F-1001", "delayMinutes": 8}}]
    });
    let (status, _) = running
        .post_json("/api/actions/delayFlight", &bearer, &changed)
        .await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    let stale = json!({
        "commandId": format!("{command_id}-stale"),
        "items": [{"params": {"flight": "F-1001", "delayMinutes": 7}, "expectedVersions": {"flight": initial_version}}]
    });
    let (status, conflict) = running
        .post_json("/api/actions/delayFlight", &bearer, &stale)
        .await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT, "{conflict}");
    assert_eq!(conflict["code"], "VERSION_CONFLICT");
    let mut stale_validate = stale.clone();
    stale_validate["validateOnly"] = json!(true);
    let (status, conflict) = running
        .post_json("/api/actions/delayFlight", &bearer, &stale_validate)
        .await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT, "{conflict}");
    assert_eq!(
        running.counts().await,
        committed_counts,
        "replay/stale/conflict must have zero new writes"
    );

    let (status, result) = running
        .get_json(&format!("/api/commands/{command_id}"), &bearer)
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{result}");
    assert_eq!(result, first);

    let denied_token = running.no_grant_token.clone();
    let (status, denied) = running
        .post_json("/api/actions/delayFlight", &denied_token, &request)
        .await;
    assert!(
        matches!(
            status,
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::NOT_FOUND
        ),
        "unauthorized principal must not replay or write: {status} {denied}"
    );

    let (status, history) = running
        .get_json("/api/objects/Flight/F-1001/history", &bearer)
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{history}");
    assert!(
        history["entries"]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| entry["action"] == "delayFlight"))
    );
    assert_eq!(
        history["entries"]
            .as_array()
            .expect("history entries")
            .len(),
        1
    );
    let active = running
        .other
        .request_snapshot()
        .await
        .expect("admitted definition");
    let mut changed = active.active.ontology.clone();
    changed.description = Some("after-action-reload".to_owned());
    reload(&running.other, &changed, &ReloadOptions::default())
        .await
        .expect("reload after Action");
    let (status, replay) = running
        .post_json("/api/actions/delayFlight", &bearer, &request)
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{replay}");
    assert_eq!(
        replay, first,
        "definition change retains original receipt and time"
    );
    let mut revoked = active.scope;
    revoked.bindings.clear();
    running.apply_scope(&revoked).await;
    let (status, denied) = running
        .post_json("/api/actions/delayFlight", &bearer, &request)
        .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "revoked action must remain hidden: {denied}"
    );
    let (status, _) = running
        .get_json(&format!("/api/commands/{command_id}"), &bearer)
        .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "revoked subject must not retrieve receipt"
    );
    assert_eq!(running.counts().await, committed_counts);
    running.finish().await;
    }).await;
}
