mod support;

use std::collections::BTreeMap;

use ontoforge_auth::scope::TypeScope;
use ontoforge_expr::{Attrs, Pred, RowValues, rowpred::Scalar};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_schema::load_from_memory;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::history::{CallerKind, HistoryRequest};
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
    let active: ActiveDefinition = (*db.store.active_definition().await.unwrap()).clone();
    let ty: ObjectTypeName = "Gate".to_owned().try_into().unwrap();
    let pk = PropertyValue::String("A1".to_owned());
    let name: PropertyName = "name".to_owned().try_into().unwrap();
    let current = Row(BTreeMap::from([(
        name.clone(),
        Scalar::String("Visible".to_owned()),
    )]));
    let scope = TypeScope {
        row: Pred::Const(true),
        columns: BTreeMap::from([(name, Pred::Const(true))]),
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
    assert_eq!(entries[0].version, 1);
}
