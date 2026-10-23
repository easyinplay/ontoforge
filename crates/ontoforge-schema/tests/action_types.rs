//! Action types through the public in-memory API: parsing (SCHEMA-05) and the
//! structural checks V09 and V12 for parameters, rules and value sources.

use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::{BaseType, Rule};
use ontoforge_schema::ir::basetype::{check_literal, widens_to};
use ontoforge_schema::{LoadOutput, load_from_memory};
use serde_json::{Value, json};

const MANIFEST: &str = "apiName: demo\n";

#[test]
fn release_object_is_a_public_rule_kind() {
    let rule = serde_json::from_value::<Rule>(json!({
        "releaseObject": {"object": "flight"}
    }));
    assert!(rule.is_ok(), "releaseObject must parse as an Action rule: {rule:?}");
}

const OBJECTS: &str = "kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
  delayMinutes: { type: integer }
  tags: { type: array, items: string }
  speed: { type: double }
  ceiling: { type: long }
backing: editOnly
---
kind: ObjectType
apiName: Pilot
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
backing: editOnly
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
backing: editOnly
";

const LINKS: &str = "kind: LinkType
apiName: flightCrew
cardinality: manyToMany
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: assignedFlights }
backing: editOnly
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

// ---------------------------------------------------------------------------
// Structural checks (Task 2)
// ---------------------------------------------------------------------------

/// Parameters shared by most checks; every line is indented for `parameters:`.
const PARAMS: &str = "  flight: { type: { objectReference: Flight }, required: true }
  pilot: { type: { objectReference: Pilot }, required: true }
  n: { type: integer }
  s: { type: string }
  sh: { type: short }
  fl: { type: float }
  lg: { type: long }
  tags: { type: array, items: string }
  nums: { type: array, items: integer }
";

/// An action with the given indented parameter lines and rule lines.
fn probe(params: &str, rules: &str) -> String {
    format!("kind: ActionType\napiName: probe\nparameters:\n{params}rules:\n{rules}")
}

/// Load `PARAMS` plus `rules`, and return the single diagnostic with `code`.
fn single(rules: &str, code: &str) -> (String, Diagnostic) {
    let text = probe(PARAMS, rules);
    let out = load(&text);
    let d = only(&out).clone();
    assert_eq!(d.code, code, "{d:#?}");
    (text, d)
}

#[test]
fn literal_validation() {
    let ok = |v: Value, t: BaseType, items: Option<BaseType>| {
        let r = check_literal(&v, t, items);
        assert!(r.is_ok(), "{v} as {t:?}: {r:?}");
    };
    let bad = |v: Value, t: BaseType, items: Option<BaseType>| {
        let r = check_literal(&v, t, items);
        assert!(r.is_err(), "{v} as {t:?} should be rejected");
    };
    bad(json!(300), BaseType::Byte, None);
    ok(json!(127), BaseType::Byte, None);
    bad(json!(1_099_511_627_776_i64), BaseType::Integer, None);
    ok(json!(2_147_483_647), BaseType::Integer, None);
    bad(json!(1.5), BaseType::Integer, None);
    ok(json!(1.5), BaseType::Double, None);
    bad(json!("1.5"), BaseType::Float, None);
    ok(json!("12.50"), BaseType::Decimal, None);
    bad(json!("twelve"), BaseType::Decimal, None);
    bad(json!(true), BaseType::String, None);
    bad(json!("yes"), BaseType::Boolean, None);
    ok(json!(false), BaseType::Boolean, None);
    bad(json!("2026-02-30"), BaseType::Date, None);
    ok(json!("2026-10-15"), BaseType::Date, None);
    bad(json!("not-a-time"), BaseType::Timestamp, None);
    ok(json!("2026-10-15T08:00:00Z"), BaseType::Timestamp, None);
    bad(json!("91,0"), BaseType::Geopoint, None);
    bad(json!("0,181"), BaseType::Geopoint, None);
    ok(json!("57.6,10.4"), BaseType::Geopoint, None);
    ok(json!(["a", "b"]), BaseType::Array, Some(BaseType::String));
    bad(json!(["a", 1]), BaseType::Array, Some(BaseType::String));
    bad(json!("a"), BaseType::Array, Some(BaseType::String));
    bad(Value::Null, BaseType::String, None);
}

/// WR-04: numeric values only widen.
#[test]
fn numeric_values_only_widen() {
    use BaseType::{Byte, Decimal, Double, Float, Integer, Long, Short, String};
    for (from, to) in [
        (Byte, Short),
        (Short, Integer),
        (Integer, Long),
        (Byte, Long),
        (Float, Double),
        (Long, Long),
    ] {
        assert!(widens_to(from, to), "{from:?} -> {to:?}");
    }
    for (from, to) in [
        (Long, Integer),
        (Integer, Byte),
        (Double, Float),
        (Integer, Double),
        (Double, Decimal),
        (Integer, String),
    ] {
        assert!(!widens_to(from, to), "{from:?} -> {to:?}");
    }

    let compatible = "  - kind: modifyObject
    object: flight
    properties:
      delayMinutes: { param: sh }
      speed: { param: fl }
      ceiling: { param: n }
      tags: { param: tags }
";
    let out = load(&probe(PARAMS, compatible));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);

    let (_, d) = single(
        "  - kind: modifyObject
    object: flight
    properties:
      delayMinutes: { param: lg }
",
        codes::V12,
    );
    assert!(d.message.contains("long"), "{}", d.message);
    assert!(d.message.contains("integer"), "{}", d.message);

    let (_, d) = single(
        "  - kind: modifyObject
    object: flight
    properties:
      tags: { param: nums }
",
        codes::V12,
    );
    assert!(d.message.contains("array"), "{}", d.message);
}

#[test]
fn references_must_exist() {
    let create = |object_type: &str, extra: &str| {
        format!(
            "  - kind: createObject
    objectType: {object_type}
    properties:
      recordId: {{ param: s }}
      aircraftTailNumber: {{ param: s }}
{extra}"
        )
    };
    let modify = |props: &str| {
        format!("  - kind: modifyObject\n    object: flight\n    properties:\n      {props}\n")
    };
    let link = |name: &str, from: &str| {
        format!("  - kind: addLink\n    link: {name}\n    from: {from}\n    to: pilot\n")
    };
    // (rules, text the diagnostic must cover, expected suggestion)
    let cases: Vec<(String, &str, Option<&str>)> = vec![
        (
            create("MaintenanceRecrd", ""),
            "MaintenanceRecrd",
            Some("MaintenanceRecord"),
        ),
        (
            create("MaintenanceRecord", "      sumary: { param: s }\n"),
            "sumary",
            Some("summary"),
        ),
        (
            "  - kind: modifyObject\n    object: flihgt\n    properties:\n      status: { static: x }\n"
                .to_owned(),
            "flihgt",
            Some("flight"),
        ),
        (modify("statuss: { static: x }"), "statuss", Some("status")),
        (
            "  - kind: deleteObject\n    object: planet\n".to_owned(),
            "planet",
            None,
        ),
        (link("flightCrw", "flight"), "flightCrw", Some("flightCrew")),
        (link("flightCrew", "flihgt"), "flihgt", Some("flight")),
        (modify("status: { param: missing }"), "missing", None),
    ];
    for (rules, covered, suggestion) in cases {
        let text = probe(PARAMS, &rules);
        let out = load(&text);
        let d = only(&out);
        assert_eq!(d.code, codes::V09, "{covered}: {d:#?}");
        assert_eq!(spanned(&text, d), covered, "{d:#?}");
        let expected = suggestion.map(|s| format!("did you mean `{s}`?"));
        assert_eq!(d.help, expected, "{covered}");
    }
}

#[test]
fn object_reference_target_must_exist() {
    let text = probe(
        "  flight: { type: { objectReference: Flght } }\n",
        "  - kind: deleteObject\n    object: flight\n",
    );
    let out = load(&text);
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert_eq!(spanned(&text, d), "Flght");
    assert_eq!(d.help.as_deref(), Some("did you mean `Flight`?"));
}

#[test]
fn create_must_assign_primary_key_and_required() {
    let (text, d) = single(
        "  - kind: createObject
    objectType: MaintenanceRecord
    properties:
      summary: { param: s }
",
        codes::V12,
    );
    assert!(d.message.contains("`recordId`"), "{}", d.message);
    assert!(d.message.contains("`aircraftTailNumber`"), "{}", d.message);
    assert!(!d.message.contains("`summary`"), "{}", d.message);
    assert_eq!(spanned(&text, &d), "properties");
}

#[test]
fn modify_cannot_assign_primary_key() {
    let (text, d) = single(
        "  - kind: modifyObject
    object: flight
    properties:
      flightId: { param: s }
",
        codes::V12,
    );
    assert!(d.message.contains("flightId"), "{}", d.message);
    assert_eq!(spanned(&text, &d), "flightId");
}

#[test]
fn modify_object_param_must_be_object_reference() {
    let (text, d) = single(
        "  - kind: modifyObject
    object: n
    properties:
      status: { static: x }
",
        codes::V12,
    );
    assert!(d.message.contains("objectReference"), "{}", d.message);
    assert_eq!(spanned(&text, &d), "n");

    let (text, d) = single("  - kind: deleteObject\n    object: s\n", codes::V12);
    assert_eq!(spanned(&text, &d), "s");
}

#[test]
fn add_link_params_must_match_link_ends() {
    let swapped = probe(
        PARAMS,
        "  - kind: addLink\n    link: flightCrew\n    from: pilot\n    to: flight\n",
    );
    let out = load(&swapped);
    assert_eq!(out.diagnostics.len(), 2, "{:#?}", out.diagnostics);
    for d in &out.diagnostics {
        assert_eq!(d.code, codes::V12);
    }
    assert_eq!(spanned(&swapped, &out.diagnostics[0]), "pilot");
    assert!(out.diagnostics[0].message.contains("Flight"));

    let (text, d) = single(
        "  - kind: removeLink\n    link: flightCrew\n    from: s\n    to: pilot\n",
        codes::V12,
    );
    assert_eq!(spanned(&text, &d), "s");
}

#[test]
fn current_user_only_on_string() {
    let (text, d) = single(
        "  - kind: modifyObject
    object: flight
    properties:
      delayMinutes: { currentUser: {} }
",
        codes::V12,
    );
    assert!(d.message.contains("string"), "{}", d.message);
    // A mapping node is located at its opening brace.
    assert_eq!((spanned(&text, &d), d.column), ("{", 36));
}

#[test]
fn current_time_only_on_timestamp() {
    let (text, d) = single(
        "  - kind: modifyObject
    object: flight
    properties:
      status: { currentTime: {} }
",
        codes::V12,
    );
    assert!(d.message.contains("timestamp"), "{}", d.message);
    assert_eq!((spanned(&text, &d), d.column), ("{", 30));
}

#[test]
fn object_reference_param_cannot_be_assigned() {
    let (text, d) = single(
        "  - kind: modifyObject
    object: flight
    properties:
      status: { param: flight }
",
        codes::V12,
    );
    assert!(d.help.as_deref().unwrap_or_default().contains("string"));
    assert_eq!(spanned(&text, &d), "flight");
}

#[test]
fn static_null_is_rejected() {
    let (text, d) = single(
        "  - kind: modifyObject
    object: flight
    properties:
      status: { static: null }
",
        codes::V12,
    );
    assert_eq!(spanned(&text, &d), "null");
}

#[test]
fn static_literal_must_match_property_type() {
    for literal in ["abc", "2147483648", "1.5"] {
        let rules = format!(
            "  - kind: modifyObject
    object: flight
    properties:
      delayMinutes: {{ static: {literal} }}
"
        );
        let (text, d) = single(&rules, codes::V12);
        assert_eq!(spanned(&text, &d), literal);
    }
}

#[test]
fn parameter_default_must_be_valid_literal() {
    let rules = "  - kind: deleteObject\n    object: flight\n";
    let text = probe(
        "  flight: { type: { objectReference: Flight } }\n  n: { type: integer, default: abc }\n",
        rules,
    );
    let out = load(&text);
    let d = only(&out);
    assert_eq!(d.code, codes::V12);
    assert_eq!(spanned(&text, d), "abc");

    let ok = probe(
        "  flight: { type: { objectReference: Flight } }\n  n: { type: integer, default: 3 }\n",
        rules,
    );
    assert!(load(&ok).diagnostics.is_empty());
}

#[test]
fn object_reference_param_cannot_have_default() {
    let text = probe(
        "  flight: { type: { objectReference: Flight }, default: F1 }\n",
        "  - kind: deleteObject\n    object: flight\n",
    );
    let out = load(&text);
    let d = only(&out);
    assert_eq!(d.code, codes::V12);
    assert_eq!(spanned(&text, d), "F1");
}

#[test]
fn parameter_items_follow_the_property_rules() {
    let rules = "  - kind: deleteObject\n    object: flight\n";
    let flight = "  flight: { type: { objectReference: Flight } }\n";

    let text = probe(&format!("{flight}  a: {{ type: array }}\n"), rules);
    let out = load(&text);
    assert_eq!(only(&out).code, codes::V05);

    let text = probe(
        &format!("{flight}  b: {{ type: string, items: string }}\n"),
        rules,
    );
    let out = load(&text);
    assert_eq!(only(&out).code, codes::V06);
}

#[test]
fn action_requires_at_least_one_rule() {
    let text = "kind: ActionType\napiName: nothing\nrules: []\n";
    let out = load(text);
    let d = only(&out);
    assert_eq!(d.code, codes::V12);
    assert_eq!((d.file.as_str(), d.line), ("actions.yaml", 3));
}

#[test]
fn empty_condition_or_failure_message_is_v12() {
    let text = "kind: ActionType
apiName: gated
parameters:
  flight: { type: { objectReference: Flight } }
rules:
  - kind: deleteObject
    object: flight
submissionCriteria:
  - condition: \"\"
    failureMessage: Not allowed
  - condition: \"true\"
    failureMessage: \"  \"
";
    let out = load(text);
    assert_eq!(out.diagnostics.len(), 2, "{:#?}", out.diagnostics);
    assert!(out.diagnostics.iter().all(|d| d.code == codes::V12));
    assert_eq!(out.diagnostics[0].line, 9);
    assert_eq!(out.diagnostics[1].line, 12);
}
