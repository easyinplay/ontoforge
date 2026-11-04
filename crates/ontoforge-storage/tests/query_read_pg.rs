mod support;

use std::collections::BTreeMap;

use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, LinkOp, ObjectOp};
use ontoforge_storage::query_read::LinkEndpoint;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use ontoforge_storage::value::PropertyValue;

const DEFS: &str = "kind: ObjectType\napiName: Gate\nbacking: source\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  name: { type: string }\n  amount: { type: decimal, precision: 8, scale: 2 }\n  openedAt: { type: timestamp }\n  tags: { type: array, items: string }\n---\nkind: ObjectType\napiName: Stand\nbacking: source\nprimaryKey: id\nproperties:\n  id: { type: integer, required: true }\n---\nkind: LinkType\napiName: gateStands\ncardinality: manyToMany\nfrom: { objectType: Gate, apiName: stands }\nto: { objectType: Stand, apiName: gates }\nbacking: source\n";

fn object() -> ObjectTypeName {
    ObjectTypeName::try_from("Gate".to_owned()).unwrap()
}

fn link() -> LinkTypeName {
    LinkTypeName::try_from("gateStands".to_owned()).unwrap()
}

fn s(value: &str) -> PropertyValue {
    PropertyValue::String(value.to_owned())
}

async fn seed(store: &EditStore, active: &ActiveDefinition) {
    let values = BTreeMap::from([
        ("name", s("Alpha")),
        ("amount", PropertyValue::Decimal("12.50".to_owned())),
        (
            "openedAt",
            PropertyValue::Timestamp("2026-10-22T08:09:10.000Z".to_owned()),
        ),
        ("tags", PropertyValue::Array(vec![s("red"), s("green")])),
    ])
    .into_iter()
    .map(|(name, value)| (PropertyName::try_from(name.to_owned()).unwrap(), value))
    .collect();
    store
        .commit(Commit {
            actor: Actor::Principal("test|alice".to_owned()),
            action: "setGate".to_owned(),
            def_version: active.def_version,
            params: serde_json::json!({}),
            edits: vec![Edit::Object {
                type_name: object(),
                pk: s("A"),
                op: ObjectOp::Set(values),
            }],
        })
        .await
        .unwrap();
    for (b, op) in [(1, LinkOp::Add), (2, LinkOp::Add), (2, LinkOp::Remove)] {
        store
            .commit(Commit {
                actor: Actor::Principal("test|alice".to_owned()),
                action: "editLink".to_owned(),
                def_version: active.def_version,
                params: serde_json::json!({}),
                edits: vec![Edit::Link {
                    link: link(),
                    a: s("A"),
                    b: PropertyValue::Int(b),
                    op,
                }],
            })
            .await
            .unwrap();
    }
}

async fn pair() -> (
    support::TestStore,
    support::PgTestStore,
    std::sync::Arc<ActiveDefinition>,
    std::sync::Arc<ActiveDefinition>,
) {
    let sqlite = support::sqlite_store().await;
    let pg = support::pg_store().await;
    let ontology = support::ontology_from(&[("all.yaml", DEFS)]);
    reload(&sqlite.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    reload(&pg.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let sqlite_active = sqlite.store.active_definition().await.unwrap();
    let pg_active = pg.store.active_definition().await.unwrap();
    seed(&sqlite.store, &sqlite_active).await;
    seed(&pg.store, &pg_active).await;
    (sqlite, pg, sqlite_active, pg_active)
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn sqlite_and_postgres_object_reads_match() {
    let (sqlite, pg, s_active, p_active) = pair().await;
    let keys = [s("A"), s("missing")];
    assert_eq!(
        sqlite
            .store
            .read_object_edits(&s_active, &object(), &keys)
            .await
            .unwrap(),
        pg.store
            .read_object_edits(&p_active, &object(), &keys)
            .await
            .unwrap(),
    );
    assert_eq!(
        sqlite
            .store
            .scan_object_edits(&s_active, &object(), None, 2)
            .await
            .unwrap(),
        pg.store
            .scan_object_edits(&p_active, &object(), None, 2)
            .await
            .unwrap(),
    );
}

#[tokio::test]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn sqlite_and_postgres_link_reads_match() {
    let (sqlite, pg, s_active, p_active) = pair().await;
    assert_eq!(
        sqlite
            .store
            .read_link_edits(&s_active, &link(), LinkEndpoint::From(s("A")), None, 2)
            .await
            .unwrap(),
        pg.store
            .read_link_edits(&p_active, &link(), LinkEndpoint::From(s("A")), None, 2)
            .await
            .unwrap(),
    );
    assert_eq!(
        sqlite
            .store
            .read_link_edits(
                &s_active,
                &link(),
                LinkEndpoint::To(PropertyValue::Int(2)),
                None,
                2
            )
            .await
            .unwrap(),
        pg.store
            .read_link_edits(
                &p_active,
                &link(),
                LinkEndpoint::To(PropertyValue::Int(2)),
                None,
                2
            )
            .await
            .unwrap(),
    );
}
