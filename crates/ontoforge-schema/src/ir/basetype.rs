//! The thirteen base types of a property (D-03) and where they may be used.

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

/// Types that accept each other's values (D-07). Byte, short and integer share a
/// class, as do float and double; every other type is its own class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueClass {
    Integer,
    Long,
    Double,
    Decimal,
    String,
    Boolean,
    Date,
    Timestamp,
    Geopoint,
    Array,
}

/// RED stub, replaced in the GREEN commit.
pub fn value_class(_t: BaseType) -> ValueClass {
    ValueClass::String
}

/// RED stub, replaced in the GREEN commit.
pub fn check_literal(
    _v: &serde_json::Value,
    _t: BaseType,
    _items: Option<BaseType>,
) -> Result<(), String> {
    Ok(())
}
