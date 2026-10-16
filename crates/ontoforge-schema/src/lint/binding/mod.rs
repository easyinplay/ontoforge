//! Source binding rules ONT-E13 to ONT-E16 and ONT-W17 (BIND-02, D-03). They check
//! the type contract between an ontology and its bindings without any connection to
//! a data source; what only the data source knows is `validate --online`.

use super::LintRule;
use crate::ir::{
    BaseType, BindingTarget, Ontology, ProfileName, PropertyDef, SourceBinding, binding_def_name,
};
use crate::loader::sourcemap::{DefKey, DefKind};

mod e13;
mod e14;
mod e15;
mod e16;
mod w17;

pub(super) fn rules() -> Vec<&'static dyn LintRule> {
    vec![&e13::RULE, &e14::RULE, &e15::RULE, &e16::RULE, &w17::RULE]
}

/// Every binding of every profile, objects before links, profiles in name order.
fn all_bindings(ontology: &Ontology) -> impl Iterator<Item = (&ProfileName, &SourceBinding)> {
    ontology.bindings.iter().flat_map(|(profile, bound)| {
        bound
            .objects
            .values()
            .chain(bound.links.values())
            .map(move |binding| (profile, binding))
    })
}

/// The definition key a binding document is registered under in the `SourceMap`.
fn binding_def(profile: &ProfileName, binding: &SourceBinding) -> DefKey {
    DefKey {
        kind: DefKind::SourceBinding,
        api_name: binding_def_name(profile, binding.target_name()),
    }
}

/// The type a column holds per value: the element type of an array, otherwise the
/// type itself.
fn element_type(property: &PropertyDef) -> BaseType {
    match (property.base_type, property.items) {
        (BaseType::Array, Some(items)) => items,
        (base, _) => base,
    }
}

/// Names what a column belongs to in a message: a property of an object type, or an
/// end of a link. `key` is a [`crate::ir::BoundColumn::key`].
fn describe(binding: &SourceBinding, key: &str) -> String {
    match &binding.target {
        BindingTarget::ObjectType(name) => format!("property `{key}` of `{name}`"),
        BindingTarget::LinkType(name) => format!("`{key}` end of `{name}`"),
    }
}
