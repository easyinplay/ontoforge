//! Loader boundaries: discovery, include globs, path safety, file limits, and
//! (further down) parser strictness, stack safety and position accuracy.

use camino::Utf8Path;
use ontoforge_schema::diag::codes;
use ontoforge_schema::{LoadOutput, load_dir, load_from_memory};

fn object(name: &str) -> String {
    format!(
        "kind: ObjectType\napiName: {name}\nprimaryKey: code\nproperties:\n  code: {{ type: string, required: true }}\n"
    )
}

fn write(dir: &tempfile::TempDir, rel: &str, content: impl AsRef<[u8]>) {
    let path = dir.path().join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn load(dir: &tempfile::TempDir) -> LoadOutput {
    load_dir(Utf8Path::from_path(dir.path()).expect("utf-8 temp dir"))
}

fn codes_of(out: &LoadOutput) -> Vec<&str> {
    out.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn names(out: &LoadOutput) -> Vec<String> {
    out.ontology
        .as_ref()
        .expect("ontology loaded")
        .object_types
        .keys()
        .map(ToString::to_string)
        .collect()
}

// ---------------------------------------------------------------- discovery

#[test]
fn nested_files_are_merged() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(&dir, "objects/airport.yaml", object("Airport"));
    write(&dir, "more/deep/flight.yaml", object("Flight"));
    let out = load(&dir);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(names(&out), ["Airport", "Flight"]);
}

#[test]
fn dot_dirs_and_build_dirs_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(&dir, "airport.yaml", object("Airport"));
    write(&dir, ".github/workflows/ci.yml", "name: ci\non: push\n");
    write(&dir, "target/x.yaml", "a: 1\n");
    write(&dir, "node_modules/p/package.json", "{\"name\": \"p\"}");
    let out = load(&dir);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(names(&out), ["Airport"]);
}

#[test]
fn manifest_and_lint_config_are_not_definitions() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(&dir, "ontoforge.toml", "[lint]\n");
    let out = load(&dir);
    assert_eq!(codes_of(&out), [codes::V14], "{:#?}", out.diagnostics);
    assert!(out.diagnostics[0].message.contains("no definition files"));
    assert!(out.ontology.is_none());
}

#[test]
fn custom_include_restricts_files() {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir,
        "ontology.yaml",
        "apiName: demo\ninclude: [\"objects/**/*.yaml\"]\n",
    );
    write(&dir, "objects/airport.yaml", object("Airport"));
    write(&dir, "drafts/x.yaml", "kind: Nope\n");
    let out = load(&dir);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(names(&out), ["Airport"]);
}

fn memory_with_include(include: &str) -> LoadOutput {
    let manifest = format!("apiName: demo\ninclude: [{include}]\n");
    load_from_memory(&[
        ("ontology.yaml", manifest.as_str()),
        ("a.yaml", "kind: Nope\n"),
    ])
}

#[test]
fn include_with_parent_segment_is_rejected() {
    let out = memory_with_include("\"../**/*.yaml\"");
    let d = &out.diagnostics[0];
    assert_eq!(d.code, codes::V14, "{:#?}", out.diagnostics);
    assert_eq!(
        (d.file.as_str(), d.line, d.column),
        ("ontology.yaml", 2, 11)
    );
    assert!(d.message.contains("escapes"), "{}", d.message);
    assert_eq!(out.diagnostics.len(), 1, "{:#?}", out.diagnostics);
}

#[test]
fn absolute_include_is_rejected() {
    for pattern in ["/etc/*.yaml", "C:/x/*.yaml", "\\\\srv\\share\\*.yaml"] {
        let quoted = format!("'{pattern}'");
        let out = memory_with_include(&quoted);
        let d = &out.diagnostics[0];
        assert_eq!(d.code, codes::V14, "{pattern}: {:#?}", out.diagnostics);
        assert_eq!((d.line, d.column), (2, 11), "{pattern}");
        assert!(out.ontology.is_none(), "{pattern}");
    }
}

#[test]
fn invalid_glob_is_rejected() {
    let out = memory_with_include("\"[\"");
    let d = &out.diagnostics[0];
    assert_eq!(d.code, codes::V14, "{:#?}", out.diagnostics);
    assert_eq!((d.line, d.column), (2, 11));
}

#[test]
fn oversized_file_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(&dir, "good.yaml", object("Airport"));
    write(&dir, "big.yaml", vec![b'#'; 4 * 1024 * 1024 + 1]);
    let out = load(&dir);
    assert_eq!(out.diagnostics.len(), 1, "{:#?}", out.diagnostics);
    let d = &out.diagnostics[0];
    assert_eq!((d.code.as_str(), d.file.as_str()), (codes::V14, "big.yaml"));
    assert!(
        out.sources.text("good.yaml").is_some(),
        "good file still loaded"
    );
}

#[test]
fn oversized_in_memory_file_is_rejected() {
    let big = "#".repeat(4 * 1024 * 1024 + 1);
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        ("big.yaml", big.as_str()),
    ]);
    assert_eq!(out.diagnostics.len(), 1, "{:#?}", out.diagnostics);
    let d = &out.diagnostics[0];
    assert_eq!((d.code.as_str(), d.file.as_str()), (codes::V14, "big.yaml"));
}

#[test]
fn non_utf8_file_is_v01() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(&dir, "bad.yaml", [0xff_u8, 0xfe, 0x00]);
    let out = load(&dir);
    let d = &out.diagnostics[0];
    assert_eq!((d.code.as_str(), d.file.as_str()), (codes::V01, "bad.yaml"));
}

#[cfg(unix)]
fn symlink_file(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(from, to)
}

#[cfg(windows)]
fn symlink_file(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(from, to)
}

#[test]
fn symlinked_file_is_not_followed() {
    let outside = tempfile::tempdir().unwrap();
    write(&outside, "secret.yaml", object("Secret"));
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(&dir, "airport.yaml", object("Airport"));
    if symlink_file(
        &outside.path().join("secret.yaml"),
        &dir.path().join("link.yaml"),
    )
    .is_err()
    {
        return; // no permission to create symlinks on this machine
    }
    let out = load(&dir);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(names(&out), ["Airport"]);
}

#[test]
fn diagnostic_paths_use_forward_slash() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(
        &dir,
        "sub/dir/bad.yaml",
        "kind: ObjectType\napiName: Bad\nprimaryKy: x\n",
    );
    let out = load(&dir);
    let d = out
        .diagnostics
        .iter()
        .find(|d| d.code == codes::V04)
        .expect("V04");
    assert_eq!(d.file, "sub/dir/bad.yaml");
}

#[test]
fn load_from_memory_applies_same_include_filter() {
    let good = object("Airport");
    let out = load_from_memory(&[
        (
            "ontology.yaml",
            "apiName: demo\ninclude: [\"objects/**/*.yaml\"]\n",
        ),
        ("objects/airport.yaml", good.as_str()),
        ("objects\\win.yaml", "kind: Nope\n"),
        ("drafts/x.yaml", "kind: Nope\n"),
        (".github/workflows/ci.yml", "a: 1\n"),
    ]);
    // `objects\win.yaml` is normalised to `objects/win.yaml` and therefore loaded.
    assert_eq!(codes_of(&out), [codes::V03], "{:#?}", out.diagnostics);
    assert_eq!(out.diagnostics[0].file, "objects/win.yaml");
}
