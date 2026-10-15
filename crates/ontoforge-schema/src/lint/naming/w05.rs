//! ONT-W05: pipeline bookkeeping must not leak into the ontology (D-13).
//!
//! A property is reported when its `split_camel` words contain one of these runs:
//! `etl`, `extracted at`, `received at`, `batched at`, `sequence`, `table version`,
//! `internal record id`. The leading underscore variant is covered by `ONT-V07`.
//!
//! Known trade-off: `sequenceNumber` contains the word `sequence` and is reported,
//! although a flight leg sequence number is a legitimate domain term. The rule is a
//! warning and the help text says how to switch it off.

use super::super::{Finding, Level, LintCx, LintRule};
use super::{off_hint, property_finding};
use crate::lint::words::split_camel;

const BOOKKEEPING: [&[&str]; 7] = [
    &["etl"],
    &["extracted", "at"],
    &["received", "at"],
    &["batched", "at"],
    &["sequence"],
    &["table", "version"],
    &["internal", "record", "id"],
];

pub(super) struct W05;

pub(super) static RULE: W05 = W05;

impl LintRule for W05 {
    fn id(&self) -> &'static str {
        "ONT-W05"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for object in cx.ontology.object_types.values() {
            for name in object.properties.keys() {
                let words = split_camel(name.as_str());
                let hit = BOOKKEEPING
                    .iter()
                    .find(|run| words.windows(run.len()).any(|w| w == **run));
                if let Some(run) = hit {
                    out.push(property_finding(
                        object,
                        name.as_str(),
                        format!(
                            "property `{name}` looks like pipeline bookkeeping (`{}`)",
                            run.join(" ")
                        ),
                        format!(
                            "data pipeline fields do not belong in an ontology, so drop it; {}",
                            off_hint(self.id())
                        ),
                    ));
                }
            }
        }
    }
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
