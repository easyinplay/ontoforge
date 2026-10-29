//! Assets embedded into the single-binary development server.

use camino::{Utf8Path, Utf8PathBuf};

pub(crate) struct Asset {
    pub(crate) path: &'static str,
    pub(crate) contents: &'static str,
}

pub(crate) const ASSETS: &[Asset] = &[
    Asset {
        path: "ontology/ontology.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/ontology.yaml"
        )),
    },
    Asset {
        path: "ontology/objects.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/objects.yaml"
        )),
    },
    Asset {
        path: "ontology/actions.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/actions.yaml"
        )),
    },
    Asset {
        path: "ontology/links.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/links.yaml"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/aircraft.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/aircraft.yaml"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/flight.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/flight.yaml"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/flight-crew.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/flight-crew.yaml"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/maintenance-record.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/maintenance-record.yaml"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/pilot.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/pilot.yaml"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/fixtures/aircraft.csv",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/fixtures/flight_crew.csv",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/fixtures/flights.csv",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/fixtures/flights.csv"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/fixtures/maintenance_records.csv",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv"
        )),
    },
    Asset {
        path: "ontology/bindings/dev/fixtures/pilots.csv",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv"
        )),
    },
    Asset {
        path: "scope/roles.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/scopes/aviation/roles.yaml"
        )),
    },
    Asset {
        path: "scope/bindings.yaml",
        contents: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/scopes/aviation/bindings.yaml"
        )),
    },
];

pub(crate) fn write_to(root: &Utf8Path) -> std::io::Result<()> {
    for asset in ASSETS {
        let path = root.join(asset.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            std::fs::write(path, asset.contents)?;
        }
    }
    Ok(())
}

pub(crate) fn ontology_root(root: &Utf8Path) -> Utf8PathBuf {
    root.join("ontology")
}

pub(crate) fn scope_root(root: &Utf8Path) -> Utf8PathBuf {
    root.join("scope")
}
