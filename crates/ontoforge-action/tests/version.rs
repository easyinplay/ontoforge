use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionItem, ActionLimits, ActionRequest, ActionService};
use ontoforge_auth::{Principal, PrincipalId, scope::{PrincipalScope, TypeScope}};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_schema::load_from_memory;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;

const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
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

async fn fixture() -> (tempfile::TempDir, EditStore, i64, Principal, PrincipalScope) {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    }).await.unwrap();
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let def_version = reload(&store, &loaded.ontology.unwrap(), &ReloadOptions::default())
        .await.unwrap().def_version.unwrap();
    store.commit(Commit {
        actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
        action: "seed".to_owned(),
        def_version,
        params: serde_json::Value::Null,
        edits: vec![Edit::Object {
            type_name: "Gate".to_owned().try_into().unwrap(),
            pk: PropertyValue::String("A1".to_owned()),
            op: ObjectOp::Set(BTreeMap::from([(
                "name".to_owned().try_into().unwrap(),
                PropertyValue::String("Allowed".to_owned()),
            )])),
        }],
    }).await.unwrap();
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    let mut scope = PrincipalScope::default();
    scope.actions.insert("renameGate".to_owned().try_into().unwrap());
    scope.types.insert("Gate".to_owned().try_into().unwrap(), TypeScope {
        row: Pred::Const(true),
        columns: ["code", "name"].into_iter().map(|name|
            (name.to_owned().try_into().unwrap(), Pred::Const(true))).collect(),
    });
    (dir, store, def_version, principal, scope)
}

fn request(token: &str) -> ActionRequest {
    ActionRequest {
        action: "renameGate".to_owned(),
        command_id: Some("bad-token".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({ "gate": "A1", "name": "Updated" })).unwrap(),
            expected_versions: BTreeMap::from([("gate".to_owned(), token.to_owned())]),
        }],
        validate_only: false,
    }
}

#[tokio::test]
async fn token_malformed_is_rejected_without_writes() {
    let (_dir, store, version, principal, scope) = fixture().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).unwrap());
    let error = service.execute(&principal, &scope, version, request("v1.bogus"))
        .await.expect_err("a forged token must not commit");
    assert_eq!(error.to_string(), "the observed edit version changed");
    let active = store.active_definition().await.unwrap();
    let row = store.read_object_edit(&active, &"Gate".to_owned().try_into().unwrap(),
        &PropertyValue::String("A1".to_owned())).await.unwrap().unwrap();
    assert_eq!(row.row.version, 1);
    assert_eq!(store.read_log(0, 10).await.unwrap().len(), 1);
}
