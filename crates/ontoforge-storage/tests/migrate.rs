//! `migrate` on the SQLite edit store: the way out of the destructive changes `reload` refuses
//! (STORE-02, D-14, D-16). Each case loads a definition, writes edit data through `commit`,
//! changes the definition and checks what the plan says and what the database holds afterwards.

mod support;

use std::collections::BTreeMap;

use ontoforge_schema::ir::{ObjectTypeName, Ontology, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::migrate::{MigrateError, MigrationStep, apply_migration, plan_migration};
use ontoforge_storage::reload::{ReloadError, ReloadOptions, reload};
use ontoforge_storage::value::PropertyValue;
use support::{TestStore, ontology_from, sqlite_store};

const ALICE: &str = "https://issuer.example|alice";

/// `Gate` with the given property lines (two-space indented, newline terminated).
fn gate(properties: &str) -> Ontology {
    let text = format!(
        "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: {{ type: string, required: true }}\n{properties}"
    );
    ontology_from(&[("defs.yaml", &text)])
}

fn with_capacity() -> Ontology {
    gate("  name: { type: string }\n  capacity: { type: integer }\n")
}

fn without_capacity() -> Ontology {
    gate("  name: { type: string }\n")
}

fn object_name(name: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(name.to_owned()).expect("object type name")
}

fn property_name(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property name")
}

/// A `Set` of gate `code` with the given values, committed against `def_version`.
async fn write_gate(t: &TestStore, def_version: i64, code: &str, values: &[(&str, PropertyValue)]) {
    let values: BTreeMap<PropertyName, PropertyValue> = values
        .iter()
        .map(|(name, value)| (property_name(name), value.clone()))
        .collect();
    t.store
        .commit(Commit {
            actor: Actor::Principal(ALICE.to_owned()),
            action: "setGate".to_owned(),
            def_version,
            params: serde_json::json!({ "code": code }),
            edits: vec![Edit::Object {
                type_name: object_name("Gate"),
                pk: PropertyValue::String(code.to_owned()),
                op: ObjectOp::Set(values),
            }],
        })
        .await
        .expect("commit");
}

/// The first value of `column` of the row `code` of `of_obj_gate`, as an integer.
async fn gate_int(t: &TestStore, column: &str, code: &str) -> Option<i64> {
    let pool = t.side_pool().await;
    let sql = format!("SELECT \"{column}\" FROM of_obj_gate WHERE code = ?");
    sqlx::query_scalar::<_, Option<i64>>(sqlx::AssertSqlSafe(sql))
        .bind(code)
        .fetch_one(&pool)
        .await
        .expect("read column")
}

async fn migration_json(t: &TestStore, def_version: i64) -> String {
    let pool = t.side_pool().await;
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT migration_json FROM of_def_version WHERE def_version = ?",
    )
    .bind(def_version)
    .fetch_one(&pool)
    .await
    .expect("migration_json")
    .unwrap_or_default()
}

/// A store with `with_capacity()` loaded and gate `G1` holding name and capacity 120.
async fn store_with_gate() -> TestStore {
    let t = sqlite_store().await;
    reload(&t.store, &with_capacity(), &ReloadOptions::default())
        .await
        .expect("first reload");
    write_gate(
        &t,
        1,
        "G1",
        &[
            ("name", PropertyValue::String("North".to_owned())),
            ("capacity", PropertyValue::Int(120)),
        ],
    )
    .await;
    t
}

#[tokio::test(flavor = "multi_thread")]
async fn archive_removed_property() {
    let t = store_with_gate().await;
    let new = without_capacity();

    let refused = reload(&t.store, &new, &ReloadOptions::default()).await;
    let Err(ReloadError::Definition(diagnostics)) = refused else {
        panic!("reload must refuse the removal: {refused:?}");
    };
    assert!(diagnostics.iter().all(|d| d.code == "ONT-V25"));

    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    assert_eq!(plan.from_def_version, 1);
    assert_eq!(
        plan.steps,
        vec![MigrationStep::ArchiveColumn {
            type_name: "Gate".to_owned(),
            property: "capacity".to_owned(),
            table: "of_obj_gate".to_owned(),
            column: "capacity".to_owned(),
            archived: "capacity__arch_v2".to_owned(),
            set_archived: "capacity__set__arch_v2".to_owned(),
        }]
    );
    assert_eq!(plan.hash.len(), 16, "{}", plan.hash);
    assert!(plan.hash.bytes().all(|b| b.is_ascii_hexdigit()));

    let version = apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("apply");
    assert_eq!(version, 2);
    assert_eq!(t.active_version().await, 2);

    let columns = t.columns("of_obj_gate").await;
    assert!(!columns.contains(&"capacity".to_owned()), "{columns:?}");
    assert!(
        !columns.contains(&"capacity__set".to_owned()),
        "{columns:?}"
    );
    assert_eq!(gate_int(&t, "capacity__arch_v2", "G1").await, Some(120));
    assert_eq!(gate_int(&t, "capacity__set__arch_v2", "G1").await, Some(1));

    let active = t.store.active_definition().await.expect("active");
    let table = &active.physical.objects[&object_name("Gate")];
    assert_eq!(table.archived_columns.len(), 1);
    assert_eq!(table.archived_columns[0].column, "capacity__arch_v2");
    assert_eq!(table.archived_columns[0].def_version, 2);

    let recorded = migration_json(&t, 2).await;
    assert!(recorded.contains("ArchiveColumn"), "{recorded}");
    assert!(recorded.contains("capacity__arch_v2"), "{recorded}");

    let again = reload(&t.store, &new, &ReloadOptions::default())
        .await
        .expect("reload after migrate");
    assert!(!again.changed, "reload must see no difference: {again:?}");
    assert_eq!(t.version_rows().await, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_with_stale_hash_is_rejected() {
    let t = store_with_gate().await;
    let columns_before = t.columns("of_obj_gate").await;

    let result = apply_migration(&t.store, &without_capacity(), &[], "0000000000000000").await;
    let Err(MigrateError::PlanChanged { expected, actual }) = result else {
        panic!("expected PlanChanged, got {result:?}");
    };
    assert_eq!(expected, "0000000000000000");
    assert_eq!(actual.len(), 16, "{actual}");

    assert_eq!(t.columns("of_obj_gate").await, columns_before);
    assert_eq!(t.active_version().await, 1);
    assert_eq!(t.version_rows().await, 1);
    assert_eq!(gate_int(&t, "capacity", "G1").await, Some(120));
}

#[tokio::test(flavor = "multi_thread")]
async fn reapply_after_success_is_rejected() {
    let t = store_with_gate().await;
    let new = without_capacity();
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("first apply");
    let columns_after = t.columns("of_obj_gate").await;

    let second = apply_migration(&t.store, &new, &[], &plan.hash).await;
    let Err(MigrateError::PlanChanged { expected, actual }) = second else {
        panic!("a plan that was applied must not apply again: {second:?}");
    };
    assert_eq!(expected, plan.hash);
    assert_ne!(actual, plan.hash);

    assert_eq!(t.columns("of_obj_gate").await, columns_after);
    assert_eq!(t.active_version().await, 2);
    assert_eq!(t.version_rows().await, 2);
}
