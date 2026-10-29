use std::collections::{BTreeMap, BTreeSet};

use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Attrs, CmpOp, Operand, Pred, Scalar};
use ontoforge_query::{
    TokenCodec,
    cursor::{VersionClaims, VersionTokenCodec},
};
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

#[derive(Debug, PartialEq)]
struct BusinessState {
    edits: Vec<(String, Option<String>, i64)>,
    counts: (i64, i64, i64, i64),
}

async fn business_state(f: &Fixture) -> BusinessState {
    let pool = f.db.side_pool().await;
    let edits = sqlx::query_as("SELECT code, name, of_version FROM of_obj_gate ORDER BY code")
        .fetch_all(&pool)
        .await
        .unwrap();
    let counts = sqlx::query_as("SELECT (SELECT count(*) FROM of_edit_log), (SELECT count(*) FROM of_edit_log_entry), (SELECT count(*) FROM of_action_command), (SELECT count(*) FROM of_webhook_outbox)")
        .fetch_one(&pool).await.unwrap();
    BusinessState { edits, counts }
}

async fn validate_success_cas_guard(f: &Fixture, service: &ActionService<'_>) {
    let ty = "Gate".to_owned().try_into().unwrap();
    let pk = PropertyValue::String("A1".to_owned());
    let row =
        f.db.store
            .read_object_edit(&f.active, &ty, &pk)
            .await
            .unwrap()
            .unwrap();
    let token = VersionTokenCodec::new(&f.tokens)
        .sign(&VersionClaims {
            principal: &f.principal,
            scope: &f.scope,
            object_type: &ty,
            pk: &pk,
            def_version: f.active.def_version,
            source: None,
            edit: Some(&row.row),
        })
        .unwrap();
    let mut valid = request("dry-success", true);
    valid.items[0]
        .expected_versions
        .insert("gate".to_owned(), token);
    let before = business_state(f).await;
    assert_eq!(
        service
            .execute_with_snapshot(&f.principal, &f.scope, &f.active, valid.clone())
            .await
            .unwrap(),
        ActionOutcome::Validated
    );
    assert_eq!(
        business_state(f).await,
        before,
        "successful validation writes no business state"
    );

    let mut guarded = f.scope.clone();
    guarded.types.get_mut("Gate").unwrap().row = Pred::Cmp {
        left: Operand::Column("name".to_owned().try_into().unwrap()),
        op: CmpOp::Eq,
        right: Operand::Lit(Scalar::String("Original".to_owned())),
    };
    let before = business_state(f).await;
    assert!(
        matches!(
            service
                .execute_with_snapshot(
                    &f.principal,
                    &guarded,
                    &f.active,
                    request("dry-guard", true)
                )
                .await,
            Err(ontoforge_action::ActionError::ScopeDenied)
        ),
        "post-state visibility guard rejects New while Original is visible"
    );
    assert_eq!(
        business_state(f).await,
        before,
        "guard rejection writes no business state"
    );

    let external = f.db.reopen().await;
    external
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.as_str().to_owned()),
            action: "external edit".to_owned(),
            def_version: f.active.def_version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: ty,
                pk,
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().unwrap(),
                    PropertyValue::String("External".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let before = business_state(f).await;
    assert!(
        matches!(
            service
                .execute_with_snapshot(&f.principal, &f.scope, &f.active, valid)
                .await,
            Err(ontoforge_action::ActionError::VersionConflict)
        ),
        "valid signed pre-edit token must fail CAS after external edit"
    );
    assert_eq!(
        business_state(f).await,
        before,
        "CAS rejection writes no business state"
    );
}

#[tokio::test]
async fn frozen_action_stale_and_validate_keep_fence_cas_guard() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    validate_success_cas_guard(&f, &service).await;
    let mut changed = f.active.ontology.clone();
    changed.description = Some("new definition".to_owned());
    let external = f.db.reopen().await;
    reload(&external, &changed, &ReloadOptions::default())
        .await
        .unwrap();
    let before = business_state(&f).await;
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
    assert_eq!(
        business_state(&f).await,
        before,
        "stale submission writes no business state"
    );
    let before = business_state(&f).await;
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
    assert_eq!(
        business_state(&f).await,
        before,
        "stale validation writes no business state"
    );
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
    let admitted = f.db.store.request_snapshot().await.unwrap();
    let mut changed = f.active.ontology.clone();
    changed.action_types.clear();
    let external = f.db.reopen().await;
    reload(&external, &changed, &ReloadOptions::default())
        .await
        .unwrap();
    let replayed = service
        .execute_with_snapshot(
            &f.principal,
            &f.scope,
            &admitted.active,
            request("replay", false),
        )
        .await;
    assert!(
        replayed.is_ok(),
        "admission snapshot replay must survive external Action removal: {replayed:?}"
    );
    assert_eq!(receipt(replayed.unwrap()), original);
    let replay = service
        .get_command_result_with_snapshot(
            &f.principal,
            &f.scope,
            &admitted.active,
            "renameGate",
            "replay",
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay, original);
    assert!(
        service
            .get_command_result_with_snapshot(
                &f.principal,
                &PrincipalScope::default(),
                &admitted.active,
                "renameGate",
                "replay"
            )
            .await
            .is_err()
    );
    let mut stranger = f.principal.clone();
    stranger.id = PrincipalId::new("https://idp.example.com", "bob");
    assert!(
        service
            .get_command_result_with_snapshot(
                &stranger,
                &f.scope,
                &admitted.active,
                "renameGate",
                "replay"
            )
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        service
            .execute_with_snapshot(
                &f.principal,
                &PrincipalScope::default(),
                &admitted.active,
                request("replay", false)
            )
            .await,
        Err(ontoforge_action::ActionError::ScopeDenied)
    ));
    assert!(matches!(
        service
            .execute(
                &f.principal,
                &f.scope,
                admitted.active.def_version,
                request("replay", false)
            )
            .await,
        Err(ontoforge_action::ActionError::ScopeDenied)
    ));
    assert!(matches!(
        service
            .get_command_result(&f.principal, &f.scope, "renameGate", "replay")
            .await,
        Err(ontoforge_action::ActionError::ScopeDenied)
    ));
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
