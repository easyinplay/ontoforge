//! `migrate` on the SQLite edit store: the way out of the destructive changes `reload` refuses
//! (STORE-02, D-14, D-16). Each case loads a definition, writes edit data through `commit`,
//! changes the definition and checks what the plan says and what the database holds afterwards.

mod support;

use std::collections::BTreeMap;

use ontoforge_schema::ir::{ObjectTypeName, Ontology, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::migrate::{
    MigrateError, MigrationStep, RenameDecl, apply_drop_archived, apply_migration,
    plan_drop_archived, plan_migration,
};
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

// ---------------------------------------------------------------------------------------------
// Renames, type and key changes, removed types, clean-up (plan 03-15 task 2)
// ---------------------------------------------------------------------------------------------

/// An object type with the given primary key and property lines.
fn object_yaml(name: &str, primary_key: &str, properties: &str) -> String {
    format!(
        "kind: ObjectType\napiName: {name}\nbacking: editOnly\nprimaryKey: {primary_key}\nproperties:\n{properties}---\n"
    )
}

const GATE_CODE: &str = "  code: { type: string, required: true }\n";

const STAND_AND_LINK: &str = "kind: ObjectType\napiName: Stand\nbacking: editOnly\nprimaryKey: id\nproperties:\n  id: { type: integer, required: true }\n---\nkind: LinkType\napiName: gateStands\ncardinality: manyToMany\nfrom: { objectType: Gate, apiName: stands }\nto: { objectType: Stand, apiName: gates }\nbacking: editOnly\n";

fn defs(text: &str) -> Ontology {
    ontology_from(&[("defs.yaml", text)])
}

fn gate_with(properties: &str) -> Ontology {
    defs(&object_yaml(
        "Gate",
        "code",
        &format!("{GATE_CODE}{properties}"),
    ))
}

/// `(name, is_unique)` of every index of `table`.
async fn index_names(t: &TestStore, table: &str) -> Vec<String> {
    use sqlx::Row;
    let pool = t.side_pool().await;
    let sql = format!("PRAGMA index_list(\"{table}\")");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(&pool)
        .await
        .expect("index_list")
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect()
}

async fn index_sql(t: &TestStore, name: &str) -> String {
    let pool = t.side_pool().await;
    sqlx::query_scalar::<_, String>(
        "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?",
    )
    .bind(name)
    .fetch_one(&pool)
    .await
    .expect("index sql")
}

/// The number of rows of `table`.
async fn row_count(t: &TestStore, table: &str) -> i64 {
    let pool = t.side_pool().await;
    let sql = format!("SELECT count(*) FROM \"{table}\"");
    sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql))
        .fetch_one(&pool)
        .await
        .expect("count")
}

/// Rows of the edit log, its entries and the history: a migration never touches them.
async fn log_rows(t: &TestStore) -> (i64, i64, i64) {
    (
        row_count(t, "of_edit_log").await,
        row_count(t, "of_edit_log_entry").await,
        row_count(t, "of_edit_history").await,
    )
}

async fn assert_reload_is_noop(t: &TestStore, ontology: &Ontology) {
    let again = reload(&t.store, ontology, &ReloadOptions::default())
        .await
        .expect("reload after migrate");
    assert!(!again.changed, "reload must see no difference: {again:?}");
}

fn property_rename(type_name: &str, from: &str, to: &str) -> RenameDecl {
    RenameDecl::Property {
        type_name: type_name.to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_property_moves_data() {
    let t = store_with_gate().await;
    let new = gate_with("  name: { type: string }\n  seats: { type: integer }\n");
    let renames = [property_rename("Gate", "capacity", "seats")];
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &renames).expect("plan");
    assert_eq!(
        plan.steps,
        vec![MigrationStep::RenameColumn {
            type_name: "Gate".to_owned(),
            table: "of_obj_gate".to_owned(),
            from: "capacity".to_owned(),
            to: "seats".to_owned(),
            from_column: "capacity".to_owned(),
            to_column: "seats".to_owned(),
        }]
    );
    let log_before = log_rows(&t).await;

    let version = apply_migration(&t.store, &new, &renames, &plan.hash)
        .await
        .expect("apply");
    assert_eq!(version, 2);
    assert_eq!(gate_int(&t, "seats", "G1").await, Some(120));
    assert_eq!(gate_int(&t, "seats__set", "G1").await, Some(1));
    let columns = t.columns("of_obj_gate").await;
    assert!(
        columns.iter().all(|c| !c.contains("capacity")),
        "no archived column: {columns:?}"
    );
    let active = t.store.active_definition().await.expect("active");
    assert!(
        active.physical.objects[&object_name("Gate")]
            .archived_columns
            .is_empty()
    );
    assert_eq!(log_rows(&t).await, log_before, "the edit log is untouched");
    assert_reload_is_noop(&t, &new).await;

    // The same call again plans nothing, so the reviewed hash no longer applies.
    let again = apply_migration(&t.store, &new, &renames, &plan.hash).await;
    assert!(
        matches!(again, Err(MigrateError::PlanChanged { .. })),
        "{again:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn undeclared_rename_archives_and_adds_an_empty_column() {
    let t = store_with_gate().await;
    let new = gate_with("  name: { type: string }\n  seats: { type: integer }\n");
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    assert_eq!(plan.steps.len(), 2, "{:#?}", plan.steps);
    assert!(matches!(
        &plan.steps[0],
        MigrationStep::ArchiveColumn { column, .. } if column == "capacity"
    ));
    assert!(matches!(
        &plan.steps[1],
        MigrationStep::Ddl(ontoforge_storage::ddl::DdlStep::AddPropertyColumn { column, .. })
            if column.column == "seats"
    ));

    apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("apply");
    assert_eq!(gate_int(&t, "capacity__arch_v2", "G1").await, Some(120));
    assert_eq!(gate_int(&t, "seats", "G1").await, None);
    assert_eq!(gate_int(&t, "seats__set", "G1").await, Some(0));
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_rebuilds_the_index_of_the_column() {
    let t = sqlite_store().await;
    let old = gate_with("  capacity: { type: integer, indexed: true }\n");
    reload(&t.store, &old, &ReloadOptions::default())
        .await
        .expect("first reload");
    assert!(
        index_names(&t, "of_obj_gate")
            .await
            .contains(&"of_ix_of_obj_gate_capacity".to_owned())
    );

    let new = gate_with("  seats: { type: integer, indexed: true }\n");
    let renames = [property_rename("Gate", "capacity", "seats")];
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &renames).expect("plan");
    assert_eq!(plan.steps.len(), 3, "{:#?}", plan.steps);
    apply_migration(&t.store, &new, &renames, &plan.hash)
        .await
        .expect("apply");

    let indexes = index_names(&t, "of_obj_gate").await;
    assert!(
        indexes.contains(&"of_ix_of_obj_gate_seats".to_owned()),
        "{indexes:?}"
    );
    assert!(
        !indexes.contains(&"of_ix_of_obj_gate_capacity".to_owned()),
        "{indexes:?}"
    );
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_with_type_change_is_rejected() {
    let t = store_with_gate().await;
    let new = gate_with("  name: { type: string }\n  seats: { type: long }\n");
    let renames = [property_rename("Gate", "capacity", "seats")];
    let active = t.store.active_definition().await.expect("active");

    let diagnostics = plan_migration(&active, &new, &renames).expect_err("type differs");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, "ONT-V25");
    assert!(
        diagnostics[0].message.contains("rename first"),
        "{}",
        diagnostics[0].message
    );

    let columns_before = t.columns("of_obj_gate").await;
    let result = apply_migration(&t.store, &new, &renames, "0000000000000000").await;
    assert!(
        matches!(&result, Err(MigrateError::Definition(d)) if d[0].code == "ONT-V25"),
        "{result:?}"
    );
    assert_eq!(t.columns("of_obj_gate").await, columns_before);
    assert_eq!(t.active_version().await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_object_type_moves_table() {
    let t = sqlite_store().await;
    let old = gate_with("  name: { type: string, indexed: true }\n");
    reload(&t.store, &old, &ReloadOptions::default())
        .await
        .expect("first reload");
    write_gate(
        &t,
        1,
        "G1",
        &[("name", PropertyValue::String("North".to_owned()))],
    )
    .await;

    let new = defs(&object_yaml(
        "Portal",
        "code",
        &format!("{GATE_CODE}  name: {{ type: string, indexed: true }}\n"),
    ));
    let renames = [RenameDecl::ObjectType {
        from: "Gate".to_owned(),
        to: "Portal".to_owned(),
    }];
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &renames).expect("plan");
    let table_renames = plan
        .steps
        .iter()
        .filter(|s| matches!(s, MigrationStep::RenameTable { .. }))
        .count();
    assert_eq!(table_renames, 1, "{:#?}", plan.steps);
    assert_eq!(plan.steps.len(), 3, "drop index, rename, create index");

    apply_migration(&t.store, &new, &renames, &plan.hash)
        .await
        .expect("apply");
    assert!(
        t.columns("of_obj_gate").await.is_empty(),
        "old table is gone"
    );
    assert_eq!(row_count(&t, "of_obj_portal").await, 1);
    let pool = t.side_pool().await;
    let name: String = sqlx::query_scalar("SELECT name FROM of_obj_portal WHERE code = 'G1'")
        .fetch_one(&pool)
        .await
        .expect("name");
    assert_eq!(name, "North");
    let indexes = index_names(&t, "of_obj_portal").await;
    assert!(
        indexes.contains(&"of_ix_of_obj_portal_name".to_owned()),
        "{indexes:?}"
    );
    assert!(
        index_sql(&t, "of_ix_of_obj_portal_name")
            .await
            .contains("\"of_obj_portal\""),
    );
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_rename_decl_is_rejected() {
    let t = store_with_gate().await;
    let new = without_capacity();
    let active = t.store.active_definition().await.expect("active");
    for decl in [
        property_rename("Gate", "nope", "x"),
        property_rename("Nope", "capacity", "x"),
        RenameDecl::ObjectType {
            from: "Nope".to_owned(),
            to: "Other".to_owned(),
        },
        // the primary key cannot be renamed
        property_rename("Gate", "code", "key"),
    ] {
        let diagnostics = plan_migration(&active, &new, std::slice::from_ref(&decl))
            .expect_err(&format!("{decl} must be refused"));
        assert!(!diagnostics.is_empty(), "{decl}");
        assert!(
            diagnostics.iter().all(|d| d.code == "ONT-V25"),
            "{decl}: {diagnostics:#?}"
        );
    }

    assert_eq!(
        "Gate.capacity=seats".parse::<RenameDecl>(),
        Ok(property_rename("Gate", "capacity", "seats"))
    );
    assert_eq!(
        "Gate=Portal".parse::<RenameDecl>(),
        Ok(RenameDecl::ObjectType {
            from: "Gate".to_owned(),
            to: "Portal".to_owned()
        })
    );
    for bad in [
        "bad",
        "Gate.x",
        "=x",
        "gate=Portal",
        "Gate.cap=Seats",
        "A=B=C",
    ] {
        assert!(bad.parse::<RenameDecl>().is_err(), "{bad} must not parse");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn type_change_archives_and_adds() {
    let t = sqlite_store().await;
    let old = gate_with("  capacity: { type: integer, indexed: true }\n");
    reload(&t.store, &old, &ReloadOptions::default())
        .await
        .expect("first reload");
    write_gate(&t, 1, "G1", &[("capacity", PropertyValue::Int(120))]).await;

    let new = gate_with("  capacity: { type: long, indexed: true }\n");
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    assert_eq!(plan.steps.len(), 4, "{:#?}", plan.steps);
    apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("apply");

    assert_eq!(gate_int(&t, "capacity__arch_v2", "G1").await, Some(120));
    assert_eq!(gate_int(&t, "capacity__set__arch_v2", "G1").await, Some(1));
    assert_eq!(gate_int(&t, "capacity", "G1").await, None);
    assert_eq!(gate_int(&t, "capacity__set", "G1").await, Some(0));
    let sql = index_sql(&t, "of_ix_of_obj_gate_capacity").await;
    assert!(
        sql.contains("(\"capacity\")"),
        "the index follows the new column: {sql}"
    );
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn primary_key_change_archives_table() {
    let t = store_with_gate().await;
    let new = defs(&object_yaml(
        "Gate",
        "name",
        "  code: { type: string }\n  name: { type: string, required: true }\n  capacity: { type: integer }\n",
    ));
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    assert_eq!(plan.steps.len(), 2, "{:#?}", plan.steps);
    assert!(matches!(
        &plan.steps[0],
        MigrationStep::ArchiveTable { table, archived, .. }
            if table == "of_obj_gate" && archived == "of_obj_gate__arch_v2"
    ));
    assert!(matches!(
        &plan.steps[1],
        MigrationStep::Ddl(ontoforge_storage::ddl::DdlStep::CreateObjectTable(_))
    ));

    apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("apply");
    assert_eq!(row_count(&t, "of_obj_gate__arch_v2").await, 1);
    assert_eq!(row_count(&t, "of_obj_gate").await, 0);
    let columns = t.columns("of_obj_gate").await;
    assert!(columns.contains(&"name".to_owned()), "{columns:?}");
    let active = t.store.active_definition().await.expect("active");
    assert_eq!(active.physical.archived_tables.len(), 1);
    assert_eq!(
        active.physical.archived_tables[0].table,
        "of_obj_gate__arch_v2"
    );
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn removed_type_archived() {
    let t = sqlite_store().await;
    let old = defs(&format!(
        "{}{STAND_AND_LINK}",
        object_yaml(
            "Gate",
            "code",
            &format!("{GATE_CODE}  name: {{ type: string }}\n")
        )
    ));
    reload(&t.store, &old, &ReloadOptions::default())
        .await
        .expect("first reload");
    let pool = t.side_pool().await;
    sqlx::query("INSERT INTO of_obj_stand (id) VALUES (7)")
        .execute(&pool)
        .await
        .expect("stand row");
    sqlx::query("INSERT INTO of_lnk_gate_stands (a_pk, b_pk) VALUES ('G1', 7)")
        .execute(&pool)
        .await
        .expect("link row");

    let new = gate_with("  name: { type: string }\n");
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    let archived: Vec<&str> = plan
        .steps
        .iter()
        .filter_map(|s| match s {
            MigrationStep::ArchiveTable { archived, .. } => Some(archived.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        archived,
        vec!["of_obj_stand__arch_v2", "of_lnk_gate_stands__arch_v2"]
    );
    apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("apply");

    assert!(t.columns("of_obj_stand").await.is_empty());
    assert!(t.columns("of_lnk_gate_stands").await.is_empty());
    assert_eq!(row_count(&t, "of_obj_stand__arch_v2").await, 1);
    assert_eq!(row_count(&t, "of_lnk_gate_stands__arch_v2").await, 1);
    let active = t.store.active_definition().await.expect("active");
    assert_eq!(active.physical.archived_tables.len(), 2);
    assert_reload_is_noop(&t, &new).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn drop_archived_cleans_up() {
    let t = sqlite_store().await;
    let old = defs(&format!(
        "{}{STAND_AND_LINK}",
        object_yaml(
            "Gate",
            "code",
            &format!(
                "{GATE_CODE}  name: {{ type: string }}\n  capacity: {{ type: integer, indexed: true }}\n"
            )
        )
    ));
    reload(&t.store, &old, &ReloadOptions::default())
        .await
        .expect("first reload");
    write_gate(&t, 1, "G1", &[("capacity", PropertyValue::Int(120))]).await;

    let new = gate_with("  name: { type: string }\n");
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &new, &[]).expect("plan");
    apply_migration(&t.store, &new, &[], &plan.hash)
        .await
        .expect("archive");
    assert!(
        t.columns("of_obj_gate")
            .await
            .contains(&"capacity__arch_v2".to_owned())
    );
    let log_before = log_rows(&t).await;

    let active = t.store.active_definition().await.expect("active");
    let drop_plan = plan_drop_archived(&active);
    let dropped_columns: Vec<&str> = drop_plan
        .steps
        .iter()
        .filter_map(|s| match s {
            MigrationStep::DropArchivedColumn { column, .. } => Some(column.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        dropped_columns,
        vec!["capacity__arch_v2", "capacity__set__arch_v2"]
    );
    let dropped_tables: Vec<&str> = drop_plan
        .steps
        .iter()
        .filter_map(|s| match s {
            MigrationStep::DropArchivedTable { table } => Some(table.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        dropped_tables,
        vec!["of_obj_stand__arch_v2", "of_lnk_gate_stands__arch_v2"]
    );

    let stale = apply_drop_archived(&t.store, "0000000000000000").await;
    assert!(
        matches!(stale, Err(MigrateError::PlanChanged { .. })),
        "{stale:?}"
    );
    assert!(
        t.columns("of_obj_gate")
            .await
            .contains(&"capacity__arch_v2".to_owned()),
        "a refused plan drops nothing"
    );

    let version = apply_drop_archived(&t.store, &drop_plan.hash)
        .await
        .expect("drop");
    assert_eq!(version, 3);
    assert_eq!(t.active_version().await, 3);
    let columns = t.columns("of_obj_gate").await;
    assert!(columns.iter().all(|c| !c.contains("arch")), "{columns:?}");
    assert!(t.columns("of_obj_stand__arch_v2").await.is_empty());
    assert!(t.columns("of_lnk_gate_stands__arch_v2").await.is_empty());
    let active = t.store.active_definition().await.expect("active");
    assert!(active.physical.archived_tables.is_empty());
    assert!(
        active.physical.objects[&object_name("Gate")]
            .archived_columns
            .is_empty()
    );
    assert!(plan_drop_archived(&active).steps.is_empty());
    assert!(migration_json(&t, 3).await.contains("DropArchivedColumn"));
    assert_eq!(log_rows(&t).await, log_before, "the edit log is untouched");
    assert_reload_is_noop(&t, &new).await;
}
