mod support;

use ontoforge_schema::ir::ObjectTypeName;
use ontoforge_storage::query_read::ObjectEdit;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::value::PropertyValue;

const DEFS: &str = "kind: ObjectType\napiName: Gate\nbacking: source\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  name: { type: string }\n";

fn gate() -> ObjectTypeName {
    ObjectTypeName::try_from("Gate".to_owned()).unwrap()
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
