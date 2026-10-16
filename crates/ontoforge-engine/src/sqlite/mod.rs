//! SQLite stand-in engine: one connection, in-memory.
//!
//! `ATTACH` and `:memory:` databases are per connection state, so the engine owns exactly one
//! connection behind a mutex instead of a pool.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use futures_util::TryStreamExt;
use ontoforge_schema::loader::MAX_TOTAL_BYTES;
use sqlx::sqlite::{SqliteConnection, SqliteRow};
use sqlx::{
    AssertSqlSafe, Column as _, Connection, Executor, Row, SqlSafeStr, Statement, TypeInfo,
    ValueRef,
};
use tokio::sync::Mutex;

pub mod fixture;

use fixture::{FixtureError, FixtureTable, SqlValue};

use crate::printer::ast::{Expr, Literal, Select, SelectItem};
use crate::printer::sqlite_ddl::{attach_memory, create_table, insert, query_only};
use crate::printer::{PrintedSql, print_select};
use crate::{
    Column, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PhysicalType, QueryContext,
    QueryOutput, TableRef, Value,
};

/// SQLite implementation of [`Engine`].
pub struct SqliteEngine {
    conn: Mutex<SqliteConnection>,
    tables: BTreeMap<TableRef, Vec<PhysicalColumn>>,
}

impl SqliteEngine {
    /// Opens an empty in-memory engine.
    pub async fn in_memory() -> Result<Self, EngineError> {
        let conn = SqliteConnection::connect("sqlite::memory:")
            .await
            .map_err(map_sqlx)?;
        Ok(Self {
            conn: Mutex::new(conn),
            tables: BTreeMap::new(),
        })
    }

    /// Builds an engine holding the given fixture tables, read from under `root`.
    ///
    /// Every table is created in the attached database `{catalog}__{schema}`. When all tables
    /// are loaded the connection is made read only, so nothing can write to the stand-in
    /// afterwards.
    pub async fn from_fixtures(
        root: &Utf8Path,
        tables: &[FixtureTable],
    ) -> Result<Self, Vec<FixtureError>> {
        Self::from_fixtures_with_budget(root, tables, MAX_TOTAL_BYTES).await
    }

    /// [`Self::from_fixtures`] with an explicit limit on the bytes of all fixture files.
    pub(crate) async fn from_fixtures_with_budget(
        root: &Utf8Path,
        tables: &[FixtureTable],
        budget: u64,
    ) -> Result<Self, Vec<FixtureError>> {
        let general = |message: String| {
            vec![FixtureError {
                file: String::new(),
                line: None,
                message,
            }]
        };
        let mut engine = Self::in_memory()
            .await
            .map_err(|e| general(e.to_string()))?;
        let Self {
            conn,
            tables: described,
        } = &mut engine;
        let conn = conn.get_mut();

        let mut errors = Vec::new();
        let mut attached = BTreeSet::new();
        let mut used = 0_u64;
        for table in tables {
            let loaded = match fixture::load_table(root, table, budget.saturating_sub(used), budget)
            {
                Ok(loaded) => loaded,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };
            used += loaded.bytes;
            let target = lower(&table.table);
            let result = store(conn, &mut attached, &table.file, &target, loaded).await;
            match result {
                Ok(columns) => {
                    described.insert(target, columns);
                }
                Err(e) => errors.push(e),
            }
        }
        if errors.is_empty() {
            run_ddl(conn, &query_only())
                .await
                .map_err(|e| general(map_sqlx(e).to_string()))?;
            Ok(engine)
        } else {
            Err(errors)
        }
    }
}

/// The table with every part in lower case: how the stand-in files it.
fn lower(table: &TableRef) -> TableRef {
    TableRef {
        catalog: table.catalog.to_lowercase(),
        schema: table.schema.to_lowercase(),
        table: table.table.to_lowercase(),
    }
}

async fn run_ddl(conn: &mut SqliteConnection, sql: &PrintedSql) -> Result<(), sqlx::Error> {
    sqlx::query(AssertSqlSafe(sql.as_str().to_owned()))
        .execute(conn)
        .await
        .map(|_| ())
}

/// Creates `target` and inserts the rows of `loaded`; returns the columns it reports.
async fn store(
    conn: &mut SqliteConnection,
    attached: &mut BTreeSet<String>,
    file: &str,
    target: &TableRef,
    loaded: fixture::LoadedTable,
) -> Result<Vec<PhysicalColumn>, FixtureError> {
    let fail = |message: String| FixtureError {
        file: file.to_owned(),
        line: None,
        message,
    };
    let database = |e: sqlx::Error| fail(map_sqlx(e).to_string());
    let print = |e: crate::PrintError| fail(e.to_string());

    let schema_name = target.sqlite_schema_name();
    if !attached.contains(&schema_name) {
        run_ddl(conn, &attach_memory(&schema_name).map_err(print)?)
            .await
            .map_err(database)?;
        attached.insert(schema_name);
    }
    let layout: Vec<_> = loaded
        .columns
        .iter()
        .map(|c| (c.name.clone(), c.storage))
        .collect();
    run_ddl(conn, &create_table(target, &layout).map_err(print)?)
        .await
        .map_err(database)?;
    let names: Vec<String> = loaded.columns.iter().map(|c| c.name.clone()).collect();
    let sql = insert(target, &names).map_err(print)?;
    for row in loaded.rows {
        let mut query = sqlx::query(AssertSqlSafe(sql.as_str().to_owned()));
        for value in row {
            query = match value {
                SqlValue::Null => query.bind(None::<String>),
                SqlValue::Text(s) => query.bind(s),
                SqlValue::Integer(i) => query.bind(i),
                SqlValue::Real(f) => query.bind(f),
            };
        }
        query.execute(&mut *conn).await.map_err(database)?;
    }
    Ok(loaded
        .columns
        .into_iter()
        .map(|c| PhysicalColumn {
            name: c.name,
            physical: c.physical,
            nullable: true,
        })
        .collect())
}

fn map_sqlx(e: sqlx::Error) -> EngineError {
    match e {
        sqlx::Error::Database(db) => EngineError::Query {
            error_name: "SQLITE_ERROR".to_owned(),
            message: db.message().to_owned(),
        },
        other => EngineError::Unavailable {
            retryable: false,
            reason: other.to_string(),
        },
    }
}

/// Decodes column `i` of `row` and returns the value with the number of TEXT / BLOB bytes
/// it holds.
fn decode(row: &SqliteRow, i: usize) -> Result<(Value, usize), EngineError> {
    let type_name = row
        .try_get_raw(i)
        .map_err(map_sqlx)?
        .type_info()
        .name()
        .to_owned();
    match type_name.as_str() {
        "INTEGER" => Ok((Value::Long(row.try_get(i).map_err(map_sqlx)?), 0)),
        "REAL" => Ok((Value::Double(row.try_get(i).map_err(map_sqlx)?), 0)),
        "TEXT" => {
            let s: String = row.try_get(i).map_err(map_sqlx)?;
            let n = s.len();
            Ok((Value::String(s), n))
        }
        "BLOB" => {
            let b: Vec<u8> = row.try_get(i).map_err(map_sqlx)?;
            let n = b.len();
            Ok((
                Value::Unsupported(serde_json::Value::String(format!("<blob {n} bytes>"))),
                n,
            ))
        }
        _ => Ok((Value::Null, 0)),
    }
}

impl Engine for SqliteEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        // `PrintedSql` is the proof that this text came out of the printer.
        let sql_text = AssertSqlSafe(sql.as_str().to_owned()).into_sql_str();
        let mut conn = self.conn.lock().await;
        let statement = (&mut *conn).prepare(sql_text).await.map_err(map_sqlx)?;
        let columns: Vec<Column> = statement
            .columns()
            .iter()
            .map(|c| {
                let declared = c.type_info().name();
                Column {
                    name: c.name().to_owned(),
                    physical: PhysicalType::Other(if declared == "NULL" {
                        String::new()
                    } else {
                        declared.to_owned()
                    }),
                }
            })
            .collect();

        let mut rows: Vec<Vec<Value>> = Vec::new();
        let mut bytes = 0usize;
        let mut truncated = false;
        let mut stream = statement.query().fetch(&mut *conn);
        while let Some(row) = stream.try_next().await.map_err(map_sqlx)? {
            if rows.len() >= ctx.max_rows {
                truncated = true;
                break;
            }
            let mut values = Vec::with_capacity(columns.len());
            let mut row_bytes = 0usize;
            for i in 0..columns.len() {
                let (value, n) = decode(&row, i)?;
                row_bytes += n;
                values.push(value);
            }
            if bytes + row_bytes > ctx.max_bytes {
                truncated = true;
                break;
            }
            bytes += row_bytes;
            rows.push(values);
        }
        Ok(QueryOutput {
            columns,
            rows,
            truncated,
        })
    }

    // The stand-in answers from the metadata it recorded while loading the fixtures; there is
    // nothing to await. Names are compared in lower case, as Trino exposes them.
    #[allow(clippy::unused_async_trait_impl)]
    async fn describe_tables(
        &self,
        tables: &[TableRef],
        _ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        if let Some(first) = tables.first()
            && let Some(other) = tables.iter().find(|t| t.catalog != first.catalog)
        {
            return Err(EngineError::Config(format!(
                "describe_tables needs one catalog per call, got `{}` and `{}`",
                first.catalog, other.catalog
            )));
        }
        Ok(tables
            .iter()
            .filter_map(|t| {
                self.tables
                    .get(&lower(t))
                    .map(|cols| (t.clone(), cols.clone()))
            })
            .collect())
    }

    async fn status(&self) -> EngineStatus {
        let probe = Select {
            projection: vec![SelectItem {
                expr: Expr::Literal(Literal::Long(1)),
                alias: None,
            }],
            from: None,
            filter: None,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
        };
        let result = match print_select(&probe, Dialect::Sqlite) {
            Ok(sql) => {
                self.execute(&sql, &QueryContext::new("status".to_owned(), Vec::new()))
                    .await
            }
            Err(e) => Err(EngineError::Print(e)),
        };
        match result {
            Ok(_) => EngineStatus::Available,
            Err(e) => EngineStatus::Unavailable {
                reason: e.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests;
