//! Scope documents (`Role`, `RoleBinding`): loading, kind dispatch and diagnostics.

use camino::Utf8Path;
use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::{RoleName, Subject, Toolset};
use ontoforge_schema::{ScopeLoadOutput, load_scope_dir, load_scope_from_memory};

const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

const ROLE: &str = "kind: Role\napiName: dispatcher\nobjectTypes:\n  Flight:\n    properties: [flightId]\n    rowFilter: \"object.status == 'X'\"\n";

fn load(files: &[(&str, &str)]) -> ScopeLoadOutput {
    load_scope_from_memory(files)
}

fn only(diagnostics: &[Diagnostic]) -> &Diagnostic {
    assert_eq!(diagnostics.len(), 1, "diagnostics: {diagnostics:#?}");
    &diagnostics[0]
}

#[test]
fn loads_aviation_scope_example() {
    let out = load_scope_dir(Utf8Path::new(AVIATION_SCOPE));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let scope = out.scope.expect("the example loads");

    let names: Vec<&str> = scope.roles.keys().map(RoleName::as_str).collect();
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
    let d = only(&out.diagnostics);
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
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V21);
    assert_eq!((d.file.as_str(), d.line, d.column), ("bindings.yaml", 2, 7));
    assert!(d.message.contains("ghost"), "{}", d.message);
}

#[test]
fn toolset_value_is_checked() {
    let role = "kind: Role\napiName: dispatcher\ntoolsets: [sql]\n";
    let out = load(&[("roles.yaml", role)]);
    let d = only(&out.diagnostics);
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

const MANIFEST: &str = "apiName: demo\n";

#[test]
fn role_in_ontology_dir_is_v03() {
    let out =
        ontoforge_schema::load_from_memory(&[("ontology.yaml", MANIFEST), ("roles.yaml", ROLE)]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V03);
    assert_eq!((d.file.as_str(), d.line, d.column), ("roles.yaml", 1, 7));
    assert!(d.message.contains("scope directory"), "{}", d.message);
    assert!(
        d.help
            .as_deref()
            .is_some_and(|h| h.contains("scope directory")),
        "{:?}",
        d.help
    );
}

#[test]
fn role_binding_in_ontology_dir_is_v03() {
    let binding = "kind: RoleBinding\nrole: dispatcher\nsubjects:\n  - group: a\n";
    let out = ontoforge_schema::load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("bindings.yaml", binding),
    ]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V03);
    assert!(d.message.contains("RoleBinding"), "{}", d.message);
}

#[test]
fn object_type_in_scope_dir_is_v03() {
    let object = "kind: ObjectType\napiName: Airport\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\nbacking: editOnly\n";
    let out = load(&[("objects.yaml", object)]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V03);
    assert_eq!((d.file.as_str(), d.line, d.column), ("objects.yaml", 1, 7));
    assert!(d.message.contains("ontology directory"), "{}", d.message);
    assert!(
        d.help
            .as_deref()
            .is_some_and(|h| h.contains("ontology directory")),
        "{:?}",
        d.help
    );
}

#[test]
fn source_binding_in_scope_dir_is_v03() {
    let binding = "kind: SourceBinding\ntarget: { objectType: Flight }\nsource: { catalog: c, schema: s, table: t }\n";
    let out = load(&[("binding.yaml", binding)]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V03);
    assert!(d.message.contains("SourceBinding"), "{}", d.message);
}

#[test]
fn unknown_kind_in_scope_dir_suggests_a_scope_kind() {
    let out = load(&[("roles.yaml", "kind: Rol\napiName: x\n")]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V03);
    assert!(
        d.help.as_deref().is_some_and(|h| h.contains("`Role`")),
        "{:?}",
        d.help
    );
}

fn binding_with(subjects: &str) -> ScopeLoadOutput {
    let text = format!("kind: RoleBinding\nrole: dispatcher\nsubjects:{subjects}\n");
    load(&[("roles.yaml", ROLE), ("bindings.yaml", &text)])
}

#[test]
fn subject_rules() {
    // (subjects YAML after `subjects:`, expected line, expected column)
    let invalid = [
        (" []", 3, 11),
        ("\n  - group: a\n  - group: a", 5, 5),
        ("\n  - user: alice", 4, 11),
        ("\n  - user: \"a|b|c\"", 4, 11),
        ("\n  - user: \"|b\"", 4, 11),
        ("\n  - user: \"a|\"", 4, 11),
        ("\n  - group: \"\"", 4, 12),
    ];
    for (subjects, line, column) in invalid {
        let out = binding_with(subjects);
        let d = only(&out.diagnostics);
        assert_eq!(d.code, codes::V21, "{subjects:?}");
        assert_eq!(
            (d.file.as_str(), d.line, d.column),
            ("bindings.yaml", line, column),
            "{subjects:?}"
        );
        assert!(d.help.is_some(), "{subjects:?}");
        assert!(out.scope.is_none(), "{subjects:?}");
    }

    for subjects in [
        "\n  - group: \"https://idp.example.com|ops\"",
        "\n  - user: \"https://idp.example.com|abc\"\n  - group: ops",
    ] {
        let out = binding_with(subjects);
        assert!(
            out.diagnostics.is_empty(),
            "{subjects:?}: {:#?}",
            out.diagnostics
        );
        assert!(out.scope.is_some(), "{subjects:?}");
    }
}

#[test]
fn duplicate_role_is_v08() {
    let out = load(&[("a.yaml", ROLE), ("b.yaml", ROLE)]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V08);
    assert_eq!((d.file.as_str(), d.line, d.column), ("b.yaml", 2, 10));
    assert!(d.message.contains("a.yaml:2:10"), "{}", d.message);
}

#[test]
fn second_binding_for_role_is_v08() {
    let first = "kind: RoleBinding\nrole: dispatcher\nsubjects:\n  - group: a\n";
    let second = "kind: RoleBinding\nrole: dispatcher\nsubjects:\n  - group: b\n";
    let out = load(&[
        ("roles.yaml", ROLE),
        ("one.yaml", first),
        ("two.yaml", second),
    ]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V08);
    assert_eq!((d.file.as_str(), d.line, d.column), ("two.yaml", 2, 7));
    assert!(d.message.contains("one.yaml:2:7"), "{}", d.message);
}

#[test]
fn binding_may_precede_its_role_in_file_order() {
    let binding = "kind: RoleBinding\nrole: dispatcher\nsubjects:\n  - group: a\n";
    let out = load(&[("a-bindings.yaml", binding), ("z-roles.yaml", ROLE)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}

#[test]
fn empty_scope_directory_is_reported() {
    let out = load(&[("README.md", "not a scope file")]);
    let d = only(&out.diagnostics);
    assert_eq!(d.code, codes::V14);
    assert!(out.scope.is_none());
}

#[test]
fn scope_diagnostics() {
    let roles = "kind: Role\napiName: dispatcher\n---\nkind: Role\napiName: dispatcher\n---\nkind: Role\napiName: planner\nobjectTypes:\n  Flight:\n    properties: [flightId]\n    rowfilter: \"object.status == 'X'\"\n";
    let bindings = "kind: RoleBinding\nrole: ghost\nsubjects:\n  - group: a\n---\nkind: RoleBinding\nrole: dispatcher\nsubjects: []\n---\nkind: RoleBinding\nrole: planner\nsubjects:\n  - user: alice\n  - group: g\n  - group: g\n";
    let misplaced = "kind: ObjectType\napiName: Airport\n";
    let out = load(&[
        ("roles.yaml", roles),
        ("bindings.yaml", bindings),
        ("misplaced.yaml", misplaced),
    ]);
    assert!(out.scope.is_none());
    insta::assert_json_snapshot!("scope_diagnostics", out.diagnostics);
}
