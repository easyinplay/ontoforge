//! ONT-W08: Rule of three.
//!
//! When at least three object types share at least three properties with the same
//! name and base type, the shared set is a candidate for an interface or a shared
//! property (a v2 capability). Each shared property set is reported once, on the
//! first object type by `apiName`.
//!
//! Shared sets are found pairwise (CR-01): the properties two object types have in
//! common, counting only properties that at least three types hold, form a shared
//! set, and the set is reported when at least three types are linked by exactly that
//! set. Every listed type holds the whole set. Types with the same candidate
//! properties are compared once, so the work grows with the square of the number of
//! different property combinations, not the cube of the number of types. Above
//! [`MAX_COMBINATIONS`] combinations the rule reports that it did not run instead.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{Finding, Level, LintCx, LintRule};
use super::{off_hint, type_finding};
use crate::ir::ObjectType;

/// Types that must share a property set.
const MIN_TYPES: usize = 3;
/// Properties the types must share.
const MIN_SHARED: usize = 3;
/// Different candidate property combinations the rule compares pairwise.
const MAX_COMBINATIONS: usize = 500;

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
        // Only keys held by at least three types can be shared by three types (T-01-35).
        // Types with the same such keys are grouped, so each combination is compared
        // once (CR-01).
        let mut combinations: BTreeMap<BTreeSet<Key<'_>>, Vec<&str>> = BTreeMap::new();
        for object in cx.ontology.object_types.values() {
            let frequent: BTreeSet<Key<'_>> = keys_of(object)
                .filter(|key| counts[key] >= MIN_TYPES)
                .collect();
            if frequent.len() >= MIN_SHARED {
                combinations
                    .entry(frequent)
                    .or_default()
                    .push(object.api_name.as_str());
            }
        }
        let combinations: Vec<(BTreeSet<Key<'_>>, Vec<&str>)> = combinations.into_iter().collect();
        if combinations.len() > MAX_COMBINATIONS {
            self.too_many(cx, &combinations, out);
            return;
        }

        // Shared property set to the names of the types that hold all of it.
        let mut found: BTreeMap<BTreeSet<Key<'_>>, BTreeSet<&str>> = BTreeMap::new();
        for (i, (set_a, types_a)) in combinations.iter().enumerate() {
            if types_a.len() > 1 {
                // Two types of one combination share all of it.
                found
                    .entry(set_a.clone())
                    .or_default()
                    .extend(types_a.iter().copied());
            }
            for (set_b, types_b) in &combinations[i + 1..] {
                let shared: BTreeSet<Key<'_>> = set_a.intersection(set_b).copied().collect();
                if shared.len() >= MIN_SHARED {
                    found
                        .entry(shared)
                        .or_default()
                        .extend(types_a.iter().chain(types_b).copied());
                }
            }
        }
        found.retain(|_, types| types.len() >= MIN_TYPES);

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

impl W08 {
    /// One finding, on the first candidate type, saying the rule did not run.
    fn too_many(
        &self,
        cx: &LintCx<'_>,
        combinations: &[(BTreeSet<Key<'_>>, Vec<&str>)],
        out: &mut Vec<Finding>,
    ) {
        let types = combinations.iter().flat_map(|(_, types)| types);
        let Some(first) = types
            .clone()
            .min()
            .and_then(|n| cx.ontology.object_types.get(*n))
        else {
            return;
        };
        out.push(type_finding(
            first,
            format!(
                "{} object types hold {} different combinations of commonly shared properties, more than the {MAX_COMBINATIONS} this rule compares, so shared property sets were not checked",
                types.count(),
                combinations.len()
            ),
            format!(
                "split the ontology or review shared properties by hand; {}",
                off_hint(self.id())
            ),
        ));
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

    /// CR-01: past the comparison budget the rule says it did not run.
    #[test]
    fn too_many_combinations_are_reported_once() {
        let docs: Vec<String> = (0..600)
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
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(
            found[0]
                .message
                .contains("600 object types hold 600 different")
        );
        assert!(found[0].message.contains("were not checked"));
    }
}
