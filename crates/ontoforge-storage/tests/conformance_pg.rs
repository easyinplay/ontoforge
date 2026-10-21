//! The conformance cases on the PostgreSQL edit store (STORE-01, STORE-03).
//!
//! Every test is `#[ignore]`d: they need a PostgreSQL server. Run them with
//! `ONTOFORGE_PG_URL=postgres://... cargo test -p ontoforge-storage --test conformance_pg -- --ignored`
//! or, on a machine that can download and run PostgreSQL, with
//! `cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg -- --ignored`.
//! Without either, `support::pg_store()` panics.

mod conformance;
mod support;

conformance::all_cases!(
    conformance::pg,
    ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"
);

/// `open_postgres` pins the session settings that the text read-back of `date`, `numeric` and
/// float arrays depends on (WR-A03). The database below has defaults that disagree with the
/// pinned values, so the `SHOW`s read what the store's own connection options really produce.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_sessions_use_the_pinned_settings() {
    use secrecy::SecretString;
    use sqlx::postgres::PgConnectOptions;
    use sqlx::{Connection, PgConnection};
    use std::str::FromStr;

    async fn show(options: &PgConnectOptions, setting: &str) -> String {
        let mut conn = PgConnection::connect_with(options)
            .await
            .expect("connect to the guard database");
        let sql = format!("SHOW {setting}");
        let value: String = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_one(&mut conn)
            .await
            .expect("SHOW");
        conn.close().await.expect("close");
        value
    }

    let pg = support::pg_store().await;
    let db = format!(
        "of_t_{}_db",
        ulid::Ulid::generate().to_string().to_lowercase()
    );
    for sql in [
        format!("CREATE DATABASE \"{db}\""),
        format!("ALTER DATABASE \"{db}\" SET \"DateStyle\" = 'SQL, DMY'"),
        format!("ALTER DATABASE \"{db}\" SET extra_float_digits = 0"),
        format!("ALTER DATABASE \"{db}\" SET \"TimeZone\" = 'America/New_York'"),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .execute(pg.side())
            .await
            .expect("prepare the guard database");
    }

    // The control: without the store's options the database defaults win, so the settings
    // below are not just what PostgreSQL does anyway.
    let plain = PgConnectOptions::from_str(pg.url())
        .expect("test url")
        .database(&db)
        .extra_float_digits(None);
    assert_eq!(show(&plain, "extra_float_digits").await, "0");

    let pinned = ontoforge_storage::store::pg_connect_options(
        &SecretString::from(pg.url().to_owned()),
        &pg.schema_sys,
    )
    .expect("pg_connect_options")
    .database(&db);
    let date_style = show(&pinned, "DateStyle").await;
    let float_digits = show(&pinned, "extra_float_digits").await;
    let time_zone = show(&pinned, "TimeZone").await;
    let search_path = show(&pinned, "search_path").await;
    let seen = format!(
        "DateStyle={date_style}, extra_float_digits={float_digits}, TimeZone={time_zone}, \
         search_path={search_path}"
    );
    assert_eq!(date_style, "ISO, MDY", "{seen}");
    assert_eq!(float_digits, "3", "{seen}");
    assert_eq!(time_zone, "UTC", "{seen}");
    assert_eq!(search_path, pg.schema_sys, "{seen}");

    pg.side().close().await;
    let admin = PgConnectOptions::from_str(pg.url()).expect("test url");
    let mut conn = PgConnection::connect_with(&admin).await.expect("admin");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE \"{db}\" WITH (FORCE)"
    )))
    .execute(&mut conn)
    .await
    .expect("drop the guard database");
}
