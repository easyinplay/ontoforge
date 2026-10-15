//! Object type definition. Plan 01-04 completes the field set.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::basetype::BaseType;
use super::names::{ObjectTypeName, PropertyName};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ObjectType {
    pub api_name: ObjectTypeName,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub primary_key: PropertyName,
    pub properties: IndexMap<PropertyName, PropertyDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PropertyDef {
    #[serde(rename = "type")]
    pub base_type: BaseType,
    #[serde(default)]
    pub required: bool,
}
