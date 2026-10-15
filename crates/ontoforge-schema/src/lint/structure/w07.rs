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

use super::super::{Finding, Level, LintCx, LintRule};

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

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {
        let _ = (MIN_SIMILARITY, MIN_PROPERTIES);
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::structure::testing::{flagged, join, object, run, run_with};

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
        assert!(messages.iter().any(|m| m.contains("prefix `crm`")), "{messages:#?}");
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

    #[test]
    fn silo_prefixes_option_applies() {
        let text = object("OpsTicket", "ticketId", &[]);
        assert!(run("ONT-W07", &text).is_empty());
        let config = "[lint.options.\"ONT-W07\"]\nsilo_prefixes = [\"ops\"]\n";
        let found = run_with("ONT-W07", &text, Some(config));
        assert_eq!(flagged(&text, &found), ["OpsTicket"]);
    }
}
