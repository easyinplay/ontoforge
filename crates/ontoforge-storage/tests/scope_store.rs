//! Roles, role bindings and the admin audit in the edit store (plan 03-16: SCOPE-01, SCOPE-07,
//! SCOPE-02 ordering; D-01..D-04).

mod support;

use std::collections::{BTreeMap, BTreeSet};

use std::time::Duration;

use camino::Utf8Path;
use ontoforge_auth::scope::{ScopeInputs, compile_scope};
use ontoforge_auth::{Principal, PrincipalId};
use ontoforge_expr::Attrs;
use ontoforge_schema::ir::{
    AttrType, ObjectTypeName, Ontology, ProfileName, PropertyName, Role, RoleName, ScopeSet,
};
use ontoforge_schema::{load_dir, load_scope_dir, load_scope_from_memory};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::scope_store::{
    AppliedScope, BindingRow, LoginRecorder, ScopeApplyError, SubjectKind, normalize_role,
    normalize_scope,
};
use support::{PgTestStore, TestStore, pg_store, sqlite_store};

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

fn role_name(name: &str) -> RoleName {
    RoleName::try_from(name.to_owned()).expect("role name")
}

fn binding(role: &str, kind: SubjectKind, subject: &str) -> BindingRow {
    BindingRow {
        role: role.to_owned(),
        kind,
        subject: subject.to_owned(),
    }
}

#[tokio::test]
async fn full_sync_removes_bindings_and_roles() {
    let f = fixture().await;
    apply(&f, &f.scope).await;

    let mut reduced = f.scope.clone();
    reduced.roles.remove(&role_name("maintenanceLead"));
    reduced.bindings.remove(&role_name("maintenanceLead"));
    reduced.bindings.remove(&role_name("dispatcher"));

    let props = row_filter_properties(&f.ontology, &reduced);
    let plan = f.t.store.scope_plan(&reduced, &props).await.expect("plan");
    assert_eq!(plan.roles_removed, ["maintenanceLead"]);
    assert!(plan.roles_added.is_empty() && plan.roles_changed.is_empty());
    assert!(plan.bindings_added.is_empty(), "{plan:?}");
    assert_eq!(
        plan.bindings_removed,
        [
            binding("dispatcher", SubjectKind::Group, "dispatchers"),
            binding("maintenanceLead", SubjectKind::User, "ontoforge-dev|mia"),
        ]
    );

    f.t.store
        .scope_apply(&reduced, &props, &plan.hash, ACTOR)
        .await
        .expect("apply");
    assert_eq!(count(&f.t, "of_role").await, 2);
    assert_eq!(count(&f.t, "of_role_binding").await, 1);
    assert_eq!(count(&f.t, "of_admin_audit").await, 2);
    let loaded = f.t.store.load_scope().await.expect("load");
    assert_eq!(loaded, normalize_scope(&reduced));

    // An empty directory removes everything, and the plan says so.
    let plan =
        f.t.store
            .scope_plan(&ScopeSet::default(), &BTreeMap::new())
            .await
            .expect("plan");
    assert_eq!(plan.roles_removed, ["crewScheduler", "dispatcher"]);
    assert_eq!(
        plan.bindings_removed,
        [binding(
            "crewScheduler",
            SubjectKind::Group,
            "crew-schedulers"
        )]
    );
}

#[tokio::test]
async fn stale_plan_hash_rejected() {
    let f = fixture().await;
    let reviewed =
        f.t.store
            .scope_plan(&f.scope, &f.props)
            .await
            .expect("plan");

    // The wanted scope changes after the review.
    let mut other = f.scope.clone();
    other.roles.remove(&role_name("maintenanceLead"));
    other.bindings.remove(&role_name("maintenanceLead"));
    let other_props = row_filter_properties(&f.ontology, &other);
    let error =
        f.t.store
            .scope_apply(&other, &other_props, &reviewed.hash, ACTOR)
            .await
            .expect_err("another scope than the reviewed one");
    assert!(
        matches!(&error, ScopeApplyError::PlanChanged { expected, .. } if *expected == reviewed.hash),
        "{error:?}"
    );
    assert_eq!(count(&f.t, "of_role").await, 0);
    assert_eq!(count(&f.t, "of_admin_audit").await, 0);

    // The store changes after the review: the same scope now has an empty plan.
    f.t.store
        .scope_apply(&f.scope, &f.props, &reviewed.hash, ACTOR)
        .await
        .expect("the reviewed plan applies");
    let error =
        f.t.store
            .scope_apply(&f.scope, &f.props, &reviewed.hash, ACTOR)
            .await
            .expect_err("the plan is empty now");
    assert!(
        matches!(error, ScopeApplyError::PlanChanged { .. }),
        "{error:?}"
    );
    assert_eq!(count(&f.t, "of_admin_audit").await, 1);
}

const ROLES_A: &str = r#"kind: Role
apiName: alpha
objectTypes:
  Flight:
    properties: [flightId, status, delayMinutes]
  Airport:
    properties: [code, name]
actions: [assignPilot, delayFlight]
sqlLevels: [raw, admin]
l2:
  catalogs: [b, a]
  schemas: [b.s, a.s]
toolsets: [core, full]
---
kind: Role
apiName: beta
objectTypes:
  Pilot:
    properties: [employeeId, fullName]
actions: [assignPilot]
---
kind: RoleBinding
role: alpha
subjects:
  - group: g1
  - user: "i|u"
  - group: g0
"#;

/// The same scope as `ROLES_A` with every order changed and some list entries repeated.
const ROLES_B: &str = r#"kind: RoleBinding
role: alpha
subjects:
  - group: g0
  - user: "i|u"
  - group: g1
---
kind: Role
apiName: beta
objectTypes:
  Pilot:
    properties: [fullName, employeeId, fullName]
actions: [assignPilot]
---
kind: Role
apiName: alpha
toolsets: [full, core, full]
l2:
  schemas: [a.s, b.s]
  catalogs: [a, b]
sqlLevels: [admin, raw]
actions: [delayFlight, assignPilot]
objectTypes:
  Airport:
    properties: [name, code]
  Flight:
    properties: [delayMinutes, status, flightId]
"#;

fn scope_from_yaml(text: &str) -> ScopeSet {
    let out = load_scope_from_memory(&[("scope.yaml", text)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.scope.expect("scope")
}

#[tokio::test]
async fn order_independent_hash() {
    let f = fixture().await;
    let a = scope_from_yaml(ROLES_A);
    let b = scope_from_yaml(ROLES_B);
    assert_ne!(a, b, "the two documents are written differently");
    let none = BTreeMap::new();
    let plan_a = f.t.store.scope_plan(&a, &none).await.expect("plan a");
    let plan_b = f.t.store.scope_plan(&b, &none).await.expect("plan b");
    assert_eq!(plan_a, plan_b);
    assert_eq!(plan_a.after_hash, plan_b.after_hash);
    assert_eq!(normalize_scope(&a), normalize_scope(&b));

    // What apply stored for the one is no change for the other.
    f.t.store
        .scope_apply(&a, &none, &plan_a.hash, ACTOR)
        .await
        .expect("apply a");
    let again = f.t.store.scope_plan(&b, &none).await.expect("plan b again");
    assert!(again.is_empty(), "{again:?}");
    assert_eq!(again.before_hash, again.after_hash);
    assert_eq!(again.after_hash, plan_a.after_hash);
}

#[tokio::test]
async fn admin_audit_is_append_only() {
    let f = fixture().await;
    apply(&f, &f.scope).await;
    let pool = f.t.side_pool().await;
    for statement in [
        "UPDATE of_admin_audit SET actor = 'someone else'",
        "DELETE FROM of_admin_audit",
    ] {
        let error = sqlx::query(statement)
            .execute(&pool)
            .await
            .expect_err(statement);
        assert!(error.to_string().contains("append-only"), "{error}");
    }
    assert_eq!(count(&f.t, "of_admin_audit").await, 1);
}

/// The names of the sources whose text contains `needle`. The guard is strict on purpose: a table
/// name in a comment of another file is found as well.
fn files_naming(needle: &str, sources: &[(String, String)]) -> Vec<String> {
    sources
        .iter()
        .filter(|(_, text)| text.contains(needle))
        .map(|(name, _)| name.clone())
        .collect()
}

#[test]
fn scope_tables_written_only_by_scope_store() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    for entry in walkdir::WalkDir::new(&src)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.path().extension().is_some_and(|e| e == "rs") {
            let name = entry
                .path()
                .strip_prefix(&src)
                .expect("below src")
                .display()
                .to_string()
                .replace('\\', "/");
            let text = std::fs::read_to_string(entry.path()).expect("source file");
            sources.push((name, text));
        }
    }

    // The scan must be able to see what it looks for.
    let sample = [(
        "sample.rs".to_owned(),
        "let t = \"of_role_binding\";".to_owned(),
    )];
    assert_eq!(files_naming("of_role", &sample), ["sample.rs"]);

    for table in ["of_role", "of_role_binding", "of_admin_audit"] {
        assert_eq!(
            files_naming(table, &sources),
            ["scope_store.rs"],
            "{table} is named in scope_store.rs only (SCOPE-07, D-03)"
        );
    }
}

fn principal(issuer: &str, subject: &str) -> Principal {
    Principal {
        id: PrincipalId::new(issuer, subject),
        issuer: issuer.to_owned(),
        subject: subject.to_owned(),
        groups: BTreeSet::from(["admins".to_owned()]),
        attrs: Attrs::new(),
    }
}

async fn login_times(t: &TestStore, id: &str) -> (String, String) {
    let pool = t.side_pool().await;
    sqlx::query_as::<_, (String, String)>(
        "SELECT created_at, last_login_at FROM of_user WHERE principal_id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("user row")
}

#[tokio::test]
async fn record_login_creates_user_once() {
    let t = sqlite_store().await;
    let alice = principal("https://idp.example.com", "alice");

    t.store.record_login(&alice).await.expect("first login");
    let (created, first) = login_times(&t, "https://idp.example.com|alice").await;
    assert_eq!(created, first);
    tokio::time::sleep(Duration::from_millis(15)).await;
    t.store.record_login(&alice).await.expect("second login");

    assert_eq!(count(&t, "of_user").await, 1);
    let (created_again, last) = login_times(&t, "https://idp.example.com|alice").await;
    assert_eq!(
        created_again, created,
        "created_at is written on insert only"
    );
    assert!(last > first, "{last} after {first}");
    assert_eq!(
        t.columns("of_user").await,
        [
            "principal_id",
            "issuer",
            "subject",
            "created_at",
            "last_login_at"
        ],
        "no groups, claims or permissions are stored (D-04)"
    );
    let pool = t.side_pool().await;
    let row: (String, String, String) =
        sqlx::query_as("SELECT principal_id, issuer, subject FROM of_user")
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(
        row,
        (
            "https://idp.example.com|alice".to_owned(),
            "https://idp.example.com".to_owned(),
            "alice".to_owned()
        )
    );
}

#[tokio::test]
async fn login_recorder_writes_once_per_interval() {
    let t = sqlite_store().await;
    let alice = principal("https://idp.example.com", "alice");
    let bob = principal("https://idp.example.com", "bob");
    let recorder = LoginRecorder::new(Duration::from_secs(60));

    assert!(recorder.record(&t.store, &alice).await.expect("first"));
    let (_, written) = login_times(&t, "https://idp.example.com|alice").await;
    tokio::time::sleep(Duration::from_millis(15)).await;
    assert!(
        !recorder.record(&t.store, &alice).await.expect("second"),
        "within the interval nothing is written"
    );
    let (_, unchanged) = login_times(&t, "https://idp.example.com|alice").await;
    assert_eq!(unchanged, written);

    assert!(
        recorder
            .record(&t.store, &bob)
            .await
            .expect("other principal"),
        "the interval is kept per principal"
    );
    assert_eq!(count(&t, "of_user").await, 2);

    let eager = LoginRecorder::new(Duration::ZERO);
    assert!(eager.record(&t.store, &alice).await.expect("eager 1"));
    assert!(eager.record(&t.store, &alice).await.expect("eager 2"));
}

#[tokio::test]
async fn apply_without_active_definition_fails() {
    let t = sqlite_store().await;
    let scope = aviation_scope();
    let none = BTreeMap::new();
    let plan = t.store.scope_plan(&scope, &none).await.expect("plan works");
    assert_eq!(plan.roles_added.len(), 3);
    assert!(plan.index_ddl.is_empty(), "no tables, no indexes");

    let error = t
        .store
        .scope_apply(&scope, &none, &plan.hash, ACTOR)
        .await
        .expect_err("no definition");
    assert!(
        matches!(error, ScopeApplyError::NoActiveDefinition),
        "{error:?}"
    );
    assert_eq!(count(&t, "of_role").await, 0);
    assert_eq!(count(&t, "of_admin_audit").await, 0);
}

// ---------------------------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------------------------

async fn pg_count(t: &PgTestStore, table: &str) -> i64 {
    let sql = format!("SELECT count(*) FROM \"{}\".{table}", t.schema_sys);
    sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql))
        .fetch_one(t.side())
        .await
        .expect("count")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_scope_apply() {
    let t = pg_store().await;
    let ontology = aviation();
    reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload aviation");
    let scope = aviation_scope();
    let props = row_filter_properties(&ontology, &scope);

    let plan = t.store.scope_plan(&scope, &props).await.expect("plan");
    assert_eq!(plan.roles_added.len(), 3);
    assert_eq!(plan.bindings_added.len(), 3);
    assert_eq!(plan.index_ddl.len(), 1, "{:?}", plan.index_ddl);
    let applied = t
        .store
        .scope_apply(&scope, &props, &plan.hash, ACTOR)
        .await
        .expect("apply");
    assert_eq!(applied.plan, plan);
    assert_eq!(pg_count(&t, "of_role").await, 3);
    assert_eq!(pg_count(&t, "of_role_binding").await, 3);
    assert_eq!(pg_count(&t, "of_admin_audit").await, 1);

    // The row filter index is in the edit schema, and the second plan has nothing left to do.
    let indexes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_indexes WHERE CAST(schemaname AS text) = $1 \
         AND CAST(indexname AS text) = 'of_ix_of_obj_flight_departure_airport_code'",
    )
    .bind(&t.schema_edit)
    .fetch_one(t.side())
    .await
    .expect("index lookup");
    assert_eq!(indexes, 1);
    let again = t
        .store
        .scope_plan(&scope, &props)
        .await
        .expect("plan again");
    assert!(again.is_empty(), "{again:?}");
    assert_eq!(
        t.store.load_scope().await.expect("load"),
        normalize_scope(&scope)
    );

    // Full sync, and a stale hash.
    let mut reduced = scope.clone();
    reduced.roles.remove(&role_name("maintenanceLead"));
    reduced.bindings.remove(&role_name("maintenanceLead"));
    let reduced_props = row_filter_properties(&ontology, &reduced);
    let stale = t
        .store
        .scope_apply(&reduced, &reduced_props, &plan.hash, ACTOR)
        .await
        .expect_err("stale hash");
    assert!(
        matches!(stale, ScopeApplyError::PlanChanged { .. }),
        "{stale:?}"
    );
    let plan = t
        .store
        .scope_plan(&reduced, &reduced_props)
        .await
        .expect("plan");
    assert_eq!(plan.roles_removed, ["maintenanceLead"]);
    t.store
        .scope_apply(&reduced, &reduced_props, &plan.hash, ACTOR)
        .await
        .expect("apply reduced");
    assert_eq!(pg_count(&t, "of_role").await, 2);
    assert_eq!(pg_count(&t, "of_role_binding").await, 2);
    assert_eq!(pg_count(&t, "of_admin_audit").await, 2);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_audit_is_append_only_and_login_is_recorded() {
    let t = pg_store().await;
    let ontology = aviation();
    reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload aviation");
    let scope = aviation_scope();
    let props = row_filter_properties(&ontology, &scope);
    let plan = t.store.scope_plan(&scope, &props).await.expect("plan");
    t.store
        .scope_apply(&scope, &props, &plan.hash, ACTOR)
        .await
        .expect("apply");

    let audit = format!("\"{}\".of_admin_audit", t.schema_sys);
    for statement in [
        format!("UPDATE {audit} SET actor = 'someone else'"),
        format!("DELETE FROM {audit}"),
        format!("TRUNCATE {audit}"),
    ] {
        let error = sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
            .execute(t.side())
            .await
            .expect_err(&statement);
        assert!(error.to_string().contains("append-only"), "{error}");
    }
    assert_eq!(pg_count(&t, "of_admin_audit").await, 1);

    let alice = principal("https://idp.example.com", "alice");
    t.store.record_login(&alice).await.expect("first login");
    t.store.record_login(&alice).await.expect("second login");
    assert_eq!(pg_count(&t, "of_user").await, 1);
}
