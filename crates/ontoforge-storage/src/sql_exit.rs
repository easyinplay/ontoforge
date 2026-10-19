//! The only module through which SQL text reaches a database driver (ADR-0024, T-03-07).
//!
//! All SQL text of this crate is either a `sea-query` product (DDL and DML) or a fixed
//! statement defined in this file. `sqlx` 0.9 refuses dynamic strings unless they are wrapped in
//! `AssertSqlSafe`; this is the only file that does so, and `tests/sql_exit_guard.rs` fails the
//! build when another source file writes SQL text or `AssertSqlSafe`.

use sea_query::{Alias, Expr, ExprTrait, Func, PostgresQueryBuilder, Query, SqliteQueryBuilder};
use sea_query_sqlx::{SqlxBinder, SqlxValues};
use sqlx::{AssertSqlSafe, PgPool, Row};

use crate::error::StoreError;
use crate::store::{ActiveDefinition, Backend, Db, WriteTx};

/// A statement built by `sea-query` together with its bound values.
pub(crate) type Built = (String, SqlxValues);

const BEGIN_IMMEDIATE: &str = "BEGIN IMMEDIATE";

const STATE_TABLE: &str = "of_def_state";
const VERSION_TABLE: &str = "of_def_version";

/// Builds `stmt` for the dialect of the transaction.
pub(crate) fn build<S: SqlxBinder>(stmt: &S, tx: &WriteTx<'_>) -> Built {
    match tx {
        WriteTx::Sqlite(_) => stmt.build_sqlx(SqliteQueryBuilder),
        WriteTx::Postgres { .. } => stmt.build_sqlx(PostgresQueryBuilder),
    }
}

/// Creates the PostgreSQL schema `name` unless it exists. The name is quoted.
pub(crate) async fn create_schema(pool: &PgPool, name: &str) -> Result<(), StoreError> {
    let sql = format!(
        "CREATE SCHEMA IF NOT EXISTS \"{}\"",
        name.replace('"', "\"\"")
    );
    sqlx::query(AssertSqlSafe(sql))
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| StoreError::from_sqlx(&e))
}

/// Starts a write transaction. SQLite takes the write lock immediately.
pub(crate) async fn begin_write(db: &Db) -> Result<WriteTx<'_>, StoreError> {
    match db {
        Db::Sqlite(pool) => pool
            .begin_with(BEGIN_IMMEDIATE)
            .await
            .map(WriteTx::Sqlite)
            .map_err(|e| StoreError::from_sqlx(&e)),
        Db::Postgres { pool, schema_edit } => pool
            .begin()
            .await
            .map(|tx| WriteTx::Postgres {
                tx,
                schema_edit: schema_edit.clone(),
            })
            .map_err(|e| StoreError::from_sqlx(&e)),
    }
}

pub(crate) async fn commit(tx: WriteTx<'_>) -> Result<(), StoreError> {
    match tx {
        WriteTx::Sqlite(t) => t.commit().await.map_err(|e| StoreError::from_sqlx(&e)),
        WriteTx::Postgres { tx, .. } => tx.commit().await.map_err(|e| StoreError::from_sqlx(&e)),
    }
}

/// Runs one DDL statement.
pub(crate) async fn execute_ddl(tx: &mut WriteTx<'_>, sql: &str) -> Result<(), StoreError> {
    match tx {
        WriteTx::Sqlite(t) => sqlx::query(AssertSqlSafe(sql.to_owned()))
            .execute(&mut **t)
            .await
            .map(|_| ())
            .map_err(|e| StoreError::from_sqlx(&e)),
        WriteTx::Postgres { tx, .. } => sqlx::query(AssertSqlSafe(sql.to_owned()))
            .execute(&mut **tx)
            .await
            .map(|_| ())
            .map_err(|e| StoreError::from_sqlx(&e)),
    }
}

/// The column names of `table` (of `schema` on PostgreSQL), empty when the table does not exist.
pub(crate) async fn table_columns(
    tx: &mut WriteTx<'_>,
    schema: Option<&str>,
    table: &str,
) -> Result<Vec<String>, StoreError> {
    match tx {
        WriteTx::Sqlite(t) => {
            // `PRAGMA` cannot bind a name, so the name is quoted. Names come from apiNames and
            // never hold a quote; the doubling is for a name that does.
            let sql = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
            let rows = sqlx::query(AssertSqlSafe(sql))
                .fetch_all(&mut **t)
                .await
                .map_err(|e| StoreError::from_sqlx(&e))?;
            rows.iter()
                .map(|row| {
                    row.try_get::<String, _>("name")
                        .map_err(|e| StoreError::from_sqlx(&e))
                })
                .collect()
        }
        WriteTx::Postgres { tx, .. } => {
            // The information schema columns are domain typed; casting keeps sqlx's type check
            // and the comparison with the bound text simple.
            let text = |column: &str| Expr::col(Alias::new(column)).cast_as(Alias::new("text"));
            let (sql, values) = Query::select()
                .expr(text("column_name"))
                .from((Alias::new("information_schema"), Alias::new("columns")))
                .and_where(text("table_schema").eq(schema.unwrap_or_default()))
                .and_where(text("table_name").eq(table))
                .build_sqlx(PostgresQueryBuilder);
            sqlx::query_scalar_with::<_, String, _>(AssertSqlSafe(sql), values)
                .fetch_all(&mut **tx)
                .await
                .map_err(|e| StoreError::from_sqlx(&e))
        }
    }
}

/// Runs a built statement and returns the number of affected rows.
pub(crate) async fn execute_built(
    tx: &mut WriteTx<'_>,
    (sql, values): Built,
) -> Result<u64, StoreError> {
    match tx {
        WriteTx::Sqlite(t) => sqlx::query_with(AssertSqlSafe(sql), values)
            .execute(&mut **t)
            .await
            .map(|done| done.rows_affected())
            .map_err(|e| StoreError::from_sqlx(&e)),
        WriteTx::Postgres { tx, .. } => sqlx::query_with(AssertSqlSafe(sql), values)
            .execute(&mut **tx)
            .await
            .map(|done| done.rows_affected())
            .map_err(|e| StoreError::from_sqlx(&e)),
    }
}

/// Runs a built statement that returns one nullable integer.
pub(crate) async fn fetch_scalar_i64(
    tx: &mut WriteTx<'_>,
    (sql, values): Built,
) -> Result<Option<i64>, StoreError> {
    match tx {
        WriteTx::Sqlite(t) => {
            sqlx::query_scalar_with::<_, Option<i64>, _>(AssertSqlSafe(sql), values)
                .fetch_one(&mut **t)
                .await
                .map_err(|e| StoreError::from_sqlx(&e))
        }
        WriteTx::Postgres { tx, .. } => {
            sqlx::query_scalar_with::<_, Option<i64>, _>(AssertSqlSafe(sql), values)
                .fetch_one(&mut **tx)
                .await
                .map_err(|e| StoreError::from_sqlx(&e))
        }
    }
}

/// The definition version to give the next definition.
pub(crate) async fn next_def_version(tx: &mut WriteTx<'_>) -> Result<i64, StoreError> {
    let select = Query::select()
        .expr(Func::max(Expr::col(Alias::new("def_version"))))
        .from(Alias::new(VERSION_TABLE))
        .to_owned();
    let built = build(&select, tx);
    Ok(fetch_scalar_i64(tx, built).await?.unwrap_or(0) + 1)
}

/// Marks the active definition as superseded.
pub(crate) async fn supersede_active(tx: &mut WriteTx<'_>) -> Result<(), StoreError> {
    let update = Query::update()
        .table(Alias::new(VERSION_TABLE))
        .value(Alias::new("status"), "superseded")
        .and_where(Expr::col(Alias::new("status")).eq("active"))
        .to_owned();
    let built = build(&update, tx);
    execute_built(tx, built).await.map(|_| ())
}

/// Stores `version` as the active definition and points `of_def_state` at it.
pub(crate) async fn activate_definition(
    tx: &mut WriteTx<'_>,
    version: i64,
    ontology_json: &str,
    physical_json: &str,
    now: &str,
) -> Result<(), StoreError> {
    // PostgreSQL types these columns `jsonb` and `timestamptz`; the values travel as text and
    // are cast there. SQLite stores the text as it is.
    let postgres = tx.backend() == Backend::Postgres;
    let typed = |value: &str, ty: &str| {
        let expr = Expr::val(value.to_owned());
        if postgres {
            expr.cast_as(Alias::new(ty))
        } else {
            expr
        }
    };
    let insert = Query::insert()
        .into_table(Alias::new(VERSION_TABLE))
        .columns([
            Alias::new("def_version"),
            Alias::new("status"),
            Alias::new("ontology_json"),
            Alias::new("physical_json"),
            Alias::new("created_at"),
            Alias::new("activated_at"),
        ])
        .values_panic([
            version.into(),
            "active".into(),
            typed(ontology_json, "jsonb"),
            typed(physical_json, "jsonb"),
            typed(now, "timestamptz"),
            typed(now, "timestamptz"),
        ])
        .to_owned();
    let built = build(&insert, tx);
    execute_built(tx, built).await?;

    let state = Query::update()
        .table(Alias::new(STATE_TABLE))
        .value(Alias::new("active_def_version"), version)
        .and_where(Expr::col(Alias::new("id")).eq(1))
        .to_owned();
    let built = build(&state, tx);
    execute_built(tx, built).await.map(|_| ())
}

/// Reads the active definition, `None` before the first reload.
pub(crate) async fn read_active_definition(
    db: &Db,
) -> Result<Option<ActiveDefinition>, StoreError> {
    let postgres = matches!(db, Db::Postgres { .. });
    let build = |stmt: &sea_query::SelectStatement| {
        if postgres {
            stmt.build_sqlx(PostgresQueryBuilder)
        } else {
            stmt.build_sqlx(SqliteQueryBuilder)
        }
    };
    let (sql, values) = build(
        Query::select()
            .column(Alias::new("active_def_version"))
            .from(Alias::new(STATE_TABLE)),
    );
    let version: i64 = match db {
        Db::Sqlite(pool) => {
            sqlx::query_scalar_with(AssertSqlSafe(sql), values)
                .fetch_one(pool)
                .await
        }
        Db::Postgres { pool, .. } => {
            sqlx::query_scalar_with(AssertSqlSafe(sql), values)
                .fetch_one(pool)
                .await
        }
    }
    .map_err(|e| StoreError::from_sqlx(&e))?;
    if version == 0 {
        return Ok(None);
    }
    // `jsonb` columns are read as text.
    let as_text = |column: &str| {
        let expr = Expr::col(Alias::new(column));
        if postgres {
            expr.cast_as(Alias::new("text"))
        } else {
            expr
        }
    };
    let (sql, values) = build(
        Query::select()
            .expr(as_text("ontology_json"))
            .expr(as_text("physical_json"))
            .from(Alias::new(VERSION_TABLE))
            .and_where(Expr::col(Alias::new("def_version")).eq(version)),
    );
    let row: Option<(String, String)> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as_with(AssertSqlSafe(sql), values)
                .fetch_optional(pool)
                .await
        }
        Db::Postgres { pool, .. } => {
            sqlx::query_as_with(AssertSqlSafe(sql), values)
                .fetch_optional(pool)
                .await
        }
    }
    .map_err(|e| StoreError::from_sqlx(&e))?;
    let (ontology_json, physical_json) = row.ok_or_else(|| {
        StoreError::Corrupt(format!(
            "of_def_state points at definition version {version}, which does not exist"
        ))
    })?;
    let corrupt = |what: &str, e: serde_json::Error| {
        StoreError::Corrupt(format!(
            "definition version {version}: cannot read {what}: {e}"
        ))
    };
    Ok(Some(ActiveDefinition {
        def_version: version,
        ontology: serde_json::from_str(&ontology_json).map_err(|e| corrupt("ontology_json", e))?,
        physical: serde_json::from_str(&physical_json).map_err(|e| corrupt("physical_json", e))?,
    }))
}
