#[path = "support/real.rs"]
mod support;

#[tokio::test]
#[ignore = "requires real PostgreSQL and an authenticated ontoforge HTTP server"]
async fn pg_http_mixed_snapshot_reload_revoke_and_history() {
    let pg = support::required("ONTOFORGE_PG_URL");
    let server = support::required("ONTOFORGE_SERVER_URL");
    assert!(pg.starts_with("postgres://") || pg.starts_with("postgresql://"));
    let bearer = support::token();
    let (status, metadata) =
        support::json(support::get(&server, "/api/ontology", &bearer).await).await;
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
    let (status, page) =
        support::json(support::post_json(&server, "/api/query", &bearer, &query).await).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    assert!(page["rows"].as_array().is_some_and(|rows| !rows.is_empty()));

    let (status, object) =
        support::json(support::get(&server, "/api/objects/Flight/F-1001", &bearer).await).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{object}");
    assert_eq!(object["pk"], "F-1001");

    let (status, denied) = support::json(
        support::get(
            &server,
            "/api/objects/Flight/F-1001",
            &support::no_grant_token(),
        )
        .await,
    )
    .await;
    assert!(
        matches!(
            status,
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::NOT_FOUND
        ),
        "{status}: {denied}"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL and an authenticated ontoforge HTTP server"]
async fn pg_http_action_validate_stale_replay_atomicity() {
    let _pg = support::required("ONTOFORGE_PG_URL");
    let server = support::required("ONTOFORGE_SERVER_URL");
    let bearer = support::token();
    let command_id = format!("pg-http-{}", std::process::id());
    let request = serde_json::json!({
        "commandId": command_id,
        "items": [{"params": {"flight": "F-1001", "delayMinutes": 7}}]
    });
    let (status, first) = support::json(
        support::post_json(&server, "/api/actions/delayFlight", &bearer, &request).await,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{first}");
    assert_eq!(first["commandId"], command_id);

    let (status, replay) = support::json(
        support::post_json(&server, "/api/actions/delayFlight", &bearer, &request).await,
    )
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
    let (status, _) = support::json(
        support::post_json(&server, "/api/actions/delayFlight", &bearer, &changed).await,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);

    let (status, result) =
        support::json(support::get(&server, &format!("/api/commands/{command_id}"), &bearer).await)
            .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{result}");
    assert_eq!(result, first);

    let (status, history) =
        support::json(support::get(&server, "/api/objects/Flight/F-1001/history", &bearer).await)
            .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{history}");
    assert!(
        history["entries"]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| entry["action"] == "delayFlight"))
    );
}
