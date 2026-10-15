//! The aviation example exists twice, as YAML and as JSON; both must load to the
//! same IR. Directories are loaded as a whole so later plans can add files to
//! both without touching this test.

use camino::Utf8PathBuf;
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
