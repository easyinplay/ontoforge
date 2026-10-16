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
}
