//! ONT-W01 (RED stub).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct W01;

pub(super) static RULE: W01 = W01;

impl LintRule for W01 {
    fn id(&self) -> &'static str {
        "ONT-W01"
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
