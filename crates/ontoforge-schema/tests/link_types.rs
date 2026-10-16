//! Link types through the public in-memory API: parsing (SCHEMA-04) and the
//! structural checks V09, V11 and V08 for link ends.

use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::{LoadOutput, load_from_memory};
use serde_json::Value;

const MANIFEST: &str = "apiName: demo\n";

const OBJECTS: &str = "kind: ObjectType
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
  departureAirportCode: { type: string }
  status: { type: string }
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
apiName: Aircraft
primaryKey: tailNumber
properties:
  tailNumber: { type: string, required: true }
backing: editOnly
---
kind: ObjectType
apiName: MaintenanceRecord
primaryKey: recordId
properties:
  recordId: { type: string, required: true }
  aircraftTailNumber: { type: string }
backing: editOnly
";

const ONE_TO_MANY: &str = "kind: LinkType
apiName: flightDepartureAirport
cardinality: oneToMany
from: { objectType: Airport, apiName: departingFlights }
to: { objectType: Flight, apiName: departureAirport, foreignKey: departureAirportCode }
";

fn load(links: &str) -> LoadOutput {
    load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", OBJECTS),
        ("links.yaml", links),
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

#[test]
fn parses_all_three_cardinalities() {
    let out = load(
        "kind: LinkType
apiName: aircraftCurrentMaintenance
displayName: Current maintenance
cardinality: oneToOne
from: { objectType: Aircraft, apiName: currentMaintenanceRecord }
to: { objectType: MaintenanceRecord, apiName: aircraft, foreignKey: aircraftTailNumber, displayName: Aircraft }
---
kind: LinkType
apiName: flightDepartureAirport
cardinality: oneToMany
from: { objectType: Airport, apiName: departingFlights, description: Flights leaving here. }
to: { objectType: Flight, apiName: departureAirport, foreignKey: departureAirportCode }
---
kind: LinkType
apiName: flightCrew
cardinality: manyToMany
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: assignedFlights }
backing: editOnly
",
    );
    let json = ontology_json(&out);
    let links = json["linkTypes"].as_object().expect("linkTypes map");
    assert_eq!(links.len(), 3, "{links:#?}");

    let one = &links["aircraftCurrentMaintenance"];
    assert_eq!(one["cardinality"], "oneToOne");
    assert_eq!(one["displayName"], "Current maintenance");
    assert_eq!(one["from"]["objectType"], "Aircraft");
    assert_eq!(one["from"]["apiName"], "currentMaintenanceRecord");
    assert_eq!(one["to"]["foreignKey"], "aircraftTailNumber");
    assert_eq!(one["to"]["displayName"], "Aircraft");

    let many = &links["flightDepartureAirport"];
    assert_eq!(many["cardinality"], "oneToMany");
    assert_eq!(many["from"]["description"], "Flights leaving here.");
    assert_eq!(many["to"]["apiName"], "departureAirport");

    let join = &links["flightCrew"];
    assert_eq!(join["cardinality"], "manyToMany");
    assert!(join["to"]["foreignKey"].is_null());
}

#[test]
fn unknown_cardinality_is_v06() {
    let out = load(&ONE_TO_MANY.replace("oneToMany", "oneToMan"));
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!((d.file.as_str(), d.line, d.column), ("links.yaml", 3, 14));
    assert_eq!(
        d.help.as_deref(),
        Some("did you mean `oneToMany`?"),
        "{d:?}"
    );
}

#[test]
fn link_end_unknown_field_is_v04() {
    let out = load(&ONE_TO_MANY.replace("foreignKey:", "foreignkey:"));
    let d = only(&out);
    assert_eq!(d.code, codes::V04);
    assert_eq!((d.line, d.column), (5, 54));
    assert_eq!(
        d.help.as_deref(),
        Some("did you mean `foreignKey`?"),
        "{d:?}"
    );
}

#[test]
fn duplicate_link_type_is_v08() {
    let out = load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", OBJECTS),
        ("a.yaml", ONE_TO_MANY),
        ("b.yaml", ONE_TO_MANY),
    ]);
    let d = only(&out);
    assert_eq!(d.code, codes::V08);
    assert_eq!(d.file, "b.yaml");
    assert!(d.message.contains("LinkType"), "{}", d.message);
    assert!(d.message.contains("a.yaml:2:10"), "{}", d.message);
}

#[test]
fn link_type_api_name_must_be_camel_case() {
    let out = load(&ONE_TO_MANY.replace("flightDepartureAirport", "FlightCrew"));
    let d = only(&out);
    assert_eq!(d.code, codes::V07);
    assert_eq!((d.line, d.column), (2, 10));
    assert_eq!(d.help.as_deref(), Some("write it as `flightCrew`"), "{d:?}");
}

#[test]
fn link_ir_is_typed() {
    use ontoforge_schema::ir::Cardinality;
    let out = load(ONE_TO_MANY);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let link = &ontology.link_types["flightDepartureAirport"];
    assert_eq!(link.cardinality, Cardinality::OneToMany);
    assert_eq!(link.from.object_type.as_str(), "Airport");
    assert_eq!(link.to.api_name.as_str(), "departureAirport");
    assert_eq!(
        link.to
            .foreign_key
            .as_ref()
            .map(ontoforge_schema::ir::PropertyName::as_str),
        Some("departureAirportCode")
    );
    assert!(link.from.foreign_key.is_none());
}

// ---- structural validation: V09, V11, V08 --------------------------------

#[test]
fn unknown_end_object_type_is_v09() {
    let out = load(&ONE_TO_MANY.replace("objectType: Flight", "objectType: Flght"));
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert_eq!((d.file.as_str(), d.line, d.column), ("links.yaml", 5, 19));
    assert!(d.message.contains("Flght"), "{}", d.message);
    assert_eq!(d.help.as_deref(), Some("did you mean `Flight`?"), "{d:?}");
}

#[test]
fn unknown_foreign_key_property_is_v09() {
    let out = load(&ONE_TO_MANY.replace("departureAirportCode", "departureAirportCod"));
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert_eq!(d.line, 5);
    assert!(d.message.contains("Flight"), "{}", d.message);
    assert_eq!(
        d.help.as_deref(),
        Some("did you mean `departureAirportCode`?"),
        "{d:?}"
    );
}

#[test]
fn many_to_many_with_foreign_key_is_v11() {
    let out = load(
        "kind: LinkType
apiName: flightCrew
cardinality: manyToMany
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: assignedFlights, foreignKey: employeeId }
backing: editOnly
",
    );
    let d = only(&out);
    assert_eq!(d.code, codes::V11);
    assert_eq!((d.line, d.column), (5, 64));
    assert!(d.message.contains("manyToMany"), "{}", d.message);
}

#[test]
fn one_to_many_without_foreign_key_is_v11() {
    let out = load(&ONE_TO_MANY.replace(", foreignKey: departureAirportCode", ""));
    let d = only(&out);
    assert_eq!(d.code, codes::V11);
    assert_eq!((d.line, d.column), (3, 14));
    assert!(d.message.contains("foreignKey"), "{}", d.message);
}

#[test]
fn foreign_key_on_from_end_is_v11() {
    let out = load(&ONE_TO_MANY.replace(
        "apiName: departingFlights",
        "apiName: departingFlights, foreignKey: code",
    ));
    let d = only(&out);
    assert_eq!(d.code, codes::V11);
    assert_eq!((d.line, d.column), (4, 69));
    assert!(d.help.as_deref().unwrap().contains("`to`"), "{d:?}");
}

#[test]
fn foreign_key_type_must_match_primary_key() {
    let objects = OBJECTS.replace(
        "departureAirportCode: { type: string }",
        "departureAirportCode: { type: integer }",
    );
    let out = load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", &objects),
        ("links.yaml", ONE_TO_MANY),
    ]);
    let d = only(&out);
    assert_eq!(d.code, codes::V11);
    assert_eq!((d.line, d.column), (5, 66));
    for needle in ["integer", "string", "Airport.code", "departureAirportCode"] {
        assert!(d.message.contains(needle), "{needle}: {}", d.message);
    }
}

#[test]
fn end_name_clashes_with_property_is_v08() {
    let out = load(&ONE_TO_MANY.replace("apiName: departureAirport,", "apiName: status,"));
    let d = only(&out);
    assert_eq!(d.code, codes::V08);
    assert_eq!((d.line, d.column), (5, 36));
    assert!(d.message.contains("status"), "{}", d.message);
    assert!(d.message.contains("Flight"), "{}", d.message);
}

#[test]
fn duplicate_end_names_on_same_type_is_v08() {
    let objects = OBJECTS.replace(
        "departureAirportCode: { type: string }",
        "departureAirportCode: { type: string }\n  arrivalAirportCode: { type: string }",
    );
    let links = format!(
        "{}---\n{}",
        ONE_TO_MANY.replace("apiName: departureAirport,", "apiName: airport,"),
        ONE_TO_MANY
            .replace("flightDepartureAirport", "flightArrivalAirport")
            .replace("apiName: departureAirport,", "apiName: airport,")
            .replace("apiName: departingFlights", "apiName: arrivingFlights")
            .replace("departureAirportCode", "arrivalAirportCode"),
    );
    let out = load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", &objects),
        ("links.yaml", &links),
    ]);
    let d = only(&out);
    assert_eq!(d.code, codes::V08);
    // Links are visited in apiName order, so the later one, flightDepartureAirport
    // (the first document of the file), is the one reported.
    assert_eq!((d.line, d.column), (5, 36));
    assert!(d.message.contains("flightArrivalAirport"), "{}", d.message);
    assert!(d.message.contains("`airport`"), "{}", d.message);
}

const EMPLOYEE: &str = "kind: ObjectType
apiName: Employee
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
  managerId: { type: string }
backing: editOnly
";

fn self_link(from_end: &str, to_end: &str) -> LoadOutput {
    let objects = format!("{OBJECTS}---\n{EMPLOYEE}");
    let links = format!(
        "kind: LinkType
apiName: reporting
cardinality: oneToMany
from: {{ objectType: Employee, apiName: {from_end} }}
to: {{ objectType: Employee, apiName: {to_end}, foreignKey: managerId }}
"
    );
    load_from_memory(&[
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", &objects),
        ("links.yaml", &links),
    ])
}

#[test]
fn self_link_ends_must_differ() {
    let out = self_link("reports", "reports");
    let d = only(&out);
    assert_eq!(d.code, codes::V08);
    assert_eq!((d.line, d.column), (5, 38));
    assert!(d.message.contains("Employee"), "{}", d.message);
}

#[test]
fn self_link_with_distinct_ends_is_valid() {
    let out = self_link("reports", "manager");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}

#[test]
fn link_checks_are_skipped_for_missing_types() {
    // `status` would clash with a Flight property and the foreign key is absent,
    // but with no Flight there is nothing to compare against.
    let out = load(
        "kind: LinkType
apiName: flightDepartureAirport
cardinality: oneToMany
from: { objectType: Airport, apiName: departingFlights }
to: { objectType: Flght, apiName: status }
",
    );
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert_eq!(d.line, 5);
}

// ---- backing (D-02) --------------------------------------------------------

const MANY_TO_MANY: &str = "kind: LinkType
apiName: flightCrew
cardinality: manyToMany
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: assignedFlights }
";

#[test]
fn many_to_many_without_backing_is_v05() {
    let out = load(MANY_TO_MANY);
    let d = only(&out);
    assert_eq!(d.code, codes::V05);
    assert!(d.message.contains("flightCrew"), "{}", d.message);
    assert!(d.message.contains("`backing`"), "{}", d.message);
    // Located on the `cardinality` value.
    assert_eq!((d.file.as_str(), d.line, d.column), ("links.yaml", 3, 14));
    assert_eq!(
        d.help.as_deref(),
        Some("add `backing: source` or `backing: editOnly`"),
        "{d:?}"
    );
}

#[test]
fn many_to_many_with_backing_loads() {
    let out = load(&format!("{MANY_TO_MANY}backing: source\n"));
    let json = ontology_json(&out);
    assert_eq!(json["linkTypes"]["flightCrew"]["backing"], "source");
}

#[test]
fn one_to_many_with_backing_is_v11() {
    let out = load(&format!("{ONE_TO_MANY}backing: source\n"));
    let d = only(&out);
    assert_eq!(d.code, codes::V11);
    // Located on the `backing` key.
    assert_eq!((d.file.as_str(), d.line, d.column), ("links.yaml", 6, 1));
    assert!(
        d.message.contains("flightDepartureAirport"),
        "{}",
        d.message
    );
    assert!(d.help.as_deref().unwrap().contains("`to`"), "{d:?}");
}

#[test]
fn one_to_one_with_backing_is_v11() {
    let out = load(
        "kind: LinkType
apiName: aircraftCurrentMaintenance
cardinality: oneToOne
from: { objectType: Aircraft, apiName: currentMaintenanceRecord }
to: { objectType: MaintenanceRecord, apiName: aircraft, foreignKey: aircraftTailNumber }
backing: editOnly
",
    );
    let d = only(&out);
    assert_eq!(d.code, codes::V11);
    assert_eq!((d.line, d.column), (6, 1));
}
