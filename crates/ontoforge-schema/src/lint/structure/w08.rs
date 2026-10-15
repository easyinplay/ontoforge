//! ONT-W08: Rule of three.
//!
//! When at least three object types share at least three properties with the same
//! name and base type, the shared set is a candidate for an interface or a shared
//! property (a v2 capability). Each shared property set is reported once, on the
//! first object type by `apiName`.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{Finding, Level, LintCx, LintRule};
use super::{off_hint, type_finding};
use crate::ir::ObjectType;

/// Types that must share a property set.
const MIN_TYPES: usize = 3;
/// Properties the types must share.
const MIN_SHARED: usize = 3;

pub(super) struct W08;

pub(super) static RULE: W08 = W08;

impl LintRule for W08 {
    fn id(&self) -> &'static str {
        "ONT-W08"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        // (property name, base type) to the number of object types that have it.
        let mut counts: BTreeMap<Key<'_>, usize> = BTreeMap::new();
        for object in cx.ontology.object_types.values() {
            for key in keys_of(object) {
                *counts.entry(key).or_default() += 1;
            }
        }
        // Only keys held by at least three types can be shared by three types, so
        // the triple search below runs over these alone (T-01-35).
        let candidates: Vec<(&ObjectType, BTreeSet<Key<'_>>)> = cx
            .ontology
            .object_types
            .values()
            .map(|object| {
                let frequent = keys_of(object)
                    .filter(|key| counts[key] >= MIN_TYPES)
                    .collect::<BTreeSet<_>>();
                (object, frequent)
            })
            .filter(|(_, frequent)| frequent.len() >= MIN_SHARED)
            .collect();

        // Shared property set to the names of the types that hold all of it.
        let mut found: BTreeMap<BTreeSet<Key<'_>>, BTreeSet<&str>> = BTreeMap::new();
        for (i, (a, set_a)) in candidates.iter().enumerate() {
            for (j, (b, set_b)) in candidates.iter().enumerate().skip(i + 1) {
                let ab: BTreeSet<Key<'_>> = set_a.intersection(set_b).copied().collect();
                if ab.len() < MIN_SHARED {
                    continue;
                }
                for (c, set_c) in candidates.iter().skip(j + 1) {
                    let shared: BTreeSet<Key<'_>> = ab.intersection(set_c).copied().collect();
                    if shared.len() >= MIN_SHARED {
                        found.entry(shared).or_default().extend([
                            a.api_name.as_str(),
                            b.api_name.as_str(),
                            c.api_name.as_str(),
                        ]);
                    }
                }
            }
        }

        for (shared, types) in found {
            let Some(first) = types.first().and_then(|n| cx.ontology.object_types.get(*n)) else {
                continue;
            };
            let list = |items: Vec<&str>| {
                items
                    .iter()
                    .map(|i| format!("`{i}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            out.push(type_finding(
                first,
                format!(
                    "object types {} share the properties {}",
                    list(types.iter().copied().collect()),
                    list(shared.iter().map(|(name, _)| *name).collect())
                ),
                format!(
                    "extract an interface or a shared property for them (a v2 capability); {}",
                    off_hint(self.id())
                ),
            ));
        }
    }
}

/// A property as `(name, base type)`.
type Key<'a> = (&'a str, &'a str);

fn keys_of(object: &ObjectType) -> impl Iterator<Item = Key<'_>> {
    object
        .properties
        .iter()
        .map(|(name, def)| (name.as_str(), def.base_type.as_str()))
}

#[cfg(test)]
mod tests {
    use crate::lint::structure::testing::{flagged, join, object, run, run_within};

    const AUDIT: [(&str, &str); 3] = [
        ("createdAt", "timestamp"),
        ("updatedAt", "timestamp"),
        ("ownerId", "string"),
    ];

    #[test]
    fn triggers() {
        let text = join(&[
            object("Alpha", "alphaId", &AUDIT),
            object("Beta", "betaId", &AUDIT),
            object("Gamma", "gammaId", &AUDIT),
        ]);
        let found = run("ONT-W08", &text);
        assert_eq!(flagged(&text, &found), ["Alpha"]);
        let message = &found[0].message;
        for word in [
            "`Alpha`",
            "`Beta`",
            "`Gamma`",
            "`createdAt`",
            "`updatedAt`",
            "`ownerId`",
        ] {
            assert!(message.contains(word), "{word} missing from {message}");
        }
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = join(&[
            // Three types share two properties only.
            object("Alpha", "alphaId", &AUDIT[..2]),
            object("Beta", "betaId", &AUDIT[..2]),
            object("Gamma", "gammaId", &AUDIT[..2]),
            // Two types share three properties.
            object(
                "Delta",
                "deltaId",
                &[("x", "string"), ("y", "string"), ("z", "string")],
            ),
            object(
                "Epsilon",
                "epsilonId",
                &[("x", "string"), ("y", "string"), ("z", "string")],
            ),
            // Same name, different base type does not count.
            object(
                "Zeta",
                "zetaId",
                &[("createdAt", "date"), ("updatedAt", "date")],
            ),
        ]);
        let found = run("ONT-W08", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn a_shared_set_is_reported_once_for_four_types() {
        let text = join(&[
            object("Alpha", "alphaId", &AUDIT),
            object("Beta", "betaId", &AUDIT),
            object("Gamma", "gammaId", &AUDIT),
            object("Delta", "deltaId", &AUDIT),
        ]);
        let found = run("ONT-W08", &text);
        assert_eq!(flagged(&text, &found), ["Alpha"]);
        assert!(found[0].message.contains("`Delta`"));
    }

    /// CR-01: a thousand types sharing the same audit fields once took cubic time.
    #[test]
    fn many_types_with_the_same_shared_set_finish_quickly() {
        let docs: Vec<String> = (0..1000)
            .map(|i| object(&format!("Thing{i}"), "id", &AUDIT))
            .collect();
        let found = run_within(5, "ONT-W08", join(&docs));
        assert_eq!(found.len(), 1, "{:#?}", found.first());
        assert!(found[0].message.contains("`Thing999`"));
    }

    /// CR-01: hundreds of types whose shared sets all differ.
    #[test]
    fn many_types_with_different_shared_sets_finish_quickly() {
        let docs: Vec<String> = (0..400)
            .map(|i| {
                let extra = [
                    format!("extra{}", i % 7),
                    format!("more{}", i % 11),
                    format!("tag{}", i % 13),
                ];
                let mut props: Vec<(&str, &str)> = AUDIT.to_vec();
                props.extend(extra.iter().map(|p| (p.as_str(), "string")));
                object(&format!("Thing{i}"), "id", &props)
            })
            .collect();
        let found = run_within(5, "ONT-W08", join(&docs));
        assert!(
            found.iter().any(|d| d.message.contains("`createdAt`")),
            "{:#?}",
            found.first()
        );
    }
}
