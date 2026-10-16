//! Wire model of a `QueryResults` page and decoding of its cells.

use serde::Deserialize;

use crate::error::EngineError;
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

impl WireError {
    pub(super) fn into_engine_error(self) -> EngineError {
        EngineError::Query {
            error_name: self.error_name.unwrap_or_else(|| "UNKNOWN".to_owned()),
            message: self.message,
        }
    }
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

fn decode_value(ty: &PhysicalType, json: &serde_json::Value) -> Value {
    if json.is_null() {
        return Value::Null;
    }
    let decoded = match (ty, json) {
        (
            PhysicalType::Tinyint
            | PhysicalType::Smallint
            | PhysicalType::Integer
            | PhysicalType::Bigint,
            serde_json::Value::Number(n),
        ) => n.as_i64().map(Value::Long),
        (PhysicalType::Varchar { .. }, serde_json::Value::String(s)) => {
            Some(Value::String(s.clone()))
        }
        (PhysicalType::Boolean, serde_json::Value::Bool(b)) => Some(Value::Boolean(*b)),
        _ => None,
    };
    decoded.unwrap_or_else(|| Value::Unsupported(json.clone()))
}
