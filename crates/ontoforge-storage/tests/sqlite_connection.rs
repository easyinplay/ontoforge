//! The SQLite edit store connection: `recursive_triggers`, the engine collations and foreign keys.

use camino::Utf8PathBuf;
use ontoforge_storage::store::sqlite_connect_options;
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;

async fn pool(max: u32) -> (TempDir, SqlitePool) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("store.db")).expect("utf8 path");
    let pool = SqlitePoolOptions::new()
        .max_connections(max)
        .connect_with(sqlite_connect_options(&path))
        .await
        .expect("connect");
    (dir, pool)
}

#[tokio::test(flavor = "multi_thread")]
async fn every_pooled_connection_has_recursive_triggers() {
    let (_dir, pool) = pool(3).await;
    let mut conns = Vec::new();
    for _ in 0..3 {
        conns.push(pool.acquire().await.expect("acquire"));
    }
    for conn in &mut conns {
        let on: i64 = sqlx::query_scalar("PRAGMA recursive_triggers")
            .fetch_one(&mut **conn)
            .await
            .expect("pragma");
        assert_eq!(on, 1);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn engine_collations_are_registered() {
    let (_dir, pool) = pool(3).await;
    sqlx::query(
        "CREATE TABLE t (v TEXT COLLATE ontoforge_decimal, ts TEXT COLLATE ontoforge_timestamp)",
    )
    .execute(&pool)
    .await
    .expect("create");
    sqlx::query("INSERT INTO t (v, ts) VALUES ('1.50', '2026-01-01 00:00:00.000')")
        .execute(&pool)
        .await
        .expect("insert");
    let decimal: i64 = sqlx::query_scalar("SELECT count(*) FROM t WHERE v = '1.5'")
        .fetch_one(&pool)
        .await
        .expect("decimal");
    assert_eq!(decimal, 1);
    let timestamp: i64 =
        sqlx::query_scalar("SELECT count(*) FROM t WHERE ts = '2026-01-01 00:00:00'")
            .fetch_one(&pool)
            .await
            .expect("timestamp");
    assert_eq!(timestamp, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn foreign_keys_on() {
    let (_dir, pool) = pool(1).await;
    let on: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .expect("pragma");
    assert_eq!(on, 1);
}
