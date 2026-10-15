//! ONT-W04: date and timestamp properties should share one naming style (D-13).
//!
//! Only properties whose base type is `date` or `timestamp` count, across all object
//! types. Two styles are recognized by `split_camel` words:
//! - suffix style: the last word is `date`, `at` or `time` (`createdDate`, `performedAt`);
//! - prefix style: the first two words are `date of` or `time of` (`dateOfBirth`).
//!
//! A name that fits neither (`birthday`) takes no side. When both styles occur, each
//! property of the less common style is reported; on a tie the prefix style is
//! reported. A string property such as `dateOfNote` is not a date and is ignored.

use super::super::{Finding, Level, LintCx, LintRule};
use super::{off_hint, property_finding};
use crate::ir::{BaseType, ObjectType};
use crate::lint::words::split_camel;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    /// `xxxDate`, `xxxAt`, `xxxTime`.
    Suffix,
    /// `dateOfXxx`, `timeOfXxx`.
    Prefix,
}

impl Style {
    fn describe(self) -> &'static str {
        match self {
            Self::Suffix => "the xxxDate / xxxAt / xxxTime style",
            Self::Prefix => "the dateOfXxx style",
        }
    }
}

fn style_of(name: &str) -> Option<Style> {
    let words = split_camel(name);
    if matches!(
        words.last().map(String::as_str),
        Some("date" | "at" | "time")
    ) {
        Some(Style::Suffix)
    } else if matches!(
        words.as_slice(),
        [first, of, ..] if matches!(first.as_str(), "date" | "time") && of == "of"
    ) {
        Some(Style::Prefix)
    } else {
        None
    }
}

pub(super) struct W04;

pub(super) static RULE: W04 = W04;

impl LintRule for W04 {
    fn id(&self) -> &'static str {
        "ONT-W04"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        let mut styled: Vec<(&ObjectType, &str, Style)> = Vec::new();
        for object in cx.ontology.object_types.values() {
            for (name, prop) in &object.properties {
                if matches!(prop.base_type, BaseType::Date | BaseType::Timestamp)
                    && let Some(style) = style_of(name.as_str())
                {
                    styled.push((object, name.as_str(), style));
                }
            }
        }
        let count = |style| styled.iter().filter(|s| s.2 == style).count();
        let (suffix, prefix) = (count(Style::Suffix), count(Style::Prefix));
        if suffix == 0 || prefix == 0 {
            return;
        }
        let (minority, majority) = if suffix < prefix {
            (Style::Suffix, Style::Prefix)
        } else {
            (Style::Prefix, Style::Suffix)
        };
        let example = styled.iter().find(|s| s.2 == majority).map_or("", |s| s.1);
        for &(object, name, _) in styled.iter().filter(|s| s.2 == minority) {
            out.push(property_finding(
                object,
                name,
                format!(
                    "`{name}` uses {}, but most date properties here use {} (such as `{example}`)",
                    minority.describe(),
                    majority.describe()
                ),
                format!(
                    "rename it to follow {} so dates are named one way; {}",
                    majority.describe(),
                    off_hint(self.id())
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::naming::testing::{flagged, objects, run};

    #[test]
    fn triggers() {
        // Style A once (`createdDate`), style B twice: the minority style A is reported.
        let text = objects(&[
            ("Order", &[("createdDate", "date")]),
            (
                "Person",
                &[("dateOfBirth", "date"), ("dateOfHire", "timestamp")],
            ),
        ]);
        let found = run("ONT-W04", &text);
        assert_eq!(flagged(&text, &found), ["createdDate"]);
        let message = &found[0].message;
        assert!(
            message.contains("`createdDate`") && message.contains("`dateOfBirth`"),
            "{message}"
        );
        assert!(found[0].help.as_deref().unwrap().contains("dateOf"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn equal_counts_report_the_b_style() {
        let text = objects(&[
            ("Order", &[("createdDate", "date")]),
            ("Person", &[("dateOfBirth", "date")]),
        ]);
        let found = run("ONT-W04", &text);
        assert_eq!(flagged(&text, &found), ["dateOfBirth"]);
    }

    #[test]
    fn clean() {
        let text = objects(&[
            (
                "Order",
                &[
                    ("createdDate", "date"),
                    ("performedAt", "timestamp"),
                    ("startTime", "timestamp"),
                ],
            ),
            ("Note", &[("dateOfNote", "string")]),
        ]);
        let found = run("ONT-W04", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
