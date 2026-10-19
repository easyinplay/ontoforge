//! SQLite stand-in engine: one connection, in-memory.
//!
//! `ATTACH` and `:memory:` databases are per connection state, so the engine owns exactly one
//! connection behind a mutex instead of a pool.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::time::Instant;

use camino::Utf8Path;
use chrono::{NaiveDateTime, TimeDelta};
use futures_util::TryStreamExt;
use ontoforge_schema::loader::MAX_TOTAL_BYTES;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqliteRow};
use sqlx::{
    AssertSqlSafe, Column as _, Connection, Executor, Row, SqlSafeStr, Statement, TypeInfo,
    ValueRef,
};
use tokio::sync::Mutex;

pub mod fixture;

use fixture::{FixtureError, FixtureTable, SqlValue};

use crate::printer::ast::{Expr, Literal, Select, SelectItem};
use crate::printer::sqlite_ddl::{
    DECIMAL_COLLATION, TIMESTAMP_COLLATION, attach_memory, create_table, insert, query_only,
};
use crate::printer::{PrintedSql, print_select};
use crate::value::split_decimal;
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
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(map_sqlx)?
            .collation(DECIMAL_COLLATION, compare_decimal)
            .collation(TIMESTAMP_COLLATION, compare_timestamp);
        let conn = SqliteConnection::connect_with(&options)
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
        let preflight = check_databases(tables);
        if !preflight.is_empty() {
            return Err(preflight);
        }
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

/// SQLite allows this many `ATTACH`ed databases.
const MAX_ATTACHED: usize = 10;

/// Checks, before anything is created, that the tables fit into at most [`MAX_ATTACHED`]
/// databases and that no two `(catalog, schema)` pairs share a database name.
fn check_databases(tables: &[FixtureTable]) -> Vec<FixtureError> {
    let mut errors = Vec::new();
    let mut databases: BTreeMap<String, (String, String)> = BTreeMap::new();
    for table in tables {
        let table_ref = lower(&table.table);
        let name = table_ref.sqlite_schema_name();
        let pair = (table_ref.catalog, table_ref.schema);
        match databases.get(&name) {
            None => {
                databases.insert(name, pair);
            }
            Some(first) if *first != pair => errors.push(FixtureError {
                file: table.file.clone(),
                line: None,
                message: format!(
                    "`{}`.`{}` and `{}`.`{}` would both be the stand-in database `{name}`; \
                     rename a catalog or schema so that `catalog__schema` differs",
                    first.0, first.1, pair.0, pair.1
                ),
            }),
            Some(_) => {}
        }
    }
    if databases.len() > MAX_ATTACHED {
        errors.push(FixtureError {
            file: tables.first().map(|t| t.file.clone()).unwrap_or_default(),
            line: None,
            message: format!(
                "the fixtures span {} catalog and schema pairs; the stand-in database holds at \
                 most {MAX_ATTACHED}",
                databases.len()
            ),
        });
    }
    errors
}

/// Orders `a` and `b` by `key`. Text without a key (never stored by the fixture loader, but a
/// query may compare against anything) sorts after every keyed text, by bytes, which keeps the
/// order total as SQLite requires of a collation.
fn compare_by<'a, K: Ord>(a: &'a str, b: &'a str, key: impl Fn(&'a str) -> Option<K>) -> Ordering {
    match (key(a), key(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.cmp(b),
    }
}

/// The magnitude of a decimal: integer digit count, integer digits (no leading zeros),
/// fraction digits (no trailing zeros). Ordering these tuples orders the magnitudes.
type Magnitude<'a> = (usize, &'a str, &'a str);

/// The value of a decimal text; variants are declared in ascending order.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum DecimalKey<'a> {
    Negative(Reverse<Magnitude<'a>>),
    NonNegative(Magnitude<'a>),
}

fn decimal_key(s: &str) -> Option<DecimalKey<'_>> {
    let parts = split_decimal(s)?;
    let integer = parts.integer.trim_start_matches('0');
    let fraction = parts.fraction.trim_end_matches('0');
    let magnitude = (integer.len(), integer, fraction);
    let zero = integer.is_empty() && fraction.is_empty();
    Some(if parts.negative && !zero {
        DecimalKey::Negative(Reverse(magnitude))
    } else {
        DecimalKey::NonNegative(magnitude)
    })
}

/// [`DECIMAL_COLLATION`]: decimal text by value.
fn compare_decimal(a: &str, b: &str) -> Ordering {
    compare_by(a, b, decimal_key)
}

/// The UTC instant of `YYYY-MM-DD HH:MM:SS[.f]` (1 to 9 fraction digits) followed by nothing,
/// `Z` or `+HH:MM` / `-HH:MM`. No offset means UTC, the session zone the Trino client sets.
fn timestamp_key(s: &str) -> Option<NaiveDateTime> {
    let clock = NaiveDateTime::parse_from_str(s.get(..19)?, "%Y-%m-%d %H:%M:%S").ok()?;
    let mut rest = &s[19..];
    let mut nanos = 0_i64;
    if let Some(after) = rest.strip_prefix('.') {
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        if !(1..=9).contains(&digits) {
            return None;
        }
        let padded = format!("{:0<9}", &after[..digits]);
        nanos = padded.parse().ok()?;
        rest = &after[digits..];
    }
    let offset_minutes = match rest.as_bytes() {
        [] | [b'Z'] => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2]
            if [h1, h2, m1, m2].iter().all(|d| d.is_ascii_digit()) =>
        {
            let minutes = i64::from((h1 - b'0') * 10 + (h2 - b'0')) * 60
                + i64::from((m1 - b'0') * 10 + (m2 - b'0'));
            if *sign == b'-' { -minutes } else { minutes }
        }
        _ => return None,
    };
    clock
        .checked_add_signed(TimeDelta::nanoseconds(nanos))?
        .checked_sub_signed(TimeDelta::minutes(offset_minutes))
}

/// [`TIMESTAMP_COLLATION`]: timestamp text by instant.
fn compare_timestamp(a: &str, b: &str) -> Ordering {
    compare_by(a, b, timestamp_key)
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
    let raw = row.try_get_raw(i).map_err(map_sqlx)?;
    // For NULL, sqlx reports the declared type of the column, so the value has to be asked.
    if raw.is_null() {
        return Ok((Value::Null, 0));
    }
    let type_name = raw.type_info().name().to_owned();
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

/// SQLite calls the progress handler about every this many virtual machine instructions.
const PROGRESS_OPS: i32 = 1_000;

fn timeout_error(ctx: &QueryContext) -> EngineError {
    EngineError::Timeout {
        after_ms: u64::try_from(ctx.timeout.as_millis()).unwrap_or(u64::MAX),
    }
}

impl SqliteEngine {
    /// The body of [`Engine::execute`]. SQLite interrupts the statement once `deadline` (if
    /// any) has passed; the failure that causes is reported as a timeout.
    async fn execute_until(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
        deadline: Option<Instant>,
    ) -> Result<QueryOutput, EngineError> {
        let expired = move || deadline.is_some_and(|d| Instant::now() >= d);
        let fail = |e: sqlx::Error| {
            if expired() {
                timeout_error(ctx)
            } else {
                map_sqlx(e)
            }
        };
        // `PrintedSql` is the proof that this text came out of the printer.
        let sql_text = AssertSqlSafe(sql.as_str().to_owned()).into_sql_str();
        let mut conn = self.conn.lock().await;
        // Replaces the handler an earlier query left behind.
        conn.lock_handle()
            .await
            .map_err(map_sqlx)?
            .set_progress_handler(PROGRESS_OPS, move || !expired());
        let statement = (&mut *conn).prepare(sql_text).await.map_err(fail)?;
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
        while let Some(row) = stream.try_next().await.map_err(fail)? {
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
}

impl Engine for SqliteEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    /// Runs `sql` within `ctx.timeout`, waiting for the connection included (WR-E04). When
    /// the time is up the caller gets [`EngineError::Timeout`] at once, and SQLite itself
    /// interrupts the statement at the same deadline, so an abandoned query does not keep the
    /// one connection busy.
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        let deadline = Instant::now().checked_add(ctx.timeout);
        match tokio::time::timeout(ctx.timeout, self.execute_until(sql, ctx, deadline)).await {
            Ok(result) => result,
            Err(_) => Err(timeout_error(ctx)),
        }
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
