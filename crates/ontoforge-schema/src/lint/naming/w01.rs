//! ONT-W01: an object type must not be named like a bucket (D-13, static part).
//!
//! Two shapes are reported:
//! - generic: every word of the name is a generic word (`Data`, `Item`, `DataRecord`);
//! - plural: the last word looks plural (`Flights`, `Categories`), judged by a suffix
//!   heuristic with a list of words that end in `s` but are singular.
//!
//! Tightened against the RESEARCH wording (Claude's Discretion, do not revert): the
//! generic check needs all words to be generic. "Only one or two words" would flag
//! `OrderItem` and `MaintenanceRecord`, which are fine names; the rule's intent is a
//! type that is called a generic word.
//!
//! The plural test is a heuristic and will misjudge some words, so the help text
//! always says how to switch the rule off.

use super::super::{Finding, Level, LintCx, LintRule};
use super::{off_hint, type_finding};
use crate::lint::words::split_camel;

const GENERIC_WORDS: [&str; 6] = ["data", "item", "record", "info", "object", "entity"];

/// Words that end in `s` (or `ies`) and are singular.
const SINGULAR_S: [&str; 18] = [
    "series", "species", "news", "status", "address", "analysis", "business", "access", "class",
    "process", "alias", "canvas", "census", "campus", "corpus", "bonus", "virus", "chassis",
];

pub(super) struct W01;

pub(super) static RULE: W01 = W01;

impl LintRule for W01 {
    fn id(&self) -> &'static str {
        "ONT-W01"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for object in cx.ontology.object_types.values() {
            let name = object.api_name.as_str();
            let words = split_camel(name);
            let off = off_hint(self.id());
            if words.iter().all(|w| GENERIC_WORDS.contains(&w.as_str())) {
                out.push(type_finding(
                    object,
                    format!("object type `{name}` is named with generic words only"),
                    format!(
                        "name what the objects are, such as `Flight` or `MaintenanceRecord`; {off}"
                    ),
                ));
            } else if let Some(singular) = singular_of(name, words.last()) {
                out.push(type_finding(
                    object,
                    format!("object type `{name}` looks plural"),
                    format!("an object type names one object; try `{singular}`; {off}"),
                ));
            }
        }
    }
}

/// The singular spelling of `name` when its last word looks plural.
fn singular_of(name: &str, last: Option<&String>) -> Option<String> {
    let last = last?;
    if SINGULAR_S.contains(&last.as_str()) {
        return None;
    }
    let (drop, add) = if last.len() > 3 && last.ends_with("ies") {
        (3, "y")
    } else if last.len() > 1
        && last.ends_with('s')
        && !["ss", "us", "is"].iter().any(|end| last.ends_with(end))
    {
        (1, "")
    } else {
        return None;
    };
    name.get(..name.len() - drop)
        .map(|stem| format!("{stem}{add}"))
}

#[cfg(test)]
mod tests {
    use crate::lint::naming::testing::{flagged, objects, run};

    #[test]
    fn triggers() {
        let text = objects(&[
            ("Data", &[]),
            ("DataRecord", &[]),
            ("Flights", &[]),
            ("Categories", &[]),
        ]);
        let found = run("ONT-W01", &text);
        assert_eq!(
            flagged(&text, &found),
            ["Data", "DataRecord", "Flights", "Categories"]
        );
        assert!(found[3].help.as_deref().unwrap().contains("`Category`"));
        assert!(found[2].help.as_deref().unwrap().contains("`Flight`"));
        assert!(found[0].help.as_deref().unwrap().contains("ONT-W01"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = objects(&[
            ("Flight", &[]),
            ("Status", &[]),
            ("Address", &[]),
            ("Series", &[]),
            ("OrderItem", &[]),
            ("MaintenanceRecord", &[]),
        ]);
        let found = run("ONT-W01", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
