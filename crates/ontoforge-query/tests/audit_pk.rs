use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::printer::{Dialect, PrintedSql};
use ontoforge_engine::{Engine, EngineError, EngineStatus, QueryContext, QueryOutput, TableRef};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use serde_json::json;
use sqlx::Row;

const AVIATION: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation");
const SCOPE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/scopes/aviation");

fn name(value: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(value.to_owned()).unwrap()
}

fn prop(value: &str) -> PropertyName {
    PropertyName::try_from(value.to_owned()).unwrap()
}

struct NeverEngine;

impl Engine for NeverEngine {
    fn dialect(&self) -> Dialect { Dialect::Sqlite }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        panic!("edit-only pk read must not call the source engine")
    }
    async fn describe_tables(
        &self, _: &[TableRef], _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("pk read must not describe source tables")
    }
    async fn status(&self) -> EngineStatus { EngineStatus::Available }
}

#[tokio::test]
async fn audit_pk_success_records_one_metadata_row() {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    }).await.unwrap();
    let ontology = load_dir(Utf8Path::new(AVIATION)).ontology.unwrap();
    reload(&store, &ontology, &ReloadOptions::default()).await.unwrap();
    let scope_dir = load_scope_dir(Utf8Path::new(SCOPE));
    let attrs = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    })).unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: Some(&ProfileName::try_from("dev".to_owned()).unwrap()),
        attrs: &attr_types(Some(&attrs)),
        scope: &scope_dir.scope.unwrap(),
        sources: Some(&scope_dir.sources),
    }).unwrap();
    let principal = Principal {
        id: PrincipalId::new("ontoforge-dev", "alice"),
        issuer: "ontoforge-dev".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::from(["dispatchers".to_owned()]),
        attrs: Default::default(),
    };
    let scope = compiled.for_principal(&principal);
    let active = store.active_definition().await.unwrap();
    store.commit(Commit {
        actor: Actor::Principal(principal.id.to_string()),
        action: "createAirport".to_owned(),
        def_version: active.def_version,
        params: json!({}),
        edits: vec![Edit::Object {
            type_name: name("Airport"),
            pk: PropertyValue::String("BER".to_owned()),
            op: ObjectOp::Set(BTreeMap::from([(
                prop("name"), PropertyValue::String("Berlin".to_owned()),
            )])),
        }],
    }).await.unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let page = QueryService {
        principal: &principal, scope: &scope, active: Arc::clone(&active),
        engine: &NeverEngine, store: &store,
        limits: ExecutionLimits::phase4_default(), tokens: &tokens,
    }.execute(QueryRequest {
        set: ObjectSet::Pk { object_type: name("Airport"), pk: json!("BER") },
        select: vec![prop("name")],
    }).await.unwrap();
    assert_eq!(page.rows.len(), 1);

    let pool = sqlx::SqlitePool::connect_with(
        ontoforge_storage::store::sqlite_connect_options(&path),
    ).await.unwrap();
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'of_read_audit'",
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(table_count, 1, "pk read must have a durable audit table");
    let rows = sqlx::query("SELECT principal, tool, object_type, row_count, status FROM of_read_audit")
        .fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<String, _>("principal"), principal.id.as_str());
    assert_eq!(rows[0].get::<String, _>("tool"), "query.pk");
    assert_eq!(rows[0].get::<String, _>("object_type"), "Airport");
    assert_eq!(rows[0].get::<i64, _>("row_count"), 1);
    assert_eq!(rows[0].get::<String, _>("status"), "success");
}
