//! Source binding checks (hard errors): the target exists (V09), it may be bound at
//! all (V16), and the selected profile binds every source-backed type (V16).

use crate::diag::{Diagnostic, Severity, SourceLocation, codes, did_you_mean};
use crate::ir::{
    Backing, BindingTarget, Cardinality, LinkTypeName, ObjectTypeName, Ontology, ProfileName,
    SourceBinding, binding_def_name,
};
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};
use crate::loader::{MANIFEST, SourceMap};

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

fn locate<'a>(
    sources: &'a SourceMap,
    key: &'a DefKey,
) -> impl Fn(&[&str], Part) -> SourceLocation + 'a {
    move |path: &[&str], part: Part| {
        let path: Vec<Seg> = path.iter().map(|s| Seg::Key((*s).to_owned())).collect();
        sources.locate(key, &path, part)
    }
}

/// Checks that need no profile selection; they run while loading.
pub fn check(ontology: &Ontology, sources: &SourceMap, out: &mut Vec<Diagnostic>) {
    for (profile, binding) in all_bindings(ontology) {
        let key = DefKey {
            kind: DefKind::SourceBinding,
            api_name: binding_def_name(profile, binding.target_name()),
        };
        let at = locate(sources, &key);
        if target_exists(ontology, profile, binding, &at, out) {
            may_be_bound(ontology, binding, &at, out);
        }
    }
}

/// V09 unless the target exists. Returns whether it does.
fn target_exists(
    ontology: &Ontology,
    profile: &ProfileName,
    binding: &SourceBinding,
    at: &At,
    out: &mut Vec<Diagnostic>,
) -> bool {
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
        return true;
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
    false
}

/// V16 when the target is `editOnly`, or is a link that is not `manyToMany` (D-02).
/// Applies to a binding in any profile.
fn may_be_bound(ontology: &Ontology, binding: &SourceBinding, at: &At, out: &mut Vec<Diagnostic>) {
    let (what, key, name, backing, cardinality) = match &binding.target {
        BindingTarget::ObjectType(name) => {
            let Some(object) = ontology.object_types.get(name.as_str()) else {
                return;
            };
            (
                "object type",
                "objectType",
                name.as_str(),
                Some(object.backing),
                None,
            )
        }
        BindingTarget::LinkType(name) => {
            let Some(link) = ontology.link_types.get(name.as_str()) else {
                return;
            };
            (
                "link type",
                "linkType",
                name.as_str(),
                link.backing,
                Some(link.cardinality),
            )
        }
    };
    let here = at(&["target", key], Part::Value);
    match cardinality {
        Some(Cardinality::OneToOne | Cardinality::OneToMany) => out.push(
            Diagnostic::error(
                codes::V16,
                format!(
                    "link type `{name}` is not manyToMany, so it cannot have a source binding"
                ),
                &here,
            )
            .with_help(
                "remove this binding: oneToOne and oneToMany links follow the foreign key on the `to` object type; only manyToMany links are bound",
            ),
        ),
        _ if backing == Some(Backing::EditOnly) => out.push(
            Diagnostic::error(
                codes::V16,
                format!("{what} `{name}` is `backing: editOnly`, so it cannot have a source binding"),
                &here,
            )
            .with_help(format!(
                "remove this binding, or set `backing: source` on `{name}`"
            )),
        ),
        _ => {}
    }
}

/// Where the selected profile name came from; decides how a bad name is located.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileOrigin {
    /// `--profile` or `ONTOFORGE_PROFILE`: there is no file position to point at.
    Flag,
    /// The manifest's `defaultProfile`.
    Manifest,
}

/// Checks that depend on the selected profile (D-02, D-05): when the ontology has
/// source-backed types, a profile must be selected, must exist, and must bind each
/// of them. An ontology without source-backed types needs no profile.
pub fn check_profile(
    ontology: &Ontology,
    sources: &SourceMap,
    selection: Option<(&ProfileName, ProfileOrigin)>,
) -> Vec<Diagnostic> {
    let source_objects: Vec<&ObjectTypeName> = ontology
        .object_types
        .iter()
        .filter(|(_, o)| o.backing == Backing::Source)
        .map(|(name, _)| name)
        .collect();
    let source_links: Vec<&LinkTypeName> = ontology
        .link_types
        .iter()
        .filter(|(_, l)| {
            l.cardinality == Cardinality::ManyToMany && l.backing == Some(Backing::Source)
        })
        .map(|(name, _)| name)
        .collect();
    let needed = source_objects.len() + source_links.len();
    if needed == 0 {
        return Vec::new();
    }

    let Some((profile, origin)) = selection else {
        return vec![
            Diagnostic::error(
                codes::V16,
                format!("no profile is selected, but {needed} source-backed type(s) need a binding"),
                &sources.locate_manifest(&[], Part::Value),
            )
            .with_help(
                "select a profile with `--profile <name>`, the `ONTOFORGE_PROFILE` environment variable, or `defaultProfile` in `ontology.yaml`",
            ),
        ];
    };
    let Some(bound) = ontology.profile_bindings(profile) else {
        return vec![unknown_profile(ontology, sources, profile, origin)];
    };

    let mut out = Vec::new();
    let missing = |what: &str, key: DefKey, target: &str, target_key: &str| {
        let at = sources.locate(&key, &[Seg::Key("backing".to_owned())], Part::Value);
        Diagnostic::error(
            codes::V16,
            format!("{what} `{target}` is `backing: source`, but profile `{profile}` has no binding for it"),
            &at,
        )
        .with_help(format!(
            "add a SourceBinding with `target: {{ {target_key}: {target} }}` under `bindings/{profile}/`"
        ))
    };
    for name in source_objects {
        if !bound.objects.contains_key(name.as_str()) {
            let key = DefKey {
                kind: DefKind::ObjectType,
                api_name: name.to_string(),
            };
            out.push(missing("object type", key, name.as_str(), "objectType"));
        }
    }
    for name in source_links {
        if !bound.links.contains_key(name.as_str()) {
            let key = DefKey {
                kind: DefKind::LinkType,
                api_name: name.to_string(),
            };
            out.push(missing("link type", key, name.as_str(), "linkType"));
        }
    }
    out
}

/// V16 for a selected profile that has no bindings at all.
fn unknown_profile(
    ontology: &Ontology,
    sources: &SourceMap,
    profile: &ProfileName,
    origin: ProfileOrigin,
) -> Diagnostic {
    let known: Vec<&str> = ontology.bindings.keys().map(ProfileName::as_str).collect();
    let help = match did_you_mean(profile.as_str(), known.iter().copied()) {
        Some(s) => format!("did you mean `{s}`?"),
        None if known.is_empty() => {
            format!("create `bindings/{profile}/` with one SourceBinding per source-backed type")
        }
        None => format!(
            "profiles with bindings: {}",
            known
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let d = match origin {
        ProfileOrigin::Manifest => {
            let at = sources.locate_manifest(&[Seg::Key("defaultProfile".to_owned())], Part::Value);
            Diagnostic::error(
                codes::V16,
                format!(
                    "`defaultProfile` is `{profile}`, but there are no source bindings under `bindings/{profile}/`"
                ),
                &at,
            )
        }
        ProfileOrigin::Flag => Diagnostic::without_location(
            codes::V16,
            Severity::Error,
            format!(
                "profile `{profile}` was selected, but there are no source bindings under `bindings/{profile}/`"
            ),
            MANIFEST,
        ),
    };
    d.with_help(help)
}
