//! `reload` on the PostgreSQL edit store: schema split, idempotency and additive change
//! (STORE-01, STORE-02).
//!
//! Every test is `#[ignore]`d: they need a PostgreSQL server. Run them with
//! `ONTOFORGE_PG_URL=postgres://... cargo test -p ontoforge-storage --test reload_pg -- --ignored`
//! or, on a machine that can download and run PostgreSQL, with
//! `cargo test -p ontoforge-storage --features embedded-pg --test reload_pg -- --ignored`.
//! Without either, `support::pg_store()` panics.

mod support;

use std::collections::BTreeSet;

use ontoforge_schema::ir::Ontology;
use ontoforge_storage::reload::{ReloadOptions, reload};
use support::{ontology_from, pg_store};

const GATE: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string }
  cost: { type: decimal, precision: 12, scale: 2 }
  openedAt: { type: timestamp }
  tags: { type: array, items: string }
  location: { type: geopoint }
---
kind: ObjectType
apiName: Stand
backing: editOnly
primaryKey: id
properties:
  id: { type: integer, required: true }
---
kind: LinkType
apiName: gateStands
cardinality: manyToMany
from: { objectType: Gate, apiName: stands }
to: { objectType: Stand, apiName: gates }
backing: editOnly
";

fn base() -> Ontology {
    ontology_from(&[("defs.yaml", GATE)])
}

fn with_zone() -> Ontology {
    let text = GATE.replacen(
        "  location: { type: geopoint }\n",
        "  location: { type: geopoint }\n  zone: { type: string }\n",
        1,
    );
    ontology_from(&[("defs.yaml", &text)])
}

fn names(columns: &[String]) -> BTreeSet<&str> {
    columns.iter().map(String::as_str).collect()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn first_reload_on_pg_puts_edit_tables_in_edit_schema() {
    let t = pg_store().await;
    let out = reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("reload");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(1));
    assert!(out.ddl.iter().all(|sql| sql.contains(&t.schema_edit)));

    let edit = t.tables(&t.schema_edit).await;
    assert_eq!(
        names(&edit),
        BTreeSet::from(["of_obj_gate", "of_obj_stand", "of_lnk_gate_stands"]),
        "edit tables only"
    );
    let sys = t.tables(&t.schema_sys).await;
    assert_eq!(
        names(&sys),
        BTreeSet::from(["_sqlx_migrations", "of_def_state", "of_def_version"]),
        "system tables only"
    );
    assert_eq!(t.active_version().await, 1);
    assert_eq!(t.version_rows().await, 1);

    // ADR-0024 point 7: column types and collation.
    let gate = "of_obj_gate";
    assert_eq!(t.data_type(&t.schema_edit, gate, "code").await, "text");
    assert_eq!(
        t.collation(&t.schema_edit, gate, "code").await.as_deref(),
        Some("C")
    );
    assert_eq!(t.data_type(&t.schema_edit, gate, "cost").await, "numeric");
    assert_eq!(
        t.data_type(&t.schema_edit, gate, "opened_at").await,
        "timestamp with time zone"
    );
    assert_eq!(t.data_type(&t.schema_edit, gate, "tags").await, "ARRAY");
    assert_eq!(t.data_type(&t.schema_edit, gate, "location").await, "jsonb");
    assert_eq!(
        t.data_type(&t.schema_edit, gate, "name__set").await,
        "boolean"
    );
    assert_eq!(
        t.data_type(&t.schema_edit, gate, "of_version").await,
        "bigint"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn second_reload_on_pg_is_noop() {
    let t = pg_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let again = reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("second");
    assert!(!again.changed);
    assert!(again.ddl.is_empty());
    assert_eq!(again.def_version, Some(1));
    assert_eq!(t.version_rows().await, 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn additive_reload_on_pg_adds_columns() {
    let t = pg_store().await;
    reload(&t.store, &base(), &ReloadOptions::default())
        .await
        .expect("first");
    let out = reload(&t.store, &with_zone(), &ReloadOptions::default())
        .await
        .expect("additive");
    assert!(out.changed);
    assert_eq!(out.def_version, Some(2));
    assert_eq!(out.ddl.len(), 2, "{:#?}", out.ddl);
    assert!(
        out.ddl
            .iter()
            .all(|sql| sql.contains("ADD COLUMN IF NOT EXISTS"))
    );
    let gate = t.columns(&t.schema_edit, "of_obj_gate").await;
    assert!(gate.contains(&"zone".to_owned()) && gate.contains(&"zone__set".to_owned()));
    assert_eq!(t.active_version().await, 2);
    assert_eq!(t.version_rows().await, 2);
}
