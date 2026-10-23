use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionError, ActionItem, ActionLimits, ActionRequest, ActionService};
use ontoforge_auth::{Principal, PrincipalId, scope::{PrincipalScope, TypeScope}};
use ontoforge_expr::{Pred, rowpred::Attrs};
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
submissionCriteria:
  - condition: 'objects.gate.name == \"Allowed\"'
    failureMessage: criterion was false
";

async fn setup() -> (tempfile::TempDir, EditStore, i64, Principal) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    }).await.expect("open store");
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(&store, &loaded.ontology.expect("ontology"), &ReloadOptions::default())
        .await.expect("reload").def_version.expect("version");
    store.commit(Commit {
        actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
        action: "seed".to_owned(), def_version: version, params: serde_json::Value::Null,
        edits: vec![Edit::Object {
            type_name: "Gate".to_owned().try_into().expect("type"),
            pk: PropertyValue::String("A1".to_owned()),
            op: ObjectOp::Set(BTreeMap::from([(
                "name".to_owned().try_into().expect("property"),
                PropertyValue::String("Allowed".to_owned()),
            )])),
        }],
    }).await.expect("seed");
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(), groups: BTreeSet::new(), attrs: Attrs::new(),
    };
    (dir, store, version, principal)
}

fn scope(visible: bool, name_visible: bool) -> PrincipalScope {
    let mut scope = PrincipalScope::default();
    scope.actions.insert("renameGate".to_owned().try_into().expect("action"));
    let mut columns = BTreeMap::new();
    columns.insert("code".to_owned().try_into().expect("column"), Pred::Const(true));
    if name_visible {
        columns.insert("name".to_owned().try_into().expect("column"), Pred::Const(true));
    }
    scope.types.insert("Gate".to_owned().try_into().expect("type"), TypeScope {
        row: Pred::Const(visible), columns,
    });
    scope
}

fn request(pk: &str, id: &str) -> ActionRequest {
    ActionRequest {
        action: "renameGate".to_owned(), command_id: Some(id.to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({
                "gate": pk, "name": "Updated"
            })).expect("params"),
            expected_versions: BTreeMap::default(),
        }], validate_only: false,
    }
}

#[tokio::test]
async fn scope_hidden_target_matches_missing_outward_shape() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let hidden = service.execute(&principal, &scope(false, true), version,
        request("A1", "hidden")).await.expect_err("hidden target must be rejected");
    let missing = service.execute(&principal, &scope(false, true), version,
        request("Z9", "missing")).await.expect_err("missing target must be rejected");
    assert_eq!(hidden.to_string(), missing.to_string());
    assert!(!hidden.to_string().contains("A1"));
    assert!(!hidden.to_string().contains("Gate"));
}

#[tokio::test]
async fn scope_masked_criteria_is_denied_before_evaluation() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let outcome = service.execute(&principal, &scope(true, false), version,
        request("A1", "masked")).await;
    assert!(matches!(outcome, Err(ActionError::ScopeDenied)),
        "masked criterion must be denied, not evaluated: {outcome:?}");
}
