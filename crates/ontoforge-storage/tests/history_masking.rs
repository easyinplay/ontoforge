//! Reading the change history of one object under a scope (plan 03-17: STORE-04, D-13).
//!
//! The history of an object is readable only by a caller who can see the object now. The values it
//! shows are those of the columns the caller may see now; every other changed column is only named.
//! The raw Action parameters go to the administration channel only.

mod support;

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use ontoforge_auth::scope::{RoleTypeGrant, TypeScope, merge};
use ontoforge_expr::rowpred::{PredContext, Scalar, compile_row_filter};
use ontoforge_expr::{Attrs, RowValues};
use ontoforge_schema::ir::{ObjectTypeName, Ontology, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp, Receipt};
use ontoforge_storage::history::{CallerKind, HistoryEntry, HistoryError, HistoryRequest};
use ontoforge_storage::log::EditOp;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::value::PropertyValue;
use serde_json::{Value, json};
use support::{TestStore, sqlite_store};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const ACTOR: &str = "https://idp.example|tester";
const DISPATCH_ROW: &str = "object.status != 'CANCELLED' || object.status == null";

struct Fixture {
    t: TestStore,
    ontology: Ontology,
    def_version: i64,
}

fn aviation() -> Ontology {
    let out = load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

/// An SQLite store with the aviation definition reloaded.
async fn fixture() -> Fixture {
    let t = sqlite_store().await;
    let ontology = aviation();
    reload(&t.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload aviation");
    let def_version = t
        .store
        .active_definition()
        .await
        .expect("active")
        .def_version;
    Fixture {
        t,
        ontology,
        def_version,
    }
}

fn type_name(name: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(name.to_owned()).expect("type name")
}

fn prop(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property name")
}

fn text(value: &str) -> PropertyValue {
    PropertyValue::String(value.to_owned())
}

fn props(names: &[&str]) -> BTreeSet<PropertyName> {
    names.iter().map(|n| prop(n)).collect()
}

impl Fixture {
    async fn commit(&self, edits: Vec<Edit>, params: Value) -> Receipt {
        self.t
            .store
            .commit(Commit {
                actor: Actor::Principal(ACTOR.to_owned()),
                action: "editFlight".to_owned(),
                def_version: self.def_version,
                params,
                edits,
            })
            .await
            .expect("commit")
    }

    /// One `Set` of `values` on the object `pk` of `ty`.
    async fn set(
        &self,
        ty: &str,
        pk: &str,
        values: &[(&str, PropertyValue)],
        params: Value,
    ) -> i64 {
        self.commit(vec![set_edit(ty, pk, values)], params)
            .await
            .seq
    }

    /// The scope of one role: `columns` of `ty`, restricted by the CEL row filter `row`.
    fn grant(&self, ty: &str, columns: &[&str], row: Option<&str>) -> RoleTypeGrant {
        let object_type = self
            .ontology
            .object_types
            .get(&type_name(ty))
            .expect("object type");
        let attrs = BTreeMap::new();
        let row = row.map(|src| {
            compile_row_filter(
                src,
                &PredContext {
                    object_type,
                    attrs: &attrs,
                },
            )
            .unwrap_or_else(|issues| panic!("{issues:?}"))
        });
        RoleTypeGrant {
            properties: props(columns),
            row,
        }
    }

    fn scope(&self, ty: &str, columns: &[&str], row: Option<&str>) -> TypeScope {
        merge(&[self.grant(ty, columns, row)]).expect("a scope")
    }
}

fn set_edit(ty: &str, pk: &str, values: &[(&str, PropertyValue)]) -> Edit {
    Edit::Object {
        type_name: type_name(ty),
        pk: text(pk),
        op: ObjectOp::Set(values.iter().map(|(n, v)| (prop(n), v.clone())).collect()),
    }
}

fn row(values: &[(&str, Scalar)]) -> BTreeMap<PropertyName, Scalar> {
    values.iter().map(|(n, v)| (prop(n), v.clone())).collect()
}

fn string(value: &str) -> Scalar {
    Scalar::String(value.to_owned())
}

/// Everything a request needs, owned, so a test can build one and tweak it.
struct Ask {
    ty: ObjectTypeName,
    pk: PropertyValue,
    attrs: Attrs,
    caller: CallerKind,
    limit: u32,
    before_seq: Option<i64>,
}

fn ask(ty: &str, pk: &str) -> Ask {
    Ask {
        ty: type_name(ty),
        pk: text(pk),
        attrs: Attrs::new(),
        caller: CallerKind::Ordinary,
        limit: 100,
        before_seq: None,
    }
}

impl Ask {
    async fn run(
        &self,
        f: &Fixture,
        scope: Option<&TypeScope>,
        current: Option<&dyn RowValues>,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        f.t.store
            .history(&HistoryRequest {
                type_name: &self.ty,
                pk: &self.pk,
                scope,
                attrs: &self.attrs,
                current_row: current,
                caller: self.caller,
                limit: self.limit,
                before_seq: self.before_seq,
            })
            .await
    }
}

fn flight_row() -> BTreeMap<PropertyName, Scalar> {
    row(&[
        ("flightId", string("F1")),
        ("status", string("DELAYED")),
        ("delayMinutes", Scalar::Int(30)),
        ("aircraftTailNumber", string("D-SECRET")),
    ])
}

/// Three commits on `F1`: status, delay and tail number, each with the parameters `params`.
async fn three_flight_commits(f: &Fixture, params: &Value) -> Vec<i64> {
    vec![
        f.set(
            "Flight",
            "F1",
            &[("status", text("DELAYED"))],
            params.clone(),
        )
        .await,
        f.set(
            "Flight",
            "F1",
            &[("delayMinutes", PropertyValue::Int(30))],
            params.clone(),
        )
        .await,
        f.set(
            "Flight",
            "F1",
            &[("aircraftTailNumber", text("D-SECRET"))],
            params.clone(),
        )
        .await,
    ]
}

fn dispatcher(f: &Fixture) -> TypeScope {
    f.scope(
        "Flight",
        &["flightId", "status", "delayMinutes"],
        Some(DISPATCH_ROW),
    )
}

#[tokio::test]
async fn ordinary_caller_sees_visible_values_only() {
    let f = fixture().await;
    let seqs = three_flight_commits(&f, &Value::Null).await;
    let scope = dispatcher(&f);
    let current = flight_row();

    let entries = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");

    assert_eq!(entries.len(), 3, "{entries:#?}");
    assert_eq!(
        entries.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [seqs[2], seqs[1], seqs[0]],
        "newest first"
    );
    // The tail number is not a column of the dispatcher: named, never valued.
    assert!(entries[0].values.is_empty(), "{:?}", entries[0]);
    assert_eq!(entries[0].masked_changes, props(&["aircraftTailNumber"]));
    assert_eq!(entries[0].op, EditOp::Set);
    assert_eq!(entries[0].version, 3);
    assert_eq!(
        entries[1].values,
        BTreeMap::from([(prop("delayMinutes"), json!(30))])
    );
    assert!(entries[1].masked_changes.is_empty());
    assert_eq!(
        entries[2].values,
        BTreeMap::from([(prop("status"), json!("DELAYED"))])
    );
    assert_eq!(entries[2].version, 1);
    for entry in &entries {
        assert!(entry.unset.is_empty());
        assert_eq!(entry.actor, ACTOR);
        assert_eq!(entry.action, "editFlight");
        assert!(!entry.event_id.is_empty());
        assert!(entry.committed_at.ends_with('Z'), "{}", entry.committed_at);
    }
}

#[tokio::test]
async fn serialized_history_never_contains_masked_values() {
    let f = fixture().await;
    three_flight_commits(&f, &Value::Null).await;
    let scope = dispatcher(&f);
    let current = flight_row();

    let entries = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    let body = serde_json::to_string(&entries).expect("serialize");

    // The scan is not vacuous: visible values and the masked column's name are in there.
    assert!(body.contains("DELAYED"), "{body}");
    assert!(body.contains("aircraftTailNumber"), "{body}");
    assert!(!body.contains("D-SECRET"), "{body}");
}

#[tokio::test]
async fn params_hidden_from_ordinary_caller() {
    let f = fixture().await;
    let params = json!({ "note": "p-secret" });
    three_flight_commits(&f, &params).await;
    let scope = dispatcher(&f);
    let current = flight_row();

    let ordinary = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(ordinary.len(), 3);
    assert!(ordinary.iter().all(|e| e.params.is_none()), "{ordinary:#?}");
    let body = serde_json::to_string(&ordinary).expect("serialize");
    assert!(!body.contains("p-secret"), "{body}");
    assert!(!body.contains("params"), "{body}");

    let mut admin = ask("Flight", "F1");
    admin.caller = CallerKind::Admin;
    let admin = admin
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(admin.len(), 3);
    assert!(
        admin.iter().all(|e| e.params.as_ref() == Some(&params)),
        "{admin:#?}"
    );
}
