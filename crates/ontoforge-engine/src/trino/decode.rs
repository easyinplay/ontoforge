//! Wire model of a `QueryResults` page and decoding of its cells.

use serde::Deserialize;

use crate::value::{Column, PhysicalType, Value};

/// One page of a statement response. Unknown fields (stats, warnings, ...) are ignored.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueryResults {
    #[serde(default)]
    pub next_uri: Option<String>,
    #[serde(default)]
    pub columns: Option<Vec<WireColumn>>,
    #[serde(default)]
    pub data: Option<Vec<Vec<serde_json::Value>>>,
    #[serde(default)]
    pub error: Option<WireError>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WireColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WireError {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub error_name: Option<String>,
}

pub(super) fn columns(wire: &[WireColumn]) -> Vec<Column> {
    wire.iter()
        .map(|c| Column {
            name: c.name.clone(),
            physical: PhysicalType::parse(&c.type_name),
        })
        .collect()
}

/// Decodes one row. A cell that does not fit its column type becomes
/// [`Value::Unsupported`]; it never fails the query.
pub(super) fn decode_row(columns: &[Column], row: &[serde_json::Value]) -> Vec<Value> {
    columns
        .iter()
        .zip(row)
        .map(|(column, cell)| decode_value(&column.physical, cell))
        .collect()
}

/// A cell is `Null`, a value of its column type, or [`Value::Unsupported`] with the raw
/// JSON; it never fails the query (spike 1, point 7).
fn decode_value(ty: &PhysicalType, json: &serde_json::Value) -> Value {
    if json.is_null() {
        return Value::Null;
    }
    decode_typed(ty, json).unwrap_or_else(|| Value::Unsupported(json.clone()))
}

/// `None` when the type is not modelled or the JSON does not have the shape of the type.
fn decode_typed(ty: &PhysicalType, json: &serde_json::Value) -> Option<Value> {
    use serde_json::Value as J;

    match (ty, json) {
        (
            PhysicalType::Tinyint
            | PhysicalType::Smallint
            | PhysicalType::Integer
            | PhysicalType::Bigint,
            J::Number(n),
        ) => n.as_i64().map(Value::Long),
        (PhysicalType::Real | PhysicalType::Double, J::Number(n)) => n.as_f64().map(Value::Double),
        // Trino writes the non-finite values as strings.
        (PhysicalType::Real | PhysicalType::Double, J::String(s)) => match s.as_str() {
            "NaN" => Some(Value::Double(f64::NAN)),
            "Infinity" => Some(Value::Double(f64::INFINITY)),
            "-Infinity" => Some(Value::Double(f64::NEG_INFINITY)),
            _ => None,
        },
        // Numeric parsing of decimals is a later phase's business (ADR-0015).
        (PhysicalType::Decimal { .. } | PhysicalType::Number, J::String(s)) => {
            Some(Value::Decimal(s.clone()))
        }
        (PhysicalType::Varchar { .. } | PhysicalType::Char { .. }, J::String(s)) => {
            Some(Value::String(s.clone()))
        }
        (PhysicalType::Boolean, J::Bool(b)) => Some(Value::Boolean(*b)),
        (PhysicalType::Date, J::String(s)) => Some(Value::Date(s.clone())),
        (
            PhysicalType::Timestamp {
                with_time_zone: false,
                ..
            },
            J::String(s),
        ) => Some(Value::Timestamp(s.clone())),
        (
            PhysicalType::Timestamp {
                with_time_zone: true,
                ..
            },
            J::String(s),
        ) => Some(Value::TimestampTz(s.clone())),
        (PhysicalType::Array(inner), J::Array(items)) => Some(Value::Array(
            items.iter().map(|item| decode_value(inner, item)).collect(),
        )),
        _ => None,
    }
}
