//! Roles, role bindings and the admin audit in the edit store (plan 03-16: SCOPE-01, SCOPE-07,
//! SCOPE-02 ordering; D-01..D-04).

mod support;

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use ontoforge_auth::scope::{ScopeInputs, compile_scope};
use ontoforge_schema::ir::{
    AttrType, ObjectTypeName, Ontology, ProfileName, PropertyName, Role, RoleName, ScopeSet,
};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::scope_store::{AppliedScope, normalize_role, normalize_scope};
use support::{TestStore, sqlite_store};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);
const ACTOR: &str = "tester@testhost";

type Props = BTreeMap<ObjectTypeName, BTreeSet<PropertyName>>;

struct Fixture {
    t: TestStore,
    ontology: Ontology,
    scope: ScopeSet,
    props: Props,
}

fn aviation() -> Ontology {
    let out = load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

fn aviation_scope() -> ScopeSet {
    let out = load_scope_dir(Utf8Path::new(AVIATION_SCOPE));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.scope.expect("aviation scope")
}

/// The properties the row predicates of `scope` read, as `scope apply` passes them on.
fn row_filter_properties(ontology: &Ontology, scope: &ScopeSet) -> Props {
    let attrs = BTreeMap::from([("homeAirports".to_owned(), AttrType::StringList)]);
    let profile = ProfileName::try_from("dev".to_owned()).expect("profile");
    compile_scope(&ScopeInputs {
        ontology,
        profile: Some(&profile),
        attrs: &attrs,
        scope,
        sources: None,
    })
    .unwrap_or_else(|d| panic!("{d:#?}"))
    .row_filter_properties()
}

/// An SQLite store with the aviation definition reloaded.
async fn fixture() -> Fixture {
    let t = sqlite_store().await;
    let ontology = aviation();
    reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload aviation");
    let scope = aviation_scope();
    let props = row_filter_properties(&ontology, &scope);
    Fixture {
        t,
        ontology,
        scope,
        props,
    }
}

/// Plans `scope` and applies exactly that plan.
async fn apply(f: &Fixture, scope: &ScopeSet) -> AppliedScope {
    let props = row_filter_properties(&f.ontology, scope);
    let plan = f.t.store.scope_plan(scope, &props).await.expect("plan");
    f.t.store
        .scope_apply(scope, &props, &plan.hash, ACTOR)
        .await
        .expect("apply")
}

async fn count(t: &TestStore, table: &str) -> i64 {
    let pool = t.side_pool().await;
    sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(&pool)
        .await
        .expect("count")
}

fn role(scope: &ScopeSet, name: &str) -> Role {
    let Some((_, role)) = scope.roles.iter().find(|(n, _)| n.as_str() == name) else {
        panic!("role {name}");
    };
    role.clone()
}

#[tokio::test]
async fn apply_writes_roles_bindings_and_audit() {
    let f = fixture().await;
    let before =
        f.t.store
            .scope_plan(&ScopeSet::default(), &BTreeMap::new())
            .await
            .expect("empty plan")
            .after_hash;
    let applied = apply(&f, &f.scope).await;
    assert_eq!(applied.plan.roles_added.len(), 3, "{:?}", applied.plan);
    assert_eq!(applied.plan.bindings_added.len(), 3, "{:?}", applied.plan);

    assert_eq!(count(&f.t, "of_role").await, 3);
    assert_eq!(count(&f.t, "of_role_binding").await, 3);
    assert_eq!(count(&f.t, "of_admin_audit").await, 1);

    let pool = f.t.side_pool().await;
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT api_name, ir_json FROM of_role ORDER BY api_name")
            .fetch_all(&pool)
            .await
            .expect("roles");
    assert_eq!(
        rows.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["crewScheduler", "dispatcher", "maintenanceLead"]
    );
    for (name, json) in &rows {
        let stored: Role = serde_json::from_str(json).expect("ir_json is a Role");
        assert_eq!(
            stored,
            normalize_role(&role(&f.scope, name)),
            "role {name} is stored normalized"
        );
    }

    let (seq, actor, action, before_hash, after_hash): (i64, String, String, String, String) =
        sqlx::query_as("SELECT seq, actor, action, before_hash, after_hash FROM of_admin_audit")
            .fetch_one(&pool)
            .await
            .expect("audit row");
    assert_eq!(seq, applied.audit_seq);
    assert_eq!(actor, ACTOR);
    assert_eq!(action, "scope.apply");
    assert_eq!(before_hash, before, "the hash of the empty scope");
    assert_eq!(after_hash, applied.plan.after_hash);
    assert_ne!(before_hash, after_hash);
}

#[tokio::test]
async fn apply_creates_row_filter_indexes() {
    let f = fixture().await;
    let pool = f.t.side_pool().await;
    let index = "of_ix_of_obj_flight_departure_airport_code";
    let exists = |pool: sqlx::SqlitePool| async move {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = ?",
        )
        .bind(index)
        .fetch_one(&pool)
        .await
        .expect("index lookup")
    };
    assert_eq!(exists(pool.clone()).await, 0, "not before apply");

    let applied = apply(&f, &f.scope).await;
    assert_eq!(exists(pool).await, 1, "crewScheduler filters on it");
    assert_eq!(
        applied.plan.index_ddl.len(),
        1,
        "{:?}",
        applied.plan.index_ddl
    );
    assert!(applied.plan.index_ddl[0].contains(index));

    // `status` is `indexed: true` already; nothing more to create, so the plan is empty.
    let again =
        f.t.store
            .scope_plan(&f.scope, &f.props)
            .await
            .expect("plan");
    assert!(again.index_ddl.is_empty(), "{again:?}");
}

#[tokio::test]
async fn load_scope_round_trips() {
    let f = fixture().await;
    assert_eq!(
        f.t.store.load_scope().await.expect("empty"),
        ScopeSet::default()
    );
    apply(&f, &f.scope).await;
    let loaded = f.t.store.load_scope().await.expect("load");
    let names: Vec<&str> = loaded.roles.keys().map(RoleName::as_str).collect();
    assert_eq!(names, ["crewScheduler", "dispatcher", "maintenanceLead"]);
    assert_eq!(loaded.bindings.len(), 3);
    assert_eq!(loaded, normalize_scope(&f.scope));
}
