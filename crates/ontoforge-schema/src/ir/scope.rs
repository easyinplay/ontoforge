//! Scope documents: who may see and do what (roles-as-code, D-01).
//!
//! A scope directory holds `kind: Role` and `kind: RoleBinding` documents, kept apart
//! from the ontology definition and its `SourceBinding` files. A `Role` lists grants
//! (ADR-0018, ADR-0019); a `RoleBinding` attaches a role to users and groups. Group
//! membership is never stored: it comes from the token's `groups` claim (D-02), and a
//! user is identified by its principal id `"<iss>|<sub>"` (D-04).
//!
//! These types only describe the document shape. Whether a named object type,
//! property or action exists is checked against the ontology elsewhere.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::names::{ActionTypeName, ObjectTypeName, PropertyName, RoleName};

/// What one role may do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Role {
    /// Role name, `camelCase`; a `RoleBinding` refers to it.
    pub api_name: RoleName,
    pub display_name: Option<String>,
    pub description: Option<String>,
    /// Per object type: the readable properties and an optional row filter.
    #[serde(default)]
    pub object_types: BTreeMap<ObjectTypeName, TypeGrant>,
    /// Action types the role may submit.
    #[serde(default)]
    pub actions: Vec<ActionTypeName>,
    /// SQL tiers beyond the implicit L0 (ontology-bound queries).
    #[serde(default)]
    pub sql_levels: Vec<SqlLevel>,
    /// Trino catalogs and schemas the role may query directly (L2).
    #[serde(default)]
    pub l2: L2Grant,
    /// MCP toolsets the role may use (ADR-0019).
    #[serde(default)]
    pub toolsets: Vec<Toolset>,
}

/// Access to one object type: which columns, and which rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TypeGrant {
    /// Property allow-list: the only properties of this type the role can read.
    pub properties: Vec<PropertyName>,
    /// CEL over the object's own properties and `principal.attrs`; absent means all
    /// rows are visible.
    pub row_filter: Option<String>,
}

/// A SQL tier a role may use besides the implicit L0.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum SqlLevel {
    /// Raw SQL over the ontology tables.
    Raw,
    /// Administrative SQL.
    Admin,
}

/// Trino namespaces a role may query directly.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct L2Grant {
    /// Whole catalogs.
    #[serde(default)]
    pub catalogs: Vec<String>,
    /// `catalog.schema` pairs.
    #[serde(default)]
    pub schemas: Vec<String>,
}

/// A group of MCP tools (ADR-0019).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum Toolset {
    Core,
    Full,
    #[serde(rename = "sql:raw")]
    SqlRaw,
}

/// Attaches one role to its subjects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RoleBinding {
    pub role: RoleName,
    pub subjects: Vec<Subject>,
}

/// Who a role is granted to. Externally tagged so that error paths survive
/// deserialization (ADR-0003).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Subject {
    /// A principal id, `"<iss>|<sub>"` (D-04).
    User(String),
    /// A group name taken from the token's `groups` claim (D-02). With several
    /// issuers, `"<iss>|<group>"`.
    Group(String),
}

/// One scope document.
///
/// Like `Definition`, the loader rewrites `{ kind: X, .. }` to `{ X: { .. } }` before
/// deserializing; the `schemars` attribute only shapes the exported JSON Schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(tag = "kind")]
pub enum ScopeDefinition {
    Role(Role),
    RoleBinding(RoleBinding),
}

/// All roles and bindings of a scope directory, each keyed by role name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeSet {
    pub roles: BTreeMap<RoleName, Role>,
    /// At most one binding per role.
    pub bindings: BTreeMap<RoleName, RoleBinding>,
}
