//! `reload` on the SQLite edit store: tables from a definition, idempotency, additive and
//! destructive changes, dry run and recovery from an interrupted run (STORE-02).

mod support;

use std::collections::BTreeSet;

use ontoforge_schema::ir::{ObjectTypeName, Ontology, PropertyName};
use ontoforge_storage::reload::{ReloadError, ReloadOptions, reload};
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
    let out = reload(&t.store, &base(), &ReloadOptions { dry_run: true })
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
