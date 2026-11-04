//! Deterministic request metadata snapshot tests.

mod support;

use std::collections::BTreeMap;
use std::time::Duration;
use std::{future::Future, pin::pin, task::Poll};

use ontoforge_auth::scope::{ScopeInputs, compile_scope};
use ontoforge_schema::ir::AttrType;
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::{request_snapshot::RequestSnapshot, store::EditStore};
use support::{pg_store, sqlite_store};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

fn aviation() -> ontoforge_schema::ir::Ontology {
    let out = load_dir(camino::Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

fn aviation_scope() -> ontoforge_schema::ir::ScopeSet {
    let out = load_scope_dir(camino::Utf8Path::new(AVIATION_SCOPE));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.scope.expect("aviation scope")
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_sqlite_never_mixes_definition_roles_bindings() {
    let fixture = sqlite_store().await;
    let other = fixture.reopen().await;
    Box::pin(metadata_contract(
        &fixture.store,
        &other,
        &LockSource::Sqlite(fixture.side_pool().await),
    ))
    .await;
}

enum LockSource {
    Sqlite(sqlx::SqlitePool),
    Postgres(sqlx::PgPool, i64),
}

enum HeldLock {
    Sqlite(sqlx::Transaction<'static, sqlx::Sqlite>),
    Postgres(sqlx::Transaction<'static, sqlx::Postgres>),
}

impl LockSource {
    async fn lock(&self) -> HeldLock {
        match self {
            Self::Sqlite(pool) => HeldLock::Sqlite(
                pool.begin_with("BEGIN IMMEDIATE")
                    .await
                    .expect("metadata lock"),
            ),
            Self::Postgres(pool, key) => {
                let mut tx = pool.begin().await.expect("metadata transaction");
                sqlx::query("SELECT pg_advisory_xact_lock($1)")
                    .bind(key)
                    .execute(&mut *tx)
                    .await
                    .expect("metadata lock");
                HeldLock::Postgres(tx)
            }
        }
    }
}

impl HeldLock {
    async fn release(self) {
        match self {
            Self::Sqlite(tx) => tx.commit().await.expect("unlock"),
            Self::Postgres(tx) => tx.commit().await.expect("unlock"),
        }
    }
}

async fn race_snapshot<T: std::fmt::Debug, E: std::fmt::Debug>(
    reader: &EditStore,
    locks: &LockSource,
    writer: impl Future<Output = Result<T, E>>,
) -> RequestSnapshot {
    let lock = locks.lock().await;
    let ready = tokio::sync::Notify::new();
    let release = async {
        ready.notified().await;
        lock.release().await;
    };
    let operations = async {
        let mut read = pin!(reader.request_snapshot());
        let mut write = pin!(writer);
        std::future::poll_fn(|cx| {
            assert!(
                read.as_mut().poll(cx).is_pending(),
                "snapshot bypassed metadata lock"
            );
            assert!(
                write.as_mut().poll(cx).is_pending(),
                "writer bypassed metadata lock"
            );
            Poll::Ready(())
        })
        .await;
        assert!(tokio::time::timeout(Duration::from_millis(100), async {
            tokio::select! {
                result = &mut read => panic!("snapshot completed while metadata lock was held: {result:?}"),
                result = &mut write => panic!("writer completed while metadata lock was held: {result:?}"),
            }
        }).await.is_err(), "both operations must wait for the held metadata lock");
        ready.notify_one();
        let (snapshot, written) = tokio::join!(read, write);
        written.expect("real metadata writer");
        snapshot.expect("snapshot after metadata contention")
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        let ((), snapshot) = tokio::join!(release, operations);
        snapshot
    })
    .await
    .expect("metadata race completes and releases lock")
}

async fn metadata_contract(store: &EditStore, other: &EditStore, locks: &LockSource) {
    let ontology = aviation();
    reload(store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload");
    let scope = aviation_scope();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &BTreeMap::from([("homeAirports".to_owned(), AttrType::StringList)]),
        scope: &scope,
        sources: None,
    })
    .expect("compile scope");
    let props = compiled.row_filter_properties();
    let plan = store.scope_plan(&scope, &props).await.expect("scope plan");
    store
        .scope_apply(&scope, &props, &plan.hash, "snapshot-test")
        .await
        .expect("scope apply");

    let normalized = ontoforge_storage::scope_store::normalize_scope(&scope);
    let old = other.request_snapshot().await.expect("admission snapshot");
    assert_eq!(old.active.def_version, 1);
    assert_eq!(old.scope, normalized);

    let mut changed = ontology.clone();
    changed.description = Some("generation two".to_owned());
    let opts = ReloadOptions::default();
    let during = Box::pin(race_snapshot(other, locks, reload(store, &changed, &opts))).await;
    assert!(matches!(during.active.def_version, 1 | 2));
    assert_eq!(during.scope, normalized);
    assert_eq!(
        during.active.ontology.description,
        if during.active.def_version == 2 {
            changed.description.clone()
        } else {
            ontology.description.clone()
        }
    );
    let published = other.request_snapshot().await.expect("published reload");
    assert_eq!(published.active.def_version, 2);
    assert_eq!(published.active.ontology, changed);

    let mut updated = scope.clone();
    updated
        .roles
        .values_mut()
        .for_each(|role| role.description = Some("generation two role".to_owned()));
    updated.bindings.values_mut().for_each(|binding| {
        binding.subjects = vec![ontoforge_schema::ir::Subject::User(
            "snapshot|new".to_owned(),
        )];
    });
    let plan = store
        .scope_plan(&updated, &props)
        .await
        .expect("new scope plan");
    let during = Box::pin(race_snapshot(
        other,
        locks,
        store.scope_apply(&updated, &props, &plan.hash, "snapshot-test"),
    ))
    .await;
    let new_scope = ontoforge_storage::scope_store::normalize_scope(&updated);
    assert_eq!(during.active.def_version, 2);
    assert!(
        during.scope == normalized || during.scope == new_scope,
        "roles and bindings must be one committed generation"
    );
    let applied = other.request_snapshot().await.expect("scope committed");
    assert_eq!(applied.active.ontology, changed);
    assert_eq!(applied.scope, new_scope);

    let revoked = ontoforge_schema::ir::ScopeSet::default();
    let plan = store
        .scope_plan(&revoked, &props)
        .await
        .expect("revoke plan");
    let during = Box::pin(race_snapshot(
        other,
        locks,
        store.scope_apply(&revoked, &props, &plan.hash, "snapshot-test"),
    ))
    .await;
    assert_eq!(during.active.def_version, 2);
    assert!(during.scope == new_scope || during.scope == revoked);
    let after = other.request_snapshot().await.expect("revoke committed");
    assert_eq!(after.active.ontology, changed);
    assert_eq!(after.scope, revoked);
    assert_eq!(old.active.def_version, 1);
    assert_eq!(old.active.ontology, ontology);
    assert_eq!(
        old.scope, normalized,
        "admitted request survives all three commits"
    );
    assert_eq!(
        applied.scope, new_scope,
        "captured grants survive external revoke"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_lock_is_released_before_compile_or_network() {
    let fixture = sqlite_store().await;
    let ontology = aviation();
    reload(&fixture.store, &ontology, &ReloadOptions::default())
        .await
        .expect("reload");
    let snapshot = fixture.store.request_snapshot().await.expect("snapshot");

    let other = fixture.reopen().await;
    let mut changed = ontology.clone();
    changed.description = Some("changed after snapshot".to_owned());
    let reloaded = tokio::time::timeout(
        Duration::from_secs(2),
        reload(&other, &changed, &ReloadOptions::default()),
    )
    .await
    .expect("reload is not blocked by a completed snapshot")
    .expect("reload");
    assert!(reloaded.changed);
    assert_eq!(snapshot.active.def_version, 1);
}

#[tokio::test]
#[ignore = "requires a real PostgreSQL URL; Phase 07-13 runs ignored tests explicitly"]
async fn snapshot_pg_never_mixes_definition_roles_bindings() {
    assert!(
        std::env::var_os("ONTOFORGE_PG_URL").is_some(),
        "ONTOFORGE_PG_URL is required when running ignored PostgreSQL snapshot tests"
    );
    let fixture = pg_store().await;
    let other = fixture.open_another(Duration::from_secs(5)).await;
    Box::pin(metadata_contract(
        &fixture.store,
        &other,
        &LockSource::Postgres(
            fixture.side().clone(),
            ontoforge_storage::store::definition_lock_key(&fixture.schema_edit),
        ),
    ))
    .await;
}
