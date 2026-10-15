//! ONT-E08: rule combinations an action may not contain (D-12).
//!
//! Checked:
//! - `addLink` / `removeLink` only on `manyToMany` links; one-to-one and
//!   one-to-many links are edited by setting the foreign key with `modifyObject`.
//! - an object that was deleted cannot be modified or deleted again.
//! - two `createObject` rules of the same type taking the primary key from the
//!   same source create the object twice.
//!
//! Not applicable in v1, so not checked here: a function rule next to other rules
//! (`kind: function` is `ONT-V03`), modifying before creating (`modifyObject` only
//! takes parameters, never an object created in the same action), and modifying
//! the primary key (`ONT-V12`).

use std::collections::HashMap;

use super::{Finding, Level, LintCx, LintRule};
use crate::ir::{ActionType, Cardinality, CreateObjectRule, Rule, ValueSource};
use crate::loader::sourcemap::{DefKey, DefKind, Part, Seg};

pub(super) struct E08;

pub(super) static RULE: E08 = E08;

impl LintRule for E08 {
    fn id(&self) -> &'static str {
        "ONT-E08"
    }

    fn default_level(&self) -> Level {
        Level::Error
    }

    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        let _ = (cx, out);
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::{Diagnostic, Severity};
    use crate::lint::lint_rule_on;

    const MANIFEST: &str = "apiName: demo\n";

    const OBJECTS: &str = "\
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
---
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  originCode: { type: string }
---
kind: ObjectType
apiName: Pilot
primaryKey: pilotId
properties:
  pilotId: { type: string, required: true }
---
kind: ObjectType
apiName: Badge
primaryKey: badgeId
properties:
  badgeId: { type: string, required: true }
  pilotId: { type: string }
";

    const LINKS: &str = "\
kind: LinkType
apiName: departures
cardinality: oneToMany
from: { objectType: Airport, apiName: departingFlights }
to: { objectType: Flight, apiName: origin, foreignKey: originCode }
---
kind: LinkType
apiName: pilotBadge
cardinality: oneToOne
from: { objectType: Pilot, apiName: badge }
to: { objectType: Badge, apiName: owner, foreignKey: pilotId }
---
kind: LinkType
apiName: crew
cardinality: manyToMany
from: { objectType: Flight, apiName: pilots }
to: { objectType: Pilot, apiName: flights }
";

    const TRIGGERS: &str = "\
kind: ActionType
apiName: addDeparture
parameters:
  airport: { type: { objectReference: Airport }, required: true }
  flight: { type: { objectReference: Flight }, required: true }
rules:
  - { kind: addLink, link: departures, from: airport, to: flight }
---
kind: ActionType
apiName: dropBadge
parameters:
  pilot: { type: { objectReference: Pilot }, required: true }
  badge: { type: { objectReference: Badge }, required: true }
rules:
  - { kind: removeLink, link: pilotBadge, from: pilot, to: badge }
---
kind: ActionType
apiName: deleteThenModify
parameters:
  flight: { type: { objectReference: Flight }, required: true }
rules:
  - { kind: deleteObject, object: flight }
  - kind: modifyObject
    object: flight
    properties:
      originCode: { static: JFK }
---
kind: ActionType
apiName: deleteTwice
parameters:
  flight: { type: { objectReference: Flight }, required: true }
rules:
  - { kind: deleteObject, object: flight }
  - { kind: deleteObject, object: flight }
---
kind: ActionType
apiName: createTwice
parameters:
  id: { type: string, required: true }
rules:
  - kind: createObject
    objectType: Flight
    properties:
      flightId: { param: id }
  - kind: createObject
    objectType: Flight
    properties:
      flightId: { param: id }
";

    const CLEAN: &str = "\
kind: ActionType
apiName: crewFlight
parameters:
  flight: { type: { objectReference: Flight }, required: true }
  pilot: { type: { objectReference: Pilot }, required: true }
rules:
  - { kind: addLink, link: crew, from: flight, to: pilot }
---
kind: ActionType
apiName: modifyThenDelete
parameters:
  flight: { type: { objectReference: Flight }, required: true }
rules:
  - kind: modifyObject
    object: flight
    properties:
      originCode: { static: JFK }
  - { kind: deleteObject, object: flight }
---
kind: ActionType
apiName: createDifferent
parameters:
  first: { type: string, required: true }
  second: { type: string, required: true }
rules:
  - kind: createObject
    objectType: Flight
    properties:
      flightId: { param: first }
  - kind: createObject
    objectType: Flight
    properties:
      flightId: { param: second }
  - kind: createObject
    objectType: Pilot
    properties:
      pilotId: { param: first }
";

    fn run(actions: &str, config: Option<&str>) -> Vec<Diagnostic> {
        lint_rule_on(
            "ONT-E08",
            &[
                ("ontology.yaml", MANIFEST),
                ("objects.yaml", OBJECTS),
                ("links.yaml", LINKS),
                ("actions.yaml", actions),
            ],
            config,
        )
    }

    #[test]
    fn triggers() {
        let found = run(TRIGGERS, None);
        assert_eq!(found.len(), 5, "{found:#?}");
        assert!(
            found
                .iter()
                .all(|d| d.code == "ONT-E08" && d.severity == Severity::Error)
        );
        insta::assert_json_snapshot!("triggers", found);
    }

    #[test]
    fn clean() {
        let found = run(CLEAN, None);
        assert!(found.is_empty(), "{found:#?}");
        insta::assert_json_snapshot!("clean", found);
    }

    #[test]
    fn link_help_points_at_the_foreign_key() {
        let found = run(TRIGGERS, None);
        let help = found[0].help.as_deref().unwrap();
        assert!(
            help.contains("modifyObject") && help.contains("originCode"),
            "{help}"
        );
    }

    #[test]
    fn config_off_disables_the_rule() {
        let found = run(TRIGGERS, Some("[lint]\n\"ONT-E08\" = \"off\"\n"));
        assert!(found.is_empty());
    }
}
