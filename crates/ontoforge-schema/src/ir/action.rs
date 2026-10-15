//! Action type definition (SCHEMA-05): the only way object data changes.
//!
//! Shape per ADR-0012. Validation of references and types lives in
//! `validate::actions`; CEL in `submissionCriteria` is checked by `ontoforge-expr`.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::basetype::BaseType;
use super::names::{ActionTypeName, LinkTypeName, ObjectTypeName, ParameterName, PropertyName};

/// A named, validated change to the object graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActionType {
    /// Unique `camelCase` name; callers and agents invoke the action by it.
    pub api_name: ActionTypeName,
    /// Human-readable name shown in UIs.
    pub display_name: Option<String>,
    /// What this action does.
    pub description: Option<String>,
    /// Inputs, keyed by their camelCase apiName.
    #[serde(default)]
    pub parameters: IndexMap<ParameterName, ParameterDef>,
    /// Edits applied in order; the order is significant and at least one rule is
    /// required. Each rule is written `kind: <ruleKind>` followed by its fields.
    pub rules: Vec<Rule>,
    /// Conditions that must hold before the rules run.
    #[serde(default)]
    pub submission_criteria: Vec<SubmissionCriterion>,
}

/// One input of an action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ParameterDef {
    /// A base type, or `{ objectReference: <ObjectType> }` for an existing object.
    #[serde(rename = "type")]
    pub param_type: ParamType,
    /// Element type; required when `type` is `array`, not allowed otherwise.
    pub items: Option<BaseType>,
    /// Whether the caller must supply a value.
    #[serde(default)]
    pub required: bool,
    /// Value used when the caller supplies none. Must be a valid literal of the
    /// parameter type; not allowed for object references.
    pub default: Option<serde_json::Value>,
    /// Human-readable name shown in UIs.
    pub display_name: Option<String>,
    /// What this parameter means.
    pub description: Option<String>,
}

/// The type of a parameter: one of the thirteen base types or an object reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ParamType {
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
    /// An existing object of this type, written `{ objectReference: Flight }`.
    ObjectReference(ObjectTypeName),
}

impl ParamType {
    /// The base type, or `None` for an object reference.
    pub fn base_type(&self) -> Option<BaseType> {
        Some(match self {
            Self::Byte => BaseType::Byte,
            Self::Short => BaseType::Short,
            Self::Integer => BaseType::Integer,
            Self::Long => BaseType::Long,
            Self::Float => BaseType::Float,
            Self::Double => BaseType::Double,
            Self::Decimal => BaseType::Decimal,
            Self::String => BaseType::String,
            Self::Boolean => BaseType::Boolean,
            Self::Date => BaseType::Date,
            Self::Timestamp => BaseType::Timestamp,
            Self::Array => BaseType::Array,
            Self::Geopoint => BaseType::Geopoint,
            Self::ObjectReference(_) => return None,
        })
    }
}

/// One edit of an action. Rules run in list order.
///
/// In definition files a rule is `{ kind: createObject, .. }`; the loader turns that
/// into this externally tagged form before deserializing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(tag = "kind")]
pub enum Rule {
    /// Create a new object.
    CreateObject(CreateObjectRule),
    /// Change properties of an existing object passed as a parameter.
    ModifyObject(ModifyObjectRule),
    /// Delete an existing object passed as a parameter.
    DeleteObject(DeleteObjectRule),
    /// Link two existing objects.
    AddLink(LinkRule),
    /// Unlink two existing objects.
    RemoveLink(LinkRule),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateObjectRule {
    /// Type of the new object.
    pub object_type: ObjectTypeName,
    /// Property values. The primary key and every required property must appear.
    pub properties: IndexMap<PropertyName, ValueSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ModifyObjectRule {
    /// Name of an `objectReference` parameter holding the object to change.
    pub object: ParameterName,
    /// Property values to set. The primary key cannot be set.
    pub properties: IndexMap<PropertyName, ValueSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DeleteObjectRule {
    /// Name of an `objectReference` parameter holding the object to delete.
    pub object: ParameterName,
}

/// Shared by `addLink` and `removeLink`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LinkRule {
    /// The link type to edit.
    pub link: LinkTypeName,
    /// `objectReference` parameter for the link's `from` object type.
    pub from: ParameterName,
    /// `objectReference` parameter for the link's `to` object type.
    pub to: ParameterName,
}

/// Where a property value comes from, written as a single-key map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ValueSource {
    /// The value of a parameter: `{ param: name }`.
    Param(ParameterName),
    /// A literal: `{ static: value }`. `null` is not allowed.
    Static(serde_json::Value),
    /// The calling user's id, for string properties: `{ currentUser: {} }`.
    CurrentUser(Empty),
    /// The time of the call, for timestamp properties: `{ currentTime: {} }`.
    CurrentTime(Empty),
}

impl ValueSource {
    /// The key this source is written under.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Param(_) => "param",
            Self::Static(_) => "static",
            Self::CurrentUser(_) => "currentUser",
            Self::CurrentTime(_) => "currentTime",
        }
    }
}

/// The empty mapping `{}`; no fields are accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

/// A condition checked before the rules run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SubmissionCriterion {
    /// A CEL expression over `params`, `user` and `objects` that must be true.
    pub condition: String,
    /// Shown to the caller when the condition is false.
    pub failure_message: String,
}
