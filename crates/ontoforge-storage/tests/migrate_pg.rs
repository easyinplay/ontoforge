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

use ontoforge_schema::ir::{ObjectTypeName, Ontology, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::migrate::{
    RenameDecl, apply_drop_archived, apply_migration, plan_drop_archived, plan_migration,
};
use ontoforge_storage::reload::{ReloadOptions, reload};
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
