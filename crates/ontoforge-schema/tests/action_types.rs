//! Action types through the public in-memory API: parsing (SCHEMA-05) and the
//! structural checks V09 and V12 for parameters, rules and value sources.

use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::{LoadOutput, load_from_memory};
use serde_json::{Value, json};

const MANIFEST: &str = "apiName: demo\n";

const OBJECTS: &str = "kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
  delayMinutes: { type: integer }
  tags: { type: array, items: string }
---
kind: ObjectType
apiName: Pilot
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
---
kind: ObjectType
apiName: MaintenanceRecord
primaryKey: recordId
properties:
  recordId: { type: string, required: true }
  aircraftTailNumber: { type: string, required: true }
  performedAt: { type: timestamp }
  performedBy: { type: string }
  summary: { type: string }
";

const LINKS: &str = "kind: LinkType
apiName: flightCrew
cardinality: manyToMany
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: assignedFlights }
";

fn load(actions: &str) -> LoadOutput {
    load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", OBJECTS),
        ("links.yaml", LINKS),
        ("actions.yaml", actions),
    ])
}

fn only(out: &LoadOutput) -> &Diagnostic {
    assert_eq!(
        out.diagnostics.len(),
        1,
        "diagnostics: {:#?}",
        out.diagnostics
    );
    &out.diagnostics[0]
}

/// The loaded ontology as JSON, so the assertions read the serialized IR shape.
fn ontology_json(out: &LoadOutput) -> Value {
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    serde_json::to_value(out.ontology.as_ref().expect("ontology")).expect("serializable IR")
}

/// The source text a diagnostic's span covers.
fn spanned<'a>(text: &'a str, d: &Diagnostic) -> &'a str {
    &text[d.span.offset..d.span.offset + d.span.length]
}

#[test]
fn parses_all_rule_kinds() {
    let out = load(
        "kind: ActionType
apiName: everything
parameters:
  flight: { type: { objectReference: Flight }, required: true }
  pilot: { type: { objectReference: Pilot }, required: true }
  recordId: { type: string, required: true }
  tail: { type: string, required: true }
rules:
  - kind: createObject
    objectType: MaintenanceRecord
    properties:
      recordId: { param: recordId }
      aircraftTailNumber: { param: tail }
  - kind: modifyObject
    object: flight
    properties:
      status: { static: CHECKED }
  - kind: deleteObject
    object: flight
  - kind: addLink
    link: flightCrew
    from: flight
    to: pilot
  - kind: removeLink
    link: flightCrew
    from: flight
    to: pilot
",
    );
    let json = ontology_json(&out);
    let rules = json["actionTypes"]["everything"]["rules"]
        .as_array()
        .expect("rules list");
    let kinds: Vec<&str> = rules
        .iter()
        .map(|r| {
            r.as_object()
                .expect("rule map")
                .keys()
                .next()
                .expect("kind")
        })
        .map(String::as_str)
        .collect();
    assert_eq!(
        kinds,
        [
            "createObject",
            "modifyObject",
            "deleteObject",
            "addLink",
            "removeLink"
        ]
    );
    assert_eq!(rules[0]["createObject"]["objectType"], "MaintenanceRecord");
    assert_eq!(rules[1]["modifyObject"]["object"], "flight");
    assert_eq!(rules[3]["addLink"]["link"], "flightCrew");
    assert_eq!(rules[4]["removeLink"]["to"], "pilot");
}

#[test]
fn parses_all_value_sources() {
    let out = load(
        "kind: ActionType
apiName: record
parameters:
  recordId: { type: string, required: true }
rules:
  - kind: createObject
    objectType: MaintenanceRecord
    properties:
      recordId: { param: recordId }
      aircraftTailNumber: { static: N123 }
      performedAt: { currentTime: {} }
      performedBy: { currentUser: {} }
",
    );
    let json = ontology_json(&out);
    let props = &json["actionTypes"]["record"]["rules"][0]["createObject"]["properties"];
    assert_eq!(props["recordId"], json!({ "param": "recordId" }));
    assert_eq!(props["aircraftTailNumber"], json!({ "static": "N123" }));
    assert_eq!(props["performedAt"], json!({ "currentTime": {} }));
    assert_eq!(props["performedBy"], json!({ "currentUser": {} }));
}

#[test]
fn parameter_types() {
    let out = load(
        "kind: ActionType
apiName: tune
displayName: Tune
description: Parameter type spellings.
parameters:
  flight: { type: { objectReference: Flight }, required: true }
  minutes: { type: integer, required: true, description: How long. }
  labels: { type: array, items: string, default: [] }
submissionCriteria:
  - condition: \"params.minutes > 0\"
    failureMessage: Minutes must be positive
rules:
  - kind: modifyObject
    object: flight
    properties:
      delayMinutes: { param: minutes }
      tags: { param: labels }
",
    );
    let json = ontology_json(&out);
    let params = &json["actionTypes"]["tune"]["parameters"];
    assert_eq!(
        params["flight"]["type"],
        json!({ "objectReference": "Flight" })
    );
    assert_eq!(params["flight"]["required"], true);
    assert_eq!(params["minutes"]["type"], "integer");
    assert_eq!(params["minutes"]["description"], "How long.");
    assert_eq!(params["labels"]["type"], "array");
    assert_eq!(params["labels"]["items"], "string");
    assert_eq!(params["labels"]["default"], json!([]));
    let criteria = &json["actionTypes"]["tune"]["submissionCriteria"][0];
    assert_eq!(criteria["condition"], "params.minutes > 0");
    assert_eq!(criteria["failureMessage"], "Minutes must be positive");
}

#[test]
fn function_rule_is_v03() {
    let text = "kind: ActionType
apiName: compute
rules:
  - kind: function
    name: doIt
";
    let out = load(text);
    let d = only(&out);
    assert_eq!(d.code, codes::V03);
    assert!(d.message.contains("function"), "{}", d.message);
    assert!(d.message.contains("v1"), "{}", d.message);
    assert_eq!((d.file.as_str(), d.line, d.column), ("actions.yaml", 4, 11));
    assert_eq!(spanned(text, d), "function");
}

#[test]
fn unknown_rule_kind_suggests() {
    let text = "kind: ActionType
apiName: make
rules:
  - kind: createObjet
    objectType: Pilot
    properties: {}
";
    let out = load(text);
    let d = only(&out);
    assert_eq!(d.code, codes::V03);
    assert_eq!(d.help.as_deref(), Some("did you mean `createObject`?"));
    assert_eq!((d.line, d.column), (4, 11));
    assert_eq!(spanned(text, d), "createObjet");
}

#[test]
fn rule_missing_kind_is_v02() {
    let text = "kind: ActionType
apiName: make
rules:
  - object: flight
";
    let out = load(text);
    let d = only(&out);
    assert_eq!(d.code, codes::V02);
    assert_eq!((d.line, d.column), (4, 5));
}

#[test]
fn unknown_field_inside_rule_is_v04() {
    let text = "kind: ActionType
apiName: crew
parameters:
  flight: { type: { objectReference: Flight } }
  pilot: { type: { objectReference: Pilot } }
rules:
  - kind: addLink
    link: flightCrew
    from: flight
    too: pilot
    to: pilot
";
    let out = load(text);
    let d = only(&out);
    assert_eq!(d.code, codes::V04);
    assert_eq!(d.help.as_deref(), Some("did you mean `to`?"));
    assert_eq!((d.line, d.column), (10, 5));
    assert_eq!(spanned(text, d), "too");
}

#[test]
fn value_source_with_two_keys_is_v06() {
    let text = "kind: ActionType
apiName: tweak
parameters:
  a: { type: string }
rules:
  - kind: createObject
    objectType: Pilot
    properties:
      employeeId: { param: a, static: x }
";
    let out = load(text);
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!((d.line, d.column), (9, 19));
}
