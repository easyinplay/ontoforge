use std::collections::BTreeSet;

use camino::Utf8Path;
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::printer::l2::{L2Policy, validate_l2};
use ontoforge_engine::{Engine, EngineError, QueryContext};
use wiremock::MockServer;

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
    let engines = build_trino_engines(&cfg, Utf8Path::new("."), &env)
        .expect("identities build");
    let policy = L2Policy {
        raw_allowed: true,
        catalogs: BTreeSet::from(["demo".to_owned()]),
        schemas: BTreeSet::from(["demo.tiny".to_owned()]),
        edit_catalog: "editstore".to_owned(),
        max_rows: 10,
    };
    let sql = validate_l2("SELECT * FROM demo.tiny.nation", &policy)
        .expect("authorized syntax");
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
        server.received_requests().await.expect("requests").is_empty(),
        "a disabled L2 identity must never submit SQL"
    );
}

#[test]
fn l2_health_rejects_reused_ontology_identity() {
    let result = parse_server_config(&config("http://127.0.0.1:8080", "ontology"));
    assert!(result.is_err(), "ontology and L2 must have different identities");
}
