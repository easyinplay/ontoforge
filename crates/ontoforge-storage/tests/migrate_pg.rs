//! `migrate` on the PostgreSQL edit store: archive, rename and clean-up statements run against a
//! real server (STORE-02, D-14, D-16). The SQLite cases in `migrate.rs` cover the planning; these
//! check that the statements are valid PostgreSQL and qualify the edit schema correctly.
//!
//! Every test is `#[ignore]`d: they need a PostgreSQL server. Run them with
//! `ONTOFORGE_PG_URL=postgres://... cargo test -p ontoforge-storage --test migrate_pg -- --ignored`
//! or, on a machine that can download and run PostgreSQL, with
//! `cargo test -p ontoforge-storage --features embedded-pg --test migrate_pg -- --ignored`.

mod support;

use std::collections::BTreeMap;

use ontoforge_schema::ir::{AttrType, ObjectTypeName, Ontology, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::migrate::{
    MigrateError, RenameDecl, apply_drop_archived, apply_migration, apply_migration_probed,
    plan_drop_archived, plan_migration,
};
use ontoforge_storage::reload::{ReloadOptions, RolesCheck, reload};
use ontoforge_storage::value::PropertyValue;
use support::{PgTestStore, ontology_from, pg_store};

fn object_yaml(name: &str, primary_key: &str, properties: &str) -> String {
    format!(
        "kind: ObjectType\napiName: {name}\nbacking: editOnly\nprimaryKey: {primary_key}\nproperties:\n{properties}---\n"
    )
}

fn defs(text: &str) -> Ontology {
    ontology_from(&[("defs.yaml", text)])
}

fn gate(properties: &str) -> Ontology {
    defs(&object_yaml(
        "Gate",
        "code",
        &format!("  code: {{ type: string, required: true }}\n{properties}"),
    ))
}

/// A store with `old` loaded and gate `G1` holding the given values.
async fn store_with_gate(old: &Ontology, values: &[(&str, PropertyValue)]) -> PgTestStore {
    let t = pg_store().await;
    reload(&t.store, old, &ReloadOptions::default())
        .await
        .expect("first reload");
    let values: BTreeMap<PropertyName, PropertyValue> = values
        .iter()
        .map(|(name, value)| {
            (
                PropertyName::try_from((*name).to_owned()).expect("property name"),
                value.clone(),
            )
        })
        .collect();
    t.store
        .commit(Commit {
            actor: Actor::Principal("https://issuer.example|alice".to_owned()),
            action: "setGate".to_owned(),
            def_version: 1,
            params: serde_json::json!({ "code": "G1" }),
            edits: vec![Edit::Object {
                type_name: ObjectTypeName::try_from("Gate".to_owned()).expect("type name"),
                pk: PropertyValue::String("G1".to_owned()),
                op: ObjectOp::Set(values),
            }],
        })
        .await
        .expect("commit");
    t
}

/// One integer of the row `G1` of `table`. A boolean flag reads as 0 or 1.
async fn int_of(t: &PgTestStore, table: &str, column: &str) -> Option<i32> {
    let sql = format!(
        "SELECT CAST(\"{column}\" AS integer) FROM \"{}\".\"{table}\" WHERE CAST(\"code\" AS text) = 'G1'",
        t.schema_edit
    );
    sqlx::query_scalar::<_, Option<i32>>(sqlx::AssertSqlSafe(sql))
        .fetch_one(t.side())
        .await
        .expect("read integer")
}

/// Names of the indexes of `table` in the edit schema.
async fn indexes(t: &PgTestStore, table: &str) -> Vec<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT CAST(indexname AS text) FROM pg_indexes \
         WHERE CAST(schemaname AS text) = $1 AND CAST(tablename AS text) = $2 ORDER BY 1",
    )
    .bind(&t.schema_edit)
    .bind(table)
    .fetch_all(t.side())
    .await
    .expect("list indexes")
}

async fn assert_reload_is_noop(t: &PgTestStore, ontology: &Ontology) {
    let again = reload(&t.store, ontology, &ReloadOptions::default())
        .await
        .expect("reload after migrate");
    assert!(!again.changed, "reload must see no difference: {again:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn archive_removed_property_on_pg() {
    let old = gate("  capacity: { type: integer, indexed: true }\n");
    let t = store_with_gate(&old, &[("capacity", PropertyValue::Int(120))]).await;
    assert!(
        indexes(&t, "of_obj_gate")
            .await
            .contains(&"of_ix_of_obj_gate_capacity".to_owned())
    );

    let new = gate("");
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    let version = apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("apply");
    assert_eq!(version, 2);

    assert_eq!(
        int_of(&t, "of_obj_gate", "capacity__arch_v2").await,
        Some(120)
    );
    let columns = t.columns(&t.schema_edit, "of_obj_gate").await;
    assert!(!columns.contains(&"capacity".to_owned()), "{columns:?}");
    assert!(
        !indexes(&t, "of_obj_gate")
            .await
            .contains(&"of_ix_of_obj_gate_capacity".to_owned())
    );
    assert_eq!(t.active_version().await, 2);
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn rename_property_and_type_on_pg() {
    let old = gate("  capacity: { type: integer, indexed: true }\n");
    let t = store_with_gate(&old, &[("capacity", PropertyValue::Int(120))]).await;

    let new = defs(&object_yaml(
        "Portal",
        "code",
        "  code: { type: string, required: true }\n  seats: { type: integer, indexed: true }\n",
    ));
    let renames = [
        RenameDecl::ObjectType {
            from: "Gate".to_owned(),
            to: "Portal".to_owned(),
        },
        RenameDecl::Property {
            type_name: "Portal".to_owned(),
            from: "capacity".to_owned(),
            to: "seats".to_owned(),
        },
    ];
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &renames).expect("plan");
    apply_migration(&t.store, &new, &renames, &plan.hash)
        .await
        .expect("apply");

    assert!(t.columns(&t.schema_edit, "of_obj_gate").await.is_empty());
    assert_eq!(int_of(&t, "of_obj_portal", "seats").await, Some(120));
    assert_eq!(int_of(&t, "of_obj_portal", "seats__set").await, Some(1));
    assert_eq!(
        indexes(&t, "of_obj_portal")
            .await
            .iter()
            .filter(|n| n.starts_with("of_ix_"))
            .cloned()
            .collect::<Vec<_>>(),
        vec!["of_ix_of_obj_portal_seats".to_owned()]
    );
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn primary_key_change_then_drop_archived_on_pg() {
    let old = gate("  name: { type: string }\n  capacity: { type: integer }\n");
    let t = store_with_gate(&old, &[("capacity", PropertyValue::Int(120))]).await;

    let new = defs(&object_yaml(
        "Gate",
        "name",
        "  code: { type: string }\n  name: { type: string, required: true }\n  capacity: { type: integer }\n",
    ));
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("apply");
    assert_eq!(
        int_of(&t, "of_obj_gate__arch_v2", "capacity").await,
        Some(120)
    );
    assert!(
        t.columns(&t.schema_edit, "of_obj_gate")
            .await
            .contains(&"name".to_owned())
    );
    assert_reload_is_noop(&t, &new).await;

    let active = t.store.active_definition().await.expect("active");
    let drop_plan = plan_drop_archived(&active);
    assert_eq!(drop_plan.steps.len(), 1, "{:#?}", drop_plan.steps);
    let version = apply_drop_archived(&t.store, &drop_plan.hash)
        .await
        .expect("drop");
    assert_eq!(version, 3);
    assert!(
        t.columns(&t.schema_edit, "of_obj_gate__arch_v2")
            .await
            .is_empty()
    );
    assert_reload_is_noop(&t, &new).await;
}

// ---------------------------------------------------------------------------------------------
// Roles that a migration would break (plan 03-22, D-18, SCOPE-06)
// ---------------------------------------------------------------------------------------------

const AVIATION_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

fn roles_check() -> RolesCheck {
    RolesCheck {
        attrs: BTreeMap::from([("homeAirports".to_owned(), AttrType::StringList)]),
        profile: None,
    }
}

fn load_aviation(root: &camino::Utf8Path) -> Ontology {
    let out = ontoforge_schema::load_dir(root);
    out.ontology
        .unwrap_or_else(|| panic!("{:#?}", out.diagnostics))
}

/// The aviation definition without `Airport.city`, a property the `dispatcher` role lists.
fn aviation_without_city() -> Ontology {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).expect("create dir");
        for entry in std::fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let target = to.join(entry.file_name());
            if entry.file_type().expect("file type").is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).expect("copy");
            }
        }
    }
    let dir = tempfile::tempdir().expect("tempdir");
    copy(std::path::Path::new(AVIATION_DIR), dir.path());
    let path = dir.path().join("objects.yaml");
    let text = std::fs::read_to_string(&path).expect("read");
    let line = "  city: { type: string, displayName: City }
";
    assert!(text.contains(line), "objects.yaml lost the city line");
    std::fs::write(&path, text.replace(line, "")).expect("write");
    load_aviation(camino::Utf8Path::from_path(dir.path()).expect("utf-8 path"))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_migrate_rechecks_applied_roles() {
    let t = pg_store().await;
    let aviation = load_aviation(camino::Utf8Path::new(AVIATION_DIR));
    reload(
        &t.store,
        &aviation,
        &ReloadOptions {
            roles: Some(roles_check()),
            ..ReloadOptions::default()
        },
    )
    .await
    .expect("v1");
    let out = ontoforge_schema::load_scope_dir(camino::Utf8Path::new(AVIATION_SCOPE));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let scope = out.scope.expect("aviation scope");
    let check = roles_check();
    let compiled = ontoforge_auth::scope::compile_scope(&ontoforge_auth::scope::ScopeInputs {
        ontology: &aviation,
        profile: None,
        attrs: &check.attrs,
        scope: &scope,
        sources: None,
    })
    .unwrap_or_else(|d| panic!("{d:#?}"));
    let props = compiled.row_filter_properties();
    let scope_plan = t
        .store
        .scope_plan(&scope, &props)
        .await
        .expect("scope plan");
    t.store
        .scope_apply(&scope, &props, &scope_plan.hash, "tester@testhost")
        .await
        .expect("scope apply");

    let new = aviation_without_city();
    let active = t.store.active_definition().await.expect("active");
    let hash = plan_migration(&active, &new, &[]).expect("plan").hash;
    let airport_before = t.columns(&t.schema_edit, "of_obj_airport").await;
    assert!(
        airport_before.contains(&"city".to_owned()),
        "{airport_before:?}"
    );

    let result =
        apply_migration_probed(&t.store, &new, &[], &hash, None, Some(&roles_check())).await;
    let Err(MigrateError::Definition(diagnostics)) = result else {
        panic!("expected a definition error, got {result:?}");
    };
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics.iter().all(|d| d.code == "ONT-V21"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("dispatcher") && d.message.contains("city")),
        "the diagnostic names the role and the property: {diagnostics:#?}"
    );
    let airport_after = t.columns(&t.schema_edit, "of_obj_airport").await;
    assert_eq!(airport_after, airport_before, "no step ran");
    assert!(
        !airport_after.iter().any(|c| c == "city__arch_v2"),
        "{airport_after:?}"
    );
    assert_eq!(t.active_version().await, 1);
    assert_eq!(t.version_rows().await, 1);

    // Without the role check the very same migration goes through: the roles stopped it.
    let version = apply_migration(&t.store, &new, &[], &hash)
        .await
        .expect("without the role check");
    assert_eq!(version, 2);
}
