use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionItem, ActionLimits, ActionRequest, ActionService};
use ontoforge_auth::{Principal, PrincipalId, scope::{PrincipalScope, TypeScope}};
use ontoforge_expr::{Pred, rowpred::Attrs};
use ontoforge_query::TokenCodec;
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

#[tokio::test]
async fn source_required_offline_is_engine_unavailable_without_tx() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    }).await.expect("open store");
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("defs.yaml", DEFS),
        ("bindings/dev/gate.yaml", BINDING),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let ontology = loaded.ontology.expect("ontology");
    let version = reload(&store, &ontology, &ReloadOptions::default())
        .await.expect("reload").def_version.expect("version");
    store.commit(Commit {
        actor: Actor::Principal("test|seed".to_owned()),
        action: "seed".to_owned(),
        def_version: version,
        params: serde_json::json!({}),
        edits: vec![Edit::Object {
            type_name: "Gate".to_owned().try_into().expect("type"),
            pk: PropertyValue::String("G1".to_owned()),
            op: ObjectOp::Set(BTreeMap::from([(
                "name".to_owned().try_into().expect("property"),
                PropertyValue::String("Old".to_owned()),
            )])),
        }],
    }).await.expect("seed partial edit");
    let principal = Principal {
        id: PrincipalId::new("test", "alice"),
        issuer: "test".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let mut scope = PrincipalScope::default();
    scope.actions.insert("renameGate".to_owned().try_into().expect("action"));
    scope.types.insert("Gate".to_owned().try_into().expect("type"), TypeScope {
        row: Pred::Const(true),
        columns: ["code", "name", "status"].into_iter().map(|name| (
            name.to_owned().try_into().expect("property"), Pred::Const(true),
        )).collect(),
    });
    let tokens = TokenCodec::new(b"source-test-key".to_vec()).expect("tokens");
    let request = ActionRequest {
        action: "renameGate".to_owned(),
        command_id: Some("required-source".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"gate":"G1","name":"New"})).expect("params"),
            expected_versions: BTreeMap::new(),
        }],
        validate_only: false,
    };
    let error = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"), &tokens)
        .execute(&principal, &scope, version, request)
        .await.expect_err("missing required source must fail before transaction");
    assert_eq!(error.to_string(), "source unavailable");
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 1);
    assert!(store.action_command(principal.id.as_str(), "required-source")
        .await.expect("command read").is_none());
}
