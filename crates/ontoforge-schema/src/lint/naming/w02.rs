//! ONT-W02: a bare generic property name next to a qualified one is ambiguous (D-13).
//!
//! `status` is reported when the same object type also has `shippingStatus` (or any
//! property whose last word is `status`): a reader cannot tell which status the bare
//! one is. A type with only `name`, or a `status` whose qualified twin lives on
//! another type, is not reported.

use super::super::{Finding, Level, LintCx, LintRule};
use super::{last_word, off_hint, property_finding};

const GENERIC_NAMES: [&str; 7] = [
    "value", "type", "status", "date", "name", "quantity", "score",
];

pub(super) struct W02;

pub(super) static RULE: W02 = W02;

impl LintRule for W02 {
    fn id(&self) -> &'static str {
        "ONT-W02"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for object in cx.ontology.object_types.values() {
            for name in object.properties.keys() {
                let lower = name.as_str().to_ascii_lowercase();
                if !GENERIC_NAMES.contains(&lower.as_str()) {
                    continue;
                }
                let twins: Vec<String> = object
                    .properties
                    .keys()
                    .filter(|other| {
                        *other != name && last_word(other.as_str()).is_some_and(|w| w == lower)
                    })
                    .map(|other| format!("`{other}`"))
                    .collect();
                if twins.is_empty() {
                    continue;
                }
                out.push(property_finding(
                    object,
                    name.as_str(),
                    format!(
                        "property `{name}` is ambiguous next to {} on `{}`",
                        twins.join(", "),
                        object.api_name
                    ),
                    format!(
                        "qualify it, such as `flight{}`; {}",
                        capitalized(&lower),
                        off_hint(self.id())
                    ),
                ));
            }
        }
    }
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| c.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::lint::naming::testing::{flagged, objects, run};

    #[test]
    fn triggers() {
        let text = objects(&[
            (
                "Shipment",
                &[("status", "string"), ("shippingStatus", "string")],
            ),
            ("Person", &[("name", "string"), ("displayName", "string")]),
        ]);
        let found = run("ONT-W02", &text);
        assert_eq!(flagged(&text, &found), ["status", "name"]);
        assert!(found[0].message.contains("`shippingStatus`"));
        assert!(found[0].help.as_deref().unwrap().contains("flightStatus"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = objects(&[
            ("Airline", &[("name", "string")]),
            ("Booking", &[("status", "string")]),
            ("Flight", &[("flightStatus", "string")]),
        ]);
        let found = run("ONT-W02", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
