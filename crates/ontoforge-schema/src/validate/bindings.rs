//! Source binding checks (hard errors): the target exists (V09) and, in later
//! checks, the binding is well formed (V08, V09, V16).

use crate::diag::{Diagnostic, SourceLocation, codes, did_you_mean};
use crate::ir::{
    BindingTarget, LinkTypeName, ObjectTypeName, Ontology, ProfileName, SourceBinding,
};
use crate::loader::SourceMap;
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

type At<'a> = dyn Fn(&[&str], Part) -> SourceLocation + 'a;

/// Every binding of every profile, objects before links.
fn all_bindings(ontology: &Ontology) -> impl Iterator<Item = (&ProfileName, &SourceBinding)> {
    ontology.bindings.iter().flat_map(|(profile, bound)| {
        bound
            .objects
            .values()
            .chain(bound.links.values())
            .map(move |binding| (profile, binding))
    })
}

/// Checks that need no profile selection; they run while loading.
pub fn check(ontology: &Ontology, sources: &SourceMap, out: &mut Vec<Diagnostic>) {
    for (profile, binding) in all_bindings(ontology) {
        let key = DefKey {
            kind: DefKind::SourceBinding,
            api_name: crate::ir::binding_def_name(profile, binding.target_name()),
        };
        let at = |path: &[&str], part: Part| {
            let path: Vec<Seg> = path.iter().map(|s| Seg::Key((*s).to_owned())).collect();
            sources.locate(&key, &path, part)
        };
        target_exists(ontology, profile, binding, &at, out);
    }
}

fn target_exists(
    ontology: &Ontology,
    profile: &ProfileName,
    binding: &SourceBinding,
    at: &At,
    out: &mut Vec<Diagnostic>,
) {
    let (what, key, name, exists, candidates): (_, _, _, _, Vec<&str>) = match &binding.target {
        BindingTarget::ObjectType(name) => (
            "object type",
            "objectType",
            name.as_str(),
            ontology.object_types.contains_key(name.as_str()),
            ontology
                .object_types
                .keys()
                .map(ObjectTypeName::as_str)
                .collect(),
        ),
        BindingTarget::LinkType(name) => (
            "link type",
            "linkType",
            name.as_str(),
            ontology.link_types.contains_key(name.as_str()),
            ontology
                .link_types
                .keys()
                .map(LinkTypeName::as_str)
                .collect(),
        ),
    };
    if exists {
        return;
    }
    let d = Diagnostic::error(
        codes::V09,
        format!("binding in profile `{profile}` targets unknown {what} `{name}`"),
        &at(&["target", key], Part::Value),
    );
    out.push(match did_you_mean(name, candidates) {
        Some(s) => d.with_help(format!("did you mean `{s}`?")),
        None => d,
    });
}
