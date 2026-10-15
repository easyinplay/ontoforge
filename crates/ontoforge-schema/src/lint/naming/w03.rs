//! ONT-W03 (RED stub).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct W03;

pub(super) static RULE: W03 = W03;

impl LintRule for W03 {
    fn id(&self) -> &'static str {
        "ONT-W03"
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
        let text = objects(&[(
            "Record",
            &[
                ("dtCreated", "date"),
                ("nTotal", "integer"),
                ("code01", "string"),
                ("field1", "string"),
                ("fieldA", "string"),
            ],
        )]);
        let found = run("ONT-W03", &text);
        assert_eq!(
            flagged(&text, &found),
            ["dtCreated", "nTotal", "code01", "field1", "fieldA"]
        );
        assert!(found[0].message.contains("^dt[A-Z]"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = objects(&[(
            "Record",
            &[
                ("createdAt", "timestamp"),
                ("level2", "integer"),
                ("airfieldCode", "string"),
                ("nCount", "integer"),
            ],
        )]);
        let found = run("ONT-W03", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
