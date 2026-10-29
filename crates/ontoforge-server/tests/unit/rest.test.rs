#[path = "../support/mod.rs"]
mod support;

use camino::Utf8Path;
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_schema::load_scope_dir;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::EditStore;
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::sync::atomic::Ordering;
use std::time::Duration;
use support::{fixture, keys, settings, success};

#[tokio::test]
async fn rest_metadata_and_all_l0_operations() {
    let f = fixture().await.start().await;
    let metadata = success(f.get("/api/ontology").await).await;
    assert_eq!(metadata["description"], "航空数据 UTF-8");
    assert_eq!(
        metadata["engineStatus"], "available",
        "metadata must expose a stable logical engine status"
    );
    f.engine.unavailable.store(true, Ordering::SeqCst);
    let degraded = success(f.get("/api/ontology").await).await;
    assert_eq!(degraded["engineStatus"], "unavailable");
    for secret in ["physical_table", "SELECT", "secret_token", "sentinel"] {
        assert!(!degraded.to_string().contains(secret));
    }
    f.engine.unavailable.store(false, Ordering::SeqCst);
    let names: Vec<_> = metadata["objectTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["apiName"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Airport", "Flight"]);
    assert!(!metadata.to_string().contains("aircraftTailNumber"));
    assert_eq!(metadata["links"].as_array().unwrap().len(), 2);
    assert_eq!(metadata["actions"].as_array().unwrap().len(), 2);
    let pk = success(f.get("/api/objects/Airport/BER").await).await;
    assert_eq!(pk["properties"]["name"], "Berlin");
    assert!(pk["_version"].as_str().is_some());
    let response = f.post("/api/query", &json!({"set":{"kind":"where","object_type":"Airport","predicate":{"op":"eq","property":"country","value":"DE"}},"select":["name","country"],"page":{"first":100}})).await;
    let query_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let filtered = success(response).await;
    assert_eq!(keys(&filtered), ["BER", "FRA"]);
    let search = f.query(json!({"kind":"search","object_type":"Airport","property":"name","query":"Berlin","mode":"all"}), json!(["name"])).await;
    assert_eq!(keys(&search), ["BER"]);
    let base = json!({"kind":"pk","object_type":"Airport","pk":"BER"});
    let around = f.query(json!({"kind":"searchAround","object_type":"Flight","base":base,"link":"flightDepartureAirport","direction":"fromTo"}), json!(["flightId","departureAirportCode"])).await;
    assert_eq!(keys(&around), ["F-1003", "F-2002", "F-2012"]);
    let one = json!({"kind":"pk","object_type":"Airport","pk":"BER"});
    let two = json!({"kind":"pk","object_type":"Airport","pk":"FRA"});
    for (set, expected) in [
        (
            json!({"kind":"union","object_type":"Airport","sets":[one,one,two]}),
            vec!["BER", "FRA"],
        ),
        (
            json!({"kind":"intersect","object_type":"Airport","sets":[{"kind":"type","object_type":"Airport"},one]}),
            vec!["BER"],
        ),
        (
            json!({"kind":"subtract","object_type":"Airport","left":{"kind":"type","object_type":"Airport"},"right":one}),
            vec!["FRA", "MUC"],
        ),
    ] {
        assert_eq!(keys(&f.query(set, json!(["name"])).await), expected);
    }
    let aggregate = success(f.post("/api/aggregate", &json!({"set":{"kind":"type","object_type":"Airport"},"groupBy":["country"],"measures":[{"op":"count"}]})).await).await;
    assert_eq!(
        aggregate["buckets"],
        json!([{"group":["DE"],"values":[2]},{"group":[null],"values":[1]}])
    );
    let rows = f.finish().await;
    assert_eq!(
        rows.len(),
        8,
        "each data operation must reserve exactly one audit record"
    );
    assert!(rows.iter().all(|r| r["status"] == "success"));
    assert!(
        rows.iter()
            .any(|r| r["request_id"] == query_id && r["tool"] == "query.list"),
        "HTTP query ID must match its durable audit reservation"
    );
}

#[tokio::test]
async fn rest_equal_sort_keys_empty_nullable_and_pagination() {
    let f = fixture().await.start().await;
    let mut cursor = Value::Null;
    let mut all = Vec::new();
    for expected_more in [true, true, false] {
        let page = success(
            f.post(
                "/api/query",
                &json!({
                    "set":{"kind":"type","object_type":"Airport"},"select":["country"],
                    "orderBy":[{"property":"country","direction":"asc","nulls":"last"}],
                    "page":{"first":1,"after":cursor}
                }),
            )
            .await,
        )
        .await;
        all.extend(keys(&page));
        assert_eq!(page["hasMore"], expected_more);
        assert_eq!(page["truncated"], false);
        if !expected_more {
            assert_eq!(page["rows"][0]["properties"]["country"], Value::Null);
        }
        cursor = page["cursor"].clone();
    }
    assert_eq!(all, ["BER", "FRA", "MUC"]);
    let empty = f.query(json!({"kind":"where","object_type":"Airport","predicate":{"op":"eq","property":"name","value":"absent"}}), json!(["name"])).await;
    assert_eq!(empty["rows"], json!([]));
    assert_eq!(empty["hasMore"], false);
    let single = f
        .query(
            json!({"kind":"pk","object_type":"Airport","pk":"MUC"}),
            json!(["name"]),
        )
        .await;
    assert_eq!(keys(&single), ["MUC"]);
    assert_eq!(single["hasMore"], false);
    assert_eq!(f.finish().await.len(), 5);

    let large = "x".repeat(9 * 1024 * 1024);
    let f = support::fixture_airports(&[
        ("A", &large, None),
        ("B", &large, None),
        ("C", &large, None),
    ])
    .await
    .start()
    .await;
    let first = success(f.post("/api/query", &json!({"set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":3}})).await).await;
    assert_eq!(keys(&first), ["A"]);
    assert_eq!(first["truncated"], true);
    assert_eq!(
        first["hasMore"], true,
        "a short bounded page is not the last page"
    );
    assert!(first["cursor"].as_str().is_some());
    let next = success(f.post("/api/query", &json!({"set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":3,"after":first["cursor"]}})).await).await;
    assert_eq!(keys(&next), ["B"]);
    assert_eq!(next["hasMore"], true);
    assert_eq!(f.finish().await.len(), 2);
}

#[tokio::test]
async fn rest_reload_and_revoke_freeze_inflight_only() {
    let f = fixture().await.start().await;
    let other = EditStore::open(settings(&f.path)).await.unwrap();
    let old = other.request_snapshot().await.unwrap();
    f.engine.block.store(true, Ordering::SeqCst);
    let client = f.client.clone();
    let url = f.url.clone();
    let token = f.token.clone();
    let pending = tokio::spawn(async move {
        client
            .get(format!("{url}/api/objects/Flight/F-1001"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(3), f.engine.entered.notified())
        .await
        .unwrap();
    let mut ontology = old.active.ontology.clone();
    ontology.description = Some("reloaded definition".to_owned());
    tokio::time::timeout(
        Duration::from_secs(3),
        reload(&other, &ontology, &ReloadOptions::default()),
    )
    .await
    .expect("pending source must not hold metadata lock")
    .unwrap();
    let mut scope = load_scope_dir(Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/scopes/aviation"
    )))
    .scope
    .unwrap();
    scope.bindings.clear();
    let config: AuthConfig = serde_json::from_value(
        json!({"principal_attrs":{"homeAirports":{"claim":"airports","type":"string_list"}}}),
    )
    .unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&config)),
        scope: &scope,
        sources: None,
    })
    .unwrap();
    let props = compiled.row_filter_properties();
    let plan = other.scope_plan(&scope, &props).await.unwrap();
    tokio::time::timeout(
        Duration::from_secs(3),
        other.scope_apply(&scope, &props, &plan.hash, "revoke"),
    )
    .await
    .expect("revoke must not wait for source")
    .unwrap();
    f.engine.block.store(false, Ordering::SeqCst);
    f.engine.release.notify_one();
    let response = pending.await.unwrap();
    let id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(success(response).await["properties"]["flightId"], "F-1001");
    assert_eq!(
        f.get("/api/objects/Flight/F-1001").await.status(),
        StatusCode::NOT_FOUND
    );
    let metadata = success(f.get("/api/ontology").await).await;
    assert_eq!(metadata["description"], "reloaded definition");
    assert_eq!(metadata["objectTypes"], json!([]));
    other.read_audit().shutdown().await.unwrap();
    let audit = f.finish().await;
    assert!(audit.iter().any(|r| r["request_id"] == id
        && r["def_version"] == old.active.def_version
        && r["status"] == "success"));
}
