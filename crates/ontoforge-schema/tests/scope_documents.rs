//! Scope documents (`Role`, `RoleBinding`): loading, kind dispatch and diagnostics.

use camino::Utf8Path;
use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::{Subject, Toolset};
use ontoforge_schema::{ScopeLoadOutput, load_scope_dir, load_scope_from_memory};

const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

const ROLE: &str = "kind: Role\napiName: dispatcher\nobjectTypes:\n  Flight:\n    properties: [flightId]\n    rowFilter: \"object.status == 'X'\"\n";

fn load(files: &[(&str, &str)]) -> ScopeLoadOutput {
    load_scope_from_memory(files)
}

fn only(out: &ScopeLoadOutput) -> &Diagnostic {
    assert_eq!(
        out.diagnostics.len(),
        1,
        "diagnostics: {:#?}",
        out.diagnostics
    );
    &out.diagnostics[0]
}

#[test]
fn loads_aviation_scope_example() {
    let out = load_scope_dir(Utf8Path::new(AVIATION_SCOPE));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let scope = out.scope.expect("the example loads");

    let names: Vec<&str> = scope.roles.keys().map(|n| n.as_str()).collect();
    assert_eq!(names, ["crewScheduler", "dispatcher", "maintenanceLead"]);
    assert_eq!(
        scope.roles["dispatcher"].object_types["Flight"]
            .row_filter
            .as_deref(),
        Some("object.status != 'CANCELLED' || object.status == null")
    );
    assert_eq!(
        scope.bindings["dispatcher"].subjects,
        [Subject::Group("dispatchers".to_owned())]
    );
    assert_eq!(
        scope.bindings["maintenanceLead"].subjects,
        [Subject::User("ontoforge-dev|mia".to_owned())]
    );
    assert_eq!(
        scope.roles["maintenanceLead"].toolsets,
        [Toolset::Full, Toolset::SqlRaw]
    );
}

#[test]
fn unknown_field_is_v04_with_location() {
    let out = load(&[("roles.yaml", &ROLE.replace("rowFilter", "rowfilter"))]);
    let d = only(&out);
    assert_eq!(d.code, codes::V04);
    assert_eq!((d.file.as_str(), d.line, d.column), ("roles.yaml", 6, 5));
    assert!(
        d.help
            .as_deref()
            .is_some_and(|h| h.contains("did you mean `rowFilter`")),
        "{:?}",
        d.help
    );
    assert!(out.scope.is_none());
}

#[test]
fn binding_to_unknown_role_is_v21() {
    let binding = "kind: RoleBinding\nrole: ghost\nsubjects:\n  - group: a\n";
    let out = load(&[("roles.yaml", ROLE), ("bindings.yaml", binding)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V21);
    assert_eq!((d.file.as_str(), d.line, d.column), ("bindings.yaml", 2, 7));
    assert!(d.message.contains("ghost"), "{}", d.message);
}

#[test]
fn toolset_value_is_checked() {
    let role = "kind: Role\napiName: dispatcher\ntoolsets: [sql]\n";
    let out = load(&[("roles.yaml", role)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!((d.file.as_str(), d.line, d.column), ("roles.yaml", 3, 12));
    assert!(d.message.contains("sql"), "{}", d.message);
}

#[test]
fn multi_document_file_loads() {
    let text = format!(
        "{ROLE}---\nkind: RoleBinding\nrole: dispatcher\nsubjects:\n  - group: dispatchers\n"
    );
    let out = load(&[("scope.yaml", &text)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let scope = out.scope.expect("loads");
    assert_eq!(scope.roles.len(), 1);
    assert_eq!(scope.bindings.len(), 1);
}
