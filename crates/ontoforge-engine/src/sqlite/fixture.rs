//! Fixture planning and loading for the SQLite stand-in.
//!
//! A fixture stands in for a physical table. Its column names come from the binding, the
//! column types from the properties the binding maps; cell text is converted to those types.

use camino::Utf8Path;
use ontoforge_schema::ir::{BaseType, Ontology, ProfileName};
use ontoforge_schema::loader::sourcemap::{DefKey, DefKind, Part};
use ontoforge_schema::loader::{
    MAX_FILE_BYTES, SourceMap, fixture_header, fixture_path, read_limited,
};

use crate::bind_check::{BindingKey, ExpectedColumn, expected_columns};
use crate::printer::sqlite_ddl::SqliteStorage;
use crate::{PhysicalType, TableRef};

/// The logical type of a fixture column, derived from the property type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalType {
    String,
    Integer { min: i64, max: i64 },
    Float,
    Decimal { precision: u32, scale: u32 },
    Boolean,
    Date,
    Timestamp { aware: bool },
    Array(Box<LogicalType>),
}

/// A column a fixture table must carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureColumn {
    pub name: String,
    pub logical: LogicalType,
}

/// One physical table to build; bindings that name the same table share it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureTable {
    pub def_names: Vec<String>,
    pub table: TableRef,
    /// Path relative to the ontology root.
    pub file: String,
    pub columns: Vec<FixtureColumn>,
}

/// A problem with a fixture. It never repeats cell contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureError {
    pub file: String,
    pub line: Option<u64>,
    pub message: String,
}

impl FixtureError {
    fn new(file: &str, line: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            file: file.to_owned(),
            line,
            message: message.into(),
        }
    }
}

/// The tables to build for one profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixturePlan {
    pub tables: Vec<FixtureTable>,
    pub without_fixture: Vec<BindingKey>,
}

/// The logical type of the column a binding expects.
fn logical_type(column: &ExpectedColumn) -> LogicalType {
    let aware = column.timezone.as_deref() == Some("aware");
    let scalar = |base: BaseType| match base {
        BaseType::Byte => LogicalType::Integer {
            min: i64::from(i8::MIN),
            max: i64::from(i8::MAX),
        },
        BaseType::Short => LogicalType::Integer {
            min: i64::from(i16::MIN),
            max: i64::from(i16::MAX),
        },
        BaseType::Integer => LogicalType::Integer {
            min: i64::from(i32::MIN),
            max: i64::from(i32::MAX),
        },
        BaseType::Long => LogicalType::Integer {
            min: i64::MIN,
            max: i64::MAX,
        },
        BaseType::Float | BaseType::Double => LogicalType::Float,
        BaseType::Decimal => LogicalType::Decimal {
            precision: column.precision,
            scale: column.scale,
        },
        BaseType::Boolean => LogicalType::Boolean,
        BaseType::Date => LogicalType::Date,
        BaseType::Timestamp => LogicalType::Timestamp { aware },
        // A geopoint is text; nested arrays are not a property type.
        BaseType::String | BaseType::Geopoint | BaseType::Array => LogicalType::String,
    };
    match column.base {
        BaseType::Array => {
            LogicalType::Array(Box::new(scalar(column.items.unwrap_or(BaseType::String))))
        }
        base => scalar(base),
    }
}

/// Plans the fixture tables of the bindings of `profile`.
pub fn plan_fixture_tables(
    ontology: &Ontology,
    sources: &SourceMap,
    profile: &ProfileName,
) -> Result<FixturePlan, Vec<FixtureError>> {
    let mut plan = FixturePlan {
        tables: Vec::new(),
        without_fixture: Vec::new(),
    };
    let mut errors = Vec::new();
    let Some(bindings) = ontology.profile_bindings(profile) else {
        return Ok(plan);
    };
    for binding in bindings.objects.values().chain(bindings.links.values()) {
        let key = BindingKey {
            profile: profile.clone(),
            target: binding.target.clone(),
        };
        let Some(fixture) = &binding.fixture else {
            plan.without_fixture.push(key);
            continue;
        };
        let def_name = key.def_name();
        let binding_file = sources
            .locate(
                &DefKey {
                    kind: DefKind::SourceBinding,
                    api_name: def_name.clone(),
                },
                &[],
                Part::Value,
            )
            .file;
        match fixture_path(&binding_file, &fixture.file) {
            Ok(file) => plan.tables.push(FixtureTable {
                def_names: vec![def_name],
                table: TableRef {
                    catalog: binding.source.catalog.clone(),
                    schema: binding.source.schema.clone(),
                    table: binding.source.table.clone(),
                },
                file,
                columns: expected_columns(ontology, binding)
                    .iter()
                    .map(|c| FixtureColumn {
                        name: c.column.clone(),
                        logical: logical_type(c),
                    })
                    .collect(),
            }),
            Err(message) => errors.push(FixtureError::new(&binding_file, None, message)),
        }
    }
    if errors.is_empty() {
        Ok(plan)
    } else {
        Err(errors)
    }
}

/// Most records one fixture may hold.
pub const MAX_RECORDS: usize = 100_000;

/// A value ready to be bound to an `INSERT`.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code, reason = "numeric values arrive with the typed conversion")]
pub(super) enum SqlValue {
    Null,
    Text(String),
    Integer(i64),
    Real(f64),
}

/// A column of a loaded table: as stored in SQLite and as reported by `describe_tables`.
#[derive(Debug)]
pub(super) struct LoadedColumn {
    pub name: String,
    pub storage: SqliteStorage,
    pub physical: PhysicalType,
}

/// A fixture read and converted, not yet inserted.
#[derive(Debug)]
pub(super) struct LoadedTable {
    pub columns: Vec<LoadedColumn>,
    pub rows: Vec<Vec<SqlValue>>,
    /// How many bytes the file had.
    pub bytes: u64,
}

fn storage_of(logical: &LogicalType) -> SqliteStorage {
    match logical {
        LogicalType::Integer { .. } | LogicalType::Boolean => SqliteStorage::Integer,
        LogicalType::Float => SqliteStorage::Real,
        _ => SqliteStorage::Text,
    }
}

fn physical_of(logical: &LogicalType) -> PhysicalType {
    match logical {
        LogicalType::String => PhysicalType::Varchar { length: None },
        LogicalType::Integer { min, max } => {
            if *min >= i64::from(i8::MIN) && *max <= i64::from(i8::MAX) {
                PhysicalType::Tinyint
            } else if *min >= i64::from(i16::MIN) && *max <= i64::from(i16::MAX) {
                PhysicalType::Smallint
            } else if *min >= i64::from(i32::MIN) && *max <= i64::from(i32::MAX) {
                PhysicalType::Integer
            } else {
                PhysicalType::Bigint
            }
        }
        LogicalType::Float => PhysicalType::Double,
        LogicalType::Decimal { precision, scale } => PhysicalType::Decimal {
            precision: u8::try_from(*precision).unwrap_or(u8::MAX),
            scale: u8::try_from(*scale).unwrap_or(u8::MAX),
        },
        LogicalType::Boolean => PhysicalType::Boolean,
        LogicalType::Date => PhysicalType::Date,
        LogicalType::Timestamp { aware } => PhysicalType::Timestamp {
            precision: 3,
            with_time_zone: *aware,
        },
        LogicalType::Array(inner) => PhysicalType::Array(Box::new(physical_of(inner))),
    }
}

/// A column of the file: the declared type of the table column, or none for a column the
/// binding does not map.
struct FileColumn {
    name: String,
    logical: Option<LogicalType>,
}

/// Reads and converts the fixture of `table`. `remaining` is how many bytes of the ontology's
/// fixture budget are left; `limit` is the whole budget, for the message.
pub(super) fn load_table(
    root: &Utf8Path,
    table: &FixtureTable,
    remaining: u64,
    limit: u64,
) -> Result<LoadedTable, FixtureError> {
    let file = table.file.as_str();
    let err = |line: Option<u64>, message: String| FixtureError::new(file, line, message);
    let bytes = read_limited(root.join(file).as_std_path(), false, MAX_FILE_BYTES)
        .map_err(|failure| err(None, format!("the fixture cannot be read: {failure:?}")))?;
    let size = bytes.len() as u64;
    if size > remaining {
        return Err(err(
            None,
            format!("the fixtures of this ontology exceed the byte budget of {limit} bytes"),
        ));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| err(None, "the fixture is not valid UTF-8".to_owned()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let header = fixture_header(file, text).map_err(|message| err(None, message))?;

    let mut seen = std::collections::HashSet::new();
    let mut columns = Vec::with_capacity(header.len());
    for name in header {
        if name.is_empty() {
            return Err(err(
                None,
                "the fixture has a column without a name".to_owned(),
            ));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(err(
                None,
                format!(
                    "the fixture has two columns named `{name}` (names are compared in lower case)"
                ),
            ));
        }
        let logical = table
            .columns
            .iter()
            .find(|c| c.name.to_lowercase() == name.to_lowercase())
            .map(|c| c.logical.clone());
        columns.push(FileColumn { name, logical });
    }
    if columns.is_empty() {
        return Err(err(None, "the fixture declares no columns".to_owned()));
    }

    let rows = if file.to_ascii_lowercase().ends_with(".csv") {
        csv_rows(file, text, &columns)?
    } else {
        return Err(err(None, "JSON fixtures are not loaded yet".to_owned()));
    };
    let loaded = columns
        .into_iter()
        .map(|c| {
            let logical = c.logical.unwrap_or(LogicalType::String);
            LoadedColumn {
                name: c.name,
                storage: storage_of(&logical),
                physical: physical_of(&logical),
            }
        })
        .collect();
    Ok(LoadedTable {
        columns: loaded,
        rows,
        bytes: size,
    })
}

fn csv_rows(
    file: &str,
    text: &str,
    columns: &[FileColumn],
) -> Result<Vec<Vec<SqlValue>>, FixtureError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| {
            let line = e.position().map(csv::Position::line);
            FixtureError::new(
                file,
                line,
                "the record is not valid CSV or has the wrong number of fields",
            )
        })?;
        let line = record.position().map(csv::Position::line);
        if rows.len() >= MAX_RECORDS {
            return Err(FixtureError::new(
                file,
                line,
                format!("the fixture has more than {MAX_RECORDS} records"),
            ));
        }
        rows.push(
            record
                .iter()
                .zip(columns)
                .map(|(cell, column)| convert_cell(column, cell))
                .collect::<Result<Vec<_>, String>>()
                .map_err(|message| FixtureError::new(file, line, message))?,
        );
    }
    Ok(rows)
}

/// Converts one CSV cell. An empty cell is NULL for every type.
#[allow(
    clippy::unnecessary_wraps,
    reason = "strict conversion arrives with the typed fixture semantics"
)]
fn convert_cell(column: &FileColumn, cell: &str) -> Result<SqlValue, String> {
    if cell.is_empty() {
        return Ok(SqlValue::Null);
    }
    let _ = &column.logical;
    Ok(SqlValue::Text(cell.to_owned()))
}
