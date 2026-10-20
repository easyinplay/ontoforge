//! Scope compilation against the aviation example (plan 03-14, SCOPE-01 / 02 / 05).

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{CompiledScope, ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_expr::{AttrValue, Attrs, Scalar};
use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::{AttrType, Ontology, ProfileName, PropertyName, SqlLevel, Toolset};
use ontoforge_schema::{load_dir, load_scope_dir, load_scope_from_memory};

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
    let names: Vec<&str> = compiled.roles.keys().map(|r| r.as_str()).collect();
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

    let roles: Vec<&str> = scope.roles.iter().map(|r| r.as_str()).collect();
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

    let actions: Vec<&str> = scope.actions.iter().map(|a| a.as_str()).collect();
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
    let roles: Vec<&str> = scope.roles.iter().map(|r| r.as_str()).collect();
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
