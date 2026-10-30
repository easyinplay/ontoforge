#[path = "support/pg_real.rs"]
mod support;

use std::sync::atomic::Ordering;
use std::time::Duration;

use ontoforge_storage::reload::{ReloadOptions, reload};
use serde_json::json;

#[tokio::test]
#[ignore = "requires real PostgreSQL and an authenticated ontoforge HTTP server"]
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
