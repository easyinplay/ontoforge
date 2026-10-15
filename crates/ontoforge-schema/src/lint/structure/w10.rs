//! ONT-W10: Time Machine (D-13).
//!
//! Versioning belongs to the edit history, not to the schema. Reported:
//! - properties named exactly `version`, `revision` or `isCurrent`;
//! - object type names ending in a four digit year (`Flight2024`) or in `v` plus
//!   digits (`FlightV2`).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct W10;

pub(super) static RULE: W10 = W10;

impl LintRule for W10 {
    fn id(&self) -> &'static str {
        "ONT-W10"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {}
}

#[cfg(test)]
mod tests {
    use crate::lint::structure::testing::{flagged, join, object, run};

    #[test]
    fn triggers() {
        let text = join(&[
            object(
                "Shipment",
                "shipmentId",
                &[
                    ("version", "integer"),
                    ("revision", "integer"),
                    ("isCurrent", "boolean"),
                ],
            ),
            object("Flight2024", "flightId", &[]),
            object("FlightV2", "flightV2Id", &[]),
        ]);
        let found = run("ONT-W10", &text);
        assert_eq!(
            flagged(&text, &found),
            ["version", "revision", "isCurrent", "Flight2024", "FlightV2"]
        );
        assert!(found[3].help.as_deref().unwrap().contains("edit"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = join(&[
            object(
                "Shipment",
                "shipmentId",
                &[("versionLabel", "string"), ("currentLeg", "integer")],
            ),
            object("Version", "versionId", &[]),
            object("Boeing747", "tailNumber", &[]),
        ]);
        let found = run("ONT-W10", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
