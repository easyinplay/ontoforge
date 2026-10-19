//! The aviation example exists twice, as YAML and as JSON; both must load to the
//! same IR. Directories are loaded as a whole so later plans can add files to
//! both without touching this test.

use camino::Utf8PathBuf;
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, ProfileName};
use ontoforge_schema::{LoadOutput, load_dir, load_from_memory};

fn example(name: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(format!(
        "{}/../../examples/ontologies/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn load_example(name: &str) -> LoadOutput {
    load_dir(&example(name))
}

#[test]
fn aviation_yaml_and_json_produce_identical_ir() {
    let yaml = load_example("aviation");
    let json = load_example("aviation-json");
    assert!(yaml.ontology.is_some(), "{:#?}", yaml.diagnostics);
    assert!(json.ontology.is_some(), "{:#?}", json.diagnostics);
    assert_eq!(yaml.ontology, json.ontology);
    let ontology = yaml.ontology.expect("checked above");
    assert_eq!(ontology.object_types.len(), 5);
    assert_eq!(ontology.link_types.len(), 5);
    assert_eq!(ontology.action_types.len(), 3);
}

#[test]
fn aviation_examples_have_no_diagnostics() {
    for name in ["aviation", "aviation-json"] {
        let out = load_example(name);
        assert!(out.diagnostics.is_empty(), "{name}: {:#?}", out.diagnostics);
    }
}

#[test]
fn aviation_file_order_does_not_matter() {
    let root = example("aviation");
    let mut files: Vec<(String, String)> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let rel = e
                .path()
                .strip_prefix(&root)
                .expect("inside root")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            let text = std::fs::read_to_string(e.path()).expect("utf-8 example file");
            (rel, text)
        })
        .collect();
    files.sort();
    files.reverse();
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let from_memory = load_from_memory(&borrowed);
    assert!(
        from_memory.diagnostics.is_empty(),
        "{:#?}",
        from_memory.diagnostics
    );
    assert_eq!(from_memory.ontology, load_example("aviation").ontology);
}

#[test]
fn aviation_dev_profile_binds_flight() {
    for name in ["aviation", "aviation-json"] {
        let ontology = load_example(name).ontology.expect("ontology");
        assert_eq!(
            ontology.default_profile.as_ref().map(ProfileName::as_str),
            Some("dev"),
            "{name}"
        );
        let flight = &ontology.bindings["dev"].objects["Flight"];
        assert_eq!(flight.source.table, "flights", "{name}");
        assert_eq!(flight.column_for("flightNumber"), "flight_no", "{name}");
        assert_eq!(flight.column_for("tags"), "tags", "{name}");
        assert_eq!(
            flight.timezone_for("scheduledDeparture"),
            Some("Europe/Berlin"),
            "{name}"
        );
    }
}

#[test]
fn aviation_dev_profile_binds_every_source_type() {
    for name in ["aviation", "aviation-json"] {
        let ontology = load_example(name).ontology.expect("ontology");
        let dev = &ontology.bindings["dev"];
        let objects: Vec<&str> = dev.objects.keys().map(ObjectTypeName::as_str).collect();
        assert_eq!(
            objects,
            ["Aircraft", "Flight", "MaintenanceRecord", "Pilot"],
            "{name}"
        );
        let links: Vec<&str> = dev.links.keys().map(LinkTypeName::as_str).collect();
        assert_eq!(links, ["flightCrew"], "{name}");
        // Each object binding leaves at least one property to the same-name default.
        for (object, binding) in &dev.objects {
            let columns = ontology.bound_columns(binding);
            assert!(
                columns.iter().any(|c| !c.explicit),
                "{name}: {object} has no property bound by the default"
            );
        }
        let crew = ontology.bound_columns(&dev.links["flightCrew"]);
        let pairs: Vec<(&str, &str)> = crew.iter().map(|c| (c.key, c.column)).collect();
        assert_eq!(pairs, [("from", "flight_id"), ("to", "pilot_id")], "{name}");
        let performed_at = &dev.objects["MaintenanceRecord"];
        assert_eq!(
            performed_at.timezone_for("performedAt"),
            Some("aware"),
            "{name}"
        );
    }
}

#[test]
fn aviation_cost_property() {
    for name in ["aviation", "aviation-json"] {
        let ontology = load_example(name).ontology.expect("ontology");
        let cost = &ontology.object_types["MaintenanceRecord"].properties["cost"];
        assert_eq!(
            cost.base_type,
            ontoforge_schema::ir::BaseType::Decimal,
            "{name}"
        );
        assert_eq!(cost.effective_decimal(), (12, 2), "{name}");
        // The dev fixture already carries the column, bound by the default mapping.
        let binding = &ontology.bindings["dev"].objects["MaintenanceRecord"];
        let columns = ontology.bound_columns(binding);
        let cost_column = columns
            .iter()
            .find(|c| c.key == "cost")
            .expect("cost column");
        assert_eq!(
            (cost_column.column, cost_column.explicit),
            ("cost", false),
            "{name}"
        );
    }
}
