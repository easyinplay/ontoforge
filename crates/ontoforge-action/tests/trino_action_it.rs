#![allow(
    clippy::unused_async_trait_impl,
    reason = "instrumented Engine delegates to Trino"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camino::Utf8Path;
use ontoforge_action::{
    ActionError, ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService,
};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::printer::{Dialect, PrintedSql};
use ontoforge_engine::{
    AnyEngine, Engine, EngineError, EngineStatus, PhysicalColumn, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_expr::{Operand, Pred, rowpred::Attrs};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService};
use ontoforge_schema::load_from_memory;
use ontoforge_storage::{
    commit::{Actor, Commit, Edit, ObjectOp},
    reload::{ReloadOptions, reload},
    store::{BackendSettings, EditStore, StoreSettings},
    value::PropertyValue,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: source
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
  status: { type: string, required: true }
---
kind: ActionType
apiName: renameGate
parameters:
  gate: { type: { objectReference: Gate }, required: true }
  name: { type: string, required: true }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
";

fn required(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => panic!("{name} is required for real Trino/PostgreSQL Action tests"),
    }
}

fn engine(url: &str) -> RecordingEngine {
    let config = parse_server_config(&format!(
        "[trino]\nurl = \"{url}\"\nconnect_timeout_ms = 1000\n\n[trino.identities.ontology]\nuser = \"ontoforge\"\nauth = \"none\"\n"
    )).expect("Trino configuration");
    RecordingEngine {
        inner: AnyEngine::Trino(
            build_trino_engines(
                &config.trino.expect("Trino config"),
                Utf8Path::new("."),
                &|_| None,
            )
            .expect("Trino engine")
            .ontology,
        ),
        calls: AtomicUsize::new(0),
        sql: Mutex::new(Vec::new()),
    }
}

struct RecordingEngine {
    inner: AnyEngine,
    calls: AtomicUsize,
    sql: Mutex<Vec<String>>,
}

impl RecordingEngine {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Engine for RecordingEngine {
    fn dialect(&self) -> Dialect {
        self.inner.dialect()
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.sql.lock().expect("sql").push(sql.as_str().to_owned());
        self.inner.execute(sql, ctx).await
    }
    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.describe_tables(tables, ctx).await
    }
    async fn status(&self) -> EngineStatus {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.status().await
    }
}

struct Fixture {
    store: EditStore,
    pool: sqlx::PgPool,
    demo: sqlx::PgPool,
    source_schema: String,
    edit_schema: String,
    sys_schema: String,
    version: i64,
    principal: Principal,
    scope: PrincipalScope,
    tokens: ontoforge_query::TokenCodec,
}

impl Fixture {
    async fn count(&self, table: &str) -> i64 {
        let sql = format!("SELECT count(*) FROM \"{}\".{table}", self.sys_schema);
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_one(&self.pool)
            .await
            .expect("artifact count")
    }

    async fn cleanup(self) {
        for (pool, schema) in [
            (&self.demo, &self.source_schema),
            (&self.pool, &self.edit_schema),
            (&self.pool, &self.sys_schema),
        ] {
            let sql = format!("DROP SCHEMA \"{schema}\" CASCADE");
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .execute(pool)
                .await
                .expect("drop test schema");
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "real source and edit schemas need isolated setup"
)]
async fn fixture(patched_status: bool, duplicate: bool) -> Fixture {
    let _trino_url = required("ONTOFORGE_TRINO_URL");
    let pg_url = required("ONTOFORGE_PG_URL");
    let options = PgConnectOptions::from_str(&pg_url).expect("PostgreSQL URL");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options.clone())
        .await
        .expect("editstore PostgreSQL");
    let demo = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options.database("demo"))
        .await
        .expect("demo PostgreSQL");
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let source_schema = format!("of_action_{suffix}");
    let edit_schema = format!("{source_schema}_edit");
    let sys_schema = format!("{source_schema}_sys");
    let create_schema = format!("CREATE SCHEMA \"{source_schema}\"");
    sqlx::query(sqlx::AssertSqlSafe(create_schema))
        .execute(&demo)
        .await
        .expect("source schema");
    let create_table =
        format!("CREATE TABLE \"{source_schema}\".gates (code text, name text, status text)");
    sqlx::query(sqlx::AssertSqlSafe(create_table))
        .execute(&demo)
        .await
        .expect("source table");
    let insert = format!(
        "INSERT INTO \"{source_schema}\".gates (code, name, status) VALUES ('G1', 'Source', 'open')"
    );
    sqlx::query(sqlx::AssertSqlSafe(insert.clone()))
        .execute(&demo)
        .await
        .expect("source row");
    if duplicate {
        sqlx::query(sqlx::AssertSqlSafe(insert))
            .execute(&demo)
            .await
            .expect("duplicate source row");
    }
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Postgres {
            url: secrecy::SecretString::from(pg_url),
            schema_edit: edit_schema.clone(),
            schema_sys: sys_schema.clone(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("real edit store");
    let binding = format!(
        "kind: SourceBinding\ntarget: {{ objectType: Gate }}\nsource: {{ catalog: demo, schema: {source_schema}, table: gates }}\n"
    );
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("defs.yaml", DEFS),
        ("bindings/dev/gate.yaml", &binding),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &store,
        &loaded.ontology.expect("ontology"),
        &ReloadOptions::default(),
    )
    .await
    .expect("reload")
    .def_version
    .expect("version");
    let mut set = BTreeMap::from([(
        "name".to_owned().try_into().expect("property"),
        PropertyValue::String("Patch".to_owned()),
    )]);
    if patched_status {
        set.insert(
            "status".to_owned().try_into().expect("property"),
            PropertyValue::String("open".to_owned()),
        );
    }
    store
        .commit(Commit {
            actor: Actor::Principal("test|seed".to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::json!({}),
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().expect("type"),
                pk: PropertyValue::String("G1".to_owned()),
                op: ObjectOp::Set(set),
            }],
        })
        .await
        .expect("partial edit");
    let principal = Principal {
        id: PrincipalId::new("test", "alice"),
        issuer: "test".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("renameGate".to_owned().try_into().expect("action"));
    scope.types.insert(
        "Gate".to_owned().try_into().expect("type"),
        TypeScope {
            row: Pred::IsNull {
                operand: Operand::Column("status".to_owned().try_into().expect("property")),
                negated: true,
            },
            columns: ["code", "name", "status"]
                .into_iter()
                .map(|name| {
                    (
                        name.to_owned().try_into().expect("property"),
                        Pred::Const(true),
                    )
                })
                .collect(),
        },
    );
    let tokens =
        ontoforge_query::TokenCodec::new(b"trino-action-test-key".to_vec()).expect("tokens");
    Fixture {
        store,
        pool,
        demo,
        source_schema,
        edit_schema,
        sys_schema,
        version,
        principal,
        scope,
        tokens,
    }
}

fn request(id: &str) -> ActionRequest {
    ActionRequest {
        action: "renameGate".to_owned(),
        command_id: Some(id.to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"gate":"G1","name":"New"}))
                .expect("params"),
            expected_versions: BTreeMap::new(),
        }],
        validate_only: false,
    }
}

fn limits() -> ActionLimits {
    ActionLimits::new(8, 8, 65536).expect("limits")
}

async fn assert_no_artifacts(f: &Fixture, id: &str) {
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 1);
    assert!(
        f.store
            .action_command(f.principal.id.as_str(), id)
            .await
            .expect("command")
            .is_none()
    );
    assert_eq!(f.count("of_action_command").await, 0);
    assert_eq!(f.count("of_webhook_outbox").await, 0);
    let active = f.store.active_definition().await.expect("active");
    let edit = f
        .store
        .read_object_edit(
            &active,
            &"Gate".to_owned().try_into().expect("type"),
            &PropertyValue::String("G1".to_owned()),
        )
        .await
        .expect("read edit")
        .expect("seed edit");
    assert_eq!(edit.row.version, 1);
    assert_eq!(
        edit.row.set["name"],
        PropertyValue::String("Patch".to_owned())
    );
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn trino_action_unique_source_executes() {
    let f = fixture(false, false).await;
    let e = engine(&required("ONTOFORGE_TRINO_URL"));
    let outcome = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&e)
        .execute(&f.principal, &f.scope, f.version, request("unique"))
        .await
        .expect("source merge and write");
    assert!(matches!(outcome, ActionOutcome::Committed(_)));
    assert_eq!(e.calls(), 1);
    let active = f.store.active_definition().await.expect("active");
    let edit = f
        .store
        .read_object_edit(
            &active,
            &"Gate".to_owned().try_into().expect("type"),
            &PropertyValue::String("G1".to_owned()),
        )
        .await
        .expect("read edit")
        .expect("edit row");
    assert_eq!(edit.row.version, 2);
    assert_eq!(
        edit.row.set["name"],
        PropertyValue::String("New".to_owned())
    );
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 2);
    assert!(
        f.store
            .action_command(f.principal.id.as_str(), "unique")
            .await
            .expect("command")
            .is_some()
    );
    assert_eq!(f.count("of_action_command").await, 1);
    assert_eq!(f.count("of_webhook_outbox").await, 1);
    let query = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active,
        engine: &e,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &f.tokens,
    };
    let row = query
        .execute(QueryRequest {
            set: ObjectSet::Pk {
                object_type: "Gate".to_owned().try_into().expect("type"),
                pk: serde_json::json!("G1"),
            },
            select: ["name", "status"]
                .into_iter()
                .map(|name| name.to_owned().try_into().expect("property"))
                .collect(),
        })
        .await
        .expect("post-Action Query overlay")
        .rows
        .remove(0);
    assert_eq!(row.properties["name"], serde_json::json!("New"));
    assert_eq!(row.properties["status"], serde_json::json!("open"));
    f.cleanup().await;
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn trino_action_source_unavailable_is_engine_unavailable() {
    let f = fixture(false, false).await;
    let e = engine("http://127.0.0.1:1");
    let error = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&e)
        .execute(&f.principal, &f.scope, f.version, request("outage"))
        .await
        .expect_err("required source outage");
    assert!(matches!(error, ActionError::EngineUnavailable), "{error:?}");
    let outward = error.to_string();
    for secret in [
        "127.0.0.1",
        "demo",
        "gates",
        "SELECT",
        "connection",
        &f.source_schema,
    ] {
        assert!(!outward.contains(secret), "source error leaked {secret}");
    }
    assert_eq!(e.calls(), 1);
    assert_no_artifacts(&f, "outage").await;
    f.cleanup().await;
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn trino_action_duplicate_pk_is_ambiguous() {
    let f = fixture(false, true).await;
    let e = engine(&required("ONTOFORGE_TRINO_URL"));
    let error = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&e)
        .execute(&f.principal, &f.scope, f.version, request("duplicate"))
        .await
        .expect_err("observed duplicate");
    assert!(matches!(error, ActionError::AmbiguousObject), "{error:?}");
    assert_eq!(e.calls(), 1);
    assert!(e.sql.lock().expect("sql")[0].contains("LIMIT 2"));
    assert_no_artifacts(&f, "duplicate").await;
    f.cleanup().await;
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_TRINO_URL and ONTOFORGE_PG_URL"]
async fn trino_action_all_patched_does_not_contact_trino() {
    let f = fixture(true, false).await;
    let e = engine("http://127.0.0.1:1");
    let outcome = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&e)
        .execute(&f.principal, &f.scope, f.version, request("patched"))
        .await
        .expect("fully patched offline Action");
    assert!(matches!(outcome, ActionOutcome::Committed(_)));
    assert_eq!(e.calls(), 0);
    assert_eq!(f.count("of_action_command").await, 1);
    assert_eq!(f.count("of_webhook_outbox").await, 1);
    f.cleanup().await;
}
