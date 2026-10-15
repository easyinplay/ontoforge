//! ONT-W02 (RED stub).

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct W02;

pub(super) static RULE: W02 = W02;

impl LintRule for W02 {
    fn id(&self) -> &'static str {
        "ONT-W02"
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
            (
                "Shipment",
                &[("status", "string"), ("shippingStatus", "string")],
            ),
            ("Person", &[("name", "string"), ("displayName", "string")]),
        ]);
        let found = run("ONT-W02", &text);
        assert_eq!(flagged(&text, &found), ["status", "name"]);
        assert!(found[0].message.contains("`shippingStatus`"));
        assert!(found[0].help.as_deref().unwrap().contains("flightStatus"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = objects(&[
            ("Airline", &[("name", "string")]),
            ("Booking", &[("status", "string")]),
            ("Flight", &[("flightStatus", "string")]),
        ]);
        let found = run("ONT-W02", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }
}
