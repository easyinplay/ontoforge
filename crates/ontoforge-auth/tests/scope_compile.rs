//! Scope compilation against the aviation example (plan 03-14, SCOPE-01 / 02 / 05).

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{CompiledScope, ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_expr::{AttrValue, Attrs, Scalar};
use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::{
    ActionTypeName, AttrType, Ontology, ProfileName, PropertyName, RoleName, SqlLevel, Toolset,
};
use ontoforge_schema::{load_dir, load_from_memory, load_scope_dir, load_scope_from_memory};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

fn aviation() -> Ontology {
    let out = load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

fn attrs() -> BTreeMap<String, AttrType> {
    let cfg: AuthConfig = serde_json::from_value(serde_json::json!({
        "principal_attrs": { "homeAirports": { "claim": "airports", "type": "string_list" } },
    }))
    .expect("auth config");
    attr_types(Some(&cfg))
}

fn dev() -> ProfileName {
    ProfileName::try_from("dev".to_owned()).expect("profile")
}

fn compile_example() -> CompiledScope {
    let ontology = aviation();
    let out = load_scope_dir(Utf8Path::new(AVIATION_SCOPE));
    let scope = out.scope.expect("scope loads");
    compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: Some(&dev()),
        attrs: &attrs(),
        scope: &scope,
        sources: Some(&out.sources),
    })
    .unwrap_or_else(|d| panic!("{d:#?}"))
}

fn principal(id: (&str, &str), groups: &[&str], airports: &[&str]) -> Principal {
    let mut attrs = Attrs::new();
    if !airports.is_empty() {
        let list = airports
            .iter()
            .map(|a| Scalar::String((*a).to_owned()))
            .collect();
        attrs
            .insert("homeAirports", AttrValue::List(list))
            .expect("attribute");
    }
    Principal {
        id: PrincipalId::new(id.0, id.1),
        issuer: id.0.to_owned(),
        subject: id.1.to_owned(),
        groups: groups.iter().map(|g| (*g).to_owned()).collect(),
        attrs,
    }
}

fn prop(name: &str) -> PropertyName {
    PropertyName::try_from(name.to_owned()).expect("property")
}

fn row(pairs: &[(&str, &str)]) -> BTreeMap<PropertyName, Scalar> {
    pairs
        .iter()
        .map(|(k, v)| (prop(k), Scalar::String((*v).to_owned())))
        .collect()
}

#[test]
fn aviation_scope_compiles() {
    let compiled = compile_example();
    let names: Vec<&str> = compiled.roles.keys().map(RoleName::as_str).collect();
    assert_eq!(names, ["crewScheduler", "dispatcher", "maintenanceLead"]);
    let flight = &compiled.roles["dispatcher"].types["Flight"];
    assert!(flight.row.is_some());
    assert_eq!(compiled.bindings.len(), 3);
}

#[test]
fn principal_with_two_roles_gets_merged_flight_scope() {
    let compiled = compile_example();
    let who = principal(
        ("ontoforge-dev", "alice"),
        &["dispatchers", "crew-schedulers"],
        &["FRA"],
    );
    let scope = compiled.for_principal(&who);

    let roles: Vec<&str> = scope.roles.iter().map(RoleName::as_str).collect();
    assert_eq!(roles, ["crewScheduler", "dispatcher"]);

    let seen = scope.types["Flight"].evaluate(
        &row(&[
            ("status", "SCHEDULED"),
            ("departureAirportCode", "MUC"),
            ("aircraftTailNumber", "D-ABCD"),
        ]),
        &who.attrs,
    );
    assert!(seen.visible, "dispatcher's filter holds for this row");
    let columns: BTreeSet<&str> = seen.columns.iter().map(PropertyName::as_str).collect();
    assert!(columns.contains("status"), "{columns:?}");
    assert!(
        !columns.contains("aircraftTailNumber"),
        "the crew scheduler's column must not show through the dispatcher's row: {columns:?}"
    );

    let actions: Vec<&str> = scope.actions.iter().map(ActionTypeName::as_str).collect();
    assert_eq!(actions, ["assignPilot", "delayFlight"]);
}

#[test]
fn principal_without_roles_sees_nothing() {
    let compiled = compile_example();
    let scope = compiled.for_principal(&principal(("ontoforge-dev", "nobody"), &[], &[]));
    assert!(scope.roles.is_empty());
    assert!(scope.types.is_empty());
    assert!(scope.actions.is_empty());
}

#[test]
fn user_subject_binding() {
    let compiled = compile_example();
    let scope = compiled.for_principal(&principal(("ontoforge-dev", "mia"), &[], &[]));
    let roles: Vec<&str> = scope.roles.iter().map(RoleName::as_str).collect();
    assert_eq!(roles, ["maintenanceLead"]);
    assert!(scope.sql_levels.contains(&SqlLevel::Raw));
    assert!(scope.toolsets.contains(&Toolset::SqlRaw));
    assert!(scope.types.contains_key("Aircraft"));
    assert!(!scope.types.contains_key("Flight"));
}

fn only_v22(diagnostics: &[Diagnostic]) -> &Diagnostic {
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, codes::V22);
    &diagnostics[0]
}

#[test]
fn bad_row_filter_is_v22_located() {
    let ontology = aviation();
    let roles = std::fs::read_to_string(format!("{AVIATION_SCOPE}/roles.yaml")).expect("roles");
    let bindings =
        std::fs::read_to_string(format!("{AVIATION_SCOPE}/bindings.yaml")).expect("bindings");
    let good = "object.status != 'CANCELLED' || object.status == null";
    assert!(
        roles.contains(good),
        "the example's dispatcher filter moved"
    );
    let roles = roles.replace(good, "object.status.size() > 0");
    let out = load_scope_from_memory(&[("roles.yaml", &roles), ("bindings.yaml", &bindings)]);
    let scope = out.scope.expect("scope loads");

    let diagnostics = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: Some(&dev()),
        attrs: &attrs(),
        scope: &scope,
        sources: Some(&out.sources),
    })
    .expect_err("an unsupported row filter must not compile");

    let d = only_v22(&diagnostics);
    assert_eq!(d.file, "roles.yaml");
    let filter_line = roles
        .lines()
        .position(|l| l.contains("object.status.size()"))
        .expect("filter line")
        + 1;
    assert_eq!(d.line as usize, filter_line);
    assert_eq!(d.column, 16, "the value, not the key");
    assert!(d.message.contains("in expression"), "{}", d.message);
    assert!(d.message.contains("dispatcher"), "{}", d.message);
}

// ---------------------------------------------------------------------------
// Task 2: reference rules (ONT-V21), collation (D-08), fail-closed and reporting.
// ---------------------------------------------------------------------------

fn compile_roles(
    ontology: &Ontology,
    profile: Option<&ProfileName>,
    yaml: &str,
    with_sources: bool,
) -> Result<CompiledScope, Vec<Diagnostic>> {
    let out = load_scope_from_memory(&[("roles.yaml", yaml)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let scope = out.scope.expect("scope loads");
    compile_scope(&ScopeInputs {
        ontology,
        profile,
        attrs: &attrs(),
        scope: &scope,
        sources: with_sources.then_some(&out.sources),
    })
}

/// A role `r` followed by `body`.
fn role(body: &str) -> String {
    format!("kind: Role\napiName: r\n{body}")
}

fn single(yaml: &str, code: &str, needle: &str) -> Diagnostic {
    let diagnostics = compile_roles(&aviation(), Some(&dev()), yaml, true)
        .expect_err("the role must not compile");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let d = diagnostics.into_iter().next().expect("one diagnostic");
    assert_eq!(d.code, code, "{d:#?}");
    assert!(d.message.contains(needle), "{needle}: {}", d.message);
    d
}

const FLIGHT_OK: &str = "objectTypes:\n  Flight:\n    properties: [flightId, status]\n";

#[test]
fn unknown_object_type_is_v21() {
    let yaml = role("objectTypes:\n  Flihgt:\n    properties: [flightId]\n");
    let d = single(&yaml, codes::V21, "unknown object type `Flihgt`");
    assert_eq!((d.line, d.column), (4, 3), "points at the type name");
}

#[test]
fn unknown_property_is_v21() {
    let yaml = role("objectTypes:\n  Flight:\n    properties: [flightId, nope]\n");
    single(&yaml, codes::V21, "unknown property `nope`");
}

#[test]
fn allow_list_without_primary_key_is_v21() {
    let yaml = role("objectTypes:\n  Flight:\n    properties: [status]\n");
    single(&yaml, codes::V21, "primary key `flightId`");
}

#[test]
fn duplicate_allow_list_entry_is_v21() {
    let yaml = role("objectTypes:\n  Flight:\n    properties: [flightId, status, status]\n");
    single(&yaml, codes::V21, "more than once");
}

#[test]
fn unknown_action_is_v21() {
    let yaml = role(&format!("{FLIGHT_OK}actions: [launchRocket]\n"));
    single(&yaml, codes::V21, "unknown action `launchRocket`");
}

#[test]
fn raw_toolset_without_raw_level_is_v21() {
    let yaml = role("toolsets: [sql:raw]\n");
    single(&yaml, codes::V21, "sql:raw");
}

#[test]
fn l2_without_raw_level_is_v21() {
    let yaml = role("l2:\n  catalogs: [demo]\n");
    single(&yaml, codes::V21, "sqlLevels");
}

#[test]
fn l2_schema_must_be_catalog_dot_schema() {
    let yaml = role("sqlLevels: [raw]\nl2:\n  schemas: [noDot]\n");
    single(&yaml, codes::V21, "`catalog.schema`");
}

#[test]
fn raw_with_l2_and_toolset_compiles() {
    let yaml = role(
        "sqlLevels: [raw]\nl2:\n  catalogs: [demo]\n  schemas: [demo.aviation]\ntoolsets: [sql:raw]\n",
    );
    compile_roles(&aviation(), Some(&dev()), &yaml, true).unwrap_or_else(|d| panic!("{d:#?}"));
}

#[test]
fn undeclared_attr_is_v22() {
    let yaml = role(
        "objectTypes:\n  Flight:\n    properties: [flightId]\n    rowFilter: \"object.status == principal.attrs.tenantId\"\n",
    );
    let d = single(&yaml, codes::V22, "tenantId");
    assert!(d.message.contains("in expression"), "{}", d.message);
}

const COLLATED: [(&str, &str); 3] = [
    ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
    (
        "objects.yaml",
        "kind: ObjectType\napiName: Airport\nbacking: source\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  name: { type: string }\n",
    ),
    (
        "bindings/dev/airport.yaml",
        "kind: SourceBinding\ntarget: { objectType: Airport }\nsource: { catalog: demo, schema: aviation, table: airports }\ncolumns:\n  code: { collation: caseInsensitive }\n",
    ),
];

const CODE_FILTER: &str = "kind: Role\napiName: r\nobjectTypes:\n  Airport:\n    properties: [code]\n    rowFilter: \"object.code == 'FRA'\"\n";

fn collated(default_profile: bool) -> Ontology {
    let mut files = COLLATED.to_vec();
    if !default_profile {
        files[0] = ("ontology.yaml", "apiName: demo\n");
    }
    let out = load_from_memory(&files);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("ontology")
}

#[test]
fn collation_case_insensitive_is_v22() {
    let ontology = collated(true);
    let diagnostics = compile_roles(&ontology, None, CODE_FILTER, true)
        .expect_err("a pushed-down comparison on a case-insensitive column is refused");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let d = &diagnostics[0];
    assert_eq!(d.code, codes::V22);
    assert!(d.message.contains("D-08"), "{}", d.message);
    assert!(d.message.contains("code"), "{}", d.message);
    assert_eq!((d.line, d.column), (6, 16), "points at the rowFilter value");
}

#[test]
fn collation_check_uses_the_selected_profile() {
    let ontology = collated(false);
    // No profile selected and no `defaultProfile`: the check is skipped.
    compile_roles(&ontology, None, CODE_FILTER, true).unwrap_or_else(|d| panic!("{d:#?}"));
    // An explicit profile brings it back.
    let d = compile_roles(&ontology, Some(&dev()), CODE_FILTER, true).expect_err("selected");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, codes::V22);
}

#[test]
fn empty_object_types_grants_nothing() {
    let yaml = "\
kind: Role
apiName: empty
objectTypes: {}
---
kind: RoleBinding
role: empty
subjects:
  - group: everyone
";
    let compiled =
        compile_roles(&aviation(), Some(&dev()), yaml, true).unwrap_or_else(|d| panic!("{d:#?}"));
    let scope = compiled.for_principal(&principal(("ontoforge-dev", "x"), &["everyone"], &[]));
    assert_eq!(scope.roles.len(), 1);
    assert!(scope.types.is_empty(), "an empty role grants no type");

    // An empty allow-list is a different thing: it cannot contain the primary key.
    let yaml = role("objectTypes:\n  Flight:\n    properties: []\n");
    single(&yaml, codes::V21, "primary key");
}

const SEVERAL_ERRORS: &str = "\
kind: Role
apiName: r
objectTypes:
  Flight:
    properties: [flightId, nope]
    rowFilter: \"object.status.size() > 0\"
actions: [launchRocket]
";

#[test]
fn all_errors_reported_together() {
    let diagnostics =
        compile_roles(&aviation(), Some(&dev()), SEVERAL_ERRORS, true).expect_err("three errors");
    let summary: Vec<(u32, &str)> = diagnostics
        .iter()
        .map(|d| (d.line, d.code.as_str()))
        .collect();
    assert_eq!(
        summary,
        [(5, "ONT-V21"), (6, "ONT-V22"), (7, "ONT-V21")],
        "{diagnostics:#?}"
    );
}

#[test]
fn stored_roles_diagnostics_without_sources() {
    let diagnostics = compile_roles(&aviation(), Some(&dev()), SEVERAL_ERRORS, false)
        .expect_err("errors without positions");
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    for d in &diagnostics {
        assert_eq!((d.file.as_str(), d.line, d.column), ("", 0, 0), "{d:#?}");
        assert!(d.message.contains("`r`"), "names the role: {}", d.message);
    }
    let all: String = diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(all.contains("Flight"), "names the type: {all}");
    assert!(all.contains("nope"), "names the property: {all}");
}

#[test]
fn scope_compile_diagnostics_json() {
    let corpus = [
        role("objectTypes:\n  Flihgt:\n    properties: [flightId]\n"),
        role("objectTypes:\n  Flight:\n    properties: [status, status, nope]\n"),
        role("toolsets: [sql:raw]\nl2:\n  schemas: [noDot]\n"),
        SEVERAL_ERRORS.to_owned(),
        role(
            "objectTypes:\n  Flight:\n    properties: [flightId, status]\n    rowFilter: \"object.status == principal.attrs.tenantId\"\n",
        ),
    ];
    let mut all = Vec::new();
    for yaml in &corpus {
        all.extend(
            compile_roles(&aviation(), Some(&dev()), yaml, true).expect_err("each entry fails"),
        );
    }
    insta::assert_json_snapshot!("scope_compile_diagnostics_json", all);
}
