#![allow(
    clippy::unused_async_trait_impl,
    reason = "counting fake Engine completes synchronously"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{
    ActionError, ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService,
};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_engine::printer::{Dialect, PrintedSql};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, QueryContext, QueryOutput, TableRef, Value as EngineValue,
};
use ontoforge_expr::{Operand, Pred, rowpred::Attrs};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::load_from_memory;
use ontoforge_storage::{
    commit::{Actor, Commit, Edit, ObjectOp},
    reload::{ReloadOptions, reload},
    store::{BackendSettings, EditStore, StoreSettings},
    value::PropertyValue,
};

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
const BINDING: &str = "kind: SourceBinding
target: { objectType: Gate }
source: { catalog: demo, schema: aviation, table: gates }
";

struct Fixture {
    _dir: tempfile::TempDir,
    store: EditStore,
    principal: Principal,
    scope: PrincipalScope,
    tokens: TokenCodec,
    version: i64,
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

async fn fixture(patched_status: bool) -> Fixture {
    fixture_with_defs(patched_status, DEFS).await
}

async fn fixture_with_defs(patched_status: bool, defs: &str) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("open store");
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("defs.yaml", defs),
        ("bindings/dev/gate.yaml", BINDING),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let ontology = loaded.ontology.expect("ontology");
    let version = reload(&store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload")
        .def_version
        .expect("version");
    let mut set = BTreeMap::from([(
        "name".to_owned().try_into().expect("property"),
        PropertyValue::String("Old".to_owned()),
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
        .expect("seed partial edit");
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
    scope
        .actions
        .insert("releaseGate".to_owned().try_into().expect("action"));
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
    let tokens = TokenCodec::new(b"source-test-key".to_vec()).expect("tokens");
    Fixture {
        _dir: dir,
        store,
        principal,
        scope,
        tokens,
        version,
    }
}

struct CountingEngine {
    rows: Vec<Vec<EngineValue>>,
    unavailable: bool,
    calls: AtomicUsize,
    sql: Mutex<Vec<String>>,
}

impl CountingEngine {
    fn rows(rows: Vec<Vec<EngineValue>>) -> Self {
        Self {
            rows,
            unavailable: false,
            calls: AtomicUsize::new(0),
            sql: Mutex::new(Vec::new()),
        }
    }
    fn unavailable() -> Self {
        Self {
            rows: Vec::new(),
            unavailable: true,
            calls: AtomicUsize::new(0),
            sql: Mutex::new(Vec::new()),
        }
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Engine for CountingEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        _: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.sql.lock().expect("sql").push(sql.as_str().to_owned());
        if self.unavailable {
            return Err(EngineError::Unavailable {
                retryable: true,
                reason: "secret endpoint".to_owned(),
            });
        }
        Ok(QueryOutput {
            columns: Vec::new(),
            rows: self.rows.clone(),
            truncated: false,
        })
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("source pk sampling must not describe tables")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

fn source_row(status: &str) -> Vec<EngineValue> {
    vec![
        EngineValue::String("G1".to_owned()),
        EngineValue::String("Old".to_owned()),
        EngineValue::String(status.to_owned()),
    ]
}

fn limits() -> ActionLimits {
    ActionLimits::new(8, 8, 65536).expect("limits")
}

#[tokio::test]
async fn source_required_offline_is_engine_unavailable_without_tx() {
    let f = fixture(false).await;
    let error = ActionService::new(&f.store, limits(), &f.tokens)
        .execute(
            &f.principal,
            &f.scope,
            f.version,
            request("required-source"),
        )
        .await
        .expect_err("missing required source must fail before transaction");
    assert_eq!(error.to_string(), "source unavailable");
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 1);
    assert!(
        f.store
            .action_command(f.principal.id.as_str(), "required-source")
            .await
            .expect("command read")
            .is_none()
    );
}

#[tokio::test]
async fn source_required_engine_failure_is_stable_and_pretransaction() {
    let f = fixture(false).await;
    let engine = CountingEngine::unavailable();
    let error = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&engine)
        .execute(&f.principal, &f.scope, f.version, request("outage"))
        .await
        .expect_err("outage");
    assert!(matches!(error, ActionError::EngineUnavailable));
    assert_eq!(engine.calls(), 1);
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 1);
    assert!(
        f.store
            .action_command(f.principal.id.as_str(), "outage")
            .await
            .expect("command")
            .is_none()
    );
}

#[tokio::test]
async fn source_duplicate_is_ambiguous_without_tx() {
    let f = fixture(false).await;
    let engine = CountingEngine::rows(vec![source_row("open"), source_row("open")]);
    let error = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&engine)
        .execute(&f.principal, &f.scope, f.version, request("duplicate"))
        .await
        .expect_err("duplicate source key");
    assert!(matches!(error, ActionError::AmbiguousObject));
    assert_eq!(engine.calls(), 1);
    assert!(engine.sql.lock().expect("sql")[0].contains("LIMIT 2"));
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 1);
}

#[tokio::test]
async fn source_unique_is_overlaid_once() {
    let f = fixture(false).await;
    let engine = CountingEngine::rows(vec![source_row("open")]);
    let result = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&engine)
        .execute(&f.principal, &f.scope, f.version, request("unique"))
        .await
        .expect("unique source row");
    assert!(matches!(result, ActionOutcome::Committed(_)));
    assert_eq!(engine.calls(), 1);
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 2);
}

#[tokio::test]
async fn all_patched_object_succeeds_with_engine_offline_and_zero_calls() {
    let f = fixture(true).await;
    let engine = CountingEngine::unavailable();
    let result = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&engine)
        .execute(&f.principal, &f.scope, f.version, request("patched"))
        .await
        .expect("all required fields are in edit overlay");
    assert!(matches!(result, ActionOutcome::Committed(_)));
    assert_eq!(engine.calls(), 0);
}

#[tokio::test]
async fn release_source_visibility_guard() {
    let defs = format!(
        "{DEFS}---\nkind: ActionType\napiName: releaseGate\nparameters:\n  gate: {{ type: {{ objectReference: Gate }}, required: true }}\nrules:\n  - kind: releaseObject\n    object: gate\n"
    );
    let f = fixture_with_defs(true, &defs).await;
    let request = |id: &str, validate_only: bool| ActionRequest {
        action: "releaseGate".to_owned(),
        command_id: Some(id.to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"gate":"G1"})).expect("params"),
            expected_versions: BTreeMap::new(),
        }],
        validate_only,
    };
    let hidden = CountingEngine::rows(vec![vec![
        EngineValue::String("G1".to_owned()),
        EngineValue::String("Old".to_owned()),
        EngineValue::Null,
    ]]);
    for (id, validate_only) in [("hidden", false), ("dry-hidden", true)] {
        let error = ActionService::new(&f.store, limits(), &f.tokens)
            .with_engine(&hidden)
            .execute(
                &f.principal,
                &f.scope,
                f.version,
                request(id, validate_only),
            )
            .await
            .expect_err("release must verify source post-state visibility");
        assert!(matches!(error, ActionError::ScopeDenied), "{error:?}");
        assert!(
            f.store
                .action_command(f.principal.id.as_str(), id)
                .await
                .expect("command")
                .is_none()
        );
    }
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 1);
    let visible = CountingEngine::rows(vec![source_row("open")]);
    let result = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&visible)
        .execute(&f.principal, &f.scope, f.version, request("release", false))
        .await
        .expect("visible source can be released");
    assert!(matches!(result, ActionOutcome::Committed(_)));
    let active = f.store.active_definition().await.expect("active");
    let edit = f
        .store
        .read_object_edit(
            &active,
            &"Gate".to_owned().try_into().expect("type"),
            &PropertyValue::String("G1".to_owned()),
        )
        .await
        .expect("edit read")
        .expect("edit row");
    assert!(edit.row.released);
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 2);
}

#[tokio::test]
async fn source_backed_query_token_roundtrip_and_source_drift() {
    let f = fixture(false).await;
    let query_engine = CountingEngine::rows(vec![source_row("open")]);
    let active = f.store.active_definition().await.expect("active");
    let query = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active,
        engine: &query_engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &f.tokens,
    };
    let page = query
        .execute(QueryRequest {
            set: ObjectSet::Pk {
                object_type: "Gate".to_owned().try_into().expect("type"),
                pk: serde_json::json!("G1"),
            },
            select: ["code", "name", "status"]
                .into_iter()
                .map(|name| name.to_owned().try_into().expect("property"))
                .collect(),
        })
        .await
        .expect("query token");
    let token = page.rows[0].version.clone();
    assert_eq!(query_engine.calls(), 1);

    let drifted = CountingEngine::rows(vec![source_row("closed")]);
    let mut stale = request("drifted-source");
    stale.items[0]
        .expected_versions
        .insert("gate".to_owned(), token.clone());
    let error = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&drifted)
        .execute(&f.principal, &f.scope, f.version, stale)
        .await
        .expect_err("source drift invalidates token");
    assert!(matches!(error, ActionError::VersionConflict), "{error:?}");
    assert_eq!(drifted.calls(), 1);
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 1);

    let matching = CountingEngine::rows(vec![source_row("open")]);
    let mut match_request = request("matching-source");
    match_request.items[0]
        .expected_versions
        .insert("gate".to_owned(), token);
    let committed = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&matching)
        .execute(&f.principal, &f.scope, f.version, match_request)
        .await
        .expect("Query token accepted by Action");
    assert!(matches!(committed, ActionOutcome::Committed(_)));
    assert_eq!(matching.calls(), 1, "commit guard must not resample Engine");
}

#[tokio::test]
async fn later_alias_criterion_samples_shared_source_key() {
    let defs = format!(
        "{DEFS}---\nkind: ActionType\napiName: renameByAlias\nparameters:\n  a: {{ type: {{ objectReference: Gate }}, required: true }}\n  z: {{ type: {{ objectReference: Gate }}, required: true }}\n  name: {{ type: string, required: true }}\nrules:\n  - kind: modifyObject\n    object: a\n    properties:\n      name: {{ param: name }}\nsubmissionCriteria:\n  - condition: 'objects.z.status == \"open\"'\n    failureMessage: source status required\n"
    );
    let mut f = fixture_with_defs(false, &defs).await;
    f.scope
        .actions
        .insert("renameByAlias".to_owned().try_into().expect("action"));
    f.scope.types.get_mut("Gate").expect("Gate scope").row = Pred::Const(true);
    let req = ActionRequest {
        action: "renameByAlias".to_owned(),
        command_id: Some("alias-criterion".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"a":"G1","z":"G1","name":"New"}))
                .expect("params"),
            expected_versions: BTreeMap::new(),
        }],
        validate_only: true,
    };
    let engine = CountingEngine::rows(vec![source_row("open")]);
    let outcome = ActionService::new(
        &f.store,
        limits().with_work_limits(2, 1).expect("one source call"),
        &f.tokens,
    )
    .with_engine(&engine)
    .execute(&f.principal, &f.scope, f.version, req)
    .await
    .expect("later criterion must see source status");
    assert!(matches!(outcome, ActionOutcome::Validated));
    assert_eq!(engine.calls(), 1);
}

#[tokio::test]
async fn later_alias_release_samples_shared_source_key() {
    let defs = format!(
        "{DEFS}---\nkind: ActionType\napiName: releaseByAlias\nparameters:\n  a: {{ type: {{ objectReference: Gate }}, required: true }}\n  z: {{ type: {{ objectReference: Gate }}, required: true }}\nrules:\n  - kind: releaseObject\n    object: z\n"
    );
    let mut f = fixture_with_defs(true, &defs).await;
    f.scope
        .actions
        .insert("releaseByAlias".to_owned().try_into().expect("action"));
    let req = ActionRequest {
        action: "releaseByAlias".to_owned(),
        command_id: Some("alias-release".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"a":"G1","z":"G1"})).expect("params"),
            expected_versions: BTreeMap::new(),
        }],
        validate_only: true,
    };
    let engine = CountingEngine::rows(vec![source_row("open")]);
    let outcome = ActionService::new(
        &f.store,
        limits().with_work_limits(2, 1).expect("one source call"),
        &f.tokens,
    )
    .with_engine(&engine)
    .execute(&f.principal, &f.scope, f.version, req)
    .await
    .expect("later release must sample source once");
    assert!(matches!(outcome, ActionOutcome::Validated));
    assert_eq!(engine.calls(), 1);
}

#[tokio::test]
async fn later_item_token_samples_shared_source_key() {
    let mut f = fixture(false).await;
    f.scope.types.get_mut("Gate").expect("Gate scope").row = Pred::Const(true);
    let query_engine = CountingEngine::rows(vec![source_row("open")]);
    let query = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: f.store.active_definition().await.expect("active"),
        engine: &query_engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &f.tokens,
    };
    let token = query
        .execute(QueryRequest {
            set: ObjectSet::Pk {
                object_type: "Gate".to_owned().try_into().expect("type"),
                pk: serde_json::json!("G1"),
            },
            select: ["code", "name", "status"]
                .into_iter()
                .map(|name| name.to_owned().try_into().expect("property"))
                .collect(),
        })
        .await
        .expect("query token")
        .rows[0]
        .version
        .clone();
    assert_eq!(query_engine.calls(), 1);

    let mut repeated = request("later-token");
    repeated.validate_only = true;
    let mut second = repeated.items[0].clone();
    second.expected_versions.insert("gate".to_owned(), token);
    repeated.items.push(second);
    let engine = CountingEngine::rows(vec![source_row("open")]);
    let outcome = ActionService::new(
        &f.store,
        limits().with_work_limits(2, 1).expect("one source call"),
        &f.tokens,
    )
    .with_engine(&engine)
    .execute(&f.principal, &f.scope, f.version, repeated)
    .await
    .expect("later token must see one source sample");
    assert!(matches!(outcome, ActionOutcome::Validated));
    assert_eq!(engine.calls(), 1);
}

#[tokio::test]
async fn batch_deduplicates_source_keys_and_enforces_limit() {
    let f = fixture(false).await;
    let engine = CountingEngine::rows(vec![source_row("open")]);
    let mut repeated = request("repeated");
    repeated.validate_only = true;
    repeated.items.push(repeated.items[0].clone());
    let result = ActionService::new(&f.store, limits(), &f.tokens)
        .with_engine(&engine)
        .execute(&f.principal, &f.scope, f.version, repeated)
        .await
        .expect("same key is sampled once");
    assert!(matches!(result, ActionOutcome::Validated));
    assert_eq!(engine.calls(), 1);

    let mut over_limit = request("over-limit");
    let mut second = over_limit.items[0].clone();
    second
        .params
        .insert("gate".to_owned(), serde_json::json!("G2"));
    over_limit.items.push(second);
    let capped = CountingEngine::rows(vec![source_row("open")]);
    let error = ActionService::new(
        &f.store,
        limits().with_work_limits(2, 1).expect("work limits"),
        &f.tokens,
    )
    .with_engine(&capped)
    .execute(&f.principal, &f.scope, f.version, over_limit)
    .await
    .expect_err("second source call exceeds host limit");
    assert!(matches!(error, ActionError::InvalidRequest(_)), "{error:?}");
    assert_eq!(
        capped.calls(),
        1,
        "limit must reject before the next Engine call"
    );
    assert!(
        f.store
            .action_command(f.principal.id.as_str(), "over-limit")
            .await
            .expect("command")
            .is_none()
    );
    assert_eq!(f.store.read_log(0, 10).await.expect("log").len(), 1);
}

#[cfg(feature = "embedded-pg")]
#[path = "../../ontoforge-storage/tests/support/mod.rs"]
mod support;

#[cfg(feature = "embedded-pg")]
#[tokio::test]
async fn source_guard_absent_target_insert_race() {
    let pg = support::pg_store().await;
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("defs.yaml", DEFS),
        ("bindings/dev/gate.yaml", BINDING),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &pg.store,
        &loaded.ontology.expect("ontology"),
        &ReloadOptions::default(),
    )
    .await
    .expect("reload")
    .def_version
    .expect("version");
    let auth = fixture(true).await;
    let other = pg.open_another(Duration::from_secs(5)).await;
    let engine = CountingEngine::rows(vec![source_row("open")]);
    let error = Box::pin(
        ActionService::new(&pg.store, limits(), &auth.tokens)
            .with_engine(&engine)
            .execute_with_test_gate(
                &auth.principal,
                &auth.scope,
                version,
                request("raced"),
                async {
                    other
                        .commit(Commit {
                            actor: Actor::Principal("test|concurrent".to_owned()),
                            action: "concurrent".to_owned(),
                            def_version: version,
                            params: serde_json::json!({}),
                            edits: vec![Edit::Object {
                                type_name: "Gate".to_owned().try_into().expect("type"),
                                pk: PropertyValue::String("G1".to_owned()),
                                op: ObjectOp::Set(BTreeMap::from([(
                                    "name".to_owned().try_into().expect("property"),
                                    PropertyValue::String("Concurrent".to_owned()),
                                )])),
                            }],
                        })
                        .await
                        .expect("independent insert");
                },
            ),
    )
    .await
    .expect_err("lock guard must reject inserted edit row");
    assert!(matches!(error, ActionError::VersionConflict), "{error:?}");
    assert_eq!(engine.calls(), 1);
    assert_eq!(pg.store.read_log(0, 10).await.expect("log").len(), 1);
    assert!(
        pg.store
            .action_command(auth.principal.id.as_str(), "raced")
            .await
            .expect("command")
            .is_none()
    );
    let outbox: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM \"{}\".of_webhook_outbox",
        pg.schema_sys
    )))
    .fetch_one(pg.side())
    .await
    .expect("outbox count");
    assert_eq!(outbox, 0);
}
