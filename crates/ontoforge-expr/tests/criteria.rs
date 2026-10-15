//! Static checks of submission criteria: references (Task 1) and types (Task 2).

use ontoforge_expr::check_ontology;
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::load_from_memory;

const MANIFEST: &str = "apiName: demo\n";

const OBJECTS: &str = "kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
  delayMinutes: { type: integer }
";

/// Header of the single action; the criterion line is appended by `check`.
const ACTION_HEAD: &str = "kind: ActionType
apiName: delayFlight
parameters:
  flight: { type: { objectReference: Flight }, required: true }
  delayMinutes: { type: integer, required: true }
  tags: { type: array, items: string }
rules:
  - kind: modifyObject
    object: flight
    properties:
      status: { static: DELAYED }
submissionCriteria:
";

fn actions_for(condition: &str) -> String {
    let quoted = condition.replace('\\', "\\\\").replace('"', "\\\"");
    format!("{ACTION_HEAD}  - condition: \"{quoted}\"\n    failureMessage: nope\n")
}

fn run(condition: &str) -> Vec<Diagnostic> {
    let actions = actions_for(condition);
    let out = load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", OBJECTS),
        ("actions.yaml", &actions),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    check_ontology(&ontology, &out.sources)
}

fn only(condition: &str) -> Diagnostic {
    let mut diags = run(condition);
    assert_eq!(diags.len(), 1, "{condition}: {diags:#?}");
    diags.remove(0)
}

fn assert_clean(condition: &str) {
    let diags = run(condition);
    assert!(diags.is_empty(), "{condition}: {diags:#?}");
}

#[test]
fn aviation_criteria_pass() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/aviation"
    );
    let texts: Vec<(String, String)> = [
        "ontology.yaml",
        "objects.yaml",
        "links.yaml",
        "actions.yaml",
    ]
    .iter()
    .map(|n| {
        let text = std::fs::read_to_string(format!("{dir}/{n}")).expect("aviation file");
        ((*n).to_owned(), text)
    })
    .collect();
    let files: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let out = load_from_memory(&files);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let diags = check_ontology(&ontology, &out.sources);
    assert!(diags.is_empty(), "{diags:#?}");
}

#[test]
fn analyzer_errors_become_v13() {
    let too_long = only(&"a".repeat(5000));
    assert_eq!(too_long.code, codes::V13);
    assert!(too_long.message.contains("4096"), "{}", too_long.message);

    let too_deep = only(&format!("{}1{}", "(".repeat(40), ")".repeat(40)));
    assert_eq!(too_deep.code, codes::V13);
    assert!(too_deep.message.contains("nesting"), "{}", too_deep.message);

    let syntax = only("params.delayMinutes +");
    assert_eq!(syntax.code, codes::V13);
    assert_eq!(syntax.severity, Severity::Error);
    assert!(
        syntax.message.contains("(at 1:22 in expression)"),
        "{}",
        syntax.message
    );
}

#[test]
fn unknown_root_variable() {
    let d = only("foo.bar == 1");
    assert_eq!(d.code, codes::V13);
    assert!(d.message.contains("`foo`"), "{}", d.message);
    assert!(
        d.message.contains("(at 1:1 in expression)"),
        "{}",
        d.message
    );
    let help = d.help.expect("help");
    assert!(
        help.contains("params") && help.contains("user") && help.contains("objects"),
        "{help}"
    );
}

#[test]
fn unknown_parameter_suggests() {
    let d = only("params.delayMinute > 0");
    assert_eq!(d.code, codes::V13);
    assert!(d.message.contains("`delayMinute`"), "{}", d.message);
    assert!(d.help.expect("help").contains("delayMinutes"));
}

#[test]
fn objects_requires_object_reference_param() {
    let d = only("objects.delayMinutes.x == 1");
    assert_eq!(d.code, codes::V13);
    assert!(d.message.contains("objectReference"), "{}", d.message);
    assert!(d.help.expect("help").contains("flight"));
}

#[test]
fn unknown_object_property_suggests() {
    let d = only("objects.flight.statu == 'X'");
    assert_eq!(d.code, codes::V13);
    assert!(d.message.contains("`statu`"), "{}", d.message);
    assert!(d.help.expect("help").contains("status"));
}

#[test]
fn user_fields_are_id_and_groups() {
    let d = only("user.email != ''");
    assert_eq!(d.code, codes::V13);
    assert!(d.message.contains("`email`"), "{}", d.message);
    assert_clean("user.id != ''");
    assert_clean("'ops' in user.groups");
}

#[test]
fn comprehension_variable_is_not_reported() {
    assert_clean("params.tags.all(t, t != '')");
}

#[test]
fn same_path_reported_once() {
    let diags = run("params.nope > 0 && params.nope < 9");
    assert_eq!(diags.len(), 1, "{diags:#?}");
}

#[test]
fn diagnostic_points_at_condition_value() {
    let actions = actions_for("params.delayMinute > 0");
    let out = load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", OBJECTS),
        ("actions.yaml", &actions),
    ]);
    let ontology = out.ontology.expect("ontology");
    let diags = check_ontology(&ontology, &out.sources);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    let d = &diags[0];
    let line_idx = actions
        .lines()
        .position(|l| l.contains("condition:"))
        .expect("condition line");
    let col = actions.lines().nth(line_idx).unwrap().find('"').unwrap() + 1;
    assert_eq!(d.file, "actions.yaml");
    assert_eq!(d.line as usize, line_idx + 1);
    assert_eq!(d.column as usize, col);
    let covered = &actions[d.span.offset..d.span.offset + d.span.length];
    assert!(covered.contains("params.delayMinute > 0"), "{covered}");
    assert!(
        d.message.contains("(at 1:1 in expression)"),
        "{}",
        d.message
    );
}
