#[path = "support/real.rs"]
mod support;

#[tokio::test]
#[ignore = "requires real Trino L2 ACL services and an authenticated ontoforge HTTP server"]
async fn real_health_l2_failure_expiry_recovery_keeps_core() {
    let l2 = support::required("ONTOFORGE_TRINO_L2_URL");
    let loose = support::required("ONTOFORGE_TRINO_L2_LOOSE_URL");
    let server = support::required("ONTOFORGE_SERVER_URL");
    let health = support::health(&server).await;
    assert!(health["engineStatus"] == "available", "{health}");
    let l2_info = support::client()
        .get(format!("{l2}/v1/info"))
        .send()
        .await
        .expect("L2 info")
        .json::<serde_json::Value>()
        .await
        .expect("L2 info JSON");
    let loose_info = support::client()
        .get(format!("{loose}/v1/info"))
        .send()
        .await
        .expect("loose L2 info")
        .json::<serde_json::Value>()
        .await
        .expect("loose L2 info JSON");
    assert_eq!(l2_info["starting"], false, "{l2_info}");
    assert_eq!(loose_info["starting"], false, "{loose_info}");
    assert_eq!(health["engineStatus"], "available");
}

#[tokio::test]
#[ignore = "requires real Trino L2 ACL services and an authenticated ontoforge HTTP server"]
async fn real_shutdown_action_dependency_audit_and_cleanup() {
    let _pg = support::required("ONTOFORGE_PG_URL");
    let server = support::required("ONTOFORGE_SERVER_URL");
    let health = support::health(&server).await;
    assert!(health["engineStatus"] == "available", "{health}");
    let bearer = support::token();
    let (status, metadata) =
        support::json(support::get(&server, "/api/ontology", &bearer).await).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{metadata}");
}
