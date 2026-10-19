//! Data-driven diagnostics: every directory under `examples/ontologies/invalid/`
//! is named `<code>-<slug>` (for example `v04-unknown-field`), must produce that
//! code, and has its full diagnostic list snapshotted.

use camino::Utf8PathBuf;

#[test]
fn invalid_examples_produce_their_code_and_a_stable_snapshot() {
    let root = Utf8PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/ontologies/invalid"
    ));
    let mut names: Vec<String> = std::fs::read_dir(&root)
        .expect("invalid examples directory")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().into_string().expect("utf-8 directory name"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no invalid examples found");

    for name in names {
        let (prefix, _) = name
            .split_once('-')
            .expect("directory name is <code>-<slug>");
        let expected = format!("ONT-{}", prefix.to_uppercase());
        let out = ontoforge::pipeline::check(&root.join(&name));
        assert!(
            out.diagnostics.iter().any(|d| d.code == expected),
            "{name}: expected {expected}, got {:?}",
            out.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
        );
        insta::assert_json_snapshot!(name.as_str(), out.diagnostics);
    }
}
