//! `reload` on the PostgreSQL edit store: schema split, idempotency and additive change
//! (STORE-01, STORE-02).
//!
//! Every test is `#[ignore]`d: they need a PostgreSQL server. Run them with
//! `ONTOFORGE_PG_URL=postgres://... cargo test -p ontoforge-storage --test reload_pg -- --ignored`
//! or, on a machine that can download and run PostgreSQL, with
//! `cargo test -p ontoforge-storage --features embedded-pg --test reload_pg -- --ignored`.
//! Without either, `support::pg_store()` panics.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use ontoforge_schema::ir::{ObjectTypeName, Ontology, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, EditError, ObjectOp};
use ontoforge_storage::error::StoreError;
use ontoforge_storage::migrate::{MigrateError, apply_migration, plan_migration};
use ontoforge_storage::reload::{ReloadError, ReloadOptions, reload};
use ontoforge_storage::scope_store::ScopeApplyError;
use ontoforge_storage::value::PropertyValue;
use support::{ontology_from, pg_store};

const GATE: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string }
  cost: { type: decimal, precision: 12, scale: 2 }
  openedAt: { type: timestamp }
  tags: { type: array, items: string }
  location: { type: geopoint }
---
kind: ObjectType
apiName: Stand
backing: editOnly
primaryKey: id
properties:
  id: { type: integer, required: true }
---
kind: LinkType
apiName: gateStands
cardinality: manyToMany
from: { objectType: Gate, apiName: stands }
to: { objectType: Stand, apiName: gates }
backing: editOnly
";

fn base() -> Ontology {
    ontology_from(&[("defs.yaml", GATE)])
}

fn with_zone() -> Ontology {
    let text = GATE.replacen(
        "  location: { type: geopoint }\n",
        "  location: { type: geopoint }\n  zone: { type: string }\n",
        1,
    );
    ontology_from(&[("defs.yaml", &text)])
}

fn names(columns: &[String]) -> BTreeSet<&str> {
    columns.iter().map(String::as_str).collect()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn first_reload_on_pg_puts_edit_tables_in_edit_schema() {
    let t = pg_store().await;
    let out = reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("reload");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(1));
    assert!(out.ddl.iter().all(|sql| sql.contains(&t.schema_edit)));

    let edit = t.tables(&t.schema_edit).await;
    assert_eq!(
        names(&edit),
        BTreeSet::from(["of_obj_gate", "of_obj_stand", "of_lnk_gate_stands"]),
        "edit tables only"
    );
    let sys = t.tables(&t.schema_sys).await;
    assert_eq!(
        names(&sys),
        BTreeSet::from([
            "_sqlx_migrations",
            "of_def_state",
            "of_def_version",
            "of_admin_audit",
            "of_edit_history",
            "of_edit_log",
            "of_edit_log_entry",
            "of_role",
            "of_role_binding",
            "of_user"
        ]),
        "system tables only"
    );
    assert_eq!(t.active_version().await, 1);
    assert_eq!(t.version_rows().await, 1);

    // ADR-0024 point 7: column types and collation.
    let gate = "of_obj_gate";
    assert_eq!(t.data_type(&t.schema_edit, gate, "code").await, "text");
    assert_eq!(
        t.collation(&t.schema_edit, gate, "code").await.as_deref(),
        Some("C")
    );
    assert_eq!(t.data_type(&t.schema_edit, gate, "cost").await, "numeric");
    assert_eq!(
        t.data_type(&t.schema_edit, gate, "opened_at").await,
        "timestamp with time zone"
    );
    assert_eq!(t.data_type(&t.schema_edit, gate, "tags").await, "ARRAY");
    assert_eq!(t.data_type(&t.schema_edit, gate, "location").await, "jsonb");
    assert_eq!(
        t.data_type(&t.schema_edit, gate, "name__set").await,
        "boolean"
    );
    assert_eq!(
        t.data_type(&t.schema_edit, gate, "of_version").await,
        "bigint"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn second_reload_on_pg_is_noop() {
    let t = pg_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let again = reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("second");
    assert!(!again.changed);
    assert!(again.ddl.is_empty());
    assert_eq!(again.def_version, Some(1));
    assert_eq!(t.version_rows().await, 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn additive_reload_on_pg_adds_columns() {
    let t = pg_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let out = reload(&t.store, &with_zone(), &ReloadOptions::default())
        .await
        .expect("additive");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(2));
    assert_eq!(out.ddl.len(), 2, "{:#?}", out.ddl);
    assert!(
        out.ddl
            .iter()
            .all(|sql| sql.contains("ADD COLUMN IF NOT EXISTS"))
    );
    let gate = t.columns(&t.schema_edit, "of_obj_gate").await;
    assert!(gate.contains(&"zone".to_owned()) && gate.contains(&"zone__set".to_owned()));
    assert_eq!(t.active_version().await, 2);
    assert_eq!(t.version_rows().await, 2);
}

fn aviation() -> Ontology {
    let root = camino::Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    ontoforge_schema::load_dir(root)
        .ontology
        .expect("aviation loads")
}

/// `(indexname, indexdef)` of every index on `table` in `schema`.
async fn index_defs(t: &support::PgTestStore, table: &str) -> Vec<(String, String)> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT CAST(indexname AS text), CAST(indexdef AS text) FROM pg_indexes \
         WHERE CAST(schemaname AS text) = $1 AND CAST(tablename AS text) = $2 ORDER BY 1",
    )
    .bind(&t.schema_edit)
    .bind(table)
    .fetch_all(t.side())
    .await
    .expect("pg_indexes")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn indexes_on_pg() {
    let t = pg_store().await;
    let ontology = aviation();
    reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload");

    let flight = index_defs(&t, "of_obj_flight").await;
    assert!(
        flight
            .iter()
            .any(|(name, _)| name == "of_ix_of_obj_flight_status"),
        "{flight:?}"
    );
    let unique = "of_ux_of_obj_maintenance_record_aircraft_tail_number";
    let maintenance = index_defs(&t, "of_obj_maintenance_record").await;
    let (_, definition) = maintenance
        .iter()
        .find(|(name, _)| name == unique)
        .unwrap_or_else(|| panic!("{maintenance:?}"));
    assert!(
        definition.contains("UNIQUE") && definition.contains("WHERE"),
        "{definition}"
    );

    // The database refuses a second live row with the same key, and allows a deleted one.
    let table = format!("\"{}\".of_obj_maintenance_record", t.schema_edit);
    let pool = t.side();
    let insert = |id: &'static str, set: bool, deleted: bool| {
        let sql = format!(
            "INSERT INTO {table} (record_id, aircraft_tail_number, aircraft_tail_number__set, of_deleted)              VALUES ($1, 'N1', $2, $3)"
        );
        async move {
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(id)
                .bind(set)
                .bind(deleted)
                .execute(pool)
                .await
        }
    };
    insert("r1", true, false).await.expect("first row");
    let second = insert("r2", true, false).await;
    let Err(sqlx::Error::Database(error)) = second else {
        panic!("a second live row must be refused, got {second:?}");
    };
    assert_eq!(error.code().as_deref(), Some("23505"));
    insert("r3", true, true).await.expect("deleted row");
    insert("r4", false, false).await.expect("unset row");

    // Relaxing the link drops the unique index only.
    let mut relaxed = ontology.clone();
    relaxed
        .link_types
        .values_mut()
        .find(|link| link.cardinality == ontoforge_schema::ir::Cardinality::OneToOne)
        .expect("a oneToOne link")
        .cardinality = ontoforge_schema::ir::Cardinality::OneToMany;
    let out = reload(&t.store, &relaxed, &ReloadOptions::default())
        .await
        .expect("relax");
    assert_eq!(out.ddl.len(), 1, "{:#?}", out.ddl);
    assert!(
        out.ddl[0].contains("DROP INDEX IF EXISTS")
            && out.ddl[0].contains(&format!("\"{}\".\"{unique}\"", t.schema_edit)),
        "{}",
        out.ddl[0]
    );
    let after = index_defs(&t, "of_obj_maintenance_record").await;
    assert!(after.iter().all(|(name, _)| name != unique), "{after:?}");
    assert_eq!(index_defs(&t, "of_obj_flight").await, flight);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn long_names_on_pg() {
    let t = pg_store().await;
    let property = format!("p{}", "x".repeat(99));
    let type_name = format!("T{}", "x".repeat(99));
    let text = format!(
        "kind: ObjectType\napiName: {type_name}\nbacking: editOnly\nprimaryKey: id\nproperties:\n  id: {{ type: integer, required: true }}\n  {property}: {{ type: string, indexed: true }}\n"
    );
    let ontology = ontology_from(&[("defs.yaml", &text)]);
    reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("long names reload on PostgreSQL");

    let active = t.store.active_definition().await.expect("active");
    let table = active.physical.objects.values().next().expect("table");
    assert!(table.table.len() <= 63, "{}", table.table);
    let column = table.properties.values().next().expect("column");
    let columns = t.columns(&t.schema_edit, &table.table).await;
    assert!(
        columns.contains(&column.column) && columns.contains(&column.set_column),
        "{columns:?}"
    );
    let index = table.indexes.first().expect("index");
    assert!(index.name.len() <= 63, "{}", index.name);
    let defs = index_defs(&t, &table.table).await;
    assert!(
        defs.iter().any(|(name, _)| *name == index.name),
        "PostgreSQL kept the index under its bounded name: {defs:?}"
    );

    let again = reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("again");
    assert!(!again.changed);
}

// ---------------------------------------------------------------------------------------------
// The definition version fence (ADR-0016 / D20, plan 03-18 task 1)
// ---------------------------------------------------------------------------------------------

fn gate_commit(def_version: i64, code: &str, name: &str) -> Commit {
    let values: BTreeMap<PropertyName, PropertyValue> = BTreeMap::from([(
        PropertyName::try_from("name".to_owned()).expect("property name"),
        PropertyValue::String(name.to_owned()),
    )]);
    Commit {
        actor: Actor::Principal("test|alice".to_owned()),
        action: "setGate".to_owned(),
        def_version,
        params: serde_json::json!({ "code": code }),
        edits: vec![Edit::Object {
            type_name: ObjectTypeName::try_from("Gate".to_owned()).expect("type name"),
            pk: PropertyValue::String(code.to_owned()),
            op: ObjectOp::Set(values),
        }],
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn in_flight_commit_blocks_switch_on_pg() {
    let t = pg_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    // Another process that opened the store while definition 1 was active.
    let writer = t.open_another(Duration::from_secs(5)).await;

    // What a commit holds between its fence read and its commit: a share lock on the state row.
    let mut in_flight = t.side().begin().await.expect("begin");
    let lock = format!(
        "SELECT active_def_version FROM \"{}\".of_def_state FOR SHARE",
        t.schema_sys
    );
    let held: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(lock))
        .fetch_one(&mut *in_flight)
        .await
        .expect("share lock");
    assert_eq!(held, 1);

    let store = std::sync::Arc::clone(&t.store);
    let switching =
        tokio::spawn(async move { reload(&store, &with_zone(), &ReloadOptions::default()).await });
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert!(
        !switching.is_finished(),
        "the switch must wait for the commit in flight"
    );
    assert_eq!(t.active_version().await, 1);

    in_flight.commit().await.expect("the commit in flight ends");
    let out = switching
        .await
        .expect("join")
        .expect("the switch goes through once the commit ended");
    assert_eq!(out.def_version, Some(2));

    let err = writer
        .commit(gate_commit(1, "A1", "North"))
        .await
        .expect_err("a write for the old definition is refused after the switch");
    assert!(
        matches!(
            err,
            EditError::DefinitionStale {
                expected: 1,
                active: 2
            }
        ),
        "{err:?}"
    );
    writer
        .commit(gate_commit(2, "A1", "North"))
        .await
        .expect("a write for the new definition goes through");
}

// ---------------------------------------------------------------------------------------------
// Lock timeout, single flight and writers racing a switch (plan 03-18 task 2)
// ---------------------------------------------------------------------------------------------

/// The advisory lock key `reload`, `migrate` and `scope apply` share (`sql_exit::DEFINITION_LOCK_KEY`;
/// the bytes spell `ontoforg`). A test holds it to stand in for "another definition change is
/// running".
const DEFINITION_LOCK_KEY: i64 = 0x6f6e_746f_666f_7267;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn lock_timeout_keeps_old_definition() {
    let t = pg_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let impatient = t.open_another(Duration::from_millis(200)).await;

    // Another connection holds the table: adding a column has to wait for it.
    let mut blocker = t.side().begin().await.expect("begin");
    let lock = format!(
        "LOCK TABLE \"{}\".of_obj_gate IN ACCESS EXCLUSIVE MODE",
        t.schema_edit
    );
    sqlx::query(sqlx::AssertSqlSafe(lock))
        .execute(&mut *blocker)
        .await
        .expect("lock the table");

    let started = std::time::Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        reload(&impatient, &with_zone(), &ReloadOptions::default()),
    )
    .await
    .expect("reload gives up within lock_timeout");
    assert!(
        matches!(result, Err(ReloadError::LockTimeout)),
        "{result:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "gave up after lock_timeout, not after {:?}",
        started.elapsed()
    );
    assert_eq!(t.active_version().await, 1, "the old definition stays");
    assert_eq!(t.version_rows().await, 1);
    assert!(
        !t.columns(&t.schema_edit, "of_obj_gate")
            .await
            .contains(&"zone".to_owned()),
        "the DDL was rolled back"
    );
    assert_eq!(
        impatient.active_definition().await.map(|a| a.def_version),
        Some(1)
    );

    // migrate takes the same path: the same lock, the same timeout.
    let active = impatient.active_definition().await.expect("active");
    let plan = plan_migration(&active, &with_zone(), &[]).expect("plan");
    let migrated = tokio::time::timeout(
        Duration::from_secs(5),
        apply_migration(&impatient, &with_zone(), &[], &plan.hash),
    )
    .await
    .expect("migrate gives up within lock_timeout");
    assert!(
        matches!(migrated, Err(MigrateError::LockTimeout)),
        "{migrated:?}"
    );
    assert_eq!(t.active_version().await, 1);

    // Once the table is free the same reload goes through.
    blocker.rollback().await.expect("release");
    let out = reload(&impatient, &with_zone(), &ReloadOptions::default())
        .await
        .expect("reload after the lock was released");
    assert_eq!(out.def_version, Some(2));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn definition_lock_is_shared_and_bounded_by_lock_timeout() {
    let t = pg_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let impatient = t.open_another(Duration::from_millis(200)).await;
    let active = impatient.active_definition().await.expect("active");
    let plan = plan_migration(&active, &with_zone(), &[]).expect("migration plan");
    let scope = ontoforge_schema::ir::ScopeSet::default();
    let props = BTreeMap::new();
    let scope_plan = impatient
        .scope_plan(&scope, &props)
        .await
        .expect("scope plan");

    // Something else is changing the definition.
    let mut holder = t.side().begin().await.expect("begin");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(DEFINITION_LOCK_KEY)
        .execute(&mut *holder)
        .await
        .expect("hold the definition lock");

    let within = Duration::from_secs(5);
    let reloaded = tokio::time::timeout(
        within,
        reload(&impatient, &with_zone(), &ReloadOptions::default()),
    )
    .await
    .expect("reload gives up within lock_timeout");
    assert!(
        matches!(reloaded, Err(ReloadError::LockTimeout)),
        "{reloaded:?}"
    );
    let migrated = tokio::time::timeout(
        within,
        apply_migration(&impatient, &with_zone(), &[], &plan.hash),
    )
    .await
    .expect("migrate gives up within lock_timeout");
    assert!(
        matches!(migrated, Err(MigrateError::LockTimeout)),
        "{migrated:?}"
    );
    let applied = tokio::time::timeout(
        within,
        impatient.scope_apply(&scope, &props, &scope_plan.hash, "tester@testhost"),
    )
    .await
    .expect("scope apply gives up within lock_timeout");
    assert!(
        matches!(
            &applied,
            Err(ScopeApplyError::Store(StoreError::Query { sqlstate: Some(code), .. })) if code == "55P03"
        ),
        "{applied:?}"
    );
    assert_eq!(t.active_version().await, 1);

    holder.rollback().await.expect("release");
    reload(&impatient, &with_zone(), &ReloadOptions::default())
        .await
        .expect("reload once the lock is free");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn concurrent_reloads_single_flight_on_pg() {
    for round in 0..3 {
        let t = pg_store().await;
        reload(&t.store, &base(), &ReloadOptions::default())
            .await
            .expect("v1");
        let other = std::sync::Arc::new(t.open_another(Duration::from_secs(5)).await);
        let a = std::sync::Arc::clone(&t.store);
        let b = std::sync::Arc::clone(&other);
        let (first, second) = tokio::join!(
            tokio::spawn(async move { reload(&a, &with_zone(), &ReloadOptions::default()).await }),
            tokio::spawn(async move { reload(&b, &with_zone(), &ReloadOptions::default()).await }),
        );
        let first = first.expect("join").expect("first reload");
        let second = second.expect("join").expect("second reload");
        assert_eq!(
            [first.changed, second.changed]
                .iter()
                .filter(|changed| **changed)
                .count(),
            1,
            "round {round}: exactly one reload changes the definition: {first:?} {second:?}"
        );
        assert_eq!(first.def_version, Some(2), "round {round}");
        assert_eq!(second.def_version, Some(2), "round {round}");
        assert_eq!(t.version_rows().await, 2, "round {round}: one new version");
        assert_eq!(t.active_version().await, 2, "round {round}");
        for store in [&t.store, &other] {
            assert_eq!(
                store.active_definition().await.map(|a| a.def_version),
                Some(2),
                "round {round}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn reload_racing_commits_never_lets_an_old_write_through_on_pg() {
    for round in 0..3 {
        let t = pg_store().await;
        reload(&t.store, &base(), &ReloadOptions::default())
            .await
            .expect("v1");
        let writer = std::sync::Arc::new(t.open_another(Duration::from_secs(5)).await);
        let switched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let task = tokio::spawn(support::write_gates_across_a_switch(
            std::sync::Arc::clone(&writer),
            std::sync::Arc::clone(&switched),
            "G",
        ));
        tokio::time::sleep(Duration::from_millis(300)).await;
        let out = reload(&t.store, &with_zone(), &ReloadOptions::default())
            .await
            .expect("v2");
        assert_eq!(out.def_version, Some(2), "round {round}");
        switched.store(true, std::sync::atomic::Ordering::SeqCst);
        let stats = task.await.expect("the writer finishes");

        assert!(stats.committed >= 10, "round {round}: {stats:?}");
        assert!(stats.stale >= 1, "round {round}: {stats:?}");
        let versions = support::assert_log_versions_never_go_back(&t.store).await;
        assert_eq!(versions.len(), stats.committed, "round {round}");
        assert!(
            versions.contains(&1) && versions.contains(&2),
            "round {round}: {versions:?}"
        );
    }
}
