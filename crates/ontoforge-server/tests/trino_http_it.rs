#[path = "support/real.rs"]
mod support;

#[tokio::test]
#[ignore = "requires real Trino and an authenticated ontoforge HTTP server"]
async fn trino_http_long_read_releases_snapshot_lock() {
    let trino = support::required("ONTOFORGE_TRINO_URL");
    let server = support::required("ONTOFORGE_SERVER_URL");
    assert!(trino.starts_with("http://") || trino.starts_with("https://"));
    let health = support::health(&server).await;
    assert!(health["engineStatus"] == "available", "{health}");
    let info = support::client()
        .get(format!("{trino}/v1/info"))
        .send()
        .await
        .expect("Trino info")
        .json::<serde_json::Value>()
        .await
        .expect("Trino info JSON");
    assert_eq!(info["starting"], false, "{info}");
    assert_eq!(info["nodeVersion"]["version"], "483", "{info}");
    let bearer = support::token();
    let (status, metadata) =
        support::json(support::get(&server, "/api/ontology", &bearer).await).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{metadata}");
}

#[tokio::test]
#[ignore = "requires real Trino L2 ACL services and an authenticated ontoforge HTTP server"]
async fn trino_http_and_mcp_six_l2_tools_all_formats() {
    let l2 = support::required("ONTOFORGE_TRINO_L2_URL");
    let loose = support::required("ONTOFORGE_TRINO_L2_LOOSE_URL");
    let server = support::required("ONTOFORGE_SERVER_URL");
    let health = support::health(&server).await;
    assert!(health["engineStatus"] == "available", "{health}");

    for url in [&l2, &loose] {
        let info = support::client()
            .get(format!("{url}/v1/info"))
            .send()
            .await
            .expect("L2 Trino info")
            .json::<serde_json::Value>()
            .await
            .expect("L2 Trino info JSON");
        assert_eq!(info["starting"], false, "{info}");
        assert_eq!(info["nodeVersion"]["version"], "483", "{info}");
    }

    let l2_allowed = support::client()
        .post(format!("{l2}/v1/statement"))
        .header("X-Trino-User", "l2")
        .body("SELECT 1 FROM demo.tiny.nation LIMIT 1")
        .send()
        .await
        .expect("L2 allow query")
        .json::<serde_json::Value>()
        .await
        .expect("L2 allow JSON");
    assert!(l2_allowed.get("error").is_none(), "{l2_allowed}");

    let l2_denied = support::client()
        .post(format!("{l2}/v1/statement"))
        .header("X-Trino-User", "l2")
        .body("SELECT 1 FROM outside.tiny.nation LIMIT 1")
        .send()
        .await
        .expect("L2 deny query")
        .json::<serde_json::Value>()
        .await
        .expect("L2 deny JSON");
    assert_eq!(
        l2_denied["error"]["errorName"], "PERMISSION_DENIED",
        "{l2_denied}"
    );

    let loose_allowed = support::client()
        .post(format!("{loose}/v1/statement"))
        .header("X-Trino-User", "l2")
        .body("SELECT 1 FROM outside.tiny.nation LIMIT 1")
        .send()
        .await
        .expect("loose ACL query")
        .json::<serde_json::Value>()
        .await
        .expect("loose ACL JSON");
    assert!(loose_allowed.get("error").is_none(), "{loose_allowed}");
}
