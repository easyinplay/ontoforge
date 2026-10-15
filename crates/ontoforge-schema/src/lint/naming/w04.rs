//! ONT-W04 (RED stub).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct W04;

pub(super) static RULE: W04 = W04;

impl LintRule for W04 {
    fn id(&self) -> &'static str {
        "ONT-W04"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {}
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
