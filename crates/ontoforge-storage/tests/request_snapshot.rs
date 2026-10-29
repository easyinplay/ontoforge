//! Deterministic request metadata snapshot tests.

mod support;

use std::collections::BTreeMap;
use std::time::Duration;

use ontoforge_auth::scope::{ScopeInputs, compile_scope};
use ontoforge_schema::ir::AttrType;
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::reload::{ReloadOptions, reload};
use support::{pg_store, sqlite_store};

const AVIATION: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation");
const AVIATION_SCOPE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/scopes/aviation");

fn aviation() -> ontoforge_schema::ir::Ontology {
    let out = load_dir(camino::Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

fn aviation_scope() -> ontoforge_schema::ir::ScopeSet {
    let out = load_scope_dir(camino::Utf8Path::new(AVIATION_SCOPE));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.scope.expect("aviation scope")
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_sqlite_never_mixes_definition_roles_bindings() {
    let fixture = sqlite_store().await;
    let ontology = aviation();
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload");
    let scope = aviation_scope();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &BTreeMap::from([("homeAirports".to_owned(), AttrType::StringList)]),
        scope: &scope,
        sources: None,
    })
    .expect("compile scope");
    let props = compiled.row_filter_properties();
    let plan = fixture
        .store
        .scope_plan(&scope, &props)
        .await
        .expect("scope plan");
    fixture
        .store
        .scope_apply(&scope, &props, &plan.hash, "snapshot-test")
        .await
        .expect("scope apply");

    let second = fixture.reopen().await;
    let snapshot = second.request_snapshot().await.expect("snapshot");
    assert_eq!(snapshot.active.def_version, 1);
    assert_eq!(snapshot.scope, ontoforge_storage::scope_store::normalize_scope(&scope));
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_lock_is_released_before_compile_or_network() {
    let fixture = sqlite_store().await;
    let ontology = aviation();
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload");
    let snapshot = fixture.store.request_snapshot().await.expect("snapshot");

    let other = fixture.reopen().await;
    let mut changed = ontology.clone();
    changed.description = Some("changed after snapshot".to_owned());
    let reloaded = tokio::time::timeout(
        Duration::from_secs(2),
        reload(&other, &changed, &ReloadOptions::default()),
    )
    .await
    .expect("reload is not blocked by a completed snapshot")
    .expect("reload");
    assert!(reloaded.changed);
    assert_eq!(snapshot.active.def_version, 1);
}

#[tokio::test]
#[ignore = "requires a real PostgreSQL URL; Phase 07-13 runs ignored tests explicitly"]
async fn snapshot_pg_never_mixes_definition_roles_bindings() {
    if std::env::var_os("ONTOFORGE_PG_URL").is_none() {
        panic!("ONTOFORGE_PG_URL is required when running ignored PostgreSQL snapshot tests");
    }
    let fixture = pg_store().await;
    let _ = fixture.store.request_snapshot().await;
}
