//! ONT-W11: generic link names (D-13).
//!
//! A link `apiName`, or the `apiName` of either end, that starts with `related` or
//! with `link` plus a digit (case-insensitive) says nothing about the relationship.
//!
//! The original rule also asks for an API name in both directions. The definition
//! format makes both end `apiName`s required (a missing one is `ONT-V05`), so that
//! check is not repeated here.

use super::super::{Finding, Level, LintCx, LintRule};

fn is_generic(_name: &str) -> bool {
    false
}

pub(super) struct W11;

pub(super) static RULE: W11 = W11;

impl LintRule for W11 {
    fn id(&self) -> &'static str {
        "ONT-W11"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {}
}

#[cfg(test)]
mod tests {
    use super::is_generic;
    use crate::lint::structure::testing::{flagged, join, object, run};

    fn link(name: &str, from_end: &str, to_end: &str, fk: &str) -> String {
        format!(
            "kind: LinkType\napiName: {name}\ncardinality: oneToMany\nfrom: {{ objectType: Airport, apiName: {from_end} }}\nto: {{ objectType: Flight, apiName: {to_end}, foreignKey: {fk} }}\n"
        )
    }

    fn types() -> [String; 2] {
        [
            object("Airport", "airportId", &[]),
            object(
                "Flight",
                "flightId",
                &[
                    ("fromCode", "string"),
                    ("toCode", "string"),
                    ("altCode", "string"),
                ],
            ),
        ]
    }

    #[test]
    fn triggers() {
        let [airport, flight] = types();
        let text = join(&[
            airport,
            flight,
            link("relatedFlights", "flights", "origin", "fromCode"),
            link("airportArrivals", "link1", "destination", "toCode"),
            link("airportAlt", "alternates", "relatedAirport", "altCode"),
        ]);
        let found = run("ONT-W11", &text);
        assert_eq!(
            flagged(&text, &found),
            ["relatedFlights", "link1", "relatedAirport"]
        );
        assert!(found[0].help.as_deref().unwrap().contains("ONT-W11"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let [airport, flight] = types();
        let text = join(&[
            airport,
            flight,
            link(
                "airportDepartures",
                "departingFlights",
                "origin",
                "fromCode",
            ),
            link(
                "airportArrivals",
                "arrivingFlights",
                "linkedAccount",
                "toCode",
            ),
        ]);
        let found = run("ONT-W11", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn matching_ignores_case() {
        for name in ["RelatedThing", "relatedThing", "LINK2", "link10x"] {
            assert!(is_generic(name), "{name}");
        }
        for name in ["linkedAccount", "link", "linkage", "unrelated"] {
            assert!(!is_generic(name), "{name}");
        }
    }
}
