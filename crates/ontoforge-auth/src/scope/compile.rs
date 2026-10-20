//! Offline validation and compilation of scope documents (D-01, SCOPE-02 / 05 / 06).
//!
//! [`compile_scope`] is the one function that decides whether a scope may take effect: it
//! cross-checks every role against the ontology, the profile bindings and the declared
//! principal attributes, compiles each `rowFilter` with
//! [`ontoforge_expr::compile_row_filter`], and reports every problem it finds instead of
//! stopping at the first one. A scope with any error does not take effect as a whole.
//!
//! [`CompiledScope::for_principal`] then resolves one caller: the roles bound to its id or
//! to one of its groups (D-02), merged per object type with [`merge`] (ADR-0018).

use std::collections::{BTreeMap, BTreeSet};

use ontoforge_engine::config::AuthConfig;
use ontoforge_expr::{PredContext, PredIssue, compile_row_filter};
use ontoforge_schema::diag::{Diagnostic, SourceLocation, codes, did_you_mean, sort_diagnostics};
use ontoforge_schema::ir::{
    ActionTypeName, AttrType, ObjectType, ObjectTypeName, Ontology, ProfileName, PropertyName,
    Role, RoleName, ScopeSet, SqlLevel, Subject, Toolset, TypeGrant,
};
use ontoforge_schema::loader::SourceMap;
use ontoforge_schema::loader::sourcemap::{DefKey, DefKind, Part, Seg};

use super::merge::{RoleTypeGrant, TypeScope, merge};
use crate::authn::Principal;

/// Everything `compile_scope` cross-checks a scope against.
pub struct ScopeInputs<'a> {
    pub ontology: &'a Ontology,
    /// The selected deployment profile; `None` falls back to the manifest's default.
    pub profile: Option<&'a ProfileName>,
    /// Declared principal attributes (`[auth.principal_attrs]`).
    pub attrs: &'a BTreeMap<String, AttrType>,
    pub scope: &'a ScopeSet,
    /// Positions of the scope documents; `None` for roles read back from the system library,
    /// whose diagnostics then carry no file position.
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
///
/// A type missing from `types` is invisible to the principal (fail-closed, SCOPE-05).
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
/// Every problem found, sorted by file, line, column and code.
pub fn compile_scope(inputs: &ScopeInputs<'_>) -> Result<CompiledScope, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut roles = BTreeMap::new();
    for (name, role) in &inputs.scope.roles {
        let at = Locator {
            sources: inputs.sources,
            def: DefKey {
                kind: DefKind::Role,
                api_name: name.as_str().to_owned(),
            },
        };
        let types = check_role(inputs, role, &at, &mut diagnostics);
        roles.insert(
            name.clone(),
            CompiledRole {
                role: role.clone(),
                types,
            },
        );
    }
    if diagnostics.is_empty() {
        let bindings = inputs
            .scope
            .bindings
            .iter()
            .map(|(name, binding)| (name.clone(), binding.subjects.clone()))
            .collect();
        Ok(CompiledScope { roles, bindings })
    } else {
        sort_diagnostics(&mut diagnostics);
        Err(diagnostics)
    }
}

/// Finds the YAML position of a part of one role; an unknown position (no source map)
/// gives the all-zero location, i.e. a diagnostic without a file position.
struct Locator<'a> {
    sources: Option<&'a SourceMap>,
    def: DefKey,
}

impl Locator<'_> {
    fn at(&self, path: &[Seg], part: Part) -> SourceLocation {
        match self.sources {
            Some(sources) => sources.locate(&self.def, path, part),
            None => SourceLocation {
                file: String::new(),
                line: 0,
                column: 0,
                offset: 0,
                length: 0,
            },
        }
    }
}

fn key(name: &str) -> Seg {
    Seg::Key(name.to_owned())
}

/// Checks one role and returns the grants that compiled.
fn check_role(
    inputs: &ScopeInputs<'_>,
    role: &Role,
    at: &Locator<'_>,
    out: &mut Vec<Diagnostic>,
) -> BTreeMap<ObjectTypeName, RoleTypeGrant> {
    let mut grants = BTreeMap::new();
    for (type_name, grant) in &role.object_types {
        let Some(object) = inputs.ontology.object_types.get(type_name.as_str()) else {
            let candidates = inputs
                .ontology
                .object_types
                .keys()
                .map(ObjectTypeName::as_str);
            let mut d = Diagnostic::error(
                codes::V21,
                format!(
                    "role `{}` grants unknown object type `{type_name}`",
                    role.api_name
                ),
                &at.at(&[key("objectTypes"), key(type_name.as_str())], Part::Key),
            );
            if let Some(near) = did_you_mean(type_name.as_str(), candidates) {
                d = d.with_help(format!("did you mean `{near}`?"));
            }
            out.push(d);
            continue;
        };
        let properties = check_properties(role, type_name, object, grant, at, out);
        let row = grant
            .row_filter
            .as_deref()
            .and_then(|src| compile_filter(inputs, role, type_name, object, src, at, out));
        grants.insert(type_name.clone(), RoleTypeGrant { properties, row });
    }
    grants
}

/// The property allow-list of one grant: every name exists, none repeats, the primary key
/// is in it (orchestrator decision 4).
fn check_properties(
    role: &Role,
    type_name: &ObjectTypeName,
    object: &ObjectType,
    grant: &TypeGrant,
    at: &Locator<'_>,
    out: &mut Vec<Diagnostic>,
) -> BTreeSet<PropertyName> {
    let mut allowed = BTreeSet::new();
    for (i, property) in grant.properties.iter().enumerate() {
        let location = at.at(
            &[
                key("objectTypes"),
                key(type_name.as_str()),
                key("properties"),
                Seg::Index(i),
            ],
            Part::Value,
        );
        if !object.properties.contains_key(property.as_str()) {
            let candidates = object.properties.keys().map(PropertyName::as_str);
            let mut d = Diagnostic::error(
                codes::V21,
                format!(
                    "role `{}` lists unknown property `{property}` of `{type_name}`",
                    role.api_name
                ),
                &location,
            );
            if let Some(near) = did_you_mean(property.as_str(), candidates) {
                d = d.with_help(format!("did you mean `{near}`?"));
            }
            out.push(d);
        } else if !allowed.insert(property.clone()) {
            out.push(Diagnostic::error(
                codes::V21,
                format!(
                    "role `{}` lists property `{property}` of `{type_name}` more than once",
                    role.api_name
                ),
                &location,
            ));
        }
    }
    if !allowed.contains(&object.primary_key) {
        out.push(
            Diagnostic::error(
                codes::V21,
                format!(
                    "role `{}` does not list the primary key `{}` of `{type_name}`",
                    role.api_name, object.primary_key
                ),
                &at.at(
                    &[
                        key("objectTypes"),
                        key(type_name.as_str()),
                        key("properties"),
                    ],
                    Part::Value,
                ),
            )
            .with_help("every property allow-list must contain the primary key"),
        );
    }
    allowed
}

/// Compiles a `rowFilter`; every problem becomes an ONT-V22 at the filter's value.
fn compile_filter(
    inputs: &ScopeInputs<'_>,
    role: &Role,
    type_name: &ObjectTypeName,
    object: &ObjectType,
    src: &str,
    at: &Locator<'_>,
    out: &mut Vec<Diagnostic>,
) -> Option<ontoforge_expr::Pred> {
    let location = at.at(
        &[
            key("objectTypes"),
            key(type_name.as_str()),
            key("rowFilter"),
        ],
        Part::Value,
    );
    let cx = PredContext {
        object_type: object,
        attrs: inputs.attrs,
    };
    match compile_row_filter(src, &cx) {
        Ok(pred) => Some(pred),
        Err(issues) => {
            out.extend(
                issues
                    .iter()
                    .map(|issue| filter_diagnostic(role, type_name, issue, &location)),
            );
            None
        }
    }
}

fn filter_diagnostic(
    role: &Role,
    type_name: &ObjectTypeName,
    issue: &PredIssue,
    location: &SourceLocation,
) -> Diagnostic {
    let place = match issue.pos {
        Some((line, column)) => format!(" (at {line}:{column} in expression)"),
        None => String::new(),
    };
    Diagnostic::error(
        codes::V22,
        format!(
            "rowFilter of role `{}` on `{type_name}`: {}{place}",
            role.api_name, issue.message
        ),
        location,
    )
}

impl CompiledScope {
    /// Resolves the roles of `principal` and merges them.
    ///
    /// A role applies when one of its subjects is the principal's id or one of its groups
    /// (D-02); a role reached through several subjects counts once. An object type no
    /// applicable role grants is absent from the result.
    #[must_use]
    pub fn for_principal(&self, principal: &Principal) -> PrincipalScope {
        let holds = |subject: &Subject| match subject {
            Subject::User(id) => id == principal.id.as_str(),
            Subject::Group(group) => principal.groups.contains(group),
        };
        let mut scope = PrincipalScope::default();
        let mut grants: BTreeMap<ObjectTypeName, Vec<RoleTypeGrant>> = BTreeMap::new();
        for (name, subjects) in &self.bindings {
            let Some(compiled) = self.roles.get(name) else {
                continue;
            };
            if !subjects.iter().any(holds) || !scope.roles.insert(name.clone()) {
                continue;
            }
            for (type_name, grant) in &compiled.types {
                grants
                    .entry(type_name.clone())
                    .or_default()
                    .push(grant.clone());
            }
            let role = &compiled.role;
            scope.actions.extend(role.actions.iter().cloned());
            scope.sql_levels.extend(role.sql_levels.iter().copied());
            scope.l2_catalogs.extend(role.l2.catalogs.iter().cloned());
            scope.l2_schemas.extend(role.l2.schemas.iter().cloned());
            scope.toolsets.extend(role.toolsets.iter().copied());
        }
        scope.types = grants
            .into_iter()
            .filter_map(|(type_name, list)| merge(&list).map(|merged| (type_name, merged)))
            .collect();
        scope
    }

    /// The properties each object type's row predicates read, over all roles. Types whose
    /// roles have no row predicate, or none that reads a property, are left out.
    #[must_use]
    pub fn row_filter_properties(&self) -> BTreeMap<ObjectTypeName, BTreeSet<PropertyName>> {
        let mut out: BTreeMap<ObjectTypeName, BTreeSet<PropertyName>> = BTreeMap::new();
        for compiled in self.roles.values() {
            for (type_name, grant) in &compiled.types {
                let Some(row) = &grant.row else { continue };
                let read = row.referenced_properties();
                if !read.is_empty() {
                    out.entry(type_name.clone()).or_default().extend(read);
                }
            }
        }
        out
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
