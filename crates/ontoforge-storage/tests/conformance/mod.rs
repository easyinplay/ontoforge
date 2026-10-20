//! Conformance cases of the edit store: one body, run on SQLite and on PostgreSQL (STORE-01).
//!
//! Every case is a `pub async fn name(&Harness)`. `all_cases!` lists them once and expands to one
//! `#[tokio::test]` per case for a backend, so a new case is added in two places of this file
//! only: its function and its line in the macro. `conformance_sqlite.rs` runs the cases, and
//! `conformance_pg.rs` runs the same cases `#[ignore]`d behind a PostgreSQL server.
#![allow(
    dead_code,
    reason = "each test target runs every case, but not every helper"
)]

use std::collections::BTreeMap;

use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use ontoforge_storage::commit::{
    Actor, Commit, Edit, EditError, LinkOp, ObjectOp, Receipt, SystemActor,
};
use ontoforge_storage::log::{EditOp, EntryKind, ReplayError, replay};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{Backend, EditStore};
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
        let select = select.join(", ");
        match self {
            Self::Sqlite(t) => {
                let filter_sql = filter
                    .map(|(column, _)| format!(" WHERE CAST(\"{column}\" AS text) = ?"))
                    .unwrap_or_default();
                let sql = format!("SELECT {select} FROM \"{table}\"{filter_sql} {order}");
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
                let filter_sql = filter
                    .map(|(column, _)| format!(" WHERE CAST(\"{column}\" AS text) = $1"))
                    .unwrap_or_default();
                let sql =
                    format!("SELECT {select} FROM \"{schema}\".\"{table}\"{filter_sql} {order}");
                let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
                if let Some((_, value)) = filter {
                    query = query.bind(value.to_owned());
                }
                let rows = query.fetch_all(t.side()).await.expect("pg query");
                pg_cells(&rows, columns.len())
            }
        }
    }

    /// A second, empty store of the same kind, with the definition of the cases loaded. Replay
    /// rebuilds one store from the log of another.
    pub async fn sibling(&self) -> Harness {
        let other = match self {
            Self::Sqlite(_) => sqlite().await,
            Self::Pg(_) => pg().await,
        };
        other.load().await;
        other
    }

    /// `table` as a statement names it: `"table"` on SQLite, `"schema_sys"."table"` on PostgreSQL.
    /// Only the system tables (log, entry, history) are addressed this way.
    pub fn sys_table(&self, table: &str) -> String {
        match self {
            Self::Sqlite(_) => format!("\"{table}\""),
            Self::Pg(t) => format!("\"{}\".\"{table}\"", t.schema_sys),
        }
    }

    /// Runs `sql` from outside the store, on a connection that has the store's connection
    /// options (SQLite: `recursive_triggers` on). `Ok` is the number of affected rows, `Err` the
    /// refusal of the database.
    pub async fn try_exec(&self, sql: &str) -> Result<u64, Refusal> {
        match self {
            Self::Sqlite(t) => {
                let pool = t.side_pool().await;
                sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
                    .execute(&pool)
                    .await
                    .map(|done| done.rows_affected())
                    .map_err(Refusal::from)
            }
            Self::Pg(t) => sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
                .execute(t.side())
                .await
                .map(|done| done.rows_affected())
                .map_err(Refusal::from),
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

/// A statement the database refused.
#[derive(Debug)]
pub struct Refusal {
    /// The SQLSTATE of PostgreSQL, when the database reported one.
    pub code: Option<String>,
    pub message: String,
}

impl From<sqlx::Error> for Refusal {
    fn from(error: sqlx::Error) -> Self {
        let code = error
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .map(std::borrow::Cow::into_owned);
        Self {
            code,
            message: error.to_string(),
        }
    }
}

fn sqlite_cells(rows: &[sqlx::sqlite::SqliteRow], width: usize) -> Vec<Vec<Option<String>>> {
    rows.iter()
        .map(|row| {
            (0..width)
                .map(|i| row.get::<Option<String>, _>(i))
                .collect()
        })
        .collect()
}

fn pg_cells(rows: &[sqlx::postgres::PgRow], width: usize) -> Vec<Vec<Option<String>>> {
    rows.iter()
        .map(|row| {
            (0..width)
                .map(|i| row.get::<Option<String>, _>(i))
                .collect()
        })
        .collect()
}

/// A boolean cell as text.
#[allow(
    clippy::ref_option,
    reason = "cells are indexed out of a row of options"
)]
pub fn flag(cell: &Option<String>) -> bool {
    match cell.as_deref() {
        Some("1" | "true") => true,
        Some("0" | "false") => false,
        other => panic!("not a boolean cell: {other:?}"),
    }
}

#[allow(clippy::unnecessary_wraps, reason = "compared with a nullable cell")]
fn text(s: &str) -> Option<String> {
    Some(s.to_owned())
}

/// The definition of the cases: an editOnly `Gate` with a column of every kind of value, a source
/// backed `Stand`, two many to many links between them (one editOnly, one source backed) and the
/// one to one link `gateStand`, whose foreign key is `Stand.gate`.
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

/// A `Set` that gives every property of `Gate` but its key a value.
fn all_values() -> ObjectOp {
    set(&[
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
        ("tags", PropertyValue::Array(vec![s("a"), s("b, \"c\"")])),
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
    ])
}

async fn commit_ok(h: &Harness, commit: Commit) -> Receipt {
    h.store().commit(commit).await.expect("commit")
}

/// Nothing was written: no log record, no gate row.
async fn assert_nothing_written(h: &Harness) {
    let log = h.store().read_log(0, 100).await.expect("read_log");
    assert!(log.is_empty(), "log records: {log:#?}");
    let gates = h.rows(Tables::Edit, "of_obj_gate", &["code"], None).await;
    assert!(gates.is_empty(), "gate rows: {gates:#?}");
    let history = h
        .rows(Tables::Sys, "of_edit_history", &["log_seq"], None)
        .await;
    assert!(history.is_empty(), "history rows: {history:#?}");
}

#[allow(
    clippy::ref_option,
    reason = "cells are indexed out of a row of options"
)]
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

#[allow(clippy::too_many_lines, reason = "one case lists every rejection")]
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
    let all = all_values();
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
    "system:import".clone_into(&mut commit.action);
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
// Append only (STORE-03, D-12)
// ---------------------------------------------------------------------------------------------

/// The three append only tables with the columns the cases compare and one statement each that
/// rewrites a column.
const LOG_TABLES: [(&str, &[&str], &str); 3] = [
    (
        "of_edit_log",
        &[
            "seq",
            "event_id",
            "actor",
            "action",
            "committed_at",
            "def_version",
            "params",
        ],
        "SET actor = 'mallory'",
    ),
    (
        "of_edit_log_entry",
        &[
            "log_seq",
            "ordinal",
            "kind",
            "type_name",
            "pk",
            "op",
            "payload",
            "version_after",
        ],
        "SET version_after = 99",
    ),
    (
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
        "SET version = 99",
    ),
];

type LogRows = Vec<Vec<Option<String>>>;

/// Every row of the three tables, to compare before and after a refused statement.
async fn log_rows(h: &Harness) -> Vec<LogRows> {
    let mut all = Vec::new();
    for (table, columns, _) in LOG_TABLES {
        all.push(h.rows(Tables::Sys, table, columns, None).await);
    }
    all
}

/// The statement was refused by the append only trigger, not by something else.
fn assert_refused_as_append_only(refusal: &Refusal, what: &str) {
    assert!(
        refusal.message.contains("append-only"),
        "{what}: refused for another reason: {refusal:?}"
    );
}

pub async fn append_only_rejects_update_and_delete(h: &Harness) {
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;
    let before = log_rows(h).await;
    assert!(
        before.iter().all(|rows| rows.len() == 1),
        "one commit, one row per table: {before:#?}"
    );

    for (table, _, assignment) in LOG_TABLES {
        let target = h.sys_table(table);
        let update = h
            .try_exec(&format!("UPDATE {target} {assignment}"))
            .await
            .expect_err(&format!("UPDATE of {table} must be refused"));
        assert_refused_as_append_only(&update, &format!("UPDATE {table}"));
        let delete = h
            .try_exec(&format!("DELETE FROM {target}"))
            .await
            .expect_err(&format!("DELETE from {table} must be refused"));
        assert_refused_as_append_only(&delete, &format!("DELETE {table}"));
        if matches!(h, Harness::Pg(_)) {
            assert_eq!(
                update.code.as_deref(),
                Some("23000"),
                "of_reject_mutation raises SQLSTATE 23000"
            );
        }
    }
    assert_eq!(
        log_rows(h).await,
        before,
        "a refused statement changes nothing"
    );
}

pub async fn append_only_rejects_truncate_on_pg(h: &Harness) {
    if matches!(h, Harness::Sqlite(_)) {
        // SQLite has no TRUNCATE; its `DELETE FROM` without a WHERE is the case above.
        assert_eq!(h.store().backend(), Backend::Sqlite);
        return;
    }
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;
    let before = log_rows(h).await;

    // `of_edit_log` is referenced by the entry table, so PostgreSQL checks the foreign key before
    // it fires the trigger when it is truncated alone. The entry and history tables are
    // truncated alone, and all three together, so the statement reaches the trigger.
    for table in ["of_edit_log_entry", "of_edit_history"] {
        let refusal = h
            .try_exec(&format!("TRUNCATE {}", h.sys_table(table)))
            .await
            .expect_err(&format!("TRUNCATE of {table} must be refused"));
        assert_refused_as_append_only(&refusal, &format!("TRUNCATE {table}"));
        assert_eq!(refusal.code.as_deref(), Some("23000"));
    }
    let all = LOG_TABLES
        .iter()
        .map(|(table, _, _)| h.sys_table(table))
        .collect::<Vec<_>>()
        .join(", ");
    let refusal = h
        .try_exec(&format!("TRUNCATE {all}"))
        .await
        .expect_err("TRUNCATE of the three tables must be refused");
    assert_refused_as_append_only(&refusal, "TRUNCATE of all three");
    assert_eq!(refusal.code.as_deref(), Some("23000"));
    assert_eq!(
        log_rows(h).await,
        before,
        "a refused statement changes nothing"
    );
}

pub async fn append_only_rejects_sqlite_replace_writes(h: &Harness) {
    if matches!(h, Harness::Pg(_)) {
        // `INSERT OR REPLACE` is SQLite syntax.
        assert_eq!(h.store().backend(), Backend::Postgres);
        return;
    }
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;
    let before = log_rows(h).await;

    // Each statement re-inserts the key of the existing row. SQLite resolves the conflict by
    // deleting the old row first; that DELETE only reaches the trigger with recursive_triggers.
    let rows = [
        (
            "of_edit_log",
            "(seq, event_id, actor, action, committed_at, def_version, params) \
             VALUES (1, 'forged', 'mallory', 'x', '2026-01-01T00:00:00.000Z', 1, '{}')",
        ),
        (
            "of_edit_log_entry",
            "(log_seq, ordinal, kind, type_name, pk, op, payload, version_after) \
             VALUES (1, 0, 'object', 'Gate', '\"A1\"', 'set', NULL, 99)",
        ),
        (
            "of_edit_history",
            "(log_seq, ordinal, kind, type_name, pk, version, snapshot) \
             VALUES (1, 0, 'object', 'Gate', '\"A1\"', 99, '{}')",
        ),
    ];
    for (table, values) in rows {
        for verb in ["INSERT OR REPLACE INTO", "REPLACE INTO"] {
            let refusal = h
                .try_exec(&format!("{verb} {} {values}", h.sys_table(table)))
                .await
                .expect_err(&format!("{verb} {table} must be refused"));
            assert_refused_as_append_only(&refusal, &format!("{verb} {table}"));
        }
    }
    let upsert = h
        .try_exec(
            "INSERT INTO \"of_edit_log\" (seq, event_id, actor, action, committed_at, def_version, params) \
             VALUES (1, 'forged', 'mallory', 'x', '2026-01-01T00:00:00.000Z', 1, '{}') \
             ON CONFLICT (seq) DO UPDATE SET actor = excluded.actor",
        )
        .await
        .expect_err("an upsert is an UPDATE and must be refused");
    assert_refused_as_append_only(&upsert, "upsert of_edit_log");
    assert_eq!(
        log_rows(h).await,
        before,
        "a refused statement changes nothing"
    );
}

// ---------------------------------------------------------------------------------------------
// Replay (D-10) and repeated commits (STORE-01 idempotency)
// ---------------------------------------------------------------------------------------------

/// Every column of the edit tables the replay cases write, by table.
const GATE_COLUMNS: &[&str] = &[
    "code",
    "name",
    "name__set",
    "capacity",
    "capacity__set",
    "level",
    "level__set",
    "fee",
    "fee__set",
    "opened_at",
    "opened_at__set",
    "closed_on",
    "closed_on__set",
    "is_open",
    "is_open__set",
    "weight",
    "weight__set",
    "tags",
    "tags__set",
    "fees",
    "fees__set",
    "location",
    "location__set",
    "of_version",
    "of_deleted",
    "of_released",
    "of_def_version",
    "of_log_seq",
];
const STAND_COLUMNS: &[&str] = &[
    "id",
    "label",
    "label__set",
    "gate",
    "gate__set",
    "of_version",
    "of_deleted",
    "of_released",
    "of_def_version",
    "of_log_seq",
];
const LINK_COLUMNS: &[&str] = &[
    "a_pk",
    "b_pk",
    "of_version",
    "of_deleted",
    "of_def_version",
    "of_log_seq",
];

fn stand_edit(id: i64, op: ObjectOp) -> Edit {
    Edit::Object {
        type_name: ObjectTypeName::try_from("Stand".to_owned()).expect("type name"),
        pk: PropertyValue::Int(id),
        op,
    }
}

/// Five commits over two types and a link: creates, a patch of an old object, a commit with two
/// edits of different kinds, and a commit that edits two objects.
async fn commit_history(h: &Harness, v: i64) {
    commit_ok(h, create_gate(v, "A1")).await;
    commit_ok(
        h,
        commit_of(
            v,
            "createGate",
            json!({ "code": "A2" }),
            vec![gate_edit("A2", all_values())],
        ),
    )
    .await;
    commit_ok(
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
    commit_ok(
        h,
        commit_of(
            v,
            "assignStand",
            json!({ "gate": "A1", "stand": 7 }),
            vec![
                stand_edit(7, set(&[("label", s("seven"))])),
                Edit::Link {
                    link: LinkTypeName::try_from("gateStands".to_owned()).expect("link name"),
                    a: s("A1"),
                    b: PropertyValue::Int(7),
                    op: LinkOp::Add,
                },
            ],
        ),
    )
    .await;
    commit_ok(
        h,
        commit_of(
            v,
            "touchGates",
            json!(null),
            vec![
                gate_edit("A1", set(&[("name", s("South"))])),
                gate_edit("A2", set(&[("isOpen", PropertyValue::Bool(false))])),
            ],
        ),
    )
    .await;
}

pub async fn replay_rebuilds_edit_store(h: &Harness) {
    let v = h.load().await;
    commit_history(h, v).await;
    let records = h.store().read_log(0, 100).await.expect("read_log");
    assert_eq!(records.len(), 5);

    let target = h.sibling().await;
    replay(&records, target.store()).await.expect("replay");

    for (table, columns) in [
        ("of_obj_gate", GATE_COLUMNS),
        ("of_obj_stand", STAND_COLUMNS),
        ("of_lnk_gate_stands", LINK_COLUMNS),
    ] {
        let original = h.rows(Tables::Edit, table, columns, None).await;
        let rebuilt = target.rows(Tables::Edit, table, columns, None).await;
        assert!(!original.is_empty(), "{table} has rows to compare");
        assert_eq!(rebuilt, original, "{table} rebuilt from the log");
    }
    // Replay rebuilds the edit tables only: the log and the history stay as they are.
    let log = target.store().read_log(0, 100).await.expect("read_log");
    assert!(log.is_empty(), "replay appended log records: {log:#?}");
    let history = target
        .rows(Tables::Sys, "of_edit_history", &["log_seq"], None)
        .await;
    assert!(history.is_empty(), "replay wrote history: {history:#?}");
}

pub async fn replay_detects_version_divergence(h: &Harness) {
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;
    commit_ok(
        h,
        commit_of(
            v,
            "resizeGate",
            json!({}),
            vec![gate_edit(
                "A1",
                set(&[("capacity", PropertyValue::Int(90))]),
            )],
        ),
    )
    .await;
    let mut records = h.store().read_log(0, 100).await.expect("read_log");
    records[1].entries[0].version_after += 1;

    let target = h.sibling().await;
    let err = replay(&records, target.store())
        .await
        .expect_err("the second record claims a version the rebuilt row does not have");
    assert!(
        matches!(err, ReplayError::Diverged { seq: 2, ordinal: 0 }),
        "{err:?}"
    );
    // A failed replay leaves the target as it was.
    let gates = target
        .rows(Tables::Edit, "of_obj_gate", &["code"], None)
        .await;
    assert!(
        gates.is_empty(),
        "gate rows after a failed replay: {gates:#?}"
    );
}

pub async fn same_commit_twice_creates_two_records(h: &Harness) {
    let v = h.load().await;
    let commit = create_gate(v, "A1");
    let first = commit_ok(h, commit.clone()).await;
    let second = commit_ok(h, commit).await;
    assert_eq!((first.seq, second.seq), (1, 2));
    assert_ne!(first.event_id, second.event_id);
    assert_eq!((first.versions, second.versions), (vec![1], vec![2]));

    let log = h.store().read_log(0, 10).await.expect("read_log");
    assert_eq!(
        log.len(),
        2,
        "the store does not deduplicate (Phase 5 does)"
    );
    assert_ne!(log[0].event_id, log[1].event_id);
    let row = h
        .row(
            Tables::Edit,
            "of_obj_gate",
            &["code", "of_version", "of_log_seq"],
            "code",
            "A1",
        )
        .await;
    assert_eq!(row[1], text("2"));
    assert_eq!(row[2], text("2"));
}

// ---------------------------------------------------------------------------------------------
// The overlay operations (STORE-07)
// ---------------------------------------------------------------------------------------------

fn unset(names: &[&str]) -> ObjectOp {
    ObjectOp::Unset(names.iter().map(|name| property(name)).collect())
}

fn link_edit(link: &str, a: PropertyValue, b: PropertyValue, op: LinkOp) -> Edit {
    Edit::Link {
        link: LinkTypeName::try_from(link.to_owned()).expect("link name"),
        a,
        b,
        op,
    }
}

/// One commit with one edit.
fn single(def_version: i64, action: &str, edit: Edit) -> Commit {
    commit_of(def_version, action, Value::Null, vec![edit])
}

async fn commit_err(h: &Harness, commit: Commit) -> EditError {
    h.store()
        .commit(commit)
        .await
        .expect_err("the commit must be refused")
}

const STAND_STATE: &[&str] = &[
    "id",
    "label",
    "label__set",
    "gate__set",
    "of_version",
    "of_deleted",
    "of_released",
];

/// `STAND_STATE` of Stand `id`.
async fn stand_state(h: &Harness, id: i64) -> Vec<Option<String>> {
    h.row(
        Tables::Edit,
        "of_obj_stand",
        STAND_STATE,
        "id",
        &id.to_string(),
    )
    .await
}

async fn log_len(h: &Harness) -> usize {
    h.store().read_log(0, 1000).await.expect("read_log").len()
}

pub async fn source_object_lifecycle(h: &Harness) {
    let v = h.load().await;
    let on_stand = |action: &str, op: ObjectOp| single(v, action, stand_edit(7, op));

    let receipt = commit_ok(
        h,
        on_stand("label", set(&[("label", s("S1")), ("gate", s("A1"))])),
    )
    .await;
    assert_eq!(receipt.versions, vec![1]);

    let receipt = commit_ok(h, on_stand("clear", unset(&["label"]))).await;
    assert_eq!(receipt.versions, vec![2]);
    let row = stand_state(h, 7).await;
    assert_eq!(row[1], None, "an unset property reads as NULL");
    assert!(!flag(&row[2]), "label__set");
    assert!(flag(&row[3]), "gate stays set");
    assert_eq!(row[4], text("2"));

    let receipt = commit_ok(h, on_stand("remove", ObjectOp::Tombstone)).await;
    assert_eq!(receipt.versions, vec![3]);
    let row = stand_state(h, 7).await;
    assert!(flag(&row[5]), "of_deleted");
    assert!(!flag(&row[6]), "of_released");

    let err = commit_err(h, on_stand("revive", set(&[("label", s("x"))]))).await;
    assert!(matches!(err, EditError::Tombstoned), "{err:?}");
    assert_eq!(stand_state(h, 7).await[4], text("3"));
    assert_eq!(log_len(h).await, 3, "the refused commit left no record");

    let receipt = commit_ok(h, on_stand("release", ObjectOp::Release)).await;
    assert_eq!(receipt.versions, vec![4]);
    let row = stand_state(h, 7).await;
    assert!(!flag(&row[5]), "of_deleted after release");
    assert!(flag(&row[6]), "of_released");
    assert!(
        !flag(&row[2]) && !flag(&row[3]),
        "release clears every __set"
    );
    assert_eq!(row[4], text("4"));

    let receipt = commit_ok(h, on_stand("relabel", set(&[("label", s("S2"))]))).await;
    assert_eq!(receipt.versions, vec![5]);
    let row = stand_state(h, 7).await;
    assert_eq!(row[1], text("S2"));
    assert!(flag(&row[2]));
    assert!(!flag(&row[3]), "only what was set after the release is set");
    assert!(!flag(&row[6]), "a set leaves the released state");
    assert_eq!(row[4], text("5"));
}

pub async fn edit_only_rules(h: &Harness) {
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;

    let err = commit_err(h, single(v, "r", gate_edit("A1", ObjectOp::Release))).await;
    assert!(matches!(err, EditError::ReleaseOnEditOnly), "{err:?}");

    let err = commit_err(h, single(v, "u", gate_edit("A1", unset(&["name"])))).await;
    assert!(
        matches!(&err, EditError::MissingRequired { property } if property == "name"),
        "{err:?}"
    );

    let err = commit_err(h, single(v, "t", gate_edit("B2", ObjectOp::Tombstone))).await;
    assert!(matches!(err, EditError::NotFound), "{err:?}");
    assert_eq!(log_len(h).await, 1, "the refused commits left no record");

    let receipt = commit_ok(h, single(v, "u", gate_edit("A1", unset(&["capacity"])))).await;
    assert_eq!(receipt.versions, vec![2]);
    let row = h
        .row(
            Tables::Edit,
            "of_obj_gate",
            &["code", "capacity", "capacity__set", "name__set"],
            "code",
            "A1",
        )
        .await;
    assert_eq!(row[1], None);
    assert!(!flag(&row[2]));
    assert!(flag(&row[3]), "the required name is untouched");

    let receipt = commit_ok(h, single(v, "t", gate_edit("A1", ObjectOp::Tombstone))).await;
    assert_eq!(receipt.versions, vec![3]);
    let err = commit_err(h, single(v, "r", gate_edit("A1", ObjectOp::Release))).await;
    assert!(
        matches!(err, EditError::ReleaseOnEditOnly),
        "a deleted editOnly object cannot come back: {err:?}"
    );
    let err = commit_err(h, single(v, "s", gate_edit("A1", set(&[("name", s("N"))])))).await;
    assert!(matches!(err, EditError::Tombstoned), "{err:?}");
}

/// `(of_version, of_deleted)` of the link row `A1 - 7` of `table`.
async fn link_state(h: &Harness, table: &str) -> (Option<String>, bool) {
    let row = h
        .row(
            Tables::Edit,
            table,
            &["a_pk", "b_pk", "of_version", "of_deleted"],
            "a_pk",
            "A1",
        )
        .await;
    (row[2].clone(), flag(&row[3]))
}

pub async fn link_add_remove_toggle(h: &Harness) {
    let v = h.load().await;
    let edge = |op: LinkOp| {
        single(
            v,
            "edge",
            link_edit("gateStands", s("A1"), PropertyValue::Int(7), op),
        )
    };

    assert_eq!(commit_ok(h, edge(LinkOp::Add)).await.versions, vec![1]);
    assert_eq!(
        link_state(h, "of_lnk_gate_stands").await,
        (text("1"), false)
    );

    assert_eq!(commit_ok(h, edge(LinkOp::Remove)).await.versions, vec![2]);
    assert_eq!(link_state(h, "of_lnk_gate_stands").await, (text("2"), true));

    assert_eq!(commit_ok(h, edge(LinkOp::Add)).await.versions, vec![3]);
    assert_eq!(
        link_state(h, "of_lnk_gate_stands").await,
        (text("3"), false)
    );

    let log = h.store().read_log(0, 10).await.expect("read_log");
    let ops: Vec<EditOp> = log.iter().map(|r| r.entries[0].op).collect();
    assert_eq!(
        ops,
        vec![EditOp::LinkAdd, EditOp::LinkRemove, EditOp::LinkAdd]
    );
    assert!(log.iter().all(|r| r.entries[0].pk == json!(["A1", 7])));
}

pub async fn link_remove_rules(h: &Harness) {
    let v = h.load().await;
    let on = |link: &str, op: LinkOp| {
        single(
            v,
            "edge",
            link_edit(link, s("A1"), PropertyValue::Int(7), op),
        )
    };

    let err = commit_err(h, on("gateStands", LinkOp::Remove)).await;
    assert!(
        matches!(err, EditError::NotFound),
        "an editOnly link that was never added: {err:?}"
    );
    assert_eq!(log_len(h).await, 0);

    // A source backed link may be removed without an edit row: the row suppresses the join row.
    let receipt = commit_ok(h, on("gateSlots", LinkOp::Remove)).await;
    assert_eq!(receipt.versions, vec![1]);
    assert_eq!(link_state(h, "of_lnk_gate_slots").await, (text("1"), true));

    let receipt = commit_ok(h, on("gateSlots", LinkOp::Add)).await;
    assert_eq!(receipt.versions, vec![2]);
    assert_eq!(link_state(h, "of_lnk_gate_slots").await, (text("2"), false));
}

pub async fn log_records_every_op(h: &Harness) {
    let v = h.load().await;
    let on_stand = |action: &str, op: ObjectOp| single(v, action, stand_edit(7, op));
    commit_ok(h, on_stand("a", set(&[("label", s("S1"))]))).await;
    commit_ok(h, on_stand("b", unset(&["label"]))).await;
    commit_ok(h, on_stand("c", ObjectOp::Tombstone)).await;
    commit_ok(h, on_stand("d", ObjectOp::Release)).await;
    let edge = |op: LinkOp| link_edit("gateStands", s("A1"), PropertyValue::Int(7), op);
    commit_ok(h, single(v, "e", edge(LinkOp::Add))).await;
    commit_ok(h, single(v, "f", edge(LinkOp::Remove))).await;

    let log = h.store().read_log(0, 10).await.expect("read_log");
    let entries: Vec<_> = log
        .iter()
        .map(|r| {
            let e = &r.entries[0];
            (e.op, e.payload.clone(), e.version_after)
        })
        .collect();
    assert_eq!(
        entries,
        vec![
            (EditOp::Set, Some(json!({ "label": "S1" })), 1),
            (EditOp::Unset, Some(json!(["label"])), 2),
            (EditOp::Tombstone, None, 3),
            (EditOp::Release, None, 4),
            (EditOp::LinkAdd, None, 1),
            (EditOp::LinkRemove, None, 2),
        ]
    );

    let history = h
        .rows(
            Tables::Sys,
            "of_edit_history",
            &["log_seq", "snapshot"],
            None,
        )
        .await;
    let snapshots: Vec<Value> = history.iter().map(|row| json_cell(&row[1])).collect();
    assert_eq!(
        snapshots,
        vec![
            json!({ "deleted": false, "released": false, "set": { "label": "S1" } }),
            json!({ "deleted": false, "released": false, "set": {} }),
            json!({ "deleted": true, "released": false, "set": {} }),
            json!({ "deleted": false, "released": true, "set": {} }),
            json!({ "deleted": false }),
            json!({ "deleted": true }),
        ]
    );
}

/// A history that uses every operation, replayed into a second store.
pub async fn replay_rebuilds_overlay_operations(h: &Harness) {
    let v = h.load().await;
    commit_ok(h, create_gate(v, "A1")).await;
    commit_ok(h, create_gate(v, "A2")).await;
    commit_ok(
        h,
        single(
            v,
            "stand",
            stand_edit(7, set(&[("label", s("seven")), ("gate", s("A1"))])),
        ),
    )
    .await;
    commit_ok(h, single(v, "unset", stand_edit(7, unset(&["label"])))).await;
    commit_ok(h, single(v, "suppress", stand_edit(8, ObjectOp::Tombstone))).await;
    commit_ok(
        h,
        commit_of(
            v,
            "cycle",
            Value::Null,
            vec![
                stand_edit(9, set(&[("label", s("nine"))])),
                stand_edit(9, ObjectOp::Tombstone),
                stand_edit(9, ObjectOp::Release),
                stand_edit(9, set(&[("label", s("again"))])),
            ],
        ),
    )
    .await;
    commit_ok(h, single(v, "drop", gate_edit("A2", ObjectOp::Tombstone))).await;
    commit_ok(h, single(v, "clear", gate_edit("A1", unset(&["capacity"])))).await;
    for (link, op) in [
        ("gateStands", LinkOp::Add),
        ("gateStands", LinkOp::Remove),
        ("gateStands", LinkOp::Add),
        ("gateSlots", LinkOp::Remove),
        ("gateSlots", LinkOp::Add),
        ("gateSlots", LinkOp::Remove),
    ] {
        commit_ok(
            h,
            single(
                v,
                "edge",
                link_edit(link, s("A1"), PropertyValue::Int(7), op),
            ),
        )
        .await;
    }
    let records = h.store().read_log(0, 100).await.expect("read_log");
    let used: std::collections::BTreeSet<&str> = records
        .iter()
        .flat_map(|r| r.entries.iter().map(|e| e.op.as_str()))
        .collect();
    assert_eq!(
        used.len(),
        6,
        "the history uses every kind of entry: {used:?}"
    );

    let target = h.sibling().await;
    replay(&records, target.store()).await.expect("replay");

    for (table, columns) in [
        ("of_obj_gate", GATE_COLUMNS),
        ("of_obj_stand", STAND_COLUMNS),
        ("of_lnk_gate_stands", LINK_COLUMNS),
        ("of_lnk_gate_slots", LINK_COLUMNS),
    ] {
        let original = h.rows(Tables::Edit, table, columns, None).await;
        let rebuilt = target.rows(Tables::Edit, table, columns, None).await;
        assert!(!original.is_empty(), "{table} has rows to compare");
        assert_eq!(rebuilt, original, "{table} rebuilt from the log");
    }
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
        $crate::conformance::conformance_case!($factory, append_only_rejects_update_and_delete $(, $attr)?);
        $crate::conformance::conformance_case!($factory, append_only_rejects_truncate_on_pg $(, $attr)?);
        $crate::conformance::conformance_case!($factory, append_only_rejects_sqlite_replace_writes $(, $attr)?);
        $crate::conformance::conformance_case!($factory, replay_rebuilds_edit_store $(, $attr)?);
        $crate::conformance::conformance_case!($factory, replay_detects_version_divergence $(, $attr)?);
        $crate::conformance::conformance_case!($factory, same_commit_twice_creates_two_records $(, $attr)?);
        $crate::conformance::conformance_case!($factory, source_object_lifecycle $(, $attr)?);
        $crate::conformance::conformance_case!($factory, edit_only_rules $(, $attr)?);
        $crate::conformance::conformance_case!($factory, link_add_remove_toggle $(, $attr)?);
        $crate::conformance::conformance_case!($factory, link_remove_rules $(, $attr)?);
        $crate::conformance::conformance_case!($factory, log_records_every_op $(, $attr)?);
        $crate::conformance::conformance_case!($factory, replay_rebuilds_overlay_operations $(, $attr)?);
    };
}

pub(crate) use all_cases;
pub(crate) use conformance_case;
