//! The intermediate representation shared by every downstream phase.
//!
//! IR types carry no source positions; those live in the loader's `SourceMap`.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod basetype;
pub mod manifest;
pub mod names;
pub mod object;

pub use basetype::BaseType;
pub use manifest::Manifest;
pub use names::{ObjectTypeName, OntologyName, PropertyName};
pub use object::{ObjectType, PropertyDef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Ontology {
    pub api_name: OntologyName,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub object_types: BTreeMap<ObjectTypeName, ObjectType>,
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
}
