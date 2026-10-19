//! The `ontology.yaml` manifest (D-01).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::names::{OntologyName, ProfileName};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub api_name: OntologyName,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub include: Option<Vec<String>>,
    /// Profile used when neither `--profile` nor `ONTOFORGE_PROFILE` selects one: the
    /// name of a directory under `bindings/` (D-05).
    pub default_profile: Option<ProfileName>,
}
