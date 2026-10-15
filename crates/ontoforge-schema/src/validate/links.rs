//! Link type structural checks (hard errors): references (V09), cardinality and
//! foreign key consistency (V11), and accessor names on each object type (V08).
//!
//! A link whose end types do not exist only reports V09; the other checks need
//! those types and would only add noise.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::diag::{Diagnostic, SourceLocation, codes, did_you_mean};
use crate::ir::{Cardinality, LinkType, ObjectType, ObjectTypeName, Ontology, PropertyName};
use crate::loader::SourceMap;
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

type At<'a> = dyn Fn(&[&str], Part) -> SourceLocation + 'a;

/// Accessor `(object type, name)` mapped to the `(link, side)` that claimed it.
type Accessors<'a> = HashMap<(&'a str, &'a str), (&'a str, &'static str)>;

pub fn check(ontology: &Ontology, sources: &SourceMap, out: &mut Vec<Diagnostic>) {
    let mut accessors = Accessors::new();
    // BTreeMap order keeps the "later definition" of a clash deterministic.
    for link in ontology.link_types.values() {
        let key = DefKey {
            kind: DefKind::LinkType,
            api_name: link.api_name.to_string(),
        };
        let at = |path: &[&str], part: Part| {
            let path: Vec<Seg> = path.iter().map(|s| Seg::Key((*s).to_owned())).collect();
            sources.locate(&key, &path, part)
        };
        let Some((from, to)) = end_types(link, ontology, &at, out) else {
            continue;
        };
        foreign_key(link, from, to, &at, out);
        end_names(link, [from, to], &at, &mut accessors, out);
    }
}

fn end_types<'a>(
    link: &LinkType,
    ontology: &'a Ontology,
    at: &At,
    out: &mut Vec<Diagnostic>,
) -> Option<(&'a ObjectType, &'a ObjectType)> {
    let mut found = [None, None];
    for (slot, (side, end)) in found
        .iter_mut()
        .zip([("from", &link.from), ("to", &link.to)])
    {
        *slot = ontology.object_types.get(end.object_type.as_str());
        if slot.is_some() {
            continue;
        }
        let d = Diagnostic::error(
            codes::V09,
            format!(
                "`{side}` end of link `{}` refers to unknown object type `{}`",
                link.api_name, end.object_type
            ),
            &at(&[side, "objectType"], Part::Value),
        );
        let names = ontology.object_types.keys().map(ObjectTypeName::as_str);
        out.push(match did_you_mean(end.object_type.as_str(), names) {
            Some(s) => d.with_help(format!("did you mean `{s}`?")),
            None => d,
        });
    }
    let [Some(from), Some(to)] = found else {
        return None;
    };
    Some((from, to))
}

fn foreign_key(
    link: &LinkType,
    from: &ObjectType,
    to: &ObjectType,
    at: &At,
    out: &mut Vec<Diagnostic>,
) {
    let name = &link.api_name;
    if link.cardinality == Cardinality::ManyToMany {
        for (side, end) in [("from", &link.from), ("to", &link.to)] {
            if end.foreign_key.is_some() {
                out.push(
                    Diagnostic::error(
                        codes::V11,
                        format!("manyToMany link `{name}` must not declare a `foreignKey`"),
                        &at(&[side, "foreignKey"], Part::Value),
                    )
                    .with_help("remove it; the system manages the join for manyToMany links"),
                );
            }
        }
        return;
    }

    if link.from.foreign_key.is_some() {
        out.push(
            Diagnostic::error(
                codes::V11,
                format!("`foreignKey` of link `{name}` is on the `from` end"),
                &at(&["from", "foreignKey"], Part::Value),
            )
            .with_help("the foreign key belongs on the `to` end"),
        );
    }
    let Some(fk) = &link.to.foreign_key else {
        out.push(
            Diagnostic::error(
                codes::V11,
                format!(
                    "{} link `{name}` needs a `foreignKey` on its `to` end",
                    cardinality_name(link.cardinality)
                ),
                &at(&["cardinality"], Part::Value),
            )
            .with_help(format!(
                "add `foreignKey: <property of {}>` to the `to` end, or use manyToMany",
                to.api_name
            )),
        );
        return;
    };
    let fk_at = at(&["to", "foreignKey"], Part::Value);
    let Some(fk_prop) = to.properties.get(fk.as_str()) else {
        let d = Diagnostic::error(
            codes::V09,
            format!("foreignKey `{fk}` is not a property of `{}`", to.api_name),
            &fk_at,
        );
        let names = to.properties.keys().map(PropertyName::as_str);
        out.push(match did_you_mean(fk.as_str(), names) {
            Some(s) => d.with_help(format!("did you mean `{s}`?")),
            None => d,
        });
        return;
    };
    // A missing or malformed primary key is V10's job.
    let Some(pk_prop) = from.properties.get(from.primary_key.as_str()) else {
        return;
    };
    if fk_prop.base_type != pk_prop.base_type {
        out.push(
            Diagnostic::error(
                codes::V11,
                format!(
                    "foreignKey `{fk}` has type `{}`, but the primary key `{}.{}` has type `{}`",
                    fk_prop.base_type.as_str(),
                    from.api_name,
                    from.primary_key,
                    pk_prop.base_type.as_str()
                ),
                &fk_at,
            )
            .with_help("a foreign key must have exactly the type of the primary key it references"),
        );
    }
}

fn cardinality_name(c: Cardinality) -> &'static str {
    match c {
        Cardinality::OneToOne => "oneToOne",
        Cardinality::OneToMany => "oneToMany",
        Cardinality::ManyToMany => "manyToMany",
    }
}

/// An end's `apiName` is an accessor on its own object type, so it must differ
/// from that type's properties and from the accessors of earlier links.
fn end_names<'a>(
    link: &'a LinkType,
    [from, to]: [&'a ObjectType; 2],
    at: &At,
    accessors: &mut Accessors<'a>,
    out: &mut Vec<Diagnostic>,
) {
    for (side, end, owner) in [("from", &link.from, from), ("to", &link.to, to)] {
        let name = end.api_name.as_str();
        let here = at(&[side, "apiName"], Part::Value);
        if owner.properties.contains_key(name) {
            out.push(
                Diagnostic::error(
                    codes::V08,
                    format!(
                        "`{side}` end `{name}` of link `{}` has the same name as a property of `{}`",
                        link.api_name, owner.api_name
                    ),
                    &here,
                )
                .with_help("rename the link end or the property"),
            );
            continue;
        }
        match accessors.entry((owner.api_name.as_str(), name)) {
            Entry::Occupied(taken) => {
                let (other, other_side) = taken.get();
                out.push(
                    Diagnostic::error(
                        codes::V08,
                        format!(
                            "accessor `{name}` on `{}` is already used by the `{other_side}` end of link `{other}`",
                            owner.api_name
                        ),
                        &here,
                    )
                    .with_help("rename one of the two link ends"),
                );
            }
            Entry::Vacant(slot) => {
                slot.insert((link.api_name.as_str(), side));
            }
        }
    }
}
