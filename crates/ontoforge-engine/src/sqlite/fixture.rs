//! Fixture planning and loading for the SQLite stand-in.
//!
//! A fixture stands in for a physical table. Its column names come from the binding, the
//! column types from the properties the binding maps; cell text is converted to those types.

use camino::Utf8Path;
use chrono::{NaiveDate, NaiveTime};
use ontoforge_schema::ir::{BaseType, Ontology, ProfileName};
use ontoforge_schema::loader::sourcemap::{DefKey, DefKind, Part};
use ontoforge_schema::loader::{
    MAX_FILE_BYTES, SourceMap, fixture_header, fixture_path, read_limited,
};

use crate::bind_check::{BindingKey, ExpectedColumn, expected_columns};
use crate::printer::sqlite_ddl::SqliteStorage;
use crate::value::split_decimal;
use crate::{PhysicalType, TableRef};

/// The logical type of a fixture column, derived from the property type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalType {
    String,
    Integer {
        min: i64,
        max: i64,
    },
    /// A `float` property: single precision, reported as `real`.
    Float,
    /// A `double` property: reported as `double`.
    Double,
    Decimal {
        precision: u32,
        scale: u32,
    },
    Boolean,
    Date,
    Timestamp {
        aware: bool,
    },
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
        BaseType::Float => LogicalType::Float,
        BaseType::Double => LogicalType::Double,
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
        let file = match fixture_path(&binding_file, &fixture.file) {
            Ok(file) => file,
            Err(message) => {
                errors.push(FixtureError::new(&binding_file, None, message));
                continue;
            }
        };
        let table = TableRef {
            catalog: binding.source.catalog.clone(),
            schema: binding.source.schema.clone(),
            table: binding.source.table.clone(),
        };
        let columns: Vec<FixtureColumn> = expected_columns(ontology, binding)
            .iter()
            .map(|c| FixtureColumn {
                name: c.column.clone(),
                logical: logical_type(c),
            })
            .collect();
        // One physical table is one stand-in table, whichever bindings name it (E18). SQLite
        // compares identifiers without regard to case, so the key is lower case.
        let existing = plan.tables.iter_mut().find(|t| {
            let (a, b) = (&t.table, &table);
            a.catalog.to_lowercase() == b.catalog.to_lowercase()
                && a.schema.to_lowercase() == b.schema.to_lowercase()
                && a.table.to_lowercase() == b.table.to_lowercase()
        });
        match existing {
            None => plan.tables.push(FixtureTable {
                def_names: vec![def_name],
                table,
                file,
                columns,
            }),
            Some(shared) => {
                if let Err(e) = merge_into(shared, &def_name, &file, columns) {
                    errors.push(e);
                } else {
                    shared.def_names.push(def_name);
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(plan)
    } else {
        Err(errors)
    }
}

/// Adds the binding `def_name` to the table `shared`: it must use the same fixture file, and a
/// column both bindings name must have the same logical type. The columns become the union.
fn merge_into(
    shared: &mut FixtureTable,
    def_name: &str,
    file: &str,
    columns: Vec<FixtureColumn>,
) -> Result<(), FixtureError> {
    let table = format!(
        "{}.{}.{}",
        shared.table.catalog, shared.table.schema, shared.table.table
    );
    let first = shared.def_names.first().map_or("", String::as_str);
    if shared.file != file {
        return Err(FixtureError::new(
            file,
            None,
            format!(
                "bindings `{first}` and `{def_name}` bind table `{table}` to different fixture \
                 files (`{}` and `{file}`); bindings that share a table must share its fixture",
                shared.file
            ),
        ));
    }
    for column in columns {
        match shared
            .columns
            .iter()
            .find(|c| c.name.to_lowercase() == column.name.to_lowercase())
        {
            Some(known) if known.logical != column.logical => {
                return Err(FixtureError::new(
                    file,
                    None,
                    format!(
                        "bindings `{first}` and `{def_name}` map column `{}` of table `{table}` \
                         to different types",
                        column.name
                    ),
                ));
            }
            Some(_) => {}
            None => shared.columns.push(column),
        }
    }
    Ok(())
}

/// Most records one fixture may hold.
pub const MAX_RECORDS: usize = 100_000;

/// A value ready to be bound to an `INSERT`.
#[derive(Debug, Clone, PartialEq)]
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
        LogicalType::Float | LogicalType::Double => SqliteStorage::Real,
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
        LogicalType::Float => PhysicalType::Real,
        LogicalType::Double => PhysicalType::Double,
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

    let rows = if file.to_ascii_lowercase().ends_with(".json") {
        json_rows(file, text, &columns)?
    } else {
        csv_rows(file, text, &columns)?
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

/// Converts one CSV cell. An empty cell is NULL for every type; a column the binding does not
/// map is kept as text.
fn convert_cell(column: &FileColumn, cell: &str) -> Result<SqlValue, String> {
    if cell.is_empty() {
        return Ok(SqlValue::Null);
    }
    let converted = match &column.logical {
        Some(logical) => convert_text(logical, cell),
        None => Ok(SqlValue::Text(cell.to_owned())),
    };
    converted.map_err(|message| format!("column `{}`: {message}", column.name))
}

/// The rows of a JSON fixture: a top-level array of objects. A missing key is NULL.
fn json_rows(
    file: &str,
    text: &str,
    columns: &[FileColumn],
) -> Result<Vec<Vec<SqlValue>>, FixtureError> {
    let not_json = |message: &str| FixtureError::new(file, None, message);
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| not_json("the fixture is not valid JSON"))?;
    let serde_json::Value::Array(objects) = value else {
        return Err(not_json("expected an array of objects at the top level"));
    };
    if objects.len() > MAX_RECORDS {
        return Err(not_json(&format!(
            "the fixture has more than {MAX_RECORDS} records"
        )));
    }
    let mut rows = Vec::with_capacity(objects.len());
    for (index, object) in objects.into_iter().enumerate() {
        let number = u64::try_from(index + 1).unwrap_or(u64::MAX);
        let at = |message: String| {
            FixtureError::new(file, Some(number), format!("object {number}: {message}"))
        };
        let serde_json::Value::Object(object) = object else {
            return Err(at("not an object".to_owned()));
        };
        let row = columns
            .iter()
            .map(|column| {
                let value = object.get(&column.name).unwrap_or(&serde_json::Value::Null);
                let converted = match &column.logical {
                    Some(logical) => convert_json(logical, value),
                    None => Ok(unmapped_json(value)),
                };
                converted.map_err(|message| format!("column `{}`: {message}", column.name))
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(at)?;
        rows.push(row);
    }
    Ok(rows)
}

/// A JSON value in a column the binding does not map: strings as they are, anything else as
/// its JSON text.
fn unmapped_json(value: &serde_json::Value) -> SqlValue {
    match value {
        serde_json::Value::Null => SqlValue::Null,
        serde_json::Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

/// Precision and scale of a decimal are limited to what Trino supports; this also bounds the
/// padding a normalized value gets.
const MAX_DECIMAL_DIGITS: u32 = 38;

/// Converts non-empty cell text to `logical`.
fn convert_text(logical: &LogicalType, cell: &str) -> Result<SqlValue, String> {
    match logical {
        LogicalType::String => Ok(SqlValue::Text(cell.to_owned())),
        LogicalType::Integer { min, max } => {
            // `i64::from_str` accepts a leading `+`; a fixture must not.
            let parsed = if cell.starts_with('+') {
                None
            } else {
                cell.parse::<i64>().ok()
            };
            parsed
                .ok_or_else(|| "expected an integer".to_owned())
                .and_then(|n| check_range(n, *min, *max))
        }
        LogicalType::Float | LogicalType::Double => cell
            .parse::<f64>()
            .ok()
            .filter(|f| f.is_finite())
            .map(SqlValue::Real)
            .ok_or_else(|| "expected a finite number".to_owned()),
        LogicalType::Decimal { precision, scale } => {
            normalize_decimal(cell, *precision, *scale).map(SqlValue::Text)
        }
        LogicalType::Boolean => match cell {
            "true" => Ok(SqlValue::Integer(1)),
            "false" => Ok(SqlValue::Integer(0)),
            _ => Err("expected `true` or `false` in lower case".to_owned()),
        },
        LogicalType::Date => parse_date(cell).map(|date| SqlValue::Text(date.to_string())),
        LogicalType::Timestamp { aware } => normalize_timestamp(cell, *aware).map(SqlValue::Text),
        LogicalType::Array(inner) => {
            let value: serde_json::Value = serde_json::from_str(cell)
                .map_err(|_| "expected a JSON array such as [\"a\",\"b\"]".to_owned())?;
            convert_json(&LogicalType::Array(inner.clone()), &value)
        }
    }
}

/// Converts a JSON value to `logical`. `null` is NULL for every type.
fn convert_json(logical: &LogicalType, value: &serde_json::Value) -> Result<SqlValue, String> {
    use serde_json::Value as Json;
    if value.is_null() {
        return Ok(SqlValue::Null);
    }
    match (logical, value) {
        (LogicalType::String, Json::String(s)) => Ok(SqlValue::Text(s.clone())),
        (LogicalType::String, _) => Err("expected a JSON string".to_owned()),
        (LogicalType::Integer { min, max }, Json::Number(n)) => n
            .as_i64()
            .ok_or_else(|| "expected an integer".to_owned())
            .and_then(|n| check_range(n, *min, *max)),
        (LogicalType::Integer { .. }, _) => Err("expected a JSON integer".to_owned()),
        (LogicalType::Float | LogicalType::Double, Json::Number(n)) => n
            .as_f64()
            .filter(|f| f.is_finite())
            .map(SqlValue::Real)
            .ok_or_else(|| "expected a finite number".to_owned()),
        (LogicalType::Float | LogicalType::Double, _) => Err("expected a JSON number".to_owned()),
        (LogicalType::Decimal { precision, scale }, Json::String(s)) => {
            normalize_decimal(s, *precision, *scale).map(SqlValue::Text)
        }
        (LogicalType::Decimal { .. }, _) => Err(
            "a decimal must be written as a JSON string, for example \"1.50\", so that it never \
             passes through a floating point number"
                .to_owned(),
        ),
        (LogicalType::Boolean, Json::Bool(b)) => Ok(SqlValue::Integer(i64::from(*b))),
        (LogicalType::Boolean, _) => Err("expected a JSON true or false".to_owned()),
        (LogicalType::Date, Json::String(s)) => {
            parse_date(s).map(|date| SqlValue::Text(date.to_string()))
        }
        (LogicalType::Timestamp { aware }, Json::String(s)) => {
            normalize_timestamp(s, *aware).map(SqlValue::Text)
        }
        (LogicalType::Date | LogicalType::Timestamp { .. }, _) => {
            Err("expected a JSON string".to_owned())
        }
        (LogicalType::Array(inner), Json::Array(items)) => {
            let elements = items
                .iter()
                .map(|item| convert_json(inner, item).map(|v| element_json(inner, v)))
                .collect::<Result<Vec<_>, String>>()
                .map_err(|message| format!("array element: {message}"))?;
            serde_json::to_string(&elements)
                .map(SqlValue::Text)
                .map_err(|_| "the array cannot be written as JSON".to_owned())
        }
        (LogicalType::Array(_), _) => Err("expected a JSON array".to_owned()),
    }
}

/// The JSON form an array element is stored as.
fn element_json(logical: &LogicalType, value: SqlValue) -> serde_json::Value {
    use serde_json::Value as Json;
    match value {
        SqlValue::Null => Json::Null,
        SqlValue::Text(s) => Json::String(s),
        SqlValue::Integer(i) if *logical == LogicalType::Boolean => Json::Bool(i != 0),
        SqlValue::Integer(i) => Json::from(i),
        SqlValue::Real(f) => serde_json::Number::from_f64(f).map_or(Json::Null, Json::Number),
    }
}

fn check_range(n: i64, min: i64, max: i64) -> Result<SqlValue, String> {
    if (min..=max).contains(&n) {
        Ok(SqlValue::Integer(n))
    } else {
        Err(format!(
            "outside the range {min} to {max} of the property type"
        ))
    }
}

/// Normalizes a decimal to exactly `scale` fractional digits without ever rounding: the value
/// is refused when it has more fractional digits than `scale` or more integer digits than
/// `precision - scale`. The syntax is [`split_decimal`]'s.
fn normalize_decimal(text: &str, precision: u32, scale: u32) -> Result<String, String> {
    if precision > MAX_DECIMAL_DIGITS || scale > MAX_DECIMAL_DIGITS {
        return Err(format!(
            "the property declares more than {MAX_DECIMAL_DIGITS} decimal digits"
        ));
    }
    let parts =
        split_decimal(text).ok_or_else(|| "expected a decimal number such as -12.50".to_owned())?;
    let scale_len = usize::try_from(scale).unwrap_or(0);
    if parts.fraction.len() > scale_len {
        return Err(format!(
            "more than {scale} fractional digits; a fixture value is never rounded"
        ));
    }
    let integer = parts.integer.trim_start_matches('0');
    let integer_limit = usize::try_from(precision.saturating_sub(scale)).unwrap_or(0);
    if integer.len() > integer_limit {
        return Err(format!(
            "more than {integer_limit} integer digits (precision {precision}, scale {scale})"
        ));
    }
    let integer = if integer.is_empty() { "0" } else { integer };
    let is_zero = integer == "0" && parts.fraction.bytes().all(|b| b == b'0');
    let mut out = String::new();
    if parts.negative && !is_zero {
        out.push('-');
    }
    out.push_str(integer);
    if scale_len > 0 {
        out.push('.');
        out.push_str(parts.fraction);
        out.extend(std::iter::repeat_n('0', scale_len - parts.fraction.len()));
    }
    Ok(out)
}

/// A date written exactly as `YYYY-MM-DD`.
fn parse_date(text: &str) -> Result<NaiveDate, String> {
    let b = text.as_bytes();
    let shaped = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shaped {
        return Err("expected a date written as YYYY-MM-DD".to_owned());
    }
    let number = |range: std::ops::Range<usize>| text[range].parse::<u32>().ok();
    let year = i32::try_from(number(0..4).unwrap_or(0)).unwrap_or(0);
    NaiveDate::from_ymd_opt(year, number(5..7).unwrap_or(0), number(8..10).unwrap_or(0))
        .ok_or_else(|| "not a calendar date".to_owned())
}

/// Splits a trailing UTC offset (`Z` or `+HH:MM` / `-HH:MM`) off a timestamp.
fn split_offset(text: &str) -> Result<(&str, Option<String>), String> {
    if let Some(body) = text.strip_suffix('Z') {
        return Ok((body, Some("+00:00".to_owned())));
    }
    // The date contributes `-` characters, all before index 11.
    let Some(sign) = text
        .get(11..)
        .and_then(|time| time.rfind(['+', '-']))
        .map(|p| p + 11)
    else {
        return Ok((text, None));
    };
    let offset = &text[sign..];
    let b = offset.as_bytes();
    let shaped =
        b.len() == 6 && b[3] == b':' && [1, 2, 4, 5].iter().all(|&i| b[i].is_ascii_digit());
    let hours = offset.get(1..3).and_then(|h| h.parse::<u32>().ok());
    let minutes = offset.get(4..6).and_then(|m| m.parse::<u32>().ok());
    match (shaped, hours, minutes) {
        (true, Some(h), Some(m)) if h <= 23 && m <= 59 => {
            Ok((&text[..sign], Some(offset.to_owned())))
        }
        _ => Err("the UTC offset must be written as Z or +HH:MM".to_owned()),
    }
}

/// Normalizes a timestamp to `YYYY-MM-DD HH:MM:SS.fff`, with a `+HH:MM` offset when the column
/// is `aware`. A local column (one with an IANA zone) must not carry an offset, an aware column
/// must. Nothing is rounded: more than three fractional digits are refused.
fn normalize_timestamp(text: &str, aware: bool) -> Result<String, String> {
    if !text.is_ascii() {
        return Err("expected a timestamp such as 2026-03-29 02:30:00".to_owned());
    }
    let (body, offset) = split_offset(text)?;
    let shape = || "expected a timestamp such as 2026-03-29 02:30:00".to_owned();
    if body.len() < 19 || !matches!(body.as_bytes()[10], b' ' | b'T') {
        return Err(shape());
    }
    let date = parse_date(&body[..10])?;
    let (clock, fraction) = match body[11..].split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (&body[11..], None),
    };
    let b = clock.as_bytes();
    if b.len() != 8 || b[2] != b':' || b[5] != b':' {
        return Err(shape());
    }
    let part = |range: std::ops::Range<usize>| clock[range].parse::<u32>().ok();
    let time = match (part(0..2), part(3..5), part(6..8)) {
        (Some(h), Some(m), Some(s)) => NaiveTime::from_hms_opt(h, m, s),
        _ => None,
    }
    .ok_or_else(|| "not a time of day".to_owned())?;
    let millis = match fraction {
        None => "000".to_owned(),
        Some(f) if (1..=3).contains(&f.len()) && f.bytes().all(|c| c.is_ascii_digit()) => {
            format!("{f:0<3}")
        }
        Some(_) => {
            return Err(
                "fractional seconds must have one to three digits (timestamp(3))".to_owned(),
            );
        }
    };
    let mut out = format!("{date} {time}.{millis}");
    match (aware, offset) {
        (true, Some(offset)) => out.push_str(&offset),
        (true, None) => {
            return Err("this column carries a UTC offset: write Z or +HH:MM".to_owned());
        }
        (false, Some(_)) => {
            return Err(
                "this column holds local times: remove the UTC offset, or mark the column \
                 `timezone: aware`"
                    .to_owned(),
            );
        }
        (false, None) => {}
    }
    Ok(out)
}
