//! The only module through which SQL text reaches a database driver (ADR-0024, T-03-07).
//!
//! All SQL text of this crate is either a `sea-query` product (DDL and DML) or a fixed
//! statement defined in this file. `sqlx` 0.9 refuses dynamic strings unless they are wrapped in
//! `AssertSqlSafe`; this is the only file that does so, and `tests/sql_exit_guard.rs` fails the
//! build when another source file writes SQL text or `AssertSqlSafe`.

use sea_query::{
    Alias, Expr, ExprTrait, Func, IntoTableRef, PostgresQueryBuilder, Query, SqliteQueryBuilder,
    TableRef,
};
use sea_query_sqlx::{SqlxBinder, SqlxValues};
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
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

/// Builds `stmt` for `backend`.
pub(crate) fn build_for<S: SqlxBinder>(stmt: &S, backend: Backend) -> Built {
    match backend {
        Backend::Sqlite => stmt.build_sqlx(SqliteQueryBuilder),
        Backend::Postgres => stmt.build_sqlx(PostgresQueryBuilder),
    }
}

/// An edit table (or any table of the edit schema): qualified on PostgreSQL, bare on SQLite.
pub(crate) fn edit_table(schema: Option<&str>, table: &str) -> TableRef {
    match schema {
        Some(schema) => (Alias::new(schema), Alias::new(table)).into_table_ref(),
        None => Alias::new(table).into_table_ref(),
    }
}

/// `value` as a typed literal: on PostgreSQL `CAST(value AS pg_type)`, on SQLite the text itself.
/// PostgreSQL types these columns `jsonb`, `timestamptz`, `numeric(p, s)` and so on; the values
/// travel as text and are cast there.
pub(crate) fn typed_text(backend: Backend, value: &str, pg_type: &str) -> Expr {
    let expr = Expr::val(value.to_owned());
    match backend {
        Backend::Postgres => expr.cast_as(Alias::new(pg_type)),
        Backend::Sqlite => expr,
    }
}

/// UTC with milliseconds, the one timestamp text of the edit log and the typed edit tables.
const PG_TIMESTAMP_TEXT: &str = r#"'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'"#;

/// A `timestamptz` column as `YYYY-MM-DDTHH:MM:SS.fffZ` text. SQLite holds the text already.
pub(crate) fn timestamp_text(backend: Backend, column: &str) -> Expr {
    match backend {
        Backend::Postgres => Expr::cust_with_expr(
            format!("to_char($1 AT TIME ZONE 'UTC', {PG_TIMESTAMP_TEXT})"),
            Expr::col(Alias::new(column)),
        ),
        Backend::Sqlite => Expr::col(Alias::new(column)),
    }
}

/// A column read as text: `jsonb`, `numeric` and `date` are cast on PostgreSQL, SQLite text is
/// read as it is.
pub(crate) fn column_text(backend: Backend, column: &str) -> Expr {
    let expr = Expr::col(Alias::new(column));
    match backend {
        Backend::Postgres => expr.cast_as(Alias::new("text")),
        Backend::Sqlite => expr,
    }
}

/// A PostgreSQL array column read as `text[]`. Timestamps are rendered in UTC with
/// milliseconds, so the text does not depend on the session time zone.
pub(crate) fn array_text(column: &str, timestamps: bool) -> Expr {
    if timestamps {
        Expr::cust_with_expr(
            format!(
                "ARRAY(SELECT to_char(x AT TIME ZONE 'UTC', {PG_TIMESTAMP_TEXT}) FROM unnest($1) AS x)"
            ),
            Expr::col(Alias::new(column)),
        )
    } else {
        Expr::col(Alias::new(column)).cast_as(Alias::new("text[]"))
    }
}

/// How to read one column of a row of a typed edit table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellKind {
    Bool,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Text,
    TextArray,
}

/// A column value without a driver type. SQLite holds arrays as JSON text, so there they come
/// back as `Text`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cell {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    TextArray(Vec<String>),
}

/// The first row of a built query as cells, `None` when there is no row.
pub(crate) async fn fetch_cells(
    tx: &mut WriteTx<'_>,
    (sql, values): Built,
    kinds: &[CellKind],
) -> Result<Option<Vec<Cell>>, StoreError> {
    match tx {
        WriteTx::Sqlite(t) => {
            let row = sqlx::query_with(AssertSqlSafe(sql), values)
                .fetch_optional(&mut **t)
                .await
                .map_err(|e| StoreError::from_sqlx(&e))?;
            row.map(|row| sqlite_cells(&row, kinds)).transpose()
        }
        WriteTx::Postgres { tx, .. } => {
            let row = sqlx::query_with(AssertSqlSafe(sql), values)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| StoreError::from_sqlx(&e))?;
            row.map(|row| pg_cells(&row, kinds)).transpose()
        }
    }
}

fn sqlite_cells(row: &SqliteRow, kinds: &[CellKind]) -> Result<Vec<Cell>, StoreError> {
    let fail = |e: &sqlx::Error| StoreError::from_sqlx(e);
    kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            Ok(match kind {
                CellKind::Bool => row
                    .try_get::<Option<bool>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Bool),
                CellKind::Int16 | CellKind::Int32 | CellKind::Int64 => row
                    .try_get::<Option<i64>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Int),
                CellKind::Float32 | CellKind::Float64 => row
                    .try_get::<Option<f64>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Float),
                CellKind::Text | CellKind::TextArray => row
                    .try_get::<Option<String>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Text),
            })
        })
        .collect()
}

fn pg_cells(row: &PgRow, kinds: &[CellKind]) -> Result<Vec<Cell>, StoreError> {
    let fail = |e: &sqlx::Error| StoreError::from_sqlx(e);
    kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            Ok(match kind {
                CellKind::Bool => row
                    .try_get::<Option<bool>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Bool),
                CellKind::Int16 => row
                    .try_get::<Option<i16>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, |v| Cell::Int(v.into())),
                CellKind::Int32 => row
                    .try_get::<Option<i32>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, |v| Cell::Int(v.into())),
                CellKind::Int64 => row
                    .try_get::<Option<i64>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Int),
                CellKind::Float32 => row
                    .try_get::<Option<f32>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, |v| Cell::Float(v.into())),
                CellKind::Float64 => row
                    .try_get::<Option<f64>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Float),
                CellKind::Text => row
                    .try_get::<Option<String>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::Text),
                CellKind::TextArray => row
                    .try_get::<Option<Vec<String>>, _>(i)
                    .map_err(|e| fail(&e))?
                    .map_or(Cell::Null, Cell::TextArray),
            })
        })
        .collect()
}

/// Every row of a built query, decoded into `O` (a tuple of plain types).
pub(crate) async fn fetch_all_as<O>(db: &Db, (sql, values): Built) -> Result<Vec<O>, StoreError>
where
    O: for<'r> sqlx::FromRow<'r, SqliteRow> + for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin,
{
    match db {
        Db::Sqlite(pool) => {
            sqlx::query_as_with::<_, O, _>(AssertSqlSafe(sql), values)
                .fetch_all(pool)
                .await
        }
        Db::Postgres { pool, .. } => {
            sqlx::query_as_with::<_, O, _>(AssertSqlSafe(sql), values)
                .fetch_all(pool)
                .await
        }
    }
    .map_err(|e| StoreError::from_sqlx(&e))
}

/// Rolls the transaction back. A failed rollback is ignored: the connection is discarded then.
pub(crate) async fn rollback(tx: WriteTx<'_>) {
    match tx {
        WriteTx::Sqlite(t) => drop(t.rollback().await),
        WriteTx::Postgres { tx, .. } => drop(tx.rollback().await),
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

/// The JSON documents and the time of a definition version that `activate_definition` stores.
pub(crate) struct DefinitionRecord<'a> {
    pub(crate) ontology_json: &'a str,
    pub(crate) physical_json: &'a str,
    /// The steps `migrate` ran to get here, `None` for a `reload`.
    pub(crate) migration_json: Option<&'a str>,
    pub(crate) now: &'a str,
}

/// Stores `version` as the active definition and points `of_def_state` at it.
pub(crate) async fn activate_definition(
    tx: &mut WriteTx<'_>,
    version: i64,
    record: &DefinitionRecord<'_>,
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
    let mut columns = vec![
        Alias::new("def_version"),
        Alias::new("status"),
        Alias::new("ontology_json"),
        Alias::new("physical_json"),
        Alias::new("created_at"),
        Alias::new("activated_at"),
    ];
    let mut values: Vec<Expr> = vec![
        version.into(),
        "active".into(),
        typed(record.ontology_json, "jsonb"),
        typed(record.physical_json, "jsonb"),
        typed(record.now, "timestamptz"),
        typed(record.now, "timestamptz"),
    ];
    if let Some(migration_json) = record.migration_json {
        columns.push(Alias::new("migration_json"));
        values.push(typed(migration_json, "jsonb"));
    }
    let insert = Query::insert()
        .into_table(Alias::new(VERSION_TABLE))
        .columns(columns)
        .values_panic(values)
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
    let (ontology_json, physical_json) = row.ok_or_else(|| missing_version(version))?;
    parse_definition(version, &ontology_json, &physical_json).map(Some)
}

fn missing_version(version: i64) -> StoreError {
    StoreError::Corrupt(format!(
        "of_def_state points at definition version {version}, which does not exist"
    ))
}

fn parse_definition(
    version: i64,
    ontology_json: &str,
    physical_json: &str,
) -> Result<ActiveDefinition, StoreError> {
    let corrupt = |what: &str, e: serde_json::Error| {
        StoreError::Corrupt(format!(
            "definition version {version}: cannot read {what}: {e}"
        ))
    };
    Ok(ActiveDefinition {
        def_version: version,
        ontology: serde_json::from_str(ontology_json).map_err(|e| corrupt("ontology_json", e))?,
        physical: serde_json::from_str(physical_json).map_err(|e| corrupt("physical_json", e))?,
    })
}

/// Reads the active definition inside the write transaction `tx`, `None` before the first
/// reload. `migrate` plans against what it reads here, so the plan it compares with the reviewed
/// one is the plan of the definition it is about to change.
pub(crate) async fn read_active_definition_in(
    tx: &mut WriteTx<'_>,
) -> Result<Option<ActiveDefinition>, StoreError> {
    let select = Query::select()
        .column(Alias::new("active_def_version"))
        .from(Alias::new(STATE_TABLE))
        .to_owned();
    let built = build(&select, tx);
    let version = fetch_scalar_i64(tx, built).await?.unwrap_or(0);
    if version == 0 {
        return Ok(None);
    }
    // `jsonb` columns are read as text.
    let backend = tx.backend();
    let select = Query::select()
        .expr(column_text(backend, "ontology_json"))
        .expr(column_text(backend, "physical_json"))
        .from(Alias::new(VERSION_TABLE))
        .and_where(Expr::col(Alias::new("def_version")).eq(version))
        .to_owned();
    let built = build(&select, tx);
    let cells = fetch_cells(tx, built, &[CellKind::Text, CellKind::Text])
        .await?
        .ok_or_else(|| missing_version(version))?;
    let [Cell::Text(ontology_json), Cell::Text(physical_json)] = cells.as_slice() else {
        return Err(missing_version(version));
    };
    parse_definition(version, ontology_json, physical_json).map(Some)
}
