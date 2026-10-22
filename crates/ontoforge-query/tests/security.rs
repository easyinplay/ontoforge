#![allow(
    clippy::unused_async_trait_impl,
    reason = "test engines return immediately"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::printer::{Dialect, PrintedSql};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, QueryContext, QueryOutput, TableRef, Value,
};
use ontoforge_query::{
    ExecutionLimits, ObjectRow, ObjectSet, QueryError, QueryPage, QueryRequest, QueryService,
    TokenCodec,
};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use serde_json::json;

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);
fn name(raw: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(raw.to_owned()).unwrap()
}
fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}

struct Fixture {
    _dir: tempfile::TempDir,
    store: EditStore,
    principal: Principal,
    scope: ontoforge_auth::scope::PrincipalScope,
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .unwrap();
    let ontology = load_dir(Utf8Path::new(AVIATION)).ontology.unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let loaded = load_scope_dir(Utf8Path::new(SCOPE));
    let cfg = serde_json::from_value(json!({
        "principal_attrs": { "homeAirports": { "claim": "airports", "type": "string_list" } }
    }))
    .unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: Some(&ProfileName::try_from("dev".to_owned()).unwrap()),
        attrs: &attr_types(Some(&cfg)),
        scope: &loaded.scope.unwrap(),
        sources: Some(&loaded.sources),
    })
    .unwrap();
    let principal = Principal {
        id: PrincipalId::new("ontoforge-dev", "alice"),
        issuer: "ontoforge-dev".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::from(["dispatchers".to_owned()]),
        attrs: ontoforge_expr::Attrs::default(),
    };
    let scope = compiled.for_principal(&principal);
    Fixture {
        _dir: dir,
        store,
        principal,
        scope,
    }
}

struct FailingEngine;
impl Engine for FailingEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        Err(EngineError::Query {
            error_name: "Trino query id 20261022_very_secret".to_owned(),
            message: "ontoforge_edit.of_obj_secret hidden_column".to_owned(),
        })
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("pk read must not describe tables")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

struct NeverEngine;
impl Engine for NeverEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        panic!("edit-only read must not use source")
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("edit-only read must not describe source")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

struct RowsEngine(&'static str);
impl Engine for RowsEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        Ok(QueryOutput {
            columns: Vec::new(),
            truncated: false,
            rows: vec![vec![
                Value::String("F-1001".to_owned()),
                Value::String("LH100".to_owned()),
                Value::String("FRA".to_owned()),
                Value::String("JFK".to_owned()),
                Value::String("D-AAAA".to_owned()),
                Value::Timestamp("2026-03-28T09:15:00".to_owned()),
                Value::String(self.0.to_owned()),
                Value::Long(0),
                Value::Array(Vec::new()),
            ]],
        })
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("pk read must not describe source")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

fn request(object_type: &str, pk: &str, select: &[&str]) -> QueryRequest {
    QueryRequest {
        set: ObjectSet::Pk {
            object_type: name(object_type),
            pk: json!(pk),
        },
        select: select.iter().map(|p| prop(p)).collect(),
    }
}

#[test]
fn token_tamper_fails_closed() {
    let signer = TokenCodec::new(b"first-key".to_vec()).unwrap();
    let fields: [&[u8]; 6] = [
        b"principal",
        b"Flight",
        b"F-1001",
        b"definition-1",
        b"source-row",
        b"edit-row",
    ];
    let token = signer.sign_version(&fields).unwrap();
    assert!(signer.verify_version(&token, &fields).is_ok());
    assert!(matches!(
        signer.verify_version("broken", &fields),
        Err(QueryError::InvalidToken)
    ));
    for i in 0..token.len() {
        let mut tampered = token.as_bytes().to_vec();
        tampered[i] = if tampered[i] == b'A' { b'B' } else { b'A' };
        assert!(
            matches!(
                signer.verify_version(std::str::from_utf8(&tampered).unwrap(), &fields),
                Err(QueryError::InvalidToken)
            ),
            "byte {i}"
        );
    }
    let rotated = TokenCodec::new(b"rotated-key".to_vec()).unwrap();
    assert!(matches!(
        rotated.verify_version(&token, &fields),
        Err(QueryError::InvalidToken)
    ));
}

#[test]
fn version_changes_with_principal_scope_definition_and_row() {
    let codec = TokenCodec::new(b"first-key".to_vec()).unwrap();
    let baseline = [
        b"principal".as_slice(),
        b"scope-a",
        b"Flight",
        b"F-1001",
        b"definition-1",
        b"source-row",
        b"edit-row",
    ];
    let token = codec.sign_version(&baseline).unwrap();
    assert_eq!(codec.sign_version(&baseline).unwrap(), token);
    for i in 0..baseline.len() {
        let mut changed = baseline;
        changed[i] = b"different";
        assert_ne!(
            codec.sign_version(&changed).unwrap(),
            token,
            "field {i} was not bound"
        );
        assert!(matches!(
            codec.verify_version(&token, &changed),
            Err(QueryError::InvalidToken)
        ));
    }
}

#[tokio::test]
async fn service_version_changes_with_scope() {
    let f = fixture().await;
    let active = f.store.active_definition().await.unwrap();
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "createAirport".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Airport"),
                pk: PropertyValue::String("BER".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    prop("name"),
                    PropertyValue::String("Berlin".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let engine = NeverEngine;
    let tokens = TokenCodec::new(b"first-key".to_vec()).unwrap();
    let first = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&active),
        engine: &engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", &["name"]))
    .await
    .unwrap();
    let mut narrower = f.scope.clone();
    narrower
        .types
        .get_mut(&name("Airport"))
        .unwrap()
        .columns
        .remove(&prop("city"));
    let second = QueryService {
        principal: &f.principal,
        scope: &narrower,
        active: Arc::clone(&active),
        engine: &engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", &["name"]))
    .await
    .unwrap();
    assert_ne!(first.rows[0].version, second.rows[0].version);
}

#[tokio::test]
async fn physical_names_never_escape() {
    let f = fixture().await;
    let tokens = TokenCodec::new(b"first-key".to_vec()).unwrap();
    let engine = FailingEngine;
    let err = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: f.store.active_definition().await.unwrap(),
        engine: &engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Flight", "F-1001", &["status"]))
    .await
    .unwrap_err();
    let outward = format!(
        "{} {} {}",
        err.code(),
        err,
        serde_json::to_string(&err).unwrap()
    );
    assert!(outward.contains("SOURCE_UNAVAILABLE"), "{outward}");
    for physical in [
        "ontoforge_edit",
        "of_obj_secret",
        "hidden_column",
        "20261022_very_secret",
    ] {
        assert!(!outward.contains(physical), "{outward}");
    }
}

#[test]
fn final_row_and_byte_caps_truncate() {
    let row = |id: &str| ObjectRow {
        object_type: name("Flight"),
        pk: json!(id),
        properties: BTreeMap::from([(prop("status"), json!("scheduled"))]),
        version: "opaque-token".to_owned(),
    };
    let one = row("F-1001");
    let two = row("F-1002");
    let bytes = serde_json::to_vec(&one).unwrap().len();
    let page = QueryPage::bounded(vec![one.clone(), two.clone()], 1, bytes * 3);
    assert_eq!(page.rows, vec![one.clone()]);
    assert!(page.truncated);
    let page = QueryPage::bounded(vec![one.clone(), two], 2, bytes);
    assert_eq!(page.rows, vec![one.clone()]);
    assert!(page.truncated);
    let page = QueryPage::bounded(vec![one.clone()], 1, bytes);
    assert_eq!(page.rows, vec![one]);
    assert!(!page.truncated);
    let page = QueryPage::bounded(vec![row("F-1001")], 1, bytes - 1);
    assert!(page.rows.is_empty());
    assert!(page.truncated);
}

#[tokio::test]
async fn pk_probe_is_indistinguishable() {
    let f = fixture().await;
    let tokens = TokenCodec::new(b"first-key".to_vec()).unwrap();
    let active = f.store.active_definition().await.unwrap();
    let missing = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "NONE", &["name"]))
    .await
    .unwrap_err();
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "deleteFlight".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Flight"),
                pk: PropertyValue::String("F-1001".to_owned()),
                op: ObjectOp::Tombstone,
            }],
        })
        .await
        .unwrap();
    let tombstone = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Flight", "F-1001", &["status"]))
    .await
    .unwrap_err();
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "releaseFlight".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Flight"),
                pk: PropertyValue::String("F-1001".to_owned()),
                op: ObjectOp::Release,
            }],
        })
        .await
        .unwrap();
    let row_false = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&active),
        engine: &RowsEngine("CANCELLED"),
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Flight", "F-1001", &["status"]))
    .await
    .unwrap_err();
    let hidden_column = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&active),
        engine: &RowsEngine("scheduled"),
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Flight", "F-1001", &["aircraftTailNumber"]))
    .await
    .unwrap_err();
    let hidden_type = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active,
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Pilot", "P-1", &["fullName"]))
    .await
    .unwrap_err();
    let shapes: Vec<_> = [missing, tombstone, row_false, hidden_column, hidden_type]
        .iter()
        .map(|e| (e.code(), e.to_string()))
        .collect();
    assert_eq!(
        shapes,
        vec![("NOT_FOUND", "object not found".to_owned()); 5]
    );
}
