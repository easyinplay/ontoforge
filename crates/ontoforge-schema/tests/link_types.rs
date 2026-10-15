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
---
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  departureAirportCode: { type: string }
  status: { type: string }
---
kind: ObjectType
apiName: Pilot
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
---
kind: ObjectType
apiName: Aircraft
primaryKey: tailNumber
properties:
  tailNumber: { type: string, required: true }
---
kind: ObjectType
apiName: MaintenanceRecord
primaryKey: recordId
properties:
  recordId: { type: string, required: true }
  aircraftTailNumber: { type: string }
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
