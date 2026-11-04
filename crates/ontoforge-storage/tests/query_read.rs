mod support;

use std::collections::BTreeMap;

use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::query_read::{LinkEndpoint, ObjectEdit};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::value::PropertyValue;

const DEFS: &str = "kind: ObjectType\napiName: Gate\nbacking: source\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  name: { type: string }\n  short: { type: short }\n  byte: { type: byte }\n  integer: { type: integer }\n  long: { type: long }\n  float: { type: float }\n  double: { type: double }\n  bool: { type: boolean }\n  decimal: { type: decimal, precision: 8, scale: 2 }\n  date: { type: date }\n  timestamp: { type: timestamp }\n  strings: { type: array, items: string }\n  point: { type: geopoint }\n---\nkind: ObjectType\napiName: Stand\nbacking: source\nprimaryKey: id\nproperties:\n  id: { type: integer, required: true }\n  gate: { type: string }\n---\nkind: LinkType\napiName: gateStands\ncardinality: manyToMany\nfrom: { objectType: Gate, apiName: stands }\nto: { objectType: Stand, apiName: gates }\nbacking: source\n---\nkind: LinkType\napiName: gateStand\ncardinality: oneToOne\nfrom: { objectType: Gate, apiName: stand }\nto: { objectType: Stand, apiName: assignedGate, foreignKey: gate }\n";

fn gate() -> ObjectTypeName {
    ObjectTypeName::try_from("Gate".to_owned()).unwrap()
}

fn link(name: &str) -> LinkTypeName {
    LinkTypeName::try_from(name.to_owned()).unwrap()
}

fn s(value: &str) -> PropertyValue {
    PropertyValue::String(value.to_owned())
}

async fn fixture() -> (
    support::TestStore,
    std::sync::Arc<ontoforge_storage::store::ActiveDefinition>,
) {
    let fixture = support::sqlite_store().await;
    let ontology = support::ontology_from(&[("gate.yaml", DEFS)]);
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = fixture.store.active_definition().await.unwrap();
    (fixture, active)
}

async fn set_name(
    fixture: &support::TestStore,
    active: &ontoforge_storage::store::ActiveDefinition,
    key: &str,
    name: PropertyValue,
) {
    fixture
        .store
        .commit(Commit {
            actor: Actor::Principal("test|alice".to_owned()),
            action: "setGate".to_owned(),
            def_version: active.def_version,
            params: serde_json::json!({}),
            edits: vec![Edit::Object {
                type_name: gate(),
                pk: s(key),
                op: ObjectOp::Set(BTreeMap::from([(
                    PropertyName::try_from("name".to_owned()).unwrap(),
                    name,
                )])),
            }],
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn batch_preserves_requested_keys_and_distinguishes_null_from_unset() {
    let (fixture, active) = fixture().await;
    set_name(&fixture, &active, "B", PropertyValue::Null).await;
    set_name(&fixture, &active, "A", s("Alpha")).await;
    let edits = fixture
        .store
        .read_object_edits(&active, &gate(), &[s("B"), s("missing"), s("A"), s("B")])
        .await
        .unwrap();
    assert_eq!(edits.len(), 4);
    assert_eq!(edits[0].as_ref().unwrap().pk, s("B"));
    assert_eq!(
        edits[0].as_ref().unwrap().row.set[&PropertyName::try_from("name".to_owned()).unwrap()],
        PropertyValue::Null
    );
    assert!(edits[1].is_none());
    assert_eq!(
        edits[2].as_ref().unwrap().row.set[&PropertyName::try_from("name".to_owned()).unwrap()],
        s("Alpha")
    );
    assert_eq!(edits[3], edits[0]);
}

#[tokio::test]
async fn batch_rejects_more_than_256_before_sql() {
    let (fixture, active) = fixture().await;
    let pool = fixture.side_pool().await;
    let table = &active.physical.objects[&gate()].table;
    let drop = format!("DROP TABLE \"{table}\"");
    sqlx::query(sqlx::AssertSqlSafe(drop))
        .execute(&pool)
        .await
        .unwrap();
    let keys = (0..257).map(|i| s(&format!("K{i}"))).collect::<Vec<_>>();
    let err = fixture
        .store
        .read_object_edits(&active, &gate(), &keys)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("256"));
    assert!(!err.to_string().contains("of_obj"));
}

#[tokio::test]
async fn batch_accepts_256_keys_and_all_value_types() {
    let (fixture, active) = fixture().await;
    let values = BTreeMap::from([
        ("short", PropertyValue::Int(32)),
        ("byte", PropertyValue::Int(7)),
        ("integer", PropertyValue::Int(1234)),
        ("long", PropertyValue::Int(5_000_000_000)),
        ("float", PropertyValue::Double(1.25)),
        ("double", PropertyValue::Double(2.5)),
        ("bool", PropertyValue::Bool(true)),
        ("decimal", PropertyValue::Decimal("12.50".to_owned())),
        ("date", PropertyValue::Date("2026-10-22".to_owned())),
        (
            "timestamp",
            PropertyValue::Timestamp("2026-10-22T08:09:10.000Z".to_owned()),
        ),
        ("strings", PropertyValue::Array(vec![s("a"), s("b")])),
        (
            "point",
            PropertyValue::Geopoint {
                lat: 20.0,
                lon: 30.0,
            },
        ),
    ])
    .into_iter()
    .map(|(name, value)| (PropertyName::try_from(name.to_owned()).unwrap(), value))
    .collect::<BTreeMap<_, _>>();
    fixture
        .store
        .commit(Commit {
            actor: Actor::Principal("test|alice".to_owned()),
            action: "setGate".to_owned(),
            def_version: active.def_version,
            params: serde_json::json!({}),
            edits: vec![Edit::Object {
                type_name: gate(),
                pk: s("A"),
                op: ObjectOp::Set(values.clone()),
            }],
        })
        .await
        .unwrap();
    let mut keys = (0..255).map(|i| s(&format!("K{i}"))).collect::<Vec<_>>();
    keys.push(s("A"));
    let edits = fixture
        .store
        .read_object_edits(&active, &gate(), &keys)
        .await
        .unwrap();
    assert_eq!(edits.len(), 256);
    assert_eq!(edits.into_iter().flatten().next().unwrap().row.set, values);
}

#[tokio::test]
async fn scan_uses_keyset_and_preserves_flags() {
    let (fixture, active) = fixture().await;
    for key in ["C", "A", "B"] {
        set_name(&fixture, &active, key, s(key)).await;
    }
    fixture
        .store
        .commit(Commit {
            actor: Actor::Principal("test|alice".to_owned()),
            action: "deleteGate".to_owned(),
            def_version: active.def_version,
            params: serde_json::json!({}),
            edits: vec![Edit::Object {
                type_name: gate(),
                pk: s("B"),
                op: ObjectOp::Tombstone,
            }],
        })
        .await
        .unwrap();
    let first = fixture
        .store
        .scan_object_edits(&active, &gate(), None, 2)
        .await
        .unwrap();
    assert_eq!(
        first.edits.iter().map(|e| &e.pk).collect::<Vec<_>>(),
        vec![&s("A"), &s("B")]
    );
    assert!(first.edits[1].row.deleted);
    assert!(first.has_more);
    let second = fixture
        .store
        .scan_object_edits(&active, &gate(), first.next.as_ref(), 2)
        .await
        .unwrap();
    assert_eq!(
        second.edits.iter().map(|e| &e.pk).collect::<Vec<_>>(),
        vec![&s("C")]
    );
    assert!(!second.has_more);
    assert!(
        fixture
            .store
            .scan_object_edits(&active, &gate(), None, 0)
            .await
            .is_err()
    );
    assert!(
        fixture
            .store
            .scan_object_edits(&active, &gate(), None, 257)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn get_batch_and_scan_preserve_rows_log_and_definition() {
    let (fixture, active) = fixture().await;
    set_name(&fixture, &active, "A", s("Alpha")).await;
    let table = &active.physical.objects[&gate()].table;
    let pool = fixture.side_pool().await;
    let snapshot = || async {
        sqlx::query_as::<_, (i64, i64, i64)>(sqlx::AssertSqlSafe(format!(
            "SELECT (SELECT count(*) FROM \"{table}\"), (SELECT count(*) FROM of_edit_log), (SELECT count(*) FROM of_def_version)"
        ))).fetch_one(&pool).await.unwrap()
    };
    let before = snapshot().await;
    assert!(
        fixture
            .store
            .read_object_edit(&active, &gate(), &s("missing"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .store
            .read_object_edit(&active, &gate(), &s("A"))
            .await
            .unwrap()
            .is_some()
    );
    fixture
        .store
        .read_object_edits(&active, &gate(), &[s("A"), s("missing")])
        .await
        .unwrap();
    fixture
        .store
        .scan_object_edits(&active, &gate(), None, 1)
        .await
        .unwrap();
    assert_eq!(before, snapshot().await);
    let edit = fixture
        .store
        .read_object_edit(&active, &gate(), &s("A"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        edit.row.set[&PropertyName::try_from("name".to_owned()).unwrap()],
        s("Alpha")
    );
}

#[tokio::test]
async fn scan_includes_release_and_skips_version_zero() {
    let (fixture, active) = fixture().await;
    set_name(&fixture, &active, "A", s("Alpha")).await;
    fixture
        .store
        .commit(Commit {
            actor: Actor::Principal("test|alice".to_owned()),
            action: "releaseGate".to_owned(),
            def_version: active.def_version,
            params: serde_json::json!({}),
            edits: vec![Edit::Object {
                type_name: gate(),
                pk: s("A"),
                op: ObjectOp::Release,
            }],
        })
        .await
        .unwrap();
    let table = &active.physical.objects[&gate()].table;
    let pk_col = &active.physical.objects[&gate()].pk.column;
    let pool = fixture.side_pool().await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO \"{table}\" (\"{pk_col}\") VALUES ('B')"
    )))
    .execute(&pool)
    .await
    .unwrap();
    let page = fixture
        .store
        .scan_object_edits(&active, &gate(), None, 2)
        .await
        .unwrap();
    assert_eq!(page.edits.len(), 1);
    assert_eq!(page.edits[0].pk, s("A"));
    assert!(page.edits[0].row.released);
    assert!(!page.has_more);
}

#[tokio::test]
async fn link_from_or_to_endpoint_is_bounded_and_preserves_removed_edges() {
    let (fixture, active) = fixture().await;
    let table = &active.physical.links[&link("gateStands")];
    let pool = fixture.side_pool().await;
    for (a, b, deleted, version) in [
        ("A", 3, false, 1),
        ("A", 1, true, 2),
        ("A", 2, false, 1),
        ("B", 1, false, 1),
        ("A", 4, false, 0),
    ] {
        let sql = format!(
            "INSERT INTO \"{}\" (\"{}\", \"{}\", of_deleted, of_version) VALUES ('{a}', {b}, {deleted}, {version})",
            table.table, table.a.column, table.b.column
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .execute(&pool)
            .await
            .unwrap();
    }
    let first = fixture
        .store
        .read_link_edits(
            &active,
            &link("gateStands"),
            LinkEndpoint::From(s("A")),
            None,
            2,
        )
        .await
        .unwrap();
    assert_eq!(
        first.edits.iter().map(|e| &e.to).collect::<Vec<_>>(),
        vec![&PropertyValue::Int(1), &PropertyValue::Int(2)]
    );
    assert!(first.edits[0].row.deleted);
    assert_eq!(first.edits[0].row.version, 2);
    assert!(first.has_more);
    let second = fixture
        .store
        .read_link_edits(
            &active,
            &link("gateStands"),
            LinkEndpoint::From(s("A")),
            first.next.as_ref(),
            2,
        )
        .await
        .unwrap();
    assert_eq!(
        second.edits.iter().map(|e| &e.to).collect::<Vec<_>>(),
        vec![&PropertyValue::Int(3)]
    );
    assert!(!second.has_more);
    let reverse = fixture
        .store
        .read_link_edits(
            &active,
            &link("gateStands"),
            LinkEndpoint::To(PropertyValue::Int(1)),
            None,
            2,
        )
        .await
        .unwrap();
    assert_eq!(
        reverse.edits.iter().map(|e| &e.from).collect::<Vec<_>>(),
        vec![&s("A"), &s("B")]
    );
    assert!(
        fixture
            .store
            .read_link_edits(
                &active,
                &link("gateStands"),
                LinkEndpoint::From(s("A")),
                None,
                0
            )
            .await
            .is_err()
    );
    assert!(
        fixture
            .store
            .read_link_edits(
                &active,
                &link("gateStands"),
                LinkEndpoint::From(s("A")),
                None,
                257
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn link_fk_request_is_rejected_with_logical_name() {
    let (fixture, active) = fixture().await;
    let err = fixture
        .store
        .read_link_edits(
            &active,
            &link("gateStand"),
            LinkEndpoint::From(s("A")),
            None,
            1,
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("gateStand"));
    assert!(!err.to_string().contains("of_lnk"));
}

#[tokio::test]
async fn corrupt_property_error_uses_logical_name() {
    let fixture = support::sqlite_store().await;
    let ontology = support::ontology_from(&[(
        "gate.yaml",
        "kind: ObjectType\napiName: Gate\nbacking: source\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  openedAt: { type: timestamp }\n",
    )]);
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = fixture.store.active_definition().await.unwrap();
    let table = &active.physical.objects[&gate()];
    let column = &table.properties[&PropertyName::try_from("openedAt".to_owned()).unwrap()];
    let pool = fixture.side_pool().await;
    let sql = format!(
        "INSERT INTO \"{}\" (\"{}\", \"{}\", \"{}\", of_version) VALUES ('A', 'invalid', true, 1)",
        table.table, table.pk.column, column.column, column.set_column
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(&pool)
        .await
        .unwrap();
    let err = fixture
        .store
        .read_object_edit(&active, &gate(), &s("A"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("openedAt"));
    assert!(!err.to_string().contains(&column.column));
}

#[tokio::test]
async fn link_reads_leave_row_log_and_definition_counts_unchanged() {
    let (fixture, active) = fixture().await;
    let table = &active.physical.links[&link("gateStands")];
    let pool = fixture.side_pool().await;
    let sql = format!(
        "INSERT INTO \"{}\" (\"{}\", \"{}\", of_version) VALUES ('A', 1, 1)",
        table.table, table.a.column, table.b.column
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(&pool)
        .await
        .unwrap();
    let snapshot = || async {
        sqlx::query_as::<_, (i64, i64, i64)>(sqlx::AssertSqlSafe(format!(
            "SELECT (SELECT count(*) FROM \"{}\"), (SELECT count(*) FROM of_edit_log), (SELECT count(*) FROM of_def_version)", table.table
        ))).fetch_one(&pool).await.unwrap()
    };
    let before = snapshot().await;
    fixture
        .store
        .read_link_edits(
            &active,
            &link("gateStands"),
            LinkEndpoint::From(s("A")),
            None,
            1,
        )
        .await
        .unwrap();
    fixture
        .store
        .read_link_edits(
            &active,
            &link("gateStands"),
            LinkEndpoint::To(PropertyValue::Int(1)),
            None,
            1,
        )
        .await
        .unwrap();
    fixture
        .store
        .read_link_edits(
            &active,
            &link("gateStands"),
            LinkEndpoint::From(s("missing")),
            None,
            1,
        )
        .await
        .unwrap();
    assert_eq!(before, snapshot().await);
}

#[tokio::test]
async fn version_zero_skeleton_is_not_an_edit_and_read_does_not_mutate() {
    let fixture = support::sqlite_store().await;
    let ontology = support::ontology_from(&[("gate.yaml", DEFS)]);
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = fixture.store.active_definition().await.unwrap();
    let table = &active.physical.objects[&gate()].table;
    let pk_col = &active.physical.objects[&gate()].pk.column;
    let pool = fixture.side_pool().await;
    let insert = format!("INSERT INTO \"{table}\" (\"{pk_col}\") VALUES ('A')");
    sqlx::query(sqlx::AssertSqlSafe(insert))
        .execute(&pool)
        .await
        .unwrap();
    let before: (i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM \"{table}\"), (SELECT count(*) FROM of_edit_log), (SELECT count(*) FROM of_def_version)"
    )))
    .fetch_one(&pool)
    .await
    .unwrap();

    let edit: Option<ObjectEdit> = fixture
        .store
        .read_object_edit(&active, &gate(), &PropertyValue::String("A".to_owned()))
        .await
        .unwrap();
    assert!(edit.is_none(), "version 0 is not an overlay candidate");

    let after: (i64, i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT (SELECT count(*) FROM \"{table}\"), (SELECT count(*) FROM of_edit_log), (SELECT count(*) FROM of_def_version)"
    )))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after);
}
