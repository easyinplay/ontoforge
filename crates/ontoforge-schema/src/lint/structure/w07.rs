//! ONT-W07: Silo (D-13, names and types only).
//!
//! Two checks:
//! - an object type whose first name word is a system or department prefix
//!   (`silo_prefixes`, default `hr`, `sales`, `billing`, `crm`) and which has at least
//!   two words (`SalesOrder` yes, `Sales` no);
//! - two object types with the same primary key name and type, at least four
//!   properties each, and a Jaccard similarity of at least 0.6 between their
//!   `(property name, base type)` sets. The later type by `apiName` is reported.
//!
//! No data is looked at, so the similarity is a hint that two types describe the same
//! thing in different systems.

use std::collections::{BTreeMap, BTreeSet};

use super::super::words::split_camel;
use super::super::{Finding, Level, LintCx, LintRule};
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

    fn option_keys(&self) -> &'static [&'static str] {
        &["silo_prefixes"]
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

/// Property sets of the types that can take part in a similar pair, grouped by
/// primary key `(name, base type)`. Only types in the same group are compared.
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
            .map(|(name, def)| (name.as_str(), def.base_type.as_str()))
            .collect();
        groups
            .entry((object.primary_key.as_str(), pk.base_type.as_str()))
            .or_default()
            .push((object, set));
    }
    for members in groups.values() {
        for (i, (a, set_a)) in members.iter().enumerate() {
            for (b, set_b) in &members[i + 1..] {
                let shared = set_a.intersection(set_b).count();
                #[allow(clippy::cast_precision_loss)] // set sizes are small
                let similarity = shared as f64 / (set_a.len() + set_b.len() - shared) as f64;
                if similarity >= MIN_SIMILARITY {
                    out.push(type_finding(
                        b,
                        format!(
                            "object types `{}` and `{}` have the same primary key and a property similarity of {similarity:.2}",
                            b.api_name, a.api_name
                        ),
                        format!(
                            "if they describe the same thing, merge them; {}",
                            off_hint(rule_id)
                        ),
                    ));
                }
            }
        }
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
                .any(|m| m.contains("`Contact`") && m.contains("0.67")),
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
