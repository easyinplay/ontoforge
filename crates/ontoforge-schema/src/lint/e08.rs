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
        for action in cx.ontology.action_types.values() {
            check_action(cx, action, out);
        }
    }
}

fn key(name: &str) -> Seg {
    Seg::Key(name.to_owned())
}

fn check_action(cx: &LintCx<'_>, action: &ActionType, out: &mut Vec<Finding>) {
    let def = DefKey {
        kind: DefKind::ActionType,
        api_name: action.api_name.to_string(),
    };
    let finding = |i: usize, field: &str, message: String, help: Option<String>| Finding {
        def: def.clone(),
        path: vec![key("rules"), Seg::Index(i), key(field)],
        part: Part::Value,
        message,
        help,
        severity: None,
    };
    // Parameter name to the index of the rule that deleted it.
    let mut deleted: HashMap<&str, usize> = HashMap::new();
    // Object type and primary key source of every createObject so far.
    let mut created: Vec<(&str, &ValueSource)> = Vec::new();

    for (i, rule) in action.rules.iter().enumerate() {
        match rule {
            Rule::AddLink(r) | Rule::RemoveLink(r) => {
                let Some(link) = cx.ontology.link_types.get(r.link.as_str()) else {
                    continue;
                };
                let cardinality = match link.cardinality {
                    Cardinality::ManyToMany => continue,
                    Cardinality::OneToOne => "oneToOne",
                    Cardinality::OneToMany => "oneToMany",
                };
                let verb = if matches!(rule, Rule::AddLink(_)) {
                    "addLink"
                } else {
                    "removeLink"
                };
                let help = match &link.to.foreign_key {
                    Some(fk) => format!(
                        "use modifyObject on `{}` to set `{fk}` instead",
                        link.to.object_type
                    ),
                    None => format!(
                        "use modifyObject on `{}` to set the foreign key instead",
                        link.to.object_type
                    ),
                };
                out.push(finding(
                    i,
                    "link",
                    format!(
                        "{verb} only works on manyToMany links, but `{}` is {cardinality}",
                        r.link
                    ),
                    Some(help),
                ));
            }
            Rule::ModifyObject(r) => {
                if let Some(&j) = deleted.get(r.object.as_str()) {
                    let message = format!(
                        "`{}` is modified here but rules[{j}] already deletes it",
                        r.object
                    );
                    out.push(finding(i, "object", message, None));
                }
            }
            Rule::DeleteObject(r) => {
                if let Some(&j) = deleted.get(r.object.as_str()) {
                    let message = format!(
                        "`{}` is deleted here but rules[{j}] already deletes it",
                        r.object
                    );
                    out.push(finding(i, "object", message, None));
                } else {
                    deleted.insert(r.object.as_str(), i);
                }
            }
            Rule::ReleaseObject(r) => {
                if let Some(&j) = deleted.get(r.object.as_str()) {
                    out.push(finding(
                        i,
                        "object",
                        format!(
                            "`{}` is released here but rules[{j}] already removes it",
                            r.object
                        ),
                        None,
                    ));
                } else {
                    deleted.insert(r.object.as_str(), i);
                }
            }
            Rule::CreateObject(r) => {
                if let Some(j) = duplicate_create(cx, r, &created) {
                    let message = format!(
                        "rules[{j}] already creates a `{}` with the same primary key; the object would be created twice",
                        r.object_type
                    );
                    out.push(finding(i, "objectType", message, None));
                } else if let Some(source) = primary_key_source(cx, r) {
                    created.push((r.object_type.as_str(), source));
                }
            }
        }
    }
}

/// Where the new object's primary key comes from, if the rule sets it.
fn primary_key_source<'a>(cx: &LintCx<'_>, rule: &'a CreateObjectRule) -> Option<&'a ValueSource> {
    let object = cx.ontology.object_types.get(rule.object_type.as_str())?;
    rule.properties.get(object.primary_key.as_str())
}

/// Index of an earlier createObject of the same type with the same key source.
fn duplicate_create(
    cx: &LintCx<'_>,
    rule: &CreateObjectRule,
    created: &[(&str, &ValueSource)],
) -> Option<usize> {
    let source = primary_key_source(cx, rule)?;
    created
        .iter()
        .position(|(ty, other)| *ty == rule.object_type.as_str() && *other == source)
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
backing: editOnly
---
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  originCode: { type: string }
backing: editOnly
---
kind: ObjectType
apiName: Pilot
primaryKey: pilotId
properties:
  pilotId: { type: string, required: true }
backing: editOnly
---
kind: ObjectType
apiName: Badge
primaryKey: badgeId
properties:
  badgeId: { type: string, required: true }
  pilotId: { type: string }
backing: editOnly
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
backing: editOnly
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
