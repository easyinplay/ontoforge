//! The edit store against a naive reference model (STORE-07, ROADMAP SC3).
//!
//! Random commit sequences run against `model::Model` and against a real SQLite edit store. After
//! every commit both agree on success or on the kind of failure, on the versions the commit
//! returns and on every edit row and link row. After the sequence the edit log is replayed into a
//! new store, which must equal the original column by column.
//!
//! The case count is 64; `PROPTEST_CASES` changes it. Failing inputs are saved next to this file.

mod model;
mod support;

use std::collections::BTreeMap;
use std::sync::LazyLock;

use model::{ErrorClass, Model, ModelOutcome, NormalizedRow};
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, EditError, LinkOp, ObjectOp};
use ontoforge_storage::log::replay;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::value::PropertyValue;
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;
use sqlx::Row;
use support::{TestStore, sqlite_store};

const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
  capacity: { type: integer }
---
kind: ObjectType
apiName: Stand
backing: source
primaryKey: id
properties:
  id: { type: integer, required: true }
  label: { type: string }
  gate: { type: string }
---
kind: LinkType
apiName: gateStands
cardinality: manyToMany
from: { objectType: Gate, apiName: stands }
to: { objectType: Stand, apiName: gates }
backing: editOnly
---
kind: LinkType
apiName: gateSlots
cardinality: manyToMany
from: { objectType: Gate, apiName: slots }
to: { objectType: Stand, apiName: slotGates }
backing: source
---
kind: LinkType
apiName: gateStand
cardinality: oneToOne
from: { objectType: Gate, apiName: stand }
to: { objectType: Stand, apiName: assignedGate, foreignKey: gate }
";

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime")
});

fn config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(64);
    ProptestConfig {
        cases,
        failure_persistence: Some(Box::new(FileFailurePersistence::WithSource(
            "proptest-regressions",
        ))),
        ..ProptestConfig::default()
    }
}

// ---------------------------------------------------------------------------------------------
// Edits
// ---------------------------------------------------------------------------------------------

/// One edit as the strategy draws it. Keys come from small spaces so that commits collide.
#[derive(Debug, Clone)]
enum Step {
    GateSet {
        code: u8,
        name: Option<u8>,
        capacity: Option<i64>,
    },
    GateUnset {
        code: u8,
        name: bool,
        capacity: bool,
    },
    GateTombstone(u8),
    GateRelease(u8),
    StandSet {
        id: i64,
        label: Option<u8>,
        gate: Option<u8>,
    },
    StandUnset {
        id: i64,
        label: bool,
        gate: bool,
    },
    StandTombstone(i64),
    StandRelease(i64),
    Link {
        editable: bool,
        code: u8,
        id: i64,
        add: bool,
    },
}

fn step() -> impl Strategy<Value = Step> {
    let code = 1u8..=3;
    let id = 1i64..=3;
    // A set names at least one property.
    let at_least_one = |first: Option<u8>, second: Option<u8>| match (first, second) {
        (None, None) => (Some(0), None),
        other => other,
    };
    prop_oneof![
        6 => (code.clone(), prop::option::of(0u8..2), prop::option::of(0i64..3))
            .prop_map(move |(code, name, capacity)| {
                let (name, capacity) = match (name, capacity) {
                    (None, None) => (Some(0), None),
                    other => other,
                };
                Step::GateSet { code, name, capacity }
            }),
        2 => (code.clone(), any::<bool>(), any::<bool>()).prop_map(|(code, name, capacity)| {
            Step::GateUnset { code, name: name || !capacity, capacity }
        }),
        1 => code.clone().prop_map(Step::GateTombstone),
        1 => code.clone().prop_map(Step::GateRelease),
        6 => (id.clone(), prop::option::of(0u8..2), prop::option::of(1u8..=2))
            .prop_map(move |(id, label, gate)| {
                let (label, gate) = at_least_one(label, gate);
                Step::StandSet { id, label, gate }
            }),
        3 => (id.clone(), any::<bool>(), any::<bool>()).prop_map(|(id, label, gate)| {
            Step::StandUnset { id, label: label || !gate, gate }
        }),
        2 => id.clone().prop_map(Step::StandTombstone),
        2 => id.clone().prop_map(Step::StandRelease),
        5 => (any::<bool>(), code, id, any::<bool>())
            .prop_map(|(editable, code, id, add)| Step::Link { editable, code, id, add }),
    ]
}

/// One to three edits per commit, one to thirty commits.
fn sequence() -> impl Strategy<Value = Vec<Vec<Step>>> {
    prop::collection::vec(prop::collection::vec(step(), 1..=3), 1..=30)
}

fn property(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property name")
}

fn gate_code(code: u8) -> PropertyValue {
    PropertyValue::String(format!("G{code}"))
}

fn text(prefix: &str, n: impl std::fmt::Display) -> PropertyValue {
    PropertyValue::String(format!("{prefix}{n}"))
}

fn object(type_name: &str, pk: PropertyValue, op: ObjectOp) -> Edit {
    Edit::Object {
        type_name: ObjectTypeName::try_from(type_name.to_owned()).expect("type name"),
        pk,
        op,
    }
}

fn names(flags: &[(&str, bool)]) -> ObjectOp {
    ObjectOp::Unset(
        flags
            .iter()
            .filter(|(_, wanted)| *wanted)
            .map(|(name, _)| property(name))
            .collect(),
    )
}

fn to_edit(step: &Step) -> Edit {
    match step {
        Step::GateSet {
            code,
            name,
            capacity,
        } => {
            let mut values = BTreeMap::new();
            if let Some(n) = name {
                values.insert(property("name"), text("n", n));
            }
            if let Some(c) = capacity {
                values.insert(property("capacity"), PropertyValue::Int(*c));
            }
            object("Gate", gate_code(*code), ObjectOp::Set(values))
        }
        Step::GateUnset {
            code,
            name,
            capacity,
        } => object(
            "Gate",
            gate_code(*code),
            names(&[("name", *name), ("capacity", *capacity)]),
        ),
        Step::GateTombstone(code) => object("Gate", gate_code(*code), ObjectOp::Tombstone),
        Step::GateRelease(code) => object("Gate", gate_code(*code), ObjectOp::Release),
        Step::StandSet { id, label, gate } => {
            let mut values = BTreeMap::new();
            if let Some(l) = label {
                values.insert(property("label"), text("l", l));
            }
            if let Some(g) = gate {
                values.insert(property("gate"), gate_code(*g));
            }
            object("Stand", PropertyValue::Int(*id), ObjectOp::Set(values))
        }
        Step::StandUnset { id, label, gate } => object(
            "Stand",
            PropertyValue::Int(*id),
            names(&[("label", *label), ("gate", *gate)]),
        ),
        Step::StandTombstone(id) => object("Stand", PropertyValue::Int(*id), ObjectOp::Tombstone),
        Step::StandRelease(id) => object("Stand", PropertyValue::Int(*id), ObjectOp::Release),
        Step::Link {
            editable,
            code,
            id,
            add,
        } => Edit::Link {
            link: LinkTypeName::try_from(
                if *editable { "gateStands" } else { "gateSlots" }.to_owned(),
            )
            .expect("link name"),
            a: gate_code(*code),
            b: PropertyValue::Int(*id),
            op: if *add { LinkOp::Add } else { LinkOp::Remove },
        },
    }
}

// ---------------------------------------------------------------------------------------------
// The store side
// ---------------------------------------------------------------------------------------------

fn classify(error: &EditError) -> ErrorClass {
    match error {
        EditError::NotFound => ErrorClass::NotFound,
        EditError::Tombstoned => ErrorClass::Tombstoned,
        EditError::InvalidTransition { .. } => ErrorClass::InvalidTransition,
        EditError::ReleaseOnEditOnly => ErrorClass::ReleaseOnEditOnly,
        EditError::MissingRequired { .. } => ErrorClass::MissingRequired,
        EditError::CardinalityViolation { link } => {
            assert_eq!(link, "gateStand");
            ErrorClass::CardinalityViolation
        }
        other => panic!("a valid commit failed for another reason: {other:?}"),
    }
}

/// Every edit row and link row with a version, read straight from the SQLite tables.
async fn store_rows(t: &TestStore) -> Vec<NormalizedRow> {
    let pool = t.side_pool().await;
    let mut all = Vec::new();

    let objects = [
        ("Gate", "of_obj_gate", "code", &["name", "capacity"][..]),
        ("Stand", "of_obj_stand", "id", &["label", "gate"][..]),
    ];
    for (table, sql_table, key, properties) in objects {
        let columns: Vec<String> = properties
            .iter()
            .flat_map(|p| [format!("CAST(\"{p}\" AS TEXT)"), format!("\"{p}__set\"")])
            .collect();
        let sql = format!(
            "SELECT CAST(\"{key}\" AS TEXT), of_version, of_deleted, of_released, {} \
             FROM \"{sql_table}\" WHERE of_version > 0",
            columns.join(", ")
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&pool)
            .await
            .expect("select edit rows");
        for row in rows {
            let mut props = BTreeMap::new();
            for (i, name) in properties.iter().enumerate() {
                let value: Option<String> = row.get(4 + 2 * i);
                let is_set: i64 = row.get(5 + 2 * i);
                match (is_set != 0, value) {
                    (true, Some(value)) => {
                        props.insert((*name).to_owned(), value);
                    }
                    (true, None) => panic!("{table}.{name} is flagged set but holds NULL"),
                    (false, Some(value)) => {
                        panic!("{table}.{name} is not flagged set but holds {value}")
                    }
                    (false, None) => {}
                }
            }
            all.push(NormalizedRow {
                table: table.to_owned(),
                key: row.get(0),
                props,
                deleted: row.get::<i64, _>(2) != 0,
                released: row.get::<i64, _>(3) != 0,
                version: row.get(1),
            });
        }
    }
    for (link, sql_table) in [
        ("gateStands", "of_lnk_gate_stands"),
        ("gateSlots", "of_lnk_gate_slots"),
    ] {
        let sql = format!(
            "SELECT CAST(a_pk AS TEXT), CAST(b_pk AS TEXT), of_version, of_deleted \
             FROM \"{sql_table}\" WHERE of_version > 0"
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&pool)
            .await
            .expect("select link rows");
        for row in rows {
            let (a, b): (String, String) = (row.get(0), row.get(1));
            all.push(NormalizedRow {
                table: link.to_owned(),
                key: format!("{a}|{b}"),
                props: BTreeMap::new(),
                deleted: row.get::<i64, _>(3) != 0,
                released: false,
                version: row.get(2),
            });
        }
    }
    all.sort();
    all
}

/// Every column of `table` as text, for the comparison of a store with its replay.
async fn dump(t: &TestStore, table: &str) -> Vec<Vec<Option<String>>> {
    let columns = t.columns(table).await;
    assert!(!columns.is_empty(), "{table} exists");
    let select: Vec<String> = columns
        .iter()
        .map(|c| format!("CAST(\"{c}\" AS TEXT)"))
        .collect();
    let sql = format!(
        "SELECT {} FROM \"{table}\" ORDER BY 1, 2",
        select.join(", ")
    );
    let pool = t.side_pool().await;
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(&pool)
        .await
        .expect("dump");
    rows.iter()
        .map(|row| {
            (0..columns.len())
                .map(|i| row.get::<Option<String>, _>(i))
                .collect()
        })
        .collect()
}

/// The invariants of the overlay that hold between two observations of the same row.
fn check_invariants(before: &[NormalizedRow], after: &[NormalizedRow]) {
    for row in after {
        assert!(
            !row.released || row.props.is_empty(),
            "a released row sets nothing: {row:?}"
        );
        assert!(
            !(row.deleted && row.released),
            "a row is not deleted and released at once: {row:?}"
        );
        let Some(old) = before
            .iter()
            .find(|old| old.table == row.table && old.key == row.key)
        else {
            continue;
        };
        assert!(
            row.version >= old.version,
            "a version went down: {old:?} -> {row:?}"
        );
        // A tombstone is left only through a release. One commit may release and then set, which
        // shows as a version that moved by two or more.
        let object = row.table == "Gate" || row.table == "Stand";
        if object && old.deleted && !row.deleted {
            assert!(
                (row.released && row.props.is_empty()) || row.version >= old.version + 2,
                "a tombstone is left only by a release: {old:?} -> {row:?}"
            );
        }
    }
}

async fn run(steps: Vec<Vec<Step>>) {
    let t = sqlite_store().await;
    let ontology = support::ontology_from(&[("defs.yaml", DEFS)]);
    let def_version = reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload")
        .def_version
        .expect("a version");

    let mut model = Model::new();
    let mut seen: Vec<NormalizedRow> = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let commit = Commit {
            actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
            action: format!("step{index}"),
            def_version,
            params: serde_json::Value::Null,
            edits: step.iter().map(to_edit).collect(),
        };
        let expected = model.apply(&commit);
        let actual = t.store.commit(commit.clone()).await;
        match (&expected, &actual) {
            (ModelOutcome::Ok(versions), Ok(receipt)) => {
                assert_eq!(versions, &receipt.versions, "versions of step {index}");
            }
            (ModelOutcome::Err(class), Err(error)) => {
                assert_eq!(
                    *class,
                    classify(error),
                    "failure of step {index}: {error:?}"
                );
            }
            _ => panic!("step {index}: the model says {expected:?}, the store says {actual:?}"),
        }
        let rows = store_rows(&t).await;
        assert_eq!(rows, model.rows(), "rows after step {index}: {commit:#?}");
        check_invariants(&seen, &rows);
        seen = rows;
    }

    // The log rebuilds the edit store.
    let records = t.store.read_log(0, 10_000).await.expect("read_log");
    let copy = sqlite_store().await;
    reload(&copy.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload the copy");
    replay(&records, &copy.store)
        .await
        .expect("replay the log into a new store");
    for table in [
        "of_obj_gate",
        "of_obj_stand",
        "of_lnk_gate_stands",
        "of_lnk_gate_slots",
    ] {
        assert_eq!(
            dump(&copy, table).await,
            dump(&t, table).await,
            "{table} rebuilt from the log"
        );
    }
}

proptest! {
    #![proptest_config(config())]

    /// The store and the reference model agree after every commit, and the log replays into an
    /// identical store.
    #[test]
    fn model_matches_store(steps in sequence()) {
        RUNTIME.block_on(run(steps));
    }
}
