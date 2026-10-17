//! Source binding checks (hard errors): the target exists (V09), it may be bound at
//! all (V16), and the selected profile binds every source-backed type (V16).

use crate::diag::{Diagnostic, Severity, SourceLocation, codes, did_you_mean};
use crate::ir::{
    Backing, BindingTarget, Cardinality, LinkTypeName, ObjectTypeName, Ontology, ProfileName,
    PropertyName, SourceBinding, binding_def_name,
};
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};
use crate::loader::{MANIFEST, SourceMap, fixture_path};

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
            column_keys(ontology, binding, &at, out);
        }
        physical_names(binding, &at, out);
        time_zones(binding, &at, out);
        fixture_reference(binding, &at, out);
    }
}

/// Fixture existence and size, per binding, through `probe` (metadata only: the
/// content is never read here). A fixture path that is itself invalid was already
/// reported by [`check`] and is skipped.
pub(crate) fn check_fixture_files(
    ontology: &Ontology,
    sources: &SourceMap,
    probe: &dyn Fn(&str) -> Result<u64, String>,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (profile, binding) in all_bindings(ontology) {
        let Some(fixture) = &binding.fixture else {
            continue;
        };
        let key = DefKey {
            kind: DefKind::SourceBinding,
            api_name: binding_def_name(profile, binding.target_name()),
        };
        let at = locate(sources, &key);
        let Ok(resolved) = fixture_path(&at(&[], Part::Value).file, &fixture.file) else {
            continue;
        };
        if let Err(reason) = probe(&resolved) {
            out.push(Diagnostic::error(
                codes::V16,
                format!("fixture `{resolved}`: {reason}"),
                &at(&["fixture", "file"], Part::Value),
            ));
        }
    }
    out
}

/// `columns` keys: property names for an object type, exactly `from` and `to` for a
/// link (D-03).
fn column_keys(ontology: &Ontology, binding: &SourceBinding, at: &At, out: &mut Vec<Diagnostic>) {
    match &binding.target {
        BindingTarget::ObjectType(name) => {
            let Some(object) = ontology.object_types.get(name.as_str()) else {
                return;
            };
            for key in binding.columns.keys() {
                if object.properties.contains_key(key.as_str()) {
                    continue;
                }
                let d = Diagnostic::error(
                    codes::V09,
                    format!(
                        "`{key}` is not a property of `{name}`, so it cannot be mapped to a column"
                    ),
                    &at(&["columns", key], Part::Key),
                );
                let candidates = object.properties.keys().map(PropertyName::as_str);
                out.push(match did_you_mean(key, candidates) {
                    Some(s) => d.with_help(format!("did you mean `{s}`?")),
                    None => d,
                });
            }
        }
        BindingTarget::LinkType(name) => {
            for end in LINK_ENDS {
                // `from: {}` names no column either: the end would fall back to a column
                // literally named `from` (WR-S03).
                if binding
                    .columns
                    .get(end)
                    .and_then(|c| c.column.as_deref())
                    .is_none()
                {
                    out.push(
                        Diagnostic::error(
                            codes::V16,
                            format!("the binding of link `{name}` must map the `{end}` end to a column"),
                            &at(&["columns"], Part::Value),
                        )
                        .with_help(format!(
                            "add `columns: {{ {end}: {{ column: <column holding the {end} key> }} }}`"
                        )),
                    );
                }
            }
            for key in binding.columns.keys() {
                if !LINK_ENDS.contains(&key.as_str()) {
                    out.push(
                        Diagnostic::error(
                            codes::V09,
                            format!("`{key}` is not an end of link `{name}`"),
                            &at(&["columns", key], Part::Key),
                        )
                        .with_help("a link binding maps only `from` and `to`"),
                    );
                }
            }
        }
    }
}

/// The two ends of a link binding, in the order columns are derived.
const LINK_ENDS: [&str; 2] = ["from", "to"];

/// Catalog, schema, table and column names must be non-empty text without control
/// characters (T-02-09). They are quoted when printed; this keeps them readable.
fn physical_names(binding: &SourceBinding, at: &At, out: &mut Vec<Diagnostic>) {
    let mut check = |what: &str, value: &str, path: &[&str]| {
        let problem = if value.is_empty() {
            "must not be empty"
        } else if value.chars().any(char::is_control) {
            "must not contain control characters"
        } else {
            return;
        };
        out.push(Diagnostic::error(
            codes::V16,
            format!("`{what}` {problem}"),
            &at(path, Part::Value),
        ));
    };
    check("catalog", &binding.source.catalog, &["source", "catalog"]);
    check("schema", &binding.source.schema, &["source", "schema"]);
    check("table", &binding.source.table, &["source", "table"]);
    for (key, mapping) in &binding.columns {
        if let Some(column) = &mapping.column {
            check("column", column, &["columns", key, "column"]);
        }
    }
}

/// A `timezone` is an IANA name or `aware`. Whether a timestamp property needs one is
/// lint territory (E14).
fn time_zones(binding: &SourceBinding, at: &At, out: &mut Vec<Diagnostic>) {
    for (key, mapping) in &binding.columns {
        let Some(zone) = &mapping.timezone else {
            continue;
        };
        if zone == AWARE || zone.parse::<chrono_tz::Tz>().is_ok() {
            continue;
        }
        out.push(
            Diagnostic::error(
                codes::V16,
                format!("`{zone}` is not an IANA time zone name or `aware`"),
                &at(&["columns", key, "timezone"], Part::Value),
            )
            .with_help(
                "write a name such as `Europe/Berlin`, or `aware` when the column already carries a time zone",
            ),
        );
    }
}

/// Timezone value for a column that already carries its zone.
const AWARE: &str = "aware";

/// The fixture path must stay inside the ontology directory and name a `.csv` or
/// `.json` file (T-02-07). Existence is checked by [`check_fixture_files`].
fn fixture_reference(binding: &SourceBinding, at: &At, out: &mut Vec<Diagnostic>) {
    let Some(fixture) = &binding.fixture else {
        return;
    };
    if let Err(reason) = fixture_path(&at(&[], Part::Value).file, &fixture.file) {
        out.push(Diagnostic::error(
            codes::V16,
            format!("fixture file `{}` cannot be used: {reason}", fixture.file),
            &at(&["fixture", "file"], Part::Value),
        ));
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
