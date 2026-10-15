//! ONT-W05 (RED stub).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct W05;

pub(super) static RULE: W05 = W05;

impl LintRule for W05 {
    fn id(&self) -> &'static str {
        "ONT-W05"
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
            "Shipment",
            &[
                ("etlBatchId", "string"),
                ("extractedAt", "timestamp"),
                ("rowSequence", "integer"),
                ("tableVersion", "integer"),
                ("internalRecordId", "string"),
            ],
        )]);
        let found = run("ONT-W05", &text);
        assert_eq!(
            flagged(&text, &found),
            [
                "etlBatchId",
                "extractedAt",
                "rowSequence",
                "tableVersion",
                "internalRecordId"
            ]
        );
        assert!(found[2].help.as_deref().unwrap().contains("ONT-W05"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = objects(&[(
            "Shipment",
            &[
                ("createdAt", "timestamp"),
                ("flightNumber", "string"),
                ("receivedBy", "string"),
            ],
        )]);
        let found = run("ONT-W05", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
