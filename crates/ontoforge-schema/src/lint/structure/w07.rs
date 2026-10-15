//! ONT-W07: Silo (D-13, names and types only).
//!
//! Two checks:
//! - an object type whose first name word is a system or department prefix
//!   (`silo_prefixes`, default `hr`, `sales`, `billing`, `crm`) and which has at least
//!   two words (`SalesOrder` yes, `Sales` no);
//! - two object types with the same primary key name and type, at least four
//!   properties each, and a Jaccard similarity of at least 0.6 between their
//!   `(property name, base type)` sets, leaving out the primary key (which such a
//!   pair always shares).
//!
//! Similar pairs are joined into clusters (CR-02): types linked by a chain of similar
//! pairs form one cluster, and each cluster is reported once, on its last type by
//! `apiName`, listing the others. A thousand near-identical types give one warning,
//! not half a million.
//!
//! No data is looked at, so the similarity is a hint that two types describe the same
//! thing in different systems.

use std::collections::{BTreeMap, BTreeSet};

use super::super::words::split_camel;
use super::super::{Finding, Level, LintCx, LintRule, OptionKind};
use super::{off_hint, type_finding};
use crate::ir::ObjectType;

const DEFAULT_PREFIXES: [&str; 4] = ["hr", "sales", "billing", "crm"];

/// Minimum Jaccard similarity of two property sets.
const MIN_SIMILARITY: f64 = 0.6;
/// Minimum number of properties on each type of a similar pair.
const MIN_PROPERTIES: usize = 4;

pub(super) struct W07;

pub(super) static RULE: W07 = W07;

impl LintRule for W07 {
    fn id(&self) -> &'static str {
        "ONT-W07"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn option_keys(&self) -> &'static [(&'static str, OptionKind)] {
        &[("silo_prefixes", OptionKind::Words)]
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        let prefixes = cx.option_str_list("silo_prefixes", &DEFAULT_PREFIXES);
        for object in cx.ontology.object_types.values() {
            let words = split_camel(object.api_name.as_str());
            if let [first, _, ..] = words.as_slice()
                && let Some(prefix) = prefixes.iter().find(|p| p.eq_ignore_ascii_case(first))
            {
                out.push(type_finding(
                    object,
                    format!(
                        "object type `{}` starts with the system or department prefix `{prefix}`",
                        object.api_name
                    ),
                    format!(
                        "name the type after the thing it represents, not the system that owns it; {}",
                        off_hint(self.id())
                    ),
                ));
            }
        }
        similar_pairs(cx, self.id(), out);
    }
}

/// Types named in one cluster message before the rest are counted.
const MAX_LISTED: usize = 20;

/// Property sets of the types that can take part in a similar pair, grouped by
/// primary key `(name, base type)`. Only types in the same group are compared, and
/// similar types are joined into clusters reported once each.
fn similar_pairs(cx: &LintCx<'_>, rule_id: &str, out: &mut Vec<Finding>) {
    type Key<'a> = (&'a str, &'a str);
    let mut groups: BTreeMap<Key<'_>, Vec<(&ObjectType, BTreeSet<Key<'_>>)>> = BTreeMap::new();
    for object in cx.ontology.object_types.values() {
        let Some(pk) = object.properties.get(object.primary_key.as_str()) else {
            continue;
        };
        if object.properties.len() < MIN_PROPERTIES {
            continue;
        }
        let set = object
            .properties
            .iter()
            .filter(|(name, _)| **name != object.primary_key)
            .map(|(name, def)| (name.as_str(), def.base_type.as_str()))
            .collect();
        groups
            .entry((object.primary_key.as_str(), pk.base_type.as_str()))
            .or_default()
            .push((object, set));
    }
    for members in groups.values() {
        let mut clusters = Clusters::new(members.len());
        for (i, (_, set_a)) in members.iter().enumerate() {
            for (j, (_, set_b)) in members.iter().enumerate().skip(i + 1) {
                let shared = set_a.intersection(set_b).count();
                #[allow(clippy::cast_precision_loss)] // set sizes are small
                let similarity = shared as f64 / (set_a.len() + set_b.len() - shared) as f64;
                if similarity >= MIN_SIMILARITY {
                    clusters.join(i, j, similarity);
                }
            }
        }
        for (indices, similarity) in clusters.into_groups() {
            // Members are in `apiName` order; the last one is reported.
            let Some((&last, others)) = indices.split_last() else {
                continue;
            };
            let reported = members[last].0;
            let mut names: Vec<String> = std::iter::once(last)
                .chain(others.iter().copied())
                .take(MAX_LISTED)
                .map(|i| format!("`{}`", members[i].0.api_name))
                .collect();
            let message = if indices.len() == 2 {
                format!(
                    "object types {} and {} have the same primary key and a property similarity of {similarity:.2}",
                    names[0], names[1]
                )
            } else {
                if indices.len() > MAX_LISTED {
                    names.push(format!("{} more", indices.len() - MAX_LISTED));
                }
                let tail = names.pop().unwrap_or_default();
                format!(
                    "object types {} and {tail} have the same primary key and property similarities of up to {similarity:.2}",
                    names.join(", ")
                )
            };
            out.push(type_finding(
                reported,
                message,
                format!(
                    "if they describe the same thing, merge them; {}",
                    off_hint(rule_id)
                ),
            ));
        }
    }
}

/// Union-find over member indices, with the highest similarity seen per cluster.
struct Clusters {
    parent: Vec<usize>,
    best: Vec<f64>,
}

impl Clusters {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            best: vec![0.0; len],
        }
    }

    fn root(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn join(&mut self, a: usize, b: usize, similarity: f64) {
        let (ra, rb) = (self.root(a), self.root(b));
        let best = self.best[ra].max(self.best[rb]).max(similarity);
        self.parent[rb] = ra;
        self.best[ra] = best;
    }

    /// Clusters of at least two members, each as ascending indices with its highest
    /// similarity.
    fn into_groups(mut self) -> Vec<(Vec<usize>, f64)> {
        let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for i in 0..self.parent.len() {
            let root = self.root(i);
            groups.entry(root).or_default().push(i);
        }
        groups
            .into_iter()
            .filter(|(_, indices)| indices.len() > 1)
            .map(|(root, indices)| (indices, self.best[root]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::structure::testing::{flagged, join, object, run, run_with, run_within};

    const CONTACT: [(&str, &str); 4] = [
        ("name", "string"),
        ("email", "string"),
        ("phone", "string"),
        ("city", "string"),
    ];

    #[test]
    fn triggers() {
        let mut crm: Vec<(&str, &str)> = CONTACT[..3].to_vec();
        crm.push(("region", "string"));
        let text = join(&[
            object("SalesOrder", "orderId", &[]),
            object("Contact", "contactId", &CONTACT),
            object("CrmContact", "contactId", &crm),
        ]);
        let found = run("ONT-W07", &text);
        // CrmContact is reported twice: the crm prefix and the similarity to Contact.
        assert_eq!(
            flagged(&text, &found),
            ["SalesOrder", "CrmContact", "CrmContact"]
        );
        let messages: Vec<&str> = found.iter().map(|d| d.message.as_str()).collect();
        assert!(
            messages.iter().any(|m| m.contains("prefix `crm`")),
            "{messages:#?}"
        );
        assert!(
            messages
                .iter()
                // 3 of 5 non-key properties shared; the primary key is left out.
                .any(|m| m.contains("`Contact`") && m.contains("0.60")),
            "{messages:#?}"
        );
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = join(&[
            object("Sales", "salesId", &[]),
            object(
                "Alpha",
                "refId",
                &[
                    ("a", "string"),
                    ("b", "string"),
                    ("c", "string"),
                    ("d", "string"),
                ],
            ),
            object(
                "Beta",
                "refId",
                &[
                    ("a", "string"),
                    ("b", "string"),
                    ("e", "string"),
                    ("f", "string"),
                    ("g", "string"),
                ],
            ),
            // Same properties as Contact, but another primary key name.
            object("Person", "personId", &CONTACT),
            object("Contact", "contactId", &CONTACT),
        ]);
        let found = run("ONT-W07", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    /// CR-02: a thousand similar types once produced one warning per pair.
    #[test]
    fn many_similar_types_are_reported_once() {
        let own: Vec<String> = (0..1000).map(|i| format!("note{i}")).collect();
        let docs: Vec<String> = (0..1000)
            .map(|i| {
                let props = [
                    ("createdAt", "timestamp"),
                    ("updatedAt", "timestamp"),
                    ("ownerId", "string"),
                    (own[i].as_str(), "string"),
                ];
                object(&format!("Thing{i}"), "id", &props)
            })
            .collect();
        let found = run_within(5, "ONT-W07", join(&docs));
        assert_eq!(found.len(), 1, "{} findings", found.len());
        assert_eq!(flagged(&join(&docs), &found), ["Thing999"]);
        assert!(
            found[0].message.contains("and 980 more have"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn a_cluster_lists_every_member() {
        let text = join(&[
            object("Contact", "contactId", &CONTACT),
            object("Person", "contactId", &CONTACT),
            object("Customer", "contactId", &CONTACT),
        ]);
        let found = run("ONT-W07", &text);
        assert_eq!(flagged(&text, &found), ["Person"]);
        assert_eq!(
            found[0].message,
            "object types `Person`, `Contact` and `Customer` have the same primary key and property similarities of up to 1.00"
        );
    }

    #[test]
    fn silo_prefixes_option_applies() {
        let text = object("OpsTicket", "ticketId", &[]);
        assert!(run("ONT-W07", &text).is_empty());
        let config = "[lint.options.\"ONT-W07\"]\nsilo_prefixes = [\"ops\"]\n";
        let found = run_with("ONT-W07", &text, Some(config));
        assert_eq!(flagged(&text, &found), ["OpsTicket"]);
    }
}
