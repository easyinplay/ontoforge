//! Action execution and edit log (ADR-0011). Implemented in Phase 3.

/// Cargo package name of this crate.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;
    use camino::Utf8PathBuf;
    use ontoforge_schema::load_from_memory;
    use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
    use ontoforge_storage::reload::{ReloadOptions, reload};
    use ontoforge_storage::store::{
        BackendSettings, EditStore, StoreSettings, sqlite_connect_options,
    };
    use ontoforge_storage::value::PropertyValue;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::BTreeMap;
    use std::time::Duration;

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

    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }

    #[tokio::test]
    async fn tracer_commits_one_action_batch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
        let store = EditStore::open(StoreSettings {
            backend: BackendSettings::Sqlite { path: path.clone() },
            lock_timeout: Duration::from_secs(5),
            probe_timeout: Duration::from_secs(5),
            trino_catalog: None,
        })
        .await
        .expect("open store");
        let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        let ontology = loaded.ontology.expect("ontology");
        let version = reload(&store, &ontology, &ReloadOptions::default())
            .await
            .expect("reload")
            .def_version
            .expect("definition version");
        let property = |name: &str| name.to_owned().try_into().expect("property name");
        let edit = |name: &str| Edit::Object {
            type_name: "Gate".to_owned().try_into().expect("type name"),
            pk: PropertyValue::String("A1".to_owned()),
            op: ObjectOp::Set(BTreeMap::from([(
                property("name"),
                PropertyValue::String(name.to_owned()),
            )])),
        };
        store
            .commit(Commit {
                actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
                action: "seed".to_owned(),
                def_version: version,
                params: serde_json::Value::Null,
                edits: vec![edit("Old")],
            })
            .await
            .expect("seed");

        let receipt = store
            .commit(Commit {
                actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
                action: "renameGate".to_owned(),
                def_version: version,
                params: serde_json::json!({"gate": "A1", "name": "New"}),
                edits: vec![edit("New")],
            })
            .await
            .expect("action commit");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlite_connect_options(&path))
            .await
            .expect("inspect store");
        let command_tables: i64 = sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'of_action_command'")
            .fetch_one(&pool).await.expect("command table query");
        assert_eq!(
            command_tables, 1,
            "Action batch must persist a command claim"
        );
        let command_count: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
            .fetch_one(&pool)
            .await
            .expect("command count");
        let outbox_count: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
            .fetch_one(&pool)
            .await
            .expect("outbox count");
        assert_eq!(command_count, 1);
        assert_eq!(outbox_count, 1);
        assert_eq!(receipt.versions, vec![2]);
    }
}
