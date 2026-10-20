//! Offline validation and compilation of scope documents (D-01, SCOPE-02 / 05 / 06).
//!
//! RED placeholder (plan 03-14 Task 1): the types and signatures are final, the bodies are
//! not. `compile_scope` accepts everything and `for_principal` grants nothing.

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::config::AuthConfig;
use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::{
    ActionTypeName, AttrType, ObjectTypeName, Ontology, ProfileName, PropertyName, Role, RoleName,
    ScopeSet, SqlLevel, Subject, Toolset,
};
use ontoforge_schema::loader::SourceMap;

use super::merge::{RoleTypeGrant, TypeScope};
use crate::authn::Principal;

/// Everything `compile_scope` cross-checks a scope against.
pub struct ScopeInputs<'a> {
    pub ontology: &'a Ontology,
    /// The selected deployment profile; `None` falls back to the manifest's default.
    pub profile: Option<&'a ProfileName>,
    /// Declared principal attributes (`[auth.principal_attrs]`).
    pub attrs: &'a BTreeMap<String, AttrType>,
    pub scope: &'a ScopeSet,
    /// Positions of the scope documents; `None` for roles read back from the system library.
    pub sources: Option<&'a SourceMap>,
}

/// A role with its row predicates compiled.
#[derive(Debug, Clone)]
pub struct CompiledRole {
    pub role: Role,
    pub types: BTreeMap<ObjectTypeName, RoleTypeGrant>,
}

/// A validated scope: roles with compiled predicates, and who holds them.
#[derive(Debug, Clone)]
pub struct CompiledScope {
    pub roles: BTreeMap<RoleName, CompiledRole>,
    pub bindings: BTreeMap<RoleName, Vec<Subject>>,
}

/// What one principal may see and do after all its roles are merged.
#[derive(Debug, Clone, Default)]
pub struct PrincipalScope {
    pub roles: BTreeSet<RoleName>,
    pub types: BTreeMap<ObjectTypeName, TypeScope>,
    pub actions: BTreeSet<ActionTypeName>,
    pub sql_levels: BTreeSet<SqlLevel>,
    pub l2_catalogs: BTreeSet<String>,
    pub l2_schemas: BTreeSet<String>,
    pub toolsets: BTreeSet<Toolset>,
}

/// Validates `inputs.scope` against the ontology and compiles it.
///
/// # Errors
///
/// Every problem found, sorted by position.
pub fn compile_scope(inputs: &ScopeInputs<'_>) -> Result<CompiledScope, Vec<Diagnostic>> {
    let _ = inputs;
    Ok(CompiledScope {
        roles: BTreeMap::new(),
        bindings: BTreeMap::new(),
    })
}

impl CompiledScope {
    /// Resolves the roles of `principal` and merges them.
    #[must_use]
    pub fn for_principal(&self, principal: &Principal) -> PrincipalScope {
        let _ = principal;
        PrincipalScope::default()
    }

    /// The properties each object type's row predicates read, over all roles.
    #[must_use]
    pub fn row_filter_properties(&self) -> BTreeMap<ObjectTypeName, BTreeSet<PropertyName>> {
        BTreeMap::new()
    }
}

/// The declared principal attributes and their types.
#[must_use]
pub fn attr_types(cfg: Option<&AuthConfig>) -> BTreeMap<String, AttrType> {
    cfg.map(|c| {
        c.principal_attrs
            .iter()
            .map(|(name, attr)| (name.clone(), attr.ty))
            .collect()
    })
    .unwrap_or_default()
}
