//! Reading the change history of one object under a scope (plan 03-17: STORE-04, D-13).
//!
//! The history of an object is readable only by a caller who can see the object now. The values it
//! shows are those of the columns the caller may see now; every other changed column is only named.
//! The raw Action parameters go to the administration channel only.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use camino::Utf8Path;
use ontoforge_auth::scope::{RoleTypeGrant, TypeScope, merge};
use ontoforge_expr::rowpred::{PredContext, Scalar, compile_row_filter};
use ontoforge_expr::{Attrs, RowValues};
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, Ontology, PropertyName};
use ontoforge_schema::load_dir;
use ontoforge_storage::commit::{Actor, Commit, Edit, LinkOp, ObjectOp, Receipt};
use ontoforge_storage::history::{CallerKind, HistoryEntry, HistoryError, HistoryRequest};
use ontoforge_storage::log::EditOp;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::EditStore;
use ontoforge_storage::value::PropertyValue;
use serde_json::{Value, json};
use support::{pg_store, sqlite_store};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const ACTOR: &str = "https://idp.example|tester";
const DISPATCH_ROW: &str = "object.status != 'CANCELLED' || object.status == null";

struct Fixture {
    /// Keeps the database alive: the temporary directory or the embedded server.
    _guard: Box<dyn std::any::Any>,
    store: Arc<EditStore>,
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
    let store = Arc::clone(&t.store);
    loaded(Box::new(t), store).await
}

/// `store` with the aviation definition reloaded.
async fn loaded(guard: Box<dyn std::any::Any>, store: Arc<EditStore>) -> Fixture {
    let ontology = aviation();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload aviation");
    let def_version = store.active_definition().await.expect("active").def_version;
    Fixture {
        _guard: guard,
        store,
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
        self.store
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
        f.store
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

// ---------------------------------------------------------------------------------------------
// Task 2: one error for every kind of "cannot see", current scope, operations, pages
// ---------------------------------------------------------------------------------------------

/// The three ways to be unable to see `F1`, as the errors of one store.
async fn blind_errors(f: &Fixture) -> Vec<HistoryError> {
    let scope = dispatcher(f);
    let cancelled = row(&[("flightId", string("F1")), ("status", string("CANCELLED"))]);
    let granted_row = flight_row();
    vec![
        // The object exists but the row predicate is False for it.
        ask("Flight", "F1")
            .run(f, Some(&scope), Some(&cancelled))
            .await
            .expect_err("hidden row"),
        // The object does not exist for the caller.
        ask("Flight", "F1")
            .run(f, Some(&scope), None)
            .await
            .expect_err("no row"),
        // No role of the caller grants the type.
        ask("Flight", "F1")
            .run(f, None, Some(&granted_row))
            .await
            .expect_err("no scope"),
    ]
}

#[tokio::test]
async fn invisible_nonexistent_and_ungranted_are_identical() {
    let with_history = fixture().await;
    for i in 0..5 {
        with_history
            .set(
                "Flight",
                "F1",
                &[("delayMinutes", PropertyValue::Int(i))],
                Value::Null,
            )
            .await;
    }
    let without_history = fixture().await;

    let mut texts = BTreeSet::new();
    for f in [&with_history, &without_history] {
        let errors = blind_errors(f).await;
        assert_eq!(errors.len(), 3);
        for error in errors {
            assert!(matches!(error, HistoryError::NotFound), "{error:?}");
            texts.insert(error.to_string());
        }
    }
    assert_eq!(texts, BTreeSet::from(["object not found".to_owned()]));
}

/// The visibility decision is made before the store is consulted: on a store without a
/// definition a caller who cannot see the object still gets `NotFound`, not a store error.
#[tokio::test]
async fn invisible_check_precedes_store_access() {
    let t = sqlite_store().await;
    let f = Fixture {
        store: Arc::clone(&t.store),
        _guard: Box::new(t),
        ontology: aviation(),
        def_version: 1,
    };
    for error in blind_errors(&f).await {
        assert!(matches!(error, HistoryError::NotFound), "{error:?}");
    }
    // A caller who can see the object does reach the store, and the store refuses.
    let scope = dispatcher(&f);
    let current = flight_row();
    let error = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect_err("no definition is loaded");
    assert!(matches!(error, HistoryError::Store(_)), "{error:?}");
}

/// A visible object without any edit has an empty history, not `NotFound`.
#[tokio::test]
async fn visible_object_without_edits_has_empty_history() {
    let f = fixture().await;
    let scope = dispatcher(&f);
    let current = flight_row();
    let entries = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert!(entries.is_empty());
}

#[tokio::test]
async fn visibility_uses_current_row() {
    let f = fixture().await;
    // A sees status where status is DELAYED; B sees the delay where it is over 60 minutes.
    let scope = merge(&[
        f.grant(
            "Flight",
            &["flightId", "status"],
            Some("object.status == 'DELAYED'"),
        ),
        f.grant(
            "Flight",
            &["flightId", "delayMinutes"],
            Some("object.delayMinutes > 60"),
        ),
    ])
    .expect("a scope");
    // B held for the row when the delay was 91357; now the delay is 30 and only A holds.
    f.set(
        "Flight",
        "F1",
        &[("delayMinutes", PropertyValue::Int(91_357))],
        Value::Null,
    )
    .await;
    f.set("Flight", "F1", &[("status", text("DELAYED"))], Value::Null)
        .await;
    f.set(
        "Flight",
        "F1",
        &[("delayMinutes", PropertyValue::Int(30))],
        Value::Null,
    )
    .await;
    let current = row(&[
        ("flightId", string("F1")),
        ("status", string("DELAYED")),
        ("delayMinutes", Scalar::Int(30)),
    ]);

    let entries = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");

    assert_eq!(entries.len(), 3, "{entries:#?}");
    // newest first: delay 30, status, delay 91357
    assert!(entries[0].values.is_empty());
    assert_eq!(entries[0].masked_changes, props(&["delayMinutes"]));
    assert_eq!(
        entries[1].values,
        BTreeMap::from([(prop("status"), json!("DELAYED"))])
    );
    assert!(entries[2].values.is_empty());
    assert_eq!(entries[2].masked_changes, props(&["delayMinutes"]));
    // The old delay is in no value. (The whole body is not scanned: ids and timestamps are digits.)
    let values = serde_json::to_string(&entries.iter().map(|e| &e.values).collect::<Vec<_>>())
        .expect("serialize");
    assert!(!values.contains("91357"), "{values}");
}

#[tokio::test]
async fn unset_tombstone_release_entries() {
    let f = fixture().await;
    let pilot = |op: ObjectOp| Edit::Object {
        type_name: type_name("Pilot"),
        pk: text("P1"),
        op,
    };
    f.commit(
        vec![pilot(ObjectOp::Set(BTreeMap::from([
            (prop("fullName"), text("Ann")),
            (prop("licenseNumber"), text("L-SECRET")),
        ])))],
        Value::Null,
    )
    .await;
    f.commit(
        vec![pilot(ObjectOp::Unset(props(&[
            "fullName",
            "licenseNumber",
        ])))],
        Value::Null,
    )
    .await;
    f.commit(vec![pilot(ObjectOp::Tombstone)], Value::Null)
        .await;
    f.commit(vec![pilot(ObjectOp::Release)], Value::Null).await;

    let scope = f.scope("Pilot", &["employeeId", "fullName"], None);
    // After the release the caller gives the row as the source shows it.
    let current = row(&[("employeeId", string("P1")), ("fullName", string("Ann"))]);
    let entries = ask("Pilot", "P1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");

    assert_eq!(
        entries.iter().map(|e| e.op).collect::<Vec<_>>(),
        [
            EditOp::Release,
            EditOp::Tombstone,
            EditOp::Unset,
            EditOp::Set
        ]
    );
    for entry in &entries[..2] {
        assert!(entry.values.is_empty() && entry.unset.is_empty());
        assert!(entry.masked_changes.is_empty());
    }
    assert_eq!(entries[2].unset, props(&["fullName"]));
    assert_eq!(entries[2].masked_changes, props(&["licenseNumber"]));
    assert!(entries[2].values.is_empty());
    assert_eq!(
        entries[3].values,
        BTreeMap::from([(prop("fullName"), json!("Ann"))])
    );
    assert_eq!(entries[3].masked_changes, props(&["licenseNumber"]));
    let body = serde_json::to_string(&entries).expect("serialize");
    assert!(!body.contains("L-SECRET"), "{body}");
}

#[tokio::test]
async fn pagination_with_before_seq() {
    let f = fixture().await;
    let mut seqs = Vec::new();
    for i in 0..7 {
        seqs.push(
            f.set(
                "Flight",
                "F1",
                &[("delayMinutes", PropertyValue::Int(i))],
                Value::Null,
            )
            .await,
        );
    }
    seqs.reverse();
    let scope = dispatcher(&f);
    let current = flight_row();

    let mut request = ask("Flight", "F1");
    request.limit = 3;
    let mut pages = Vec::new();
    loop {
        let page = request
            .run(&f, Some(&scope), Some(&current))
            .await
            .expect("page");
        let Some(last) = page.last() else { break };
        request.before_seq = Some(last.seq);
        pages.push(page.iter().map(|e| e.seq).collect::<Vec<_>>());
    }
    assert_eq!(
        pages,
        [
            seqs[0..3].to_vec(),
            seqs[3..6].to_vec(),
            seqs[6..7].to_vec()
        ]
    );

    // A limit far above the cap is accepted and returns what there is.
    let mut all = ask("Flight", "F1");
    all.limit = 10_000;
    let all = all
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(all.len(), 7);
}

#[tokio::test]
async fn limit_zero_counts_as_one() {
    let f = fixture().await;
    for i in 0..2 {
        f.set(
            "Flight",
            "F1",
            &[("delayMinutes", PropertyValue::Int(i))],
            Value::Null,
        )
        .await;
    }
    let scope = dispatcher(&f);
    let current = flight_row();
    let mut request = ask("Flight", "F1");
    request.limit = 0;
    let entries = request
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].version, 2);
}

/// Two edits of one object in one commit share a `seq`. A page that ends between them must not
/// lose the second one when the next page starts at `before_seq`.
#[tokio::test]
async fn pagination_keeps_entries_of_one_commit_together() {
    let f = fixture().await;
    let seq = f
        .commit(
            vec![
                set_edit("Flight", "F1", &[("status", text("DELAYED"))]),
                set_edit("Flight", "F1", &[("delayMinutes", PropertyValue::Int(30))]),
            ],
            Value::Null,
        )
        .await
        .seq;
    let scope = dispatcher(&f);
    let current = flight_row();

    let mut request = ask("Flight", "F1");
    request.limit = 1;
    let mut seen = Vec::new();
    for _ in 0..4 {
        let page = request
            .run(&f, Some(&scope), Some(&current))
            .await
            .expect("page");
        let Some(last) = page.last() else { break };
        request.before_seq = Some(last.seq);
        seen.extend(page.iter().map(|e| (e.seq, e.version)));
    }
    assert_eq!(seen, [(seq, 2), (seq, 1)]);
}

#[tokio::test]
async fn link_entries_are_not_object_history() {
    let f = fixture().await;
    let seq = f
        .commit(
            vec![
                set_edit("Flight", "F1", &[("status", text("DELAYED"))]),
                Edit::Link {
                    link: LinkTypeName::try_from("flightCrew".to_owned()).expect("link"),
                    a: text("F1"),
                    b: text("P1"),
                    op: LinkOp::Add,
                },
            ],
            Value::Null,
        )
        .await
        .seq;
    let scope = dispatcher(&f);
    let current = flight_row();

    let entries = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(entries.len(), 1, "{entries:#?}");
    assert_eq!(entries[0].seq, seq);
    assert_eq!(entries[0].op, EditOp::Set);

    // The pilot at the other end of the link has no object history either.
    let pilot_scope = f.scope("Pilot", &["employeeId"], None);
    let pilot = row(&[("employeeId", string("P1"))]);
    let entries = ask("Pilot", "P1")
        .run(&f, Some(&pilot_scope), Some(&pilot))
        .await
        .expect("history");
    assert!(entries.is_empty(), "{entries:#?}");
}

/// The same masking on PostgreSQL: jsonb keys, UTC timestamps and `Admin` params.
/// Run with `--ignored` against `ONTOFORGE_PG_URL` or with `--features embedded-pg`.
#[tokio::test]
#[ignore = "needs a PostgreSQL server"]
async fn pg_history_masks_and_pages() {
    let pg = pg_store().await;
    let store = Arc::clone(&pg.store);
    let f = loaded(Box::new(pg), store).await;
    let params = json!({ "note": "p-secret" });
    let seqs = three_flight_commits(&f, &params).await;
    let scope = dispatcher(&f);
    let current = flight_row();

    let entries = ask("Flight", "F1")
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(
        entries.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [seqs[2], seqs[1], seqs[0]]
    );
    assert_eq!(entries[0].masked_changes, props(&["aircraftTailNumber"]));
    assert!(entries[0].committed_at.ends_with('Z'), "{entries:#?}");
    let body = serde_json::to_string(&entries).expect("serialize");
    assert!(
        !body.contains("D-SECRET") && !body.contains("p-secret"),
        "{body}"
    );

    let mut admin = ask("Flight", "F1");
    admin.caller = CallerKind::Admin;
    admin.limit = 1;
    let page = admin
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].params.as_ref(), Some(&params));
    admin.before_seq = Some(page[0].seq);
    let page = admin
        .run(&f, Some(&scope), Some(&current))
        .await
        .expect("history");
    assert_eq!(page[0].seq, seqs[1]);

    for error in blind_errors(&f).await {
        assert!(matches!(error, HistoryError::NotFound), "{error:?}");
    }
}
