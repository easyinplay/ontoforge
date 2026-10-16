//! SQLite stand-in engine: one connection, in-memory.
//!
//! `ATTACH` and `:memory:` databases are per connection state, so the engine owns exactly one
//! connection behind a mutex instead of a pool.

use std::collections::BTreeMap;

use futures_util::TryStreamExt;
use sqlx::sqlite::{SqliteConnection, SqliteRow};
use sqlx::{
    AssertSqlSafe, Column as _, Connection, Executor, Row, SqlSafeStr, Statement, TypeInfo,
    ValueRef,
};
use tokio::sync::Mutex;

use crate::printer::ast::{Expr, Literal, Select, SelectItem};
use crate::printer::print_select;
use crate::{
    Column, Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PhysicalType, PrintedSql,
    QueryContext, QueryOutput, TableRef, Value,
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

    // The stand-in answers from its in-memory metadata map; later plans make it query SQLite.
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
            .filter_map(|t| self.tables.get(t).map(|cols| (t.clone(), cols.clone())))
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
