//! Object type definition (SCHEMA-02) and its properties (SCHEMA-03).

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::basetype::BaseType;
use super::binding::Backing;
use super::names::{ObjectTypeName, PropertyName};

/// A kind of object, for example `Flight`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ObjectType {
    /// Unique `PascalCase` name used in code, APIs and references.
    pub api_name: ObjectTypeName,
    /// Human-readable name shown in UIs.
    pub display_name: Option<String>,
    /// What this object type represents.
    pub description: Option<String>,
    /// Alternative names, used for search and by agents.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Hide the object type from default listings.
    #[serde(default)]
    pub hidden: bool,
    /// Where the objects come from. Required: `source` needs a source binding in
    /// the selected profile, `editOnly` must have none (D-02).
    pub backing: Backing,
    /// Property that uniquely identifies an object. It must be declared in
    /// `properties`, set `required: true`, and be a string, integer or short
    /// (other types are discouraged or invalid).
    pub primary_key: PropertyName,
    /// Property used as the object's title; defaults to `primaryKey`.
    pub title_key: Option<PropertyName>,
    /// Properties keyed by their camelCase apiName.
    pub properties: IndexMap<PropertyName, PropertyDef>,
}

impl ObjectType {
    /// The title property: `titleKey` when set, otherwise the primary key.
    pub fn effective_title_key(&self) -> &PropertyName {
        self.title_key.as_ref().unwrap_or(&self.primary_key)
    }
}

/// One property of an object type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PropertyDef {
    /// Base type of the property value.
    #[serde(rename = "type")]
    pub base_type: BaseType,
    /// Whether every object must have a value.
    #[serde(default)]
    pub required: bool,
    /// Human-readable name shown in UIs.
    pub display_name: Option<String>,
    /// What this property holds.
    pub description: Option<String>,
    /// Element type; required when `type` is `array`, not allowed otherwise.
    pub items: Option<BaseType>,
    /// Total number of digits of a `decimal` (or of an `array` of `decimal`).
    /// Defaults to 38 (ADR-0015, ADR-0021).
    pub precision: Option<u32>,
    /// Number of digits after the decimal point of a `decimal` (or of an `array`
    /// of `decimal`). Defaults to 10 (ADR-0015, ADR-0021).
    pub scale: Option<u32>,
    /// Create an index on this property's edit store column (ADR-0016). Defaults
    /// to false. The primary key column always has an index, so it needs no
    /// declaration.
    #[serde(default)]
    pub indexed: bool,
}

/// Precision of a `decimal` that does not write `precision`.
pub const DEFAULT_DECIMAL_PRECISION: u32 = 38;

/// Scale of a `decimal` that does not write `scale`.
pub const DEFAULT_DECIMAL_SCALE: u32 = 10;

impl PropertyDef {
    /// `(precision, scale)` with each missing value replaced by its own default.
    ///
    /// The only definition of the decimal defaults; lint, the engine and the binding
    /// comparison all call it.
    pub fn effective_decimal(&self) -> (u32, u32) {
        (
            self.precision.unwrap_or(DEFAULT_DECIMAL_PRECISION),
            self.scale.unwrap_or(DEFAULT_DECIMAL_SCALE),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decimal(precision: Option<u32>, scale: Option<u32>) -> PropertyDef {
        PropertyDef {
            base_type: BaseType::Decimal,
            required: false,
            display_name: None,
            description: None,
            items: None,
            precision,
            scale,
            indexed: false,
        }
    }

    #[test]
    fn effective_decimal_defaults() {
        assert_eq!(decimal(None, None).effective_decimal(), (38, 10));
        assert_eq!(decimal(Some(12), None).effective_decimal(), (12, 10));
        assert_eq!(decimal(None, Some(2)).effective_decimal(), (38, 2));
        assert_eq!(decimal(Some(12), Some(2)).effective_decimal(), (12, 2));
        assert_eq!((DEFAULT_DECIMAL_PRECISION, DEFAULT_DECIMAL_SCALE), (38, 10));
    }
}
