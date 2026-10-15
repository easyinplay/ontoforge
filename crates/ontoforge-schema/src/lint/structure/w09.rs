//! ONT-W09: Action Sprawl (D-13).
//!
//! Two checks:
//! - an object type that more than `max_actions_per_type` actions (default 10)
//!   create, modify or delete is reported on the object type. An action belongs to
//!   the type it creates or to the target type of the `objectReference` parameter a
//!   `modifyObject` / `deleteObject` rule names; it counts once per type;
//! - an action named `set...` or `update...` (camelCase, so `setStatus`) whose only
//!   rule is a `modifyObject` assigning one property is a property setter in disguise
//!   and is reported on the action. The original rule talks about display names with
//!   spaces; here the `apiName` is checked.

use super::super::{Finding, Level, LintCx, LintRule};

pub(super) struct W09;

pub(super) static RULE: W09 = W09;

impl LintRule for W09 {
    fn id(&self) -> &'static str {
        "ONT-W09"
    }

    fn default_level(&self) -> Level {
        Level::Warn
    }

    fn option_keys(&self) -> &'static [&'static str] {
        &["max_actions_per_type"]
    }

    fn check(&self, _cx: &LintCx<'_>, _out: &mut Vec<Finding>) {}
}

#[cfg(test)]
mod tests {
    use crate::lint::structure::testing::{flagged, join, object, run, run_with};

    fn flight() -> String {
        object(
            "Flight",
            "flightId",
            &[("status", "string"), ("code", "string")],
        )
    }

    fn pilot() -> String {
        object("Pilot", "pilotId", &[("status", "string")])
    }

    /// An action with one `modifyObject` rule on a `Flight` or `Pilot` parameter.
    fn modify(name: &str, target: &str, props: &[&str]) -> String {
        let assigned: Vec<String> = props
            .iter()
            .map(|p| format!("      {p}: {{ param: value }}"))
            .collect();
        format!(
            "kind: ActionType\napiName: {name}\nparameters:\n  target: {{ type: {{ objectReference: {target} }}, required: true }}\n  value: {{ type: string }}\nrules:\n  - kind: modifyObject\n    object: target\n    properties:\n{}\n",
            assigned.join("\n")
        )
    }

    fn create_flight(name: &str) -> String {
        format!(
            "kind: ActionType\napiName: {name}\nparameters:\n  flightId: {{ type: string, required: true }}\nrules:\n  - kind: createObject\n    objectType: Flight\n    properties:\n      flightId: {{ param: flightId }}\n"
        )
    }

    #[test]
    fn triggers() {
        let mut docs = vec![flight(), pilot()];
        docs.extend((0..11).map(|i| modify(&format!("change{i}"), "Flight", &["status", "code"])));
        docs.push(modify("setStatus", "Pilot", &["status"]));
        let text = join(&docs);
        let found = run("ONT-W09", &text);
        assert_eq!(flagged(&text, &found), ["Flight", "setStatus"]);
        assert!(found[0].message.contains("11"), "{}", found[0].message);
        assert!(found[0].message.contains("10"), "{}", found[0].message);
        assert!(found[1].help.as_deref().unwrap().contains("intent"));
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let text = join(&[
            flight(),
            pilot(),
            modify("delayFlight", "Flight", &["status", "code"]),
            // Not `set` followed by a capital letter.
            modify("settleInvoice", "Flight", &["status"]),
            // Named like a setter but creates an object.
            create_flight("setFlight"),
        ]);
        let found = run("ONT-W09", &text);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn max_actions_per_type_option_applies() {
        let mut docs = vec![flight()];
        docs.extend((0..3).map(|i| modify(&format!("change{i}"), "Flight", &["status", "code"])));
        let text = join(&docs);
        assert!(run("ONT-W09", &text).is_empty());
        let config = "[lint.options.\"ONT-W09\"]\nmax_actions_per_type = 2\n";
        let found = run_with("ONT-W09", &text, Some(config));
        assert_eq!(flagged(&text, &found), ["Flight"]);
    }

    #[test]
    fn create_and_modify_count_for_the_same_type() {
        let mut docs = vec![flight(), create_flight("openFlight")];
        docs.extend((0..2).map(|i| modify(&format!("change{i}"), "Flight", &["status", "code"])));
        let text = join(&docs);
        let config = "[lint.options.\"ONT-W09\"]\nmax_actions_per_type = 2\n";
        let found = run_with("ONT-W09", &text, Some(config));
        assert_eq!(flagged(&text, &found), ["Flight"]);
    }
}
