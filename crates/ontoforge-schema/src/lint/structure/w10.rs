//! ONT-W10: Time Machine (D-13).
//!
//! Versioning belongs to the edit history, not to the schema. Reported:
//! - properties named exactly `version`, `revision` or `isCurrent`;
//! - object type names ending in a four digit year (`Flight2024`) or in `v` plus
//!   digits (`FlightV2`).

use std::sync::LazyLock;

use regex::Regex;

use super::super::{Finding, Level, LintCx, LintRule};
use super::{off_hint, type_finding};
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

const VERSION_PROPERTIES: [&str; 3] = ["version", "revision", "isCurrent"];

/// A four digit year or `v` plus digits at the end of the name.
static VERSIONED_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4}|[vV]\d+)$").expect("fixed pattern"));

pub(super) struct W10;

pub(super) static RULE: W10 = W10;

impl LintRule for W10 {
    fn id(&self) -> &'static str {
        "ONT-W10"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        let help = format!(
            "keep history in the edit log instead of in the schema; {}",
            off_hint(self.id())
        );
        for object in cx.ontology.object_types.values() {
            for name in object.properties.keys() {
                if VERSION_PROPERTIES.contains(&name.as_str()) {
                    out.push(Finding {
                        def: DefKey {
                            kind: DefKind::ObjectType,
                            api_name: object.api_name.to_string(),
                        },
                        path: vec![
                            Seg::Key("properties".to_owned()),
                            Seg::Key(name.to_string()),
                        ],
                        part: Part::Key,
                        message: format!("property `{name}` versions the object inside the schema"),
                        help: Some(help.clone()),
                        severity: None,
                    });
                }
            }
            if VERSIONED_TYPE.is_match(object.api_name.as_str()) {
                out.push(type_finding(
                    object,
                    format!(
                        "object type `{}` has a year or version number in its name",
                        object.api_name
                    ),
                    help.clone(),
                ));
            }
        }
    }
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
