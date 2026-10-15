//! The shipped examples must stay clean under every registered lint rule with the
//! default configuration. New rules are covered automatically.

use camino::Utf8PathBuf;
use ontoforge_schema::lint::{LintConfig, run};
use ontoforge_schema::load_dir;

fn example(name: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(format!(
        "{}/../../examples/ontologies/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

#[test]
fn aviation_lints_clean() {
    for name in ["aviation", "aviation-json"] {
        let out = load_dir(&example(name));
        let ontology = out.ontology.expect("example validates");
        let found = run(&ontology, &out.sources, &LintConfig::default());
        assert!(found.is_empty(), "{name}: {found:#?}");
    }
}
