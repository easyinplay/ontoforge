//! The thirteen base types of a property (D-03).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
