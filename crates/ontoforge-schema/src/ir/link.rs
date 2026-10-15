//! Link type definition (SCHEMA-04): a typed relationship between two object types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::names::{LinkEndName, LinkTypeName, ObjectTypeName, PropertyName};

/// How many objects each side of a link may relate to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Cardinality {
    /// One `from` object relates to at most one `to` object, and the other way round.
    /// The `to` end carries the foreign key.
    OneToOne,
    /// One `from` object relates to many `to` objects. The `to` end carries the
    /// foreign key.
    OneToMany,
    /// Many objects on each side. The system manages a join table, so no foreign
    /// key is written.
    ManyToMany,
}

/// A relationship between two object types, for example `Airport` to `Flight`.
///
/// Both ends are written in the link type. An end's `apiName` is the accessor on
/// that end's own `objectType` that reaches the other end. This is the reverse of
/// how Foundry words it, where the name written on one end lives on the opposite
/// object type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LinkType {
    /// Unique `camelCase` name; actions refer to the link by it.
    pub api_name: LinkTypeName,
    /// Human-readable name shown in UIs.
    pub display_name: Option<String>,
    /// What this relationship means.
    pub description: Option<String>,
    /// `oneToOne`, `oneToMany` or `manyToMany`.
    pub cardinality: Cardinality,
    /// The first end. In `oneToMany` this is the "one" side.
    pub from: LinkEnd,
    /// The second end. In `oneToMany` this is the "many" side and holds the
    /// foreign key.
    pub to: LinkEnd,
}

/// One end of a link type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LinkEnd {
    /// The object type at this end.
    pub object_type: ObjectTypeName,
    /// The accessor on this end's `objectType` that reaches the other end. It must
    /// be unique among that object type's accessors and must not equal one of its
    /// properties.
    pub api_name: LinkEndName,
    /// Human-readable name of the accessor shown in UIs.
    pub display_name: Option<String>,
    /// What the accessor returns.
    pub description: Option<String>,
    /// Property holding the reference. Only on the `to` end of `oneToOne` and
    /// `oneToMany`; it must be a property of the `to` object type whose type equals
    /// the `from` object type's primary key type. Never written for `manyToMany`.
    pub foreign_key: Option<PropertyName>,
}
