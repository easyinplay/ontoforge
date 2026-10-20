//! Conformance cases of the edit store: one body, run on SQLite and on PostgreSQL (STORE-01).
//!
//! Every case is a `pub async fn name(&Harness)`. `all_cases!` lists them once and expands to one
//! `#[tokio::test]` per case for a backend, so a new case is added in two places of this file
//! only: its function and its line in the macro. `conformance_sqlite.rs` runs the cases, and
//! `conformance_pg.rs` runs the same cases `#[ignore]`d behind a PostgreSQL server.
#![allow(dead_code, reason = "each test target runs every case, but not every helper")]

use std::collections::BTreeMap;

use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use ontoforge_storage::commit::{
    Actor, Commit, Edit, EditError, LinkOp, ObjectOp, Receipt, SystemActor,
};
use ontoforge_storage::log::{EditOp, EntryKind};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::EditStore;
use ontoforge_storage::value::PropertyValue;
use serde_json::{Value, json};
use sqlx::Row;

use crate::support::{self, PgTestStore, TestStore, ontology_from};

/// The edit store of one case together with a way to look at its tables from outside.
pub enum Harness {
    Sqlite(TestStore),
    Pg(PgTestStore),
}

/// Which family of tables a raw query reads. PostgreSQL keeps them in different schemas.
#[derive(Clone, Copy)]
pub enum Tables {
    /// `of_obj_*` and `of_lnk_*`.
    Edit,
    /// `of_edit_log`, `of_edit_log_entry`, `of_edit_history`.
    Sys,
}

pub async fn sqlite() -> Harness {
    Harness::Sqlite(support::sqlite_store().await)
}

pub async fn pg() -> Harness {
    Harness::Pg(support::pg_store().await)
}

impl Harness {
    pub fn store(&self) -> &EditStore {
        match self {
            Self::Sqlite(t) => &t.store,
            Self::Pg(t) => &t.store,
        }
    }

    /// Loads the definition of the cases and returns its definition version.
    pub async fn load(&self) -> i64 {
        let ontology = ontology_from(&[("defs.yaml", DEFS)]);
        reload(self.store(), &ontology, &ReloadOptions::default())
            .await
            .expect("reload")
            .def_version
            .expect("a version")
    }

    /// `columns` of the rows of `table`, every value as text (`CAST(.. AS text)`), ordered by the
    /// first column. `filter` is `(column, text)`. A boolean reads `1` / `0` on SQLite and
    /// `true` / `false` on PostgreSQL: use [`flag`].
    pub async fn rows(
        &self,
        tables: Tables,
        table: &str,
        columns: &[&str],
        filter: Option<(&str, &str)>,
    ) -> Vec<Vec<Option<String>>> {
        let select: Vec<String> = columns
            .iter()
            .map(|c| format!("CAST(\"{c}\" AS text)"))
            .collect();
        let order = format!("ORDER BY \"{}\"", columns[0]);
        match self {
            Self::Sqlite(t) => {
                let mut sql = format!("SELECT {} FROM \"{table}\"", select.join(", "));
                if let Some((column, _)) = filter {
                    sql.push_str(&format!(" WHERE CAST(\"{column}\" AS text) = ?"));
                }
                sql.push_str(&format!(" {order}"));
                let pool = t.side_pool().await;
                let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
                if let Some((_, value)) = filter {
                    query = query.bind(value.to_owned());
                }
                let rows = query.fetch_all(&pool).await.expect("sqlite query");
                sqlite_cells(&rows, columns.len())
            }
            Self::Pg(t) => {
                let schema = match tables {
                    Tables::Edit => &t.schema_edit,
                    Tables::Sys => &t.schema_sys,
                };
                let mut sql = format!(
                    "SELECT {} FROM \"{schema}\".\"{table}\"",
                    select.join(", ")
                );
                if let Some((column, _)) = filter {
                    sql.push_str(&format!(" WHERE CAST(\"{column}\" AS text) = $1"));
                }
                sql.push_str(&format!(" {order}"));
                let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
                if let Some((_, value)) = filter {
                    query = query.bind(value.to_owned());
                }
                let rows = query.fetch_all(t.side()).await.expect("pg query");
                pg_cells(&rows, columns.len())
            }
        }
    }

    /// The one row of `table` whose `key_column` is `key`, `columns` as text.
    pub async fn row(
        &self,
        tables: Tables,
        table: &str,
        columns: &[&str],
        key_column: &str,
        key: &str,
    ) -> Vec<Option<String>> {
        let mut rows = self
            .rows(tables, table, columns, Some((key_column, key)))
            .await;
        assert_eq!(rows.len(), 1, "{table} where {key_column} = {key}");
        rows.remove(0)
    }
}

fn sqlite_cells(rows: &[sqlx::sqlite::SqliteRow], width: usize) -> Vec<Vec<Option<String>>> {
    rows.iter()
        .map(|row| (0..width).map(|i| row.get::<Option<String>, _>(i)).collect())
        .collect()
}

fn pg_cells(rows: &[sqlx::postgres::PgRow], width: usize) -> Vec<Vec<Option<String>>> {
    rows.iter()
        .map(|row| (0..width).map(|i| row.get::<Option<String>, _>(i)).collect())
        .collect()
}

/// A boolean cell as text.
pub fn flag(cell: &Option<String>) -> bool {
    match cell.as_deref() {
        Some("1" | "true") => true,
        Some("0" | "false") => false,
        other => panic!("not a boolean cell: {other:?}"),
    }
}

fn text(s: &str) -> Option<String> {
    Some(s.to_owned())
}

/// The definition of the cases: an editOnly `Gate` with a column of every kind of value, an
/// editOnly `Stand`, and the many to many link between them.
const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
  capacity: { type: integer }
  level: { type: byte }
  fee: { type: decimal, precision: 8, scale: 2 }
  openedAt: { type: timestamp }
  closedOn: { type: date }
  isOpen: { type: boolean }
  weight: { type: double }
  tags: { type: array, items: string }
  fees: { type: array, items: decimal, precision: 8, scale: 2 }
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

const ALICE: &str = "https://idp.example.com|alice";

fn property(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property name")
}

fn gate() -> ObjectTypeName {
    ObjectTypeName::try_from("Gate".to_owned()).expect("type name")
}

fn s(value: &str) -> PropertyValue {
    PropertyValue::String(value.to_owned())
}

fn set(pairs: &[(&str, PropertyValue)]) -> ObjectOp {
    ObjectOp::Set(
        pairs
            .iter()
            .map(|(name, value)| (property(name), value.clone()))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn gate_edit(code: &str, op: ObjectOp) -> Edit {
    Edit::Object {
        type_name: gate(),
        pk: s(code),
        op,
    }
}

fn commit_of(def_version: i64, action: &str, params: Value, edits: Vec<Edit>) -> Commit {
    Commit {
        actor: Actor::Principal(ALICE.to_owned()),
        action: action.to_owned(),
        def_version,
        params,
        edits,
    }
}

/// `Set {name, capacity}` on gate `code`.
fn create_gate(def_version: i64, code: &str) -> Commit {
    commit_of(
        def_version,
        "createGate",
        json!({ "code": code }),
        vec![gate_edit(
            code,
            set(&[("name", s("North")), ("capacity", PropertyValue::Int(120))]),
        )],
    )
}

async fn commit_ok(h: &Harness, commit: Commit) -> Receipt {
    h.store().commit(commit).await.expect("commit")
}

/// Nothing was written: no log record, no gate row.
async fn assert_nothing_written(h: &Harness) {
    let log = h.store().read_log(0, 100).await.expect("read_log");
    assert!(log.is_empty(), "log records: {log:#?}");
    let gates = h
        .rows(Tables::Edit, "of_obj_gate", &["code"], None)
        .await;
    assert!(gates.is_empty(), "gate rows: {gates:#?}");
    let history = h
        .rows(Tables::Sys, "of_edit_history", &["log_seq"], None)
        .await;
    assert!(history.is_empty(), "history rows: {history:#?}");
}

fn json_cell(cell: &Option<String>) -> Value {
    serde_json::from_str(cell.as_deref().expect("a json cell")).expect("json cell")
}

// ---------------------------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------------------------

pub async fn commit_set_creates_edit_row(h: &Harness) {
    let v = h.load().await;
    let receipt = commit_ok(h, create_gate(v, "A1")).await;
    assert_eq!(receipt.seq, 1);
    assert_eq!(receipt.versions, vec![1]);

    let row = h
        .row(
            Tables::Edit,
            "of_obj_gate",
            &[
                "code",
                "name",
                "name__set",
                "capacity",
                "capacity__set",
                "level__set",
                "of_version",
                "of_deleted",
                "of_released",
                "of_def_version",
                "of_log_seq",
            ],
            "code",
            "A1",
        )
        .await;
    assert_eq!(row[0], text("A1"));
    assert_eq!(row[1], text("North"));
    assert!(flag(&row[2]));
    assert_eq!(row[3], text("120"));
    assert!(flag(&row[4]));
    assert!(!flag(&row[5]), "a property nobody set stays unset");
    assert_eq!(row[6], text("1"));
    assert!(!flag(&row[7]));
    assert!(!flag(&row[8]));
    assert_eq!(row[9], text("1"));
    assert_eq!(row[10], text("1"));
}

pub async fn commit_appends_one_log_record(h: &Harness) {
    let v = h.load().await;
    let receipt = commit_ok(h, create_gate(v, "A1")).await;

    let log = h.store().read_log(0, 10).await.expect("read_log");
    assert_eq!(log.len(), 1);
    let record = &log[0];
    assert_eq!(record.seq, 1);
    assert_eq!(record.actor, ALICE);
    assert_eq!(record.action, "createGate");
    assert_eq!(record.def_version, 1);
    assert_eq!(record.params, json!({ "code": "A1" }));
    assert_eq!(record.event_id.len(), 26);
    assert_eq!(record.event_id, receipt.event_id.to_string());
    let at = &record.committed_at;
    assert_eq!(at.len(), 24, "committed_at {at}");
    assert!(at.ends_with('Z') && at.as_bytes()[10] == b'T', "{at}");

    assert_eq!(record.entries.len(), 1);
    let entry = &record.entries[0];
    assert_eq!(entry.ordinal, 0);
    assert_eq!(entry.kind, EntryKind::Object);
    assert_eq!(entry.type_name, "Gate");
    assert_eq!(entry.pk, json!("A1"));
    assert_eq!(entry.op, EditOp::Set);
    assert_eq!(
        entry.payload,
        Some(json!({ "capacity": 120, "name": "North" }))
    );
    assert_eq!(entry.version_after, 1);
}

pub async fn commit_writes_history_snapshot(h: &Harness) {
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;

    let rows = h
        .rows(
            Tables::Sys,
            "of_edit_history",
            &[
                "log_seq",
                "ordinal",
                "kind",
                "type_name",
                "pk",
                "version",
                "snapshot",
            ],
            None,
        )
        .await;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row[0], text("1"));
    assert_eq!(row[1], text("0"));
    assert_eq!(row[2], text("object"));
    assert_eq!(row[3], text("Gate"));
    assert_eq!(json_cell(&row[4]), json!("A1"));
    assert_eq!(row[5], text("1"));
    assert_eq!(
        json_cell(&row[6]),
        json!({
            "deleted": false,
            "released": false,
            "set": { "capacity": 120, "name": "North" }
        })
    );
}

pub async fn second_set_patches_only_given_properties(h: &Harness) {
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;
    let receipt = commit_ok(
        h,
        commit_of(
            v,
            "resizeGate",
            json!({ "code": "A1", "capacity": 90 }),
            vec![gate_edit(
                "A1",
                set(&[("capacity", PropertyValue::Int(90))]),
            )],
        ),
    )
    .await;
    assert_eq!(receipt.seq, 2);
    assert_eq!(receipt.versions, vec![2]);

    let row = h
        .row(
            Tables::Edit,
            "of_obj_gate",
            &["code", "name", "capacity", "of_version", "of_log_seq"],
            "code",
            "A1",
        )
        .await;
    assert_eq!(row[1], text("North"), "name keeps the first value");
    assert_eq!(row[2], text("90"));
    assert_eq!(row[3], text("2"));
    assert_eq!(row[4], text("2"));

    let log = h.store().read_log(1, 10).await.expect("read_log");
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].entries[0].payload, Some(json!({ "capacity": 90 })));
    assert_eq!(log[0].entries[0].version_after, 2);

    let snapshot = h
        .row(
            Tables::Sys,
            "of_edit_history",
            &["log_seq", "snapshot"],
            "log_seq",
            "2",
        )
        .await;
    assert_eq!(
        json_cell(&snapshot[1]),
        json!({
            "deleted": false,
            "released": false,
            "set": { "capacity": 90, "name": "North" }
        })
    );
}

pub async fn invalid_values_are_rejected_without_writes(h: &Harness) {
    let v = h.load().await;
    let gate_commit = |edits: Vec<Edit>| commit_of(v, "createGate", json!({}), edits);

    let wrong_type = h
        .store()
        .commit(gate_commit(vec![gate_edit(
            "A1",
            set(&[("name", s("N")), ("capacity", s("x"))]),
        )]))
        .await
        .expect_err("a string for an integer");
    assert!(
        matches!(&wrong_type, EditError::Value { property, .. } if property == "capacity"),
        "{wrong_type:?}"
    );

    let unknown = h
        .store()
        .commit(gate_commit(vec![gate_edit(
            "A1",
            set(&[("name", s("N")), ("colour", s("red"))]),
        )]))
        .await
        .expect_err("unknown property");
    assert!(
        matches!(&unknown, EditError::UnknownProperty { property, .. } if property == "colour"),
        "{unknown:?}"
    );

    let key = h
        .store()
        .commit(gate_commit(vec![gate_edit(
            "A1",
            set(&[("name", s("N")), ("code", s("B2"))]),
        )]))
        .await
        .expect_err("pk in a set");
    assert!(matches!(key, EditError::PrimaryKeyNotSettable), "{key:?}");

    let missing = h
        .store()
        .commit(gate_commit(vec![gate_edit(
            "A1",
            set(&[("capacity", PropertyValue::Int(5))]),
        )]))
        .await
        .expect_err("a new editOnly object without its required name");
    assert!(
        matches!(&missing, EditError::MissingRequired { property } if property == "name"),
        "{missing:?}"
    );

    let byte = h
        .store()
        .commit(gate_commit(vec![gate_edit(
            "A1",
            set(&[("name", s("N")), ("level", PropertyValue::Int(200))]),
        )]))
        .await
        .expect_err("a byte out of range");
    assert!(
        matches!(&byte, EditError::Value { property, .. } if property == "level"),
        "{byte:?}"
    );

    let decimal = h
        .store()
        .commit(gate_commit(vec![gate_edit(
            "A1",
            set(&[
                ("name", s("N")),
                ("fee", PropertyValue::Decimal("1.234".to_owned())),
            ]),
        )]))
        .await
        .expect_err("a decimal with too many fractional digits");
    assert!(
        matches!(&decimal, EditError::Value { property, .. } if property == "fee"),
        "{decimal:?}"
    );

    let empty = h
        .store()
        .commit(gate_commit(Vec::new()))
        .await
        .expect_err("no edits");
    assert!(matches!(empty, EditError::Value { .. }), "{empty:?}");

    let unknown_type = h
        .store()
        .commit(gate_commit(vec![Edit::Object {
            type_name: ObjectTypeName::try_from("Hangar".to_owned()).expect("name"),
            pk: s("H1"),
            op: set(&[("name", s("N"))]),
        }]))
        .await
        .expect_err("unknown type");
    assert!(
        matches!(&unknown_type, EditError::UnknownType(name) if name == "Hangar"),
        "{unknown_type:?}"
    );

    let mut anonymous = create_gate(v, "A1");
    anonymous.actor = Actor::Principal("alice".to_owned());
    let anonymous = h
        .store()
        .commit(anonymous)
        .await
        .expect_err("a principal id without `iss|sub`");
    assert!(
        matches!(&anonymous, EditError::Value { property, .. } if property == "actor"),
        "{anonymous:?}"
    );

    assert_nothing_written(h).await;
}

pub async fn stale_def_version_rejected(h: &Harness) {
    let v = h.load().await;
    assert_eq!(v, 1);
    let err = h
        .store()
        .commit(create_gate(2, "A1"))
        .await
        .expect_err("a commit for a definition that is not active");
    assert!(
        matches!(
            err,
            EditError::DefinitionStale {
                expected: 2,
                active: 1
            }
        ),
        "{err:?}"
    );
    assert_nothing_written(h).await;
}

pub async fn two_edits_of_one_object_bump_each_version(h: &Harness) {
    let v = h.load().await;
    let receipt = commit_ok(
        h,
        commit_of(
            v,
            "createAndResize",
            json!(null),
            vec![
                gate_edit("A1", set(&[("name", s("North"))])),
                gate_edit("A1", set(&[("capacity", PropertyValue::Int(7))])),
            ],
        ),
    )
    .await;
    assert_eq!(receipt.versions, vec![1, 2]);

    let log = h.store().read_log(0, 10).await.expect("read_log");
    assert_eq!(log.len(), 1, "one commit, one record");
    assert_eq!(log[0].params, Value::Null);
    let entries = &log[0].entries;
    assert_eq!(entries.len(), 2);
    assert_eq!((entries[0].ordinal, entries[0].version_after), (0, 1));
    assert_eq!((entries[1].ordinal, entries[1].version_after), (1, 2));

    let row = h
        .row(
            Tables::Edit,
            "of_obj_gate",
            &["code", "name", "capacity", "of_version"],
            "code",
            "A1",
        )
        .await;
    assert_eq!(row[1], text("North"));
    assert_eq!(row[2], text("7"));
    assert_eq!(row[3], text("2"));

    let history = h
        .rows(
            Tables::Sys,
            "of_edit_history",
            &["ordinal", "version", "snapshot"],
            None,
        )
        .await;
    assert_eq!(history.len(), 2);
    assert_eq!(
        json_cell(&history[0][2]),
        json!({ "deleted": false, "released": false, "set": { "name": "North" } })
    );
    assert_eq!(
        json_cell(&history[1][2]),
        json!({
            "deleted": false,
            "released": false,
            "set": { "capacity": 7, "name": "North" }
        })
    );
}

pub async fn every_value_type_round_trips(h: &Harness) {
    let v = h.load().await;
    let all = set(&[
        ("name", s("North")),
        ("capacity", PropertyValue::Int(120)),
        ("level", PropertyValue::Int(100)),
        ("fee", PropertyValue::Decimal("12.5".to_owned())),
        (
            "openedAt",
            PropertyValue::Timestamp("2026-03-29T02:30:00+02:00".to_owned()),
        ),
        ("closedOn", PropertyValue::Date("2026-03-29".to_owned())),
        ("isOpen", PropertyValue::Bool(true)),
        ("weight", PropertyValue::Double(1.5)),
        (
            "tags",
            PropertyValue::Array(vec![s("a"), s("b, \"c\"")]),
        ),
        (
            "fees",
            PropertyValue::Array(vec![
                PropertyValue::Decimal("1".to_owned()),
                PropertyValue::Decimal("2.5".to_owned()),
            ]),
        ),
        (
            "location",
            PropertyValue::Geopoint {
                lat: 52.5,
                lon: 13.4,
            },
        ),
    ]);
    commit_ok(
        h,
        commit_of(v, "createGate", json!({}), vec![gate_edit("A1", all)]),
    )
    .await;

    let canonical = json!({
        "name": "North",
        "capacity": 120,
        "level": 100,
        "fee": "12.50",
        "openedAt": "2026-03-29T00:30:00.000Z",
        "closedOn": "2026-03-29",
        "isOpen": true,
        "weight": 1.5,
        "tags": ["a", "b, \"c\""],
        "fees": ["1.00", "2.50"],
        "location": { "lat": 52.5, "lon": 13.4 }
    });
    let log = h.store().read_log(0, 10).await.expect("read_log");
    assert_eq!(log[0].entries[0].payload, Some(canonical.clone()));

    let row = h
        .row(
            Tables::Edit,
            "of_obj_gate",
            &["code", "fee", "closed_on", "is_open", "level"],
            "code",
            "A1",
        )
        .await;
    assert_eq!(row[1], text("12.50"));
    assert_eq!(row[2], text("2026-03-29"));
    assert!(flag(&row[3]));
    assert_eq!(row[4], text("100"));

    // The second commit reads every stored value back to build its snapshot: what the database
    // returns must equal what the first commit logged.
    commit_ok(
        h,
        commit_of(
            v,
            "renameGate",
            json!({}),
            vec![gate_edit("A1", set(&[("name", s("South"))]))],
        ),
    )
    .await;
    let snapshot = h
        .row(
            Tables::Sys,
            "of_edit_history",
            &["log_seq", "snapshot"],
            "log_seq",
            "2",
        )
        .await;
    let mut expected = canonical;
    expected["name"] = json!("South");
    assert_eq!(
        json_cell(&snapshot[1]),
        json!({ "deleted": false, "released": false, "set": expected })
    );
}

pub async fn import_actor_is_stored_as_system_import(h: &Harness) {
    let v = h.load().await;
    let mut commit = create_gate(v, "A1");
    commit.actor = Actor::System(SystemActor::Import);
    commit.action = "system:import".to_owned();
    commit_ok(h, commit).await;
    let log = h.store().read_log(0, 10).await.expect("read_log");
    assert_eq!(log[0].actor, "system:import");
    assert_eq!(log[0].action, "system:import");
}

pub async fn read_log_pages_by_seq(h: &Harness) {
    let v = h.load().await;
    for code in ["A1", "A2", "A3"] {
        commit_ok(h, create_gate(v, code)).await;
    }
    let first = h.store().read_log(0, 2).await.expect("first page");
    assert_eq!(first.iter().map(|r| r.seq).collect::<Vec<_>>(), vec![1, 2]);
    assert!(first.iter().all(|r| r.entries.len() == 1));
    let rest = h.store().read_log(2, 10).await.expect("second page");
    assert_eq!(rest.iter().map(|r| r.seq).collect::<Vec<_>>(), vec![3]);
    assert_eq!(rest[0].entries[0].pk, json!("A3"));
    let none = h.store().read_log(3, 10).await.expect("empty page");
    assert!(none.is_empty());
}

pub async fn link_add_creates_link_row(h: &Harness) {
    let v = h.load().await;
    let receipt = commit_ok(
        h,
        commit_of(
            v,
            "assignStand",
            json!({ "gate": "A1", "stand": 7 }),
            vec![Edit::Link {
                link: LinkTypeName::try_from("gateStands".to_owned()).expect("link name"),
                a: s("A1"),
                b: PropertyValue::Int(7),
                op: LinkOp::Add,
            }],
        ),
    )
    .await;
    assert_eq!(receipt.versions, vec![1]);

    let row = h
        .row(
            Tables::Edit,
            "of_lnk_gate_stands",
            &["a_pk", "b_pk", "of_version", "of_deleted", "of_log_seq"],
            "a_pk",
            "A1",
        )
        .await;
    assert_eq!(row[1], text("7"));
    assert_eq!(row[2], text("1"));
    assert!(!flag(&row[3]));
    assert_eq!(row[4], text("1"));

    let log = h.store().read_log(0, 10).await.expect("read_log");
    let entry = &log[0].entries[0];
    assert_eq!(entry.kind, EntryKind::Link);
    assert_eq!(entry.type_name, "gateStands");
    assert_eq!(entry.pk, json!(["A1", 7]));
    assert_eq!(entry.op, EditOp::LinkAdd);
    assert_eq!(entry.payload, None);
    assert_eq!(entry.version_after, 1);

    let snapshot = h
        .row(
            Tables::Sys,
            "of_edit_history",
            &["log_seq", "snapshot"],
            "log_seq",
            "1",
        )
        .await;
    assert_eq!(json_cell(&snapshot[1]), json!({ "deleted": false }));
}

// ---------------------------------------------------------------------------------------------
// Expansion
// ---------------------------------------------------------------------------------------------

macro_rules! conformance_case {
    ($factory:path, $name:ident $(, $attr:meta)?) => {
        #[tokio::test(flavor = "multi_thread")]
        $(#[$attr])?
        async fn $name() {
            let harness = $factory().await;
            $crate::conformance::$name(&harness).await;
        }
    };
}

/// Expands to one `#[tokio::test]` per case for the harness `$factory` builds. The optional
/// second argument is an attribute for every test (`ignore = ".."` for PostgreSQL).
macro_rules! all_cases {
    ($factory:path $(, $attr:meta)?) => {
        $crate::conformance::conformance_case!($factory, commit_set_creates_edit_row $(, $attr)?);
        $crate::conformance::conformance_case!($factory, commit_appends_one_log_record $(, $attr)?);
        $crate::conformance::conformance_case!($factory, commit_writes_history_snapshot $(, $attr)?);
        $crate::conformance::conformance_case!($factory, second_set_patches_only_given_properties $(, $attr)?);
        $crate::conformance::conformance_case!($factory, invalid_values_are_rejected_without_writes $(, $attr)?);
        $crate::conformance::conformance_case!($factory, stale_def_version_rejected $(, $attr)?);
        $crate::conformance::conformance_case!($factory, two_edits_of_one_object_bump_each_version $(, $attr)?);
        $crate::conformance::conformance_case!($factory, every_value_type_round_trips $(, $attr)?);
        $crate::conformance::conformance_case!($factory, import_actor_is_stored_as_system_import $(, $attr)?);
        $crate::conformance::conformance_case!($factory, read_log_pages_by_seq $(, $attr)?);
        $crate::conformance::conformance_case!($factory, link_add_creates_link_row $(, $attr)?);
    };
}

pub(crate) use all_cases;
pub(crate) use conformance_case;
