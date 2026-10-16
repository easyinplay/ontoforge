//! The intermediate representation shared by every downstream phase.
//!
//! IR types carry no source positions; those live in the loader's `SourceMap`.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod action;
pub mod basetype;
pub mod binding;
pub mod link;
pub mod manifest;
pub mod names;
pub mod object;

pub use action::{
    ActionType, CreateObjectRule, DeleteObjectRule, Empty, LinkRule, ModifyObjectRule, ParamType,
    ParameterDef, Rule, SubmissionCriterion, ValueSource,
};
pub use basetype::BaseType;
pub use binding::{
    Backing, BindingTarget, BoundColumn, Collation, ColumnBinding, FixtureRef, ProfileBindings,
    SourceBinding, SourceRef,
};
pub use link::{Cardinality, LinkEnd, LinkType};
pub use manifest::Manifest;
pub use names::{
    ActionTypeName, LinkEndName, LinkTypeName, ObjectTypeName, OntologyName, ParameterName,
    ProfileName, PropertyName,
};
pub use object::{
    DEFAULT_DECIMAL_PRECISION, DEFAULT_DECIMAL_SCALE, ObjectType, PropertyDef,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Ontology {
    pub api_name: OntologyName,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub object_types: BTreeMap<ObjectTypeName, ObjectType>,
    #[serde(default)]
    pub link_types: BTreeMap<LinkTypeName, LinkType>,
    #[serde(default)]
    pub action_types: BTreeMap<ActionTypeName, ActionType>,
    /// The manifest's `defaultProfile`.
    pub default_profile: Option<ProfileName>,
    /// Source bindings by profile, from `bindings/<profile>/`.
    #[serde(default)]
    pub bindings: BTreeMap<ProfileName, ProfileBindings>,
}

impl Ontology {
    /// The bindings of profile `p`, if any binding file exists for it.
    pub fn profile_bindings(&self, p: &ProfileName) -> Option<&ProfileBindings> {
        self.bindings.get(p)
    }
}

/// The name a binding is registered under in the source map: `{profile}/{target}`.
///
/// Object type names start with an upper case letter and link type names with a
/// lower case one, so an object type and a link type never share a target name.
pub fn binding_def_name(profile: &ProfileName, target: &str) -> String {
    format!("{profile}/{target}")
}

/// One definition document.
///
/// The loader rewrites `{ kind: X, .. }` to `{ X: { .. } }` before deserializing so
/// that error paths survive (an internally tagged enum would lose them); the
/// `schemars` attribute only shapes the exported JSON Schema as `oneOf` with a
/// `kind` constant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(tag = "kind")]
pub enum Definition {
    ObjectType(ObjectType),
    LinkType(LinkType),
    ActionType(ActionType),
    SourceBinding(SourceBinding),
}
