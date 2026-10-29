use std::collections::{BTreeMap, BTreeSet};

use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::{
    commit::{Actor, Commit, Edit, ObjectOp},
    reload::{ReloadOptions, reload},
    value::PropertyValue,
};

#[path = "../../ontoforge-storage/tests/support/mod.rs"]
mod support;

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
  name: { type: string, default: Default }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
";

struct Fixture {
    db: support::TestStore,
    principal: Principal,
    scope: PrincipalScope,
    active: ontoforge_storage::store::ActiveDefinition,
    tokens: TokenCodec,
}

async fn fixture() -> Fixture {
    let db = support::sqlite_store().await;
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let ontology = loaded.ontology.unwrap();
    let version = reload(&db.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap()
        .def_version
        .unwrap();
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    db.store
        .commit(Commit {
            actor: Actor::Principal(principal.id.as_str().to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().unwrap(),
                pk: PropertyValue::String("A1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().unwrap(),
                    PropertyValue::String("Original".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("renameGate".to_owned().try_into().unwrap());
    scope.types.insert(
        "Gate".to_owned().try_into().unwrap(),
        TypeScope {
            row: Pred::Const(true),
            columns: ["code", "name"]
                .into_iter()
                .map(|name| (name.to_owned().try_into().unwrap(), Pred::Const(true)))
                .collect(),
        },
    );
    let active = (*db.store.active_definition().await.unwrap()).clone();
    Fixture {
        db,
        principal,
        scope,
        active,
        tokens: TokenCodec::new(b"frozen-definition".to_vec()).unwrap(),
    }
}

fn request(id: &str, validate_only: bool) -> ActionRequest {
    ActionRequest {
        action: "renameGate".to_owned(),
        command_id: (!validate_only).then(|| id.to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"gate": "A1", "name": "New"}))
                .unwrap(),
            expected_versions: BTreeMap::new(),
        }],
        validate_only,
    }
}

fn receipt(outcome: ActionOutcome) -> ontoforge_action::ActionReceipt {
    let ActionOutcome::Committed(receipt) = outcome else {
        panic!("expected committed")
    };
    receipt
}

#[tokio::test]
async fn frozen_action_stale_and_validate_keep_fence_cas_guard() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let mut changed = f.active.ontology.clone();
    changed.description = Some("new definition".to_owned());
    reload(&f.db.store, &changed, &ReloadOptions::default())
        .await
        .unwrap();
    let error = service
        .execute_with_snapshot(&f.principal, &f.scope, &f.active, request("stale", false))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ontoforge_action::ActionError::Commit(ontoforge_storage::commit::ActionCommitError::Edit(
            ontoforge_storage::commit::EditError::DefinitionStale { .. }
        ))
    ));
    let pool = f.db.side_pool().await;
    let edits: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(edits, 1, "only the seed edit exists");
    let dry = service
        .execute_with_snapshot(&f.principal, &f.scope, &f.active, request("dry", true))
        .await
        .unwrap_err();
    assert!(matches!(
        dry,
        ontoforge_action::ActionError::Commit(ontoforge_storage::commit::ActionCommitError::Edit(
            ontoforge_storage::commit::EditError::DefinitionStale { .. }
        ))
    ));
}

#[tokio::test]
async fn frozen_replay_uses_current_grant_keeps_original_receipt() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let original = receipt(
        service
            .execute(
                &f.principal,
                &f.scope,
                f.active.def_version,
                request("replay", false),
            )
            .await
            .unwrap(),
    );
    let mut changed = f.active.ontology.clone();
    changed.description = Some("reloaded".to_owned());
    reload(&f.db.store, &changed, &ReloadOptions::default())
        .await
        .unwrap();
    let current =
        f.db.store
            .current_action_definition()
            .await
            .unwrap()
            .unwrap();
    let replay = service
        .get_command_result_with_snapshot(&f.principal, &f.scope, &current, "renameGate", "replay")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay, original);
    assert!(
        service
            .get_command_result_with_snapshot(
                &f.principal,
                &PrincipalScope::default(),
                &current,
                "renameGate",
                "replay"
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn replay_across_clock_and_definition_change_keeps_winner_time() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let first = receipt(
        service
            .execute_with_snapshot(&f.principal, &f.scope, &f.active, request("clock", false))
            .await
            .unwrap(),
    );
    let changed = f.active.ontology.clone();
    reload(&f.db.store, &changed, &ReloadOptions::default())
        .await
        .unwrap();
    let current =
        f.db.store
            .current_action_definition()
            .await
            .unwrap()
            .unwrap();
    let replay = service
        .get_command_result_with_snapshot(&f.principal, &f.scope, &current, "renameGate", "clock")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay, first);
}
