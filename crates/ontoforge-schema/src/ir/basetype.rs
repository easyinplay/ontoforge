//! The thirteen base types of a property (D-03) and where they may be used.

use chrono::{DateTime, NaiveDate};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BaseType {
    Byte,
    Short,
    Integer,
    Long,
    Float,
    Double,
    Decimal,
    String,
    Boolean,
    Date,
    Timestamp,
    Array,
    Geopoint,
}

impl BaseType {
    /// The spelling used in definition files.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Byte => "byte",
            Self::Short => "short",
            Self::Integer => "integer",
            Self::Long => "long",
            Self::Float => "float",
            Self::Double => "double",
            Self::Decimal => "decimal",
            Self::String => "string",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Timestamp => "timestamp",
            Self::Array => "array",
            Self::Geopoint => "geopoint",
        }
    }
}

/// How well a base type serves as a primary key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkSuitability {
    /// Accepted without comment.
    Valid,
    /// Accepted by `validate`; `lint` warns (E09).
    Discouraged,
    /// Rejected by `validate` (`ONT-V10`).
    Invalid,
}

/// Primary key suitability of `t`. Defined once; `validate` (V10) and lint (E09)
/// both read this table.
pub fn pk_suitability(t: BaseType) -> PkSuitability {
    match t {
        BaseType::String | BaseType::Integer | BaseType::Short => PkSuitability::Valid,
        BaseType::Date
        | BaseType::Timestamp
        | BaseType::Boolean
        | BaseType::Byte
        | BaseType::Long => PkSuitability::Discouraged,
        BaseType::Float
        | BaseType::Double
        | BaseType::Decimal
        | BaseType::Geopoint
        | BaseType::Array => PkSuitability::Invalid,
    }
}

/// Whether a property of type `t` (with element type `items`) can be a title.
/// Every v1 type can; arrays are judged by their element type.
pub fn is_title_suitable(t: BaseType, items: Option<BaseType>) -> bool {
    match (t, items) {
        (BaseType::Array, Some(inner)) => is_title_suitable(inner, None),
        _ => true,
    }
}

/// Whether a value of type `from` can be assigned to a property of type `to` without
/// loss (D-07, WR-04): the same type, or a wider one along `byte ⊂ short ⊂ integer ⊂
/// long` or `float ⊂ double`. Narrowing (`long` into `integer`) is refused.
pub fn widens_to(from: BaseType, to: BaseType) -> bool {
    use BaseType::{Byte, Double, Float, Integer, Long, Short};
    let rank = |t| match t {
        Byte => Some(0),
        Short => Some(1),
        Integer => Some(2),
        Long => Some(3),
        _ => None,
    };
    match (rank(from), rank(to)) {
        (Some(a), Some(b)) => a <= b,
        _ => from == to || (from == Float && to == Double),
    }
}

/// Check that the JSON value `v` is a legal literal of type `t`. `items` is the
/// element type for arrays; without it array elements are not checked. The error
/// is a short English phrase meant to sit inside a diagnostic message.
pub fn check_literal(v: &Value, t: BaseType, items: Option<BaseType>) -> Result<(), String> {
    if v.is_null() {
        return Err("null is not a valid value".to_owned());
    }
    let wrong = |expected: &str| Err(format!("expected {expected}, found {}", describe(v)));
    match t {
        BaseType::Byte => integer_in(v, "byte", i8::MIN.into(), i8::MAX.into()),
        BaseType::Short => integer_in(v, "short", i16::MIN.into(), i16::MAX.into()),
        BaseType::Integer => integer_in(v, "integer", i32::MIN.into(), i32::MAX.into()),
        BaseType::Long => integer_in(v, "long", i64::MIN, i64::MAX),
        BaseType::Float | BaseType::Double => match v {
            Value::Number(_) => Ok(()),
            _ => wrong("a number"),
        },
        BaseType::Decimal => match v {
            Value::Number(_) => Ok(()),
            Value::String(s) if is_decimal(s) => Ok(()),
            Value::String(s) => Err(format!("`{s}` is not a decimal number")),
            _ => wrong("a number or a decimal string"),
        },
        BaseType::String => match v {
            Value::String(_) => Ok(()),
            _ => wrong("a string"),
        },
        BaseType::Boolean => match v {
            Value::Bool(_) => Ok(()),
            _ => wrong("true or false"),
        },
        BaseType::Date => match v {
            Value::String(s) if is_date(s) => Ok(()),
            Value::String(s) => Err(format!("`{s}` is not a date written as YYYY-MM-DD")),
            _ => wrong("a date string"),
        },
        BaseType::Timestamp => match v {
            Value::String(s) if DateTime::parse_from_rfc3339(s).is_ok() => Ok(()),
            Value::String(s) => Err(format!("`{s}` is not an RFC 3339 timestamp")),
            _ => wrong("a timestamp string"),
        },
        BaseType::Geopoint => match v {
            Value::String(s) => geopoint(s),
            _ => wrong("a \"lat,lon\" string"),
        },
        BaseType::Array => match v {
            Value::Array(elements) => {
                let Some(inner) = items else {
                    return Ok(());
                };
                elements.iter().enumerate().try_for_each(|(i, e)| {
                    check_literal(e, inner, None).map_err(|m| format!("element {i}: {m}"))
                })
            }
            _ => wrong("an array"),
        },
    }
}

fn describe(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "a mapping",
    }
}

fn integer_in(v: &Value, name: &str, lo: i64, hi: i64) -> Result<(), String> {
    let Value::Number(n) = v else {
        return Err(format!("expected an integer, found {}", describe(v)));
    };
    match n.as_i64() {
        Some(i) if (lo..=hi).contains(&i) => Ok(()),
        Some(_) => Err(format!("{n} is out of range for {name} ({lo} to {hi})")),
        None if n.is_u64() => Err(format!("{n} is out of range for {name} ({lo} to {hi})")),
        None => Err(format!("{n} is not an integer")),
    }
}

fn is_decimal(s: &str) -> bool {
    let unsigned = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    digits(whole) && digits(fraction) && !(whole.is_empty() && fraction.is_empty())
}

fn is_date(s: &str) -> bool {
    // `%Y` alone would also take signs and long years; ten bytes pins YYYY-MM-DD.
    s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

fn geopoint(s: &str) -> Result<(), String> {
    let parsed = s.split_once(',').and_then(|(lat, lon)| {
        let lat = lat.trim().parse::<f64>().ok()?;
        let lon = lon.trim().parse::<f64>().ok()?;
        Some((lat, lon))
    });
    match parsed {
        Some((lat, lon)) if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) => {
            Ok(())
        }
        Some(_) => Err(format!(
            "`{s}` is outside latitude -90 to 90 or longitude -180 to 180"
        )),
        None => Err(format!("`{s}` is not a \"lat,lon\" pair")),
    }
}
