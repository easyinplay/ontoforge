mod support;

use std::collections::BTreeMap;

use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::{Attrs, Pred, RowValues, rowpred::Scalar};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_schema::load_from_memory;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::history::{CallerKind, HistoryError, HistoryRequest};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::ActiveDefinition;
use ontoforge_storage::value::PropertyValue;

struct Row(BTreeMap<PropertyName, Scalar>);

impl RowValues for Row {
    fn value(&self, property: &PropertyName) -> Scalar {
        self.0.get(property).cloned().unwrap_or(Scalar::Null)
    }
}

#[tokio::test]
async fn frozen_history_keeps_request_definition_and_current_scope() {
    let db = support::sqlite_store().await;
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        (
            "defs.yaml",
            "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  name: { type: string }\n",
        ),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let ontology = loaded.ontology.unwrap();
    let version = reload(&db.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap()
        .def_version
        .unwrap();
    db.store
        .commit(Commit {
            actor: Actor::Principal("https://idp.example|tester".to_owned()),
            action: "edit".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().unwrap(),
                pk: PropertyValue::String("A1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().unwrap(),
                    PropertyValue::String("Visible".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let active: ActiveDefinition = (*db.store.request_snapshot().await.unwrap().active).clone();
    let external = db.reopen().await;
    let mut changed = ontology.clone();
    changed.description = Some("externally published definition".to_owned());
    let mut added = changed.object_types.get("Gate").unwrap().clone();
    added.api_name = "NewGate".to_owned().try_into().unwrap();
    changed.object_types.insert(added.api_name.clone(), added);
    let published = reload(&external, &changed, &ReloadOptions::default())
        .await
        .unwrap()
        .def_version
        .unwrap();
    assert_ne!(
        published, active.def_version,
        "definition versions, independent of edit version"
    );
    assert_eq!(active.ontology, ontology);
    assert_eq!(
        external.request_snapshot().await.unwrap().active.ontology,
        changed
    );
    let ty: ObjectTypeName = "Gate".to_owned().try_into().unwrap();
    let pk = PropertyValue::String("A1".to_owned());
    let name: PropertyName = "name".to_owned().try_into().unwrap();
    let current = Row(BTreeMap::from([(
        name.clone(),
        Scalar::String("Visible".to_owned()),
    )]));
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([(name.clone(), Pred::Const(true))]),
    };
    let entries = db
        .store
        .history_with_snapshot(
            &HistoryRequest {
                type_name: &ty,
                pk: &pk,
                scope: Some(&scope),
                attrs: &Attrs::new(),
                current_row: Some(&current),
                caller: CallerKind::Ordinary,
                limit: 20,
                before_seq: None,
            },
            &active,
        )
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].values[&name], serde_json::json!("Visible"));
    let attrs = Attrs::new();
    let mut req = HistoryRequest {
        type_name: &ty,
        pk: &pk,
        scope: Some(&scope),
        attrs: &attrs,
        current_row: Some(&current),
        caller: CallerKind::Ordinary,
        limit: 20,
        before_seq: None,
    };
    assert_eq!(
        external.history(&req).await.unwrap(),
        entries,
        "current wrapper remains usable"
    );
    let masked = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::new(),
    };
    req.scope = Some(&masked);
    let hidden = db.store.history_with_snapshot(&req, &active).await.unwrap();
    assert!(hidden[0].values.is_empty());
    assert!(hidden[0].masked_changes.contains(&name));
    assert!(!serde_json::to_string(&hidden).unwrap().contains("Visible"));
    req.scope = None;
    assert!(matches!(
        db.store.history_with_snapshot(&req, &active).await,
        Err(HistoryError::NotFound)
    ));
    let invisible = TypeScope {
        row: Pred::Const(false),
        columns: scope.columns.clone(),
    };
    req.scope = Some(&invisible);
    assert!(matches!(
        db.store.history_with_snapshot(&req, &active).await,
        Err(HistoryError::NotFound)
    ));
    req.scope = Some(&scope);
    req.current_row = None;
    assert!(matches!(
        db.store.history_with_snapshot(&req, &active).await,
        Err(HistoryError::NotFound)
    ));
    req.current_row = Some(&current);

    // A type published after admission distinguishes the captured definition from the current one.
    external
        .commit(Commit {
            actor: Actor::Principal("https://idp.example|tester".to_owned()),
            action: "edit".to_owned(),
            def_version: published,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "NewGate".to_owned().try_into().unwrap(),
                pk: pk.clone(),
                op: ObjectOp::Set(BTreeMap::from([(
                    name.clone(),
                    PropertyValue::String("Later".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let new_ty = "NewGate".to_owned().try_into().unwrap();
    req.type_name = &new_ty;
    assert!(matches!(
        db.store.history_with_snapshot(&req, &active).await,
        Err(HistoryError::NotFound)
    ));
    assert_eq!(
        external.history(&req).await.unwrap()[0].values[&name],
        serde_json::json!("Later")
    );
    assert_eq!(active.def_version, version);
}
