//! `reload` on the SQLite edit store: tables from a definition, idempotency, additive and
//! destructive changes, dry run and recovery from an interrupted run (STORE-02).

mod support;

use std::collections::{BTreeMap, BTreeSet};

use futures_util::future::BoxFuture;
use ontoforge_engine::{EngineError, PhysicalColumn, PhysicalType, TableRef};
use ontoforge_schema::ir::{AttrType, ObjectTypeName, Ontology, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, EditError, ObjectOp};
use ontoforge_storage::migrate::{MigrateError, apply_migration_probed, plan_migration};
use ontoforge_storage::reload::{
    ProbeContext, ProbeTarget, ReloadError, ReloadOptions, RolesCheck, reload,
};
use ontoforge_storage::value::PropertyValue;
use support::{ontology_from, sqlite_store};

const GATE_PROPERTIES: &str = "  code: { type: string, required: true }
  name: { type: string }
  capacity: { type: integer }
";

const STAND: &str = "---
kind: ObjectType
apiName: Stand
backing: source
primaryKey: id
properties:
  id: { type: integer, required: true }
  label: { type: string }
---
kind: LinkType
apiName: gateStands
cardinality: manyToMany
from: { objectType: Gate, apiName: stands }
to: { objectType: Stand, apiName: gates }
backing: editOnly
";

fn definition(primary_key: &str, gate_properties: &str, with_stand: bool) -> Ontology {
    let mut text = format!(
        "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: {primary_key}\nproperties:\n{gate_properties}"
    );
    if with_stand {
        text.push_str(STAND);
    }
    ontology_from(&[("defs.yaml", &text)])
}

fn base() -> Ontology {
    definition("code", GATE_PROPERTIES, true)
}

fn names(columns: &[String]) -> BTreeSet<&str> {
    columns.iter().map(String::as_str).collect()
}

const GATE_COLUMNS: [&str; 10] = [
    "code",
    "name",
    "name__set",
    "capacity",
    "capacity__set",
    "of_version",
    "of_deleted",
    "of_released",
    "of_def_version",
    "of_log_seq",
];

#[tokio::test(flavor = "multi_thread")]
async fn first_reload_creates_tables() {
    let t = sqlite_store().await;
    let out = reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("reload");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(1));
    assert_eq!(out.previous_def_version, None);
    assert!(!out.ddl.is_empty());

    let gate = t.columns("of_obj_gate").await;
    assert_eq!(names(&gate), BTreeSet::from(GATE_COLUMNS));
    assert!(!gate.contains(&"code__set".to_owned()), "pk has no __set");

    let stand = t.columns("of_obj_stand").await;
    assert_eq!(
        names(&stand),
        BTreeSet::from([
            "id",
            "label",
            "label__set",
            "of_version",
            "of_deleted",
            "of_released",
            "of_def_version",
            "of_log_seq"
        ])
    );

    let link = t.columns("of_lnk_gate_stands").await;
    assert_eq!(
        names(&link),
        BTreeSet::from([
            "a_pk",
            "b_pk",
            "of_deleted",
            "of_version",
            "of_def_version",
            "of_log_seq"
        ])
    );
    assert_eq!(t.active_version().await, 1);
    assert_eq!(t.version_rows().await, 1);
    assert_eq!(
        t.store.active_definition().await.map(|a| a.def_version),
        Some(1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn second_reload_is_noop() {
    let t = sqlite_store().await;
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
async fn reload_after_reopen_is_noop() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let reopened = t.reopen().await;
    let active = reopened
        .active_definition()
        .await
        .expect("active definition");
    assert_eq!(active.def_version, 1);
    let again = reload(&reopened, &base(), &ReloadOptions::default())
        .await
        .expect("after reopen");
    assert!(!again.changed, "the stored definition reads back equal");
}

#[tokio::test(flavor = "multi_thread")]
async fn additive_property_adds_columns() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let with_zone = format!("{GATE_PROPERTIES}  zone: {{ type: string }}\n");
    let out = reload(
        &t.store,
        &definition("code", &with_zone, true),
        &ReloadOptions::default(),
    )
    .await
    .expect("additive");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(2));
    assert_eq!(out.previous_def_version, Some(1));
    assert_eq!(out.ddl.len(), 2, "{:#?}", out.ddl);
    assert!(out.ddl.iter().all(|sql| sql.contains("ADD COLUMN")));
    let gate = t.columns("of_obj_gate").await;
    assert!(gate.contains(&"zone".to_owned()) && gate.contains(&"zone__set".to_owned()));
    assert_eq!(t.active_version().await, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn metadata_only_change_records_a_version_without_ddl() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let described = "  code: { type: string, required: true }
  name: { type: string, displayName: Name }
  capacity: { type: integer }
";
    let out = reload(
        &t.store,
        &definition("code", described, true),
        &ReloadOptions::default(),
    )
    .await
    .expect("metadata");
    assert!(out.changed);
    assert!(out.ddl.is_empty());
    assert_eq!(out.def_version, Some(2));
}

#[tokio::test(flavor = "multi_thread")]
async fn destructive_changes_are_rejected() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let gate_before = t.columns("of_obj_gate").await;

    let cases: Vec<(&str, Ontology, usize)> = vec![
        (
            "property removed",
            definition(
                "code",
                "  code: { type: string, required: true }\n  capacity: { type: integer }\n",
                true,
            ),
            1,
        ),
        (
            "property type changed",
            definition(
                "code",
                "  code: { type: string, required: true }\n  name: { type: string }\n  capacity: { type: long }\n",
                true,
            ),
            1,
        ),
        (
            "required property added",
            definition(
                "code",
                &format!("{GATE_PROPERTIES}  zone: {{ type: string, required: true }}\n"),
                true,
            ),
            1,
        ),
        (
            "object type removed",
            definition("code", GATE_PROPERTIES, false),
            2,
        ),
        (
            "primary key changed",
            definition(
                "name",
                "  code: { type: string }\n  name: { type: string, required: true }\n  capacity: { type: integer }\n",
                true,
            ),
            2,
        ),
    ];

    for (label, ontology, expected) in cases {
        let result = reload(&t.store, &ontology, &ReloadOptions::default()).await;
        let Err(ReloadError::Definition(diagnostics)) = result else {
            panic!("{label}: expected a definition error, got {result:?}");
        };
        assert_eq!(diagnostics.len(), expected, "{label}: {diagnostics:#?}");
        assert!(
            diagnostics.iter().all(|d| d.code == "ONT-V25"),
            "{label}: {diagnostics:#?}"
        );
        assert_eq!(t.columns("of_obj_gate").await, gate_before, "{label}");
        assert!(!t.columns("of_obj_stand").await.is_empty(), "{label}");
        assert_eq!(t.active_version().await, 1, "{label}");
        assert_eq!(t.version_rows().await, 1, "{label}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn dry_run_changes_nothing() {
    let t = sqlite_store().await;
    let out = reload(
        &t.store,
        &base(),
        &ReloadOptions {
            dry_run: true,
            ..ReloadOptions::default()
        },
    )
    .await
    .expect("dry run");
    assert!(out.changed);
    assert!(!out.ddl.is_empty());
    assert!(out.ddl.iter().any(|sql| sql.contains("of_obj_gate")));
    assert_eq!(t.version_rows().await, 0);
    assert!(t.columns("of_obj_gate").await.is_empty());
    assert!(t.store.active_definition().await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn reload_resumes_after_partial_ddl() {
    let t = sqlite_store().await;
    // What an interrupted earlier run (or a manual fix) can leave behind.
    let side = t.side_pool().await;
    sqlx::query("CREATE TABLE of_obj_gate (code TEXT NOT NULL PRIMARY KEY, name TEXT)")
        .execute(&side)
        .await
        .expect("pre-create");
    drop(side);

    let out = reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("resume");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(1));
    let gate = t.columns("of_obj_gate").await;
    assert_eq!(gate.len(), GATE_COLUMNS.len(), "no column twice: {gate:?}");
    assert_eq!(names(&gate), BTreeSet::from(GATE_COLUMNS));
}

// ---------------------------------------------------------------------------------------------
// Indexes (STORE-02, STORE-05, plan 03-10)
// ---------------------------------------------------------------------------------------------

fn aviation() -> Ontology {
    let root = camino::Utf8Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    ));
    ontoforge_schema::load_dir(root)
        .ontology
        .expect("aviation loads")
}

/// Sets `indexed` of `type_name.property` in a copy of `ontology`.
fn with_indexed(ontology: &Ontology, type_name: &str, property: &str, indexed: bool) -> Ontology {
    let mut changed = ontology.clone();
    let object = changed
        .object_types
        .get_mut(&ObjectTypeName::try_from(type_name.to_owned()).expect("type name"))
        .expect("object type");
    object
        .properties
        .get_mut(&PropertyName::try_from(property.to_owned()).expect("property name"))
        .expect("property")
        .indexed = indexed;
    changed
}

/// `(name, is_unique)` of every index of `table`.
async fn index_list(t: &support::TestStore, table: &str) -> Vec<(String, bool)> {
    use sqlx::Row;
    let pool = t.side_pool().await;
    let sql = format!("PRAGMA index_list(\"{table}\")");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(&pool)
        .await
        .expect("index_list")
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("name"),
                row.get::<i64, _>("unique") == 1,
            )
        })
        .collect()
}

async fn index_sql(t: &support::TestStore, name: &str) -> String {
    let pool = t.side_pool().await;
    sqlx::query_scalar::<_, String>(
        "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?",
    )
    .bind(name)
    .fetch_one(&pool)
    .await
    .expect("index sql")
}

const INSERT_MAINTENANCE: &str = "INSERT INTO of_obj_maintenance_record      (record_id, aircraft_tail_number, aircraft_tail_number__set, of_deleted) VALUES (?, 'N1', ?, ?)";

#[tokio::test(flavor = "multi_thread")]
async fn aviation_reload_creates_indexes() {
    let t = sqlite_store().await;
    reload(&t.store, &aviation(), &ReloadOptions::default())
        .await
        .expect("reload");

    let flight = index_list(&t, "of_obj_flight").await;
    assert!(
        flight.contains(&("of_ix_of_obj_flight_status".to_owned(), false)),
        "{flight:?}"
    );

    let unique = "of_ux_of_obj_maintenance_record_aircraft_tail_number";
    let maintenance = index_list(&t, "of_obj_maintenance_record").await;
    assert!(
        maintenance.contains(&(unique.to_owned(), true)),
        "{maintenance:?}"
    );
    assert!(index_sql(&t, unique).await.contains("WHERE"));
}

#[tokio::test(flavor = "multi_thread")]
async fn one_to_one_unique_index_rejects_second_row() {
    let t = sqlite_store().await;
    reload(&t.store, &aviation(), &ReloadOptions::default())
        .await
        .expect("reload");
    let pool = t.side_pool().await;
    let insert = |id: &'static str, set: i64, deleted: i64| {
        sqlx::query(INSERT_MAINTENANCE)
            .bind(id)
            .bind(set)
            .bind(deleted)
            .execute(&pool)
    };

    insert("r1", 1, 0).await.expect("first row");
    let second = insert("r2", 1, 0).await;
    assert!(
        second.is_err(),
        "a second live row with the same key must be refused"
    );
    insert("r3", 1, 1)
        .await
        .expect("a deleted row may repeat the key");
    insert("r4", 0, 0)
        .await
        .expect("a row without the key set may repeat it");
}

#[tokio::test(flavor = "multi_thread")]
async fn index_added_on_second_reload() {
    let t = sqlite_store().await;
    let first = aviation();
    reload(&t.store, &first, &ReloadOptions::default())
        .await
        .expect("first");
    let out = reload(
        &t.store,
        &with_indexed(&first, "Pilot", "fullName", true),
        &ReloadOptions::default(),
    )
    .await
    .expect("second");
    assert_eq!(out.def_version, Some(2));
    assert_eq!(out.ddl.len(), 1, "{:#?}", out.ddl);
    assert!(
        out.ddl[0].contains("CREATE INDEX IF NOT EXISTS"),
        "{}",
        out.ddl[0]
    );
    let pilot = index_list(&t, "of_obj_pilot").await;
    assert!(
        pilot.contains(&("of_ix_of_obj_pilot_full_name".to_owned(), false)),
        "{pilot:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_indexed_keeps_index() {
    let t = sqlite_store().await;
    let first = aviation();
    reload(&t.store, &first, &ReloadOptions::default())
        .await
        .expect("first");
    let relaxed = with_indexed(&first, "Flight", "status", false);
    let out = reload(&t.store, &relaxed, &ReloadOptions::default())
        .await
        .expect("second");
    assert!(out.changed);
    assert!(out.ddl.is_empty(), "{:#?}", out.ddl);
    let flight = index_list(&t, "of_obj_flight").await;
    assert!(
        flight
            .iter()
            .any(|(name, _)| name == "of_ix_of_obj_flight_status"),
        "{flight:?}"
    );
    let again = reload(&t.store, &relaxed, &ReloadOptions::default())
        .await
        .expect("third");
    assert!(
        !again.changed,
        "the kept index must not make every reload a change"
    );
}

// ---------------------------------------------------------------------------------------------
// Physical names, link and precision changes, cardinality changes (plan 03-10 task 2)
// ---------------------------------------------------------------------------------------------

fn object_yaml(name: &str, properties: &str) -> String {
    format!(
        "kind: ObjectType\napiName: {name}\nbacking: editOnly\nprimaryKey: id\nproperties:\n  id: {{ type: integer, required: true }}\n{properties}---\n"
    )
}

/// Names of the tables in the database file.
async fn table_names(t: &support::TestStore) -> Vec<String> {
    let pool = t.side_pool().await;
    sqlx::query_scalar::<_, String>("SELECT name FROM sqlite_master WHERE type = 'table'")
        .fetch_all(&pool)
        .await
        .expect("tables")
}

async fn assert_v24(ontology: Ontology) {
    let t = sqlite_store().await;
    let result = reload(&t.store, &ontology, &ReloadOptions::default()).await;
    let Err(ReloadError::Definition(diagnostics)) = result else {
        panic!("expected ONT-V24, got {result:?}");
    };
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics.iter().all(|d| d.code == "ONT-V24"),
        "{diagnostics:#?}"
    );
    assert_eq!(t.version_rows().await, 0, "no definition version");
    assert!(
        table_names(&t)
            .await
            .iter()
            .all(|n| !n.starts_with("of_obj_")),
        "no DDL ran"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn snake_collision_is_v24() {
    let text = format!("{}{}", object_yaml("ABTest", ""), object_yaml("AbTest", ""));
    assert_v24(ontology_from(&[("defs.yaml", &text)])).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn system_column_collision_is_v24() {
    let text = object_yaml("Item", "  ofVersion: { type: integer }\n");
    assert_v24(ontology_from(&[("defs.yaml", &text)])).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn long_names_round_trip() {
    let t = sqlite_store().await;
    let property = format!("p{}", "x".repeat(99));
    let type_name = format!("T{}", "x".repeat(99));
    let text = object_yaml(
        &type_name,
        &format!("  {property}: {{ type: string, indexed: true }}\n"),
    );
    let ontology = ontology_from(&[("defs.yaml", &text)]);
    reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("long names reload");

    let active = t.store.active_definition().await.expect("active");
    let table = active
        .physical
        .objects
        .values()
        .next()
        .expect("one object table");
    assert!(table.table.len() <= 36, "{}", table.table);
    let columns = t.columns(&table.table).await;
    assert!(
        !columns.is_empty(),
        "the table exists under its bounded name"
    );
    let column = table.properties.values().next().expect("property column");
    assert!(columns.contains(&column.column) && columns.contains(&column.set_column));
    assert!(
        table.indexes.iter().all(|ix| ix.name.len() <= 63),
        "{:?}",
        table.indexes
    );

    let again = reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("again");
    assert!(!again.changed);
}

fn link_yaml(to: &str) -> String {
    format!(
        "kind: LinkType\napiName: gateStands\ncardinality: manyToMany\nfrom: {{ objectType: Gate, apiName: stands }}\nto: {{ objectType: {to}, apiName: gates }}\nbacking: editOnly\n"
    )
}

fn three_types() -> String {
    ["Gate", "Stand", "Bay"]
        .iter()
        .map(|n| object_yaml(n, "  label: { type: string }\n"))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn link_changes_are_destructive() {
    let t = sqlite_store().await;
    let types = three_types();
    reload(
        &t.store,
        &ontology_from(&[("defs.yaml", &format!("{types}{}", link_yaml("Stand")))]),
        &ReloadOptions::default(),
    )
    .await
    .expect("first");

    for (label, text) in [
        ("link removed", types.clone()),
        ("link end changed", format!("{types}{}", link_yaml("Bay"))),
    ] {
        let result = reload(
            &t.store,
            &ontology_from(&[("defs.yaml", &text)]),
            &ReloadOptions::default(),
        )
        .await;
        let Err(ReloadError::Definition(diagnostics)) = result else {
            panic!("{label}: expected a definition error, got {result:?}");
        };
        assert_eq!(diagnostics.len(), 1, "{label}: {diagnostics:#?}");
        assert_eq!(diagnostics[0].code, "ONT-V25", "{label}");
    }
    assert_eq!(t.active_version().await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_many_to_many_link_creates_a_table() {
    let t = sqlite_store().await;
    let types = three_types();
    reload(
        &t.store,
        &ontology_from(&[("defs.yaml", &types)]),
        &ReloadOptions::default(),
    )
    .await
    .expect("first");
    let out = reload(
        &t.store,
        &ontology_from(&[("defs.yaml", &format!("{types}{}", link_yaml("Stand")))]),
        &ReloadOptions::default(),
    )
    .await
    .expect("with link");
    assert!(
        out.ddl.iter().any(|sql| sql.contains("of_lnk_gate_stands")),
        "{:#?}",
        out.ddl
    );
    assert!(!t.columns("of_lnk_gate_stands").await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn precision_and_items_changes_are_destructive() {
    let t = sqlite_store().await;
    let definition = |scale: &str, items: &str| {
        let text = object_yaml(
            "Item",
            &format!(
                "  price: {{ type: decimal, precision: 12, scale: {scale} }}\n  tags: {{ type: array, items: {items} }}\n"
            ),
        );
        ontology_from(&[("defs.yaml", &text)])
    };
    reload(
        &t.store,
        &definition("2", "string"),
        &ReloadOptions::default(),
    )
    .await
    .expect("first");
    for (label, ontology) in [
        ("scale", definition("3", "string")),
        ("items", definition("2", "integer")),
    ] {
        let result = reload(&t.store, &ontology, &ReloadOptions::default()).await;
        let Err(ReloadError::Definition(diagnostics)) = result else {
            panic!("{label}: expected a definition error, got {result:?}");
        };
        assert_eq!(diagnostics.len(), 1, "{label}: {diagnostics:#?}");
        assert_eq!(diagnostics[0].code, "ONT-V25", "{label}");
    }
}

fn flights(cardinality: &str) -> Ontology {
    let text = format!(
        "{}{}kind: LinkType\napiName: airportFlights\ncardinality: {cardinality}\nfrom: {{ objectType: Airport, apiName: flights }}\nto: {{ objectType: Flight, apiName: airport, foreignKey: airportId }}\n",
        object_yaml("Airport", ""),
        object_yaml("Flight", "  airportId: { type: integer }\n"),
    );
    ontology_from(&[("defs.yaml", &text)])
}

const INSERT_FLIGHT: &str =
    "INSERT INTO of_obj_flight (id, airport_id, airport_id__set) VALUES (?, 7, 1)";

#[tokio::test(flavor = "multi_thread")]
async fn tightening_to_one_to_one_with_conflicts_keeps_old_definition() {
    let t = sqlite_store().await;
    reload(&t.store, &flights("oneToMany"), &ReloadOptions::default())
        .await
        .expect("first");
    let pool = t.side_pool().await;
    for id in [1_i64, 2] {
        sqlx::query(INSERT_FLIGHT)
            .bind(id)
            .execute(&pool)
            .await
            .expect("insert");
    }

    let result = reload(&t.store, &flights("oneToOne"), &ReloadOptions::default()).await;
    let Err(ReloadError::Store(error)) = result else {
        panic!("expected a store error, got {result:?}");
    };
    let message = error.to_string();
    assert!(message.contains("airportFlights"), "{message}");
    assert!(message.contains("oneToOne"), "{message}");
    assert_eq!(t.active_version().await, 1);
    assert_eq!(t.version_rows().await, 1);
    assert!(
        index_list(&t, "of_obj_flight")
            .await
            .iter()
            .all(|(name, _)| !name.starts_with("of_ux_")),
        "the unique index was rolled back"
    );
    assert_eq!(
        t.store.active_definition().await.map(|a| a.def_version),
        Some(1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn relaxing_one_to_one_drops_unique_index() {
    let t = sqlite_store().await;
    reload(&t.store, &flights("oneToOne"), &ReloadOptions::default())
        .await
        .expect("first");
    let unique = "of_ux_of_obj_flight_airport_id";
    assert!(
        index_list(&t, "of_obj_flight")
            .await
            .contains(&(unique.to_owned(), true))
    );

    let out = reload(&t.store, &flights("oneToMany"), &ReloadOptions::default())
        .await
        .expect("relax");
    assert_eq!(out.ddl.len(), 1, "{:#?}", out.ddl);
    assert!(
        out.ddl[0].contains("DROP INDEX IF EXISTS"),
        "{}",
        out.ddl[0]
    );
    assert!(
        index_list(&t, "of_obj_flight")
            .await
            .iter()
            .all(|(n, _)| n != unique)
    );

    let pool = t.side_pool().await;
    for id in [1_i64, 2] {
        sqlx::query(INSERT_FLIGHT)
            .bind(id)
            .execute(&pool)
            .await
            .expect("duplicates are fine now");
    }
}

// ---------------------------------------------------------------------------------------------
// The definition version fence (ADR-0016 / D20, plan 03-18 task 1)
// ---------------------------------------------------------------------------------------------

/// A commit that sets the string properties `pairs` on the gate `code`, validated against
/// `def_version`.
fn gate_commit(def_version: i64, code: &str, pairs: &[(&str, &str)]) -> Commit {
    let values: BTreeMap<PropertyName, PropertyValue> = pairs
        .iter()
        .map(|(name, value)| {
            (
                PropertyName::try_from((*name).to_owned()).expect("property name"),
                PropertyValue::String((*value).to_owned()),
            )
        })
        .collect();
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

fn with_zone() -> Ontology {
    let properties = format!(
        "{GATE_PROPERTIES}  zone: {{ type: string }}
"
    );
    definition("code", &properties, true)
}

async fn count(t: &support::TestStore, table: &str) -> i64 {
    let pool = t.side_pool().await;
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM \"{table}\""
    )))
    .fetch_one(&pool)
    .await
    .expect("count")
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_writer_rejected_after_switch() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    // A second process: it opened the store while definition 1 was active.
    let writer = t.reopen().await;
    writer
        .commit(gate_commit(1, "A1", &[("name", "North")]))
        .await
        .expect("a commit for the active definition");

    let out = reload(&t.store, &with_zone(), &ReloadOptions::default())
        .await
        .expect("v2");
    assert_eq!(out.def_version, Some(2));

    // The writer still holds definition 1 in memory; the database says 2.
    let err = writer
        .commit(gate_commit(1, "A2", &[("name", "South")]))
        .await
        .expect_err("a commit for a definition that is no longer active");
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
    assert_eq!(count(&t, "of_obj_gate").await, 1, "no gate row was written");
    assert_eq!(
        count(&t, "of_edit_log").await,
        1,
        "no log record was written"
    );

    // The refusal refreshed the writer's definition from the database.
    assert_eq!(
        writer.active_definition().await.map(|a| a.def_version),
        Some(2)
    );
    writer
        .commit(gate_commit(2, "A2", &[("zone", "Z1")]))
        .await
        .expect("a commit for definition 2 may use the property it added");
    assert_eq!(count(&t, "of_obj_gate").await, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn writer_without_a_loaded_definition_follows_the_database() {
    let t = sqlite_store().await;
    // Opened before any reload: nothing is cached.
    let writer = t.reopen().await;
    assert!(writer.active_definition().await.is_none());
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    writer
        .commit(gate_commit(1, "A1", &[("name", "North")]))
        .await
        .expect("the fence loads the definition the database makes active");
    assert_eq!(
        writer.active_definition().await.map(|a| a.def_version),
        Some(1)
    );
}

// ---------------------------------------------------------------------------------------------
// Single flight, roles and row predicate indexes (plan 03-18 task 2)
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

fn with_roles() -> ReloadOptions<'static> {
    ReloadOptions {
        roles: Some(roles_check()),
        ..ReloadOptions::default()
    }
}

/// Applies the aviation scope to a store that follows the aviation definition.
async fn apply_aviation_scope(t: &support::TestStore, ontology: &Ontology) {
    let out = ontoforge_schema::load_scope_dir(camino::Utf8Path::new(AVIATION_SCOPE));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let scope = out.scope.expect("aviation scope");
    let check = roles_check();
    let compiled = ontoforge_auth::scope::compile_scope(&ontoforge_auth::scope::ScopeInputs {
        ontology,
        profile: None,
        attrs: &check.attrs,
        scope: &scope,
        sources: None,
    })
    .unwrap_or_else(|d| panic!("{d:#?}"));
    let props = compiled.row_filter_properties();
    let plan = t.store.scope_plan(&scope, &props).await.expect("plan");
    t.store
        .scope_apply(&scope, &props, &plan.hash, "tester@testhost")
        .await
        .expect("apply");
}

/// The aviation definition with a nullable `Flight.gate`, and `delayFlight` checking it in its
/// submission criteria. The `dispatcher` role grants `delayFlight` but not the column.
fn aviation_checking_gate() -> Ontology {
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
    let patch = |file: &str, from: &str, to: &str| {
        let path = dir.path().join(file);
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains(from), "{file} lost `{from}`");
        std::fs::write(&path, text.replace(from, to)).expect("write");
    };
    patch(
        "objects.yaml",
        "  delayMinutes: { type: integer, displayName: Delay in minutes }",
        "  delayMinutes: { type: integer, displayName: Delay in minutes }\n  gate: { type: string, displayName: Gate }",
    );
    patch(
        "actions.yaml",
        "objects.flight.status != 'CANCELLED'",
        "objects.flight.gate != 'X'",
    );
    let root = camino::Utf8Path::from_path(dir.path()).expect("utf-8 path");
    let out = ontoforge_schema::load_dir(root);
    out.ontology
        .unwrap_or_else(|| panic!("{:#?}", out.diagnostics))
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_reloads_single_flight() {
    for round in 0..5 {
        let t = sqlite_store().await;
        reload(&t.store, &base(), &ReloadOptions::default())
            .await
            .expect("v1");
        // Two processes: separate stores, separate in-memory definitions, one database file.
        let other = std::sync::Arc::new(t.reopen().await);
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
        let gate = t.columns("of_obj_gate").await;
        assert_eq!(
            gate.iter().filter(|c| *c == "zone").count(),
            1,
            "round {round}: the column was added once"
        );
        for store in [&t.store, &other] {
            assert_eq!(
                store.active_definition().await.map(|a| a.def_version),
                Some(2),
                "round {round}: both processes follow the new definition"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn reload_rechecks_applied_roles() {
    let t = sqlite_store().await;
    let aviation = aviation();
    reload(&t.store, &aviation, &with_roles())
        .await
        .expect("v1 with roles");
    apply_aviation_scope(&t, &aviation).await;
    let flight_before = t.columns("of_obj_flight").await;

    let result = reload(&t.store, &aviation_checking_gate(), &with_roles()).await;
    let Err(ReloadError::Definition(diagnostics)) = result else {
        panic!("expected a definition error, got {result:?}");
    };
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics.iter().all(|d| d.code == "ONT-V23"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("dispatcher") && d.message.contains("Flight")),
        "the diagnostic names the role and the type: {diagnostics:#?}"
    );
    assert_eq!(
        t.columns("of_obj_flight").await,
        flight_before,
        "no DDL ran"
    );
    assert_eq!(t.active_version().await, 1);
    assert_eq!(t.version_rows().await, 1);
    assert_eq!(
        t.store.active_definition().await.map(|a| a.def_version),
        Some(1)
    );

    // Without the role check the very same definition goes through: the roles stopped it.
    let out = reload(
        &t.store,
        &aviation_checking_gate(),
        &ReloadOptions::default(),
    )
    .await
    .expect("without the role check");
    assert_eq!(out.def_version, Some(2));
}

#[tokio::test(flavor = "multi_thread")]
async fn reload_creates_row_filter_indexes() {
    const INDEX: &str = "of_ix_of_obj_flight_departure_airport_code";
    let t = sqlite_store().await;
    let aviation = aviation();
    reload(&t.store, &aviation, &with_roles())
        .await
        .expect("v1 with roles");
    apply_aviation_scope(&t, &aviation).await;
    let names = |flight: Vec<(String, bool)>| -> Vec<String> {
        flight.into_iter().map(|(name, _)| name).collect()
    };
    assert!(names(index_list(&t, "of_obj_flight").await).contains(&INDEX.to_owned()));

    let pool = t.side_pool().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP INDEX {INDEX}")))
        .execute(&pool)
        .await
        .expect("drop the index");
    assert!(!names(index_list(&t, "of_obj_flight").await).contains(&INDEX.to_owned()));

    let out = reload(&t.store, &aviation, &with_roles())
        .await
        .expect("reload with unchanged definition");
    assert!(!out.changed, "an index is not a new definition version");
    assert_eq!(out.def_version, Some(1));
    assert_eq!(out.ddl.len(), 1, "{:#?}", out.ddl);
    assert!(out.ddl[0].contains(INDEX), "{}", out.ddl[0]);
    assert!(names(index_list(&t, "of_obj_flight").await).contains(&INDEX.to_owned()));
    assert_eq!(t.version_rows().await, 1);

    let again = reload(&t.store, &aviation, &with_roles())
        .await
        .expect("again");
    assert!(!again.changed);
    assert!(again.ddl.is_empty(), "{:#?}", again.ddl);
}

#[tokio::test(flavor = "multi_thread")]
async fn reload_racing_commits_never_lets_an_old_write_through() {
    for round in 0..3 {
        let t = sqlite_store().await;
        reload(&t.store, &base(), &ReloadOptions::default())
            .await
            .expect("v1");
        // Another process that writes while this one reloads.
        let writer = std::sync::Arc::new(t.reopen().await);
        let switched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let task = tokio::spawn(support::write_gates_across_a_switch(
            std::sync::Arc::clone(&writer),
            std::sync::Arc::clone(&switched),
            "G",
        ));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let out = reload(&t.store, &with_zone(), &ReloadOptions::default())
            .await
            .expect("v2");
        assert_eq!(out.def_version, Some(2), "round {round}");
        switched.store(true, std::sync::atomic::Ordering::SeqCst);
        let stats = task.await.expect("the writer finishes");

        assert!(stats.committed >= 10, "round {round}: {stats:?}");
        assert!(
            stats.stale >= 1,
            "round {round}: the writer's first commit after the switch is refused: {stats:?}"
        );
        let versions = support::assert_log_versions_never_go_back(&t.store).await;
        assert_eq!(versions.len(), stats.committed, "round {round}");
        assert!(
            versions.contains(&1) && versions.contains(&2),
            "round {round}: writes happened on both sides of the switch: {versions:?}"
        );
        assert_eq!(
            count(&t, "of_obj_gate").await,
            i64::try_from(stats.committed).expect("fits"),
            "round {round}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The Trino visibility probe and the two step switch (plan 03-18 task 3)
// ---------------------------------------------------------------------------------------------

/// A Trino that shows the edit tables as scripted: the `base` columns from the start, the `late`
/// ones from the `visible_from`-th `describe` on. It counts what it is asked.
struct ScriptedTrino {
    visible_from: usize,
    base: BTreeMap<String, Vec<String>>,
    late: BTreeMap<String, Vec<String>>,
    flushes: std::sync::Mutex<Vec<String>>,
    asked: std::sync::Mutex<Vec<Vec<String>>>,
}

impl ScriptedTrino {
    /// Shows `of_obj_gate` without `late` until the `visible_from`-th `describe`.
    fn gate(visible_from: usize, late: &[&str]) -> Self {
        let columns = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
        Self {
            visible_from,
            base: BTreeMap::from([
                (
                    "of_obj_gate".to_owned(),
                    columns(&["code", "name", "capacity"]),
                ),
                ("of_obj_stand".to_owned(), columns(&["id", "label"])),
            ]),
            late: BTreeMap::from([("of_obj_gate".to_owned(), columns(late))]),
            flushes: std::sync::Mutex::default(),
            asked: std::sync::Mutex::default(),
        }
    }

    fn describes(&self) -> usize {
        self.asked.lock().expect("lock").len()
    }

    fn flushes(&self) -> Vec<String> {
        self.flushes.lock().expect("lock").clone()
    }

    /// The tables of every `describe`, by name.
    fn asked(&self) -> Vec<Vec<String>> {
        self.asked.lock().expect("lock").clone()
    }
}

impl ProbeTarget for ScriptedTrino {
    fn flush_metadata<'a>(&'a self, catalog: &'a str) -> BoxFuture<'a, Result<(), EngineError>> {
        self.flushes.lock().expect("lock").push(catalog.to_owned());
        Box::pin(async { Ok(()) })
    }

    fn describe<'a>(
        &'a self,
        tables: &'a [TableRef],
    ) -> BoxFuture<'a, Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError>> {
        let call = {
            let mut asked = self.asked.lock().expect("lock");
            asked.push(tables.iter().map(|t| t.table.clone()).collect());
            asked.len()
        };
        let mut shown = BTreeMap::new();
        for table in tables {
            let Some(base) = self.base.get(&table.table) else {
                continue;
            };
            let mut names = base.clone();
            if call >= self.visible_from {
                names.extend(self.late.get(&table.table).cloned().unwrap_or_default());
            }
            shown.insert(
                table.clone(),
                names
                    .into_iter()
                    .map(|name| PhysicalColumn {
                        name,
                        physical: PhysicalType::parse("varchar"),
                        nullable: true,
                    })
                    .collect(),
            );
        }
        Box::pin(async move { Ok(shown) })
    }
}

fn probing(target: &ScriptedTrino, timeout_ms: u64) -> ReloadOptions<'_> {
    ReloadOptions {
        probe: Some(ProbeContext {
            target,
            catalog: "editstore".to_owned(),
            timeout: std::time::Duration::from_millis(timeout_ms),
        }),
        ..ReloadOptions::default()
    }
}

/// `(def_version, status)` of every definition version.
async fn version_status(t: &support::TestStore) -> Vec<(i64, String)> {
    let pool = t.side_pool().await;
    sqlx::query_as("SELECT def_version, status FROM of_def_version ORDER BY def_version")
        .fetch_all(&pool)
        .await
        .expect("versions")
}

fn statuses(pairs: &[(i64, &str)]) -> Vec<(i64, String)> {
    pairs.iter().map(|(v, s)| (*v, (*s).to_owned())).collect()
}

fn with_zone_and_level() -> Ontology {
    let properties =
        format!("{GATE_PROPERTIES}  zone: {{ type: string }}\n  level: {{ type: byte }}\n");
    definition("code", &properties, true)
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_waits_until_visible() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let trino = ScriptedTrino::gate(3, &["zone"]);

    let out = reload(&t.store, &with_zone(), &probing(&trino, 5_000))
        .await
        .expect("reload once the column is visible");
    assert!(out.changed);
    assert_eq!(out.previous_def_version, Some(1));
    assert_eq!(out.def_version, Some(2));
    assert_eq!(trino.describes(), 3, "polled until the third look");
    assert_eq!(
        trino.flushes(),
        ["editstore"],
        "flushed once, before looking"
    );
    assert!(
        trino
            .asked()
            .iter()
            .all(|tables| tables == &["of_obj_gate".to_owned()]),
        "only the table the reload changed is looked at: {:?}",
        trino.asked()
    );
    assert_eq!(
        version_status(&t).await,
        statuses(&[(1, "superseded"), (2, "active")])
    );
    assert_eq!(t.active_version().await, 2);
    assert_eq!(
        t.store.active_definition().await.map(|a| a.def_version),
        Some(2)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_timeout_keeps_pending() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let never = ScriptedTrino::gate(usize::MAX, &["zone"]);

    let result = reload(&t.store, &with_zone(), &probing(&never, 500)).await;
    let Err(ReloadError::NotVisible { missing }) = result else {
        panic!("expected NotVisible, got {result:?}");
    };
    assert_eq!(missing, ["of_obj_gate.zone"]);
    assert!(never.describes() >= 2, "it kept looking until the timeout");
    // The DDL is committed, the definition is not active.
    assert!(t.columns("of_obj_gate").await.contains(&"zone".to_owned()));
    assert_eq!(t.active_version().await, 1);
    assert_eq!(
        version_status(&t).await,
        statuses(&[(1, "active"), (2, "pending")])
    );
    assert_eq!(
        t.store.active_definition().await.map(|a| a.def_version),
        Some(1)
    );

    // Run again once Trino shows the column: the pending version is the one that becomes active.
    let visible = ScriptedTrino::gate(1, &["zone"]);
    let out = reload(&t.store, &with_zone(), &probing(&visible, 5_000))
        .await
        .expect("second attempt");
    assert_eq!(out.def_version, Some(2), "the pending version is reused");
    assert_eq!(
        version_status(&t).await,
        statuses(&[(1, "superseded"), (2, "active")])
    );

    // A different definition replaces a pending one instead of piling up behind it.
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let result = reload(&t.store, &with_zone(), &probing(&never, 300)).await;
    assert!(
        matches!(result, Err(ReloadError::NotVisible { .. })),
        "{result:?}"
    );
    let both = ScriptedTrino::gate(1, &["zone", "level"]);
    let out = reload(&t.store, &with_zone_and_level(), &probing(&both, 5_000))
        .await
        .expect("a different definition");
    assert_eq!(out.def_version, Some(3));
    assert_eq!(
        version_status(&t).await,
        statuses(&[(1, "superseded"), (2, "superseded"), (3, "active")]),
        "no second pending version"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn array_columns_are_not_probed() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    // Trino's PostgreSQL connector skips array columns unless it is told otherwise (RESEARCH
    // Pitfall 7): a Trino that never shows `tags` must not hold the reload back.
    let blind = ScriptedTrino::gate(usize::MAX, &["tags"]);

    let tags = format!("{GATE_PROPERTIES}  tags: {{ type: array, items: string }}\n");
    let out = reload(
        &t.store,
        &definition("code", &tags, true),
        &probing(&blind, 300),
    )
    .await
    .expect("an array column alone needs no probe");
    assert_eq!(out.def_version, Some(2));
    assert_eq!(blind.describes(), 0, "nothing to wait for, nobody asked");
    assert!(blind.flushes().is_empty());

    // Next to a column the connector does show, only that one is waited for.
    let both = format!("{tags}  zone: {{ type: string }}\n");
    let zone_only = ScriptedTrino::gate(2, &["zone"]);
    let out = reload(
        &t.store,
        &definition("code", &both, true),
        &probing(&zone_only, 5_000),
    )
    .await
    .expect("zone becomes visible, tags never does");
    assert_eq!(out.def_version, Some(3));
    assert_eq!(zone_only.describes(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn metadata_only_change_needs_no_probe() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let trino = ScriptedTrino::gate(usize::MAX, &[]);
    let described = "  code: { type: string, required: true }
  name: { type: string, displayName: Name }
  capacity: { type: integer }
";
    let out = reload(
        &t.store,
        &definition("code", described, true),
        &probing(&trino, 300),
    )
    .await
    .expect("metadata");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(2));
    assert_eq!(trino.describes(), 0);
    assert!(trino.flushes().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn migrate_waits_for_trino_after_the_switch() {
    // `capacity` goes away: migrate archives the column, which cannot be undone by a retry, so
    // the definition switches in one transaction and Trino is awaited afterwards.
    let removed = definition(
        "code",
        "  code: { type: string, required: true }\n  name: { type: string }\n",
        true,
    );
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let active = t.store.active_definition().await.expect("active");
    let plan = plan_migration(&active, &removed, &[]).expect("plan");

    let blind = ScriptedTrino::gate(usize::MAX, &["capacity__arch_v2"]);
    let probe = ProbeContext {
        target: &blind,
        catalog: "editstore".to_owned(),
        timeout: std::time::Duration::from_millis(300),
    };
    let result = apply_migration_probed(&t.store, &removed, &[], &plan.hash, Some(&probe)).await;
    let Err(MigrateError::NotVisible { missing }) = result else {
        panic!("expected NotVisible, got {result:?}");
    };
    assert_eq!(missing, ["of_obj_gate.capacity__arch_v2"]);
    assert_eq!(
        t.active_version().await,
        2,
        "the migration itself is applied"
    );

    // With a Trino that shows the archived column the call returns the new version.
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let seeing = ScriptedTrino::gate(2, &["capacity__arch_v2"]);
    let probe = ProbeContext {
        target: &seeing,
        catalog: "editstore".to_owned(),
        timeout: std::time::Duration::from_secs(5),
    };
    let version = apply_migration_probed(&t.store, &removed, &[], &plan.hash, Some(&probe))
        .await
        .expect("migrate");
    assert_eq!(version, 2);
    assert_eq!(seeing.describes(), 2);
    assert_eq!(seeing.flushes(), ["editstore"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_old_definition_keeps_serving_while_the_probe_waits() {
    let t = sqlite_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("v1");
    let writer = std::sync::Arc::new(t.reopen().await);
    let trino = std::sync::Arc::new(ScriptedTrino::gate(4, &["zone"]));

    let store = std::sync::Arc::clone(&t.store);
    let looking = std::sync::Arc::clone(&trino);
    let switching =
        tokio::spawn(async move { reload(&store, &with_zone(), &probing(&looking, 5_000)).await });
    // Wait until the DDL is committed and the probe is looking.
    while trino.describes() == 0 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(!switching.is_finished(), "the probe is still waiting");
    assert_eq!(t.active_version().await, 1, "the old definition is active");

    // Writes against definition 1 go through while the reload waits: no lock is held.
    for i in 0..3 {
        writer
            .commit(gate_commit(1, &format!("W{i}"), &[("name", "North")]))
            .await
            .expect("a write under the old definition during the wait");
    }
    assert!(!switching.is_finished());

    let out = switching
        .await
        .expect("join")
        .expect("the reload ends once Trino shows the column");
    assert_eq!(out.def_version, Some(2));
    let err = writer
        .commit(gate_commit(1, "W9", &[("name", "South")]))
        .await
        .expect_err("a write under the old definition after the switch");
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
    support::assert_log_versions_never_go_back(&t.store).await;
}
