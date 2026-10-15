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

/// WR-07: the definition files together have a byte budget.
#[test]
fn total_size_budget_stops_reading() {
    use ontoforge_schema::loader::{MAX_FILE_BYTES, MAX_TOTAL_BYTES};
    let len = usize::try_from(MAX_FILE_BYTES).unwrap();
    let full = format!("#{}\n", "a".repeat(len - 2));
    let count = usize::try_from(MAX_TOTAL_BYTES / MAX_FILE_BYTES).unwrap() + 1;
    let names: Vec<String> = (0..count).map(|i| format!("f{i:03}.yaml")).collect();
    let mut files = vec![("ontology.yaml", "apiName: demo\n")];
    files.extend(names.iter().map(|n| (n.as_str(), full.as_str())));
    let out = load_from_memory(&files);
    assert_eq!(codes_of(&out), [codes::V14], "{:#?}", out.diagnostics);
    let d = &out.diagnostics[0];
    assert!(d.message.contains("byte budget"), "{}", d.message);
    assert_eq!(d.file, names[count - 1]);
    assert!(out.ontology.is_none());
}

/// WR-07: and a file count budget.
#[test]
fn file_count_budget_stops_reading() {
    use ontoforge_schema::loader::MAX_FILES;
    let names: Vec<String> = (0..=MAX_FILES).map(|i| format!("f{i:05}.yaml")).collect();
    let mut files = vec![("ontology.yaml", "apiName: demo\n")];
    files.extend(names.iter().map(|n| (n.as_str(), "# empty\n")));
    let out = load_from_memory(&files);
    assert_eq!(codes_of(&out), [codes::V14], "{:#?}", out.diagnostics);
    assert!(
        out.diagnostics[0].message.contains("more than"),
        "{}",
        out.diagnostics[0].message
    );
    assert!(out.ontology.is_none());
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

/// A file or directory name that is not valid Unicode.
#[cfg(unix)]
fn non_utf8_name(stem: &str) -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    let mut bytes = stem.as_bytes().to_vec();
    bytes.push(0xff);
    std::ffi::OsString::from_vec(bytes)
}

#[cfg(windows)]
fn non_utf8_name(stem: &str) -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt;
    let mut wide: Vec<u16> = stem.encode_utf16().collect();
    wide.push(0xd800); // an unpaired surrogate
    std::ffi::OsString::from_wide(&wide)
}

/// WR-12: names that are not UTF-8 are reported, not silently dropped.
#[test]
fn non_utf8_file_and_directory_names_are_v14() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "ontology.yaml", "apiName: demo\n");
    write(&dir, "airport.yaml", object("Airport"));
    let mut file = non_utf8_name("bad");
    file.push(".yaml");
    let sub = dir.path().join(non_utf8_name("sub"));
    let made = std::fs::write(dir.path().join(&file), object("Lost"))
        .and_then(|()| std::fs::create_dir(&sub))
        .and_then(|()| std::fs::write(sub.join("deep.yaml"), object("Deep")));
    if made.is_err() {
        return; // this file system only stores Unicode names
    }
    let out = load(&dir);
    let messages: Vec<&str> = out
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::V14)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(messages.len(), 2, "{:#?}", out.diagnostics);
    assert!(
        messages.iter().all(|m| m.contains("not valid UTF-8")),
        "{messages:#?}"
    );
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

/// CR-03: the manifest is read directly, not through the walk, so it checks links
/// itself.
#[test]
fn symlinked_manifest_is_not_followed() {
    let outside = tempfile::tempdir().unwrap();
    write(&outside, "secret.yaml", "apiName: secret\n");
    let dir = tempfile::tempdir().unwrap();
    if symlink_file(
        &outside.path().join("secret.yaml"),
        &dir.path().join("ontology.yaml"),
    )
    .is_err()
    {
        return; // no permission to create symlinks on this machine
    }
    let out = load(&dir);
    assert_eq!(codes_of(&out), [codes::V14], "{:#?}", out.diagnostics);
    assert!(out.diagnostics[0].message.contains("not a regular file"));
}

/// WR-06: a manifest spelled with other letter case is never a definition file, and
/// is reported the same way on case-sensitive and case-insensitive file systems.
#[test]
fn manifest_with_other_letter_case_is_v14_and_not_a_definition() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir, "Ontology.yaml", "apiName: demo\n");
    write(&dir, "airport.yaml", object("Airport"));
    let out = load(&dir);
    assert!(
        !codes_of(&out).contains(&codes::V02),
        "{:#?}",
        out.diagnostics
    );
    assert!(
        out.diagnostics.iter().any(|d| d.code == codes::V14
            && d.file == "Ontology.yaml"
            && d.message.contains("`ontology.yaml`")),
        "{:#?}",
        out.diagnostics
    );
}

#[test]
fn manifest_that_is_not_a_file_is_v14() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("ontology.yaml")).unwrap();
    let out = load(&dir);
    assert_eq!(codes_of(&out), [codes::V14], "{:#?}", out.diagnostics);
    let d = &out.diagnostics[0];
    assert_eq!(d.file, "ontology.yaml");
    assert!(d.message.contains("not a regular file"), "{}", d.message);
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

// ------------------------------------------------------------------- parsing

const MANIFEST: &str = "apiName: demo\n";

fn mem(files: &[(&str, &str)]) -> LoadOutput {
    let mut all = vec![("ontology.yaml", MANIFEST)];
    all.extend_from_slice(files);
    load_from_memory(&all)
}

fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("loader must not crash the calling thread")
}

#[test]
fn deeply_nested_yaml_is_rejected_not_crashing() {
    let codes = on_small_stack(|| {
        let mut doc = String::from("kind: ObjectType\napiName: Airport\nproperties:\n  code:\n");
        for depth in 0..200 {
            doc.push_str(&"  ".repeat(depth + 2));
            doc.push_str("a:\n");
        }
        mem(&[("objects.yaml", doc.as_str())])
            .diagnostics
            .into_iter()
            .map(|d| d.code)
            .collect::<Vec<_>>()
    });
    assert!(codes.iter().any(|c| c == "ONT-V01"), "{codes:?}");
}

#[test]
fn deeply_nested_json_is_rejected_not_crashing() {
    for depth in [100_usize, 200] {
        let codes = on_small_stack(move || {
            let doc = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
            mem(&[("objects.json", doc.as_str())])
                .diagnostics
                .into_iter()
                .map(|d| d.code)
                .collect::<Vec<_>>()
        });
        assert!(codes.iter().any(|c| c == "ONT-V01"), "{depth}: {codes:?}");
    }
}

#[test]
fn json_trailing_comma_is_rejected() {
    let text = "{\"kind\":\"ObjectType\",}";
    let out = mem(&[("objects.json", text)]);
    let d = &out.diagnostics[0];
    assert_eq!(d.code, codes::V01, "{:#?}", out.diagnostics);
    assert_eq!((d.file.as_str(), d.line, d.column), ("objects.json", 1, 22));
    assert_eq!(&text[d.span.offset..d.span.offset + d.span.length], "}");
}

#[test]
fn json_comment_is_rejected() {
    let out = mem(&[(
        "objects.json",
        "{\n  // note\n  \"kind\": \"ObjectType\"\n}",
    )]);
    let d = &out.diagnostics[0];
    assert_eq!((d.code.as_str(), d.line), (codes::V01, 2), "{d:?}");
}

#[test]
fn yaml_yes_no_stay_strings() {
    let doc = object("Airport").replace("apiName: Airport", "apiName: Airport\ndisplayName: no");
    let out = mem(&[("objects.yaml", doc.as_str())]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let airport = &out.ontology.unwrap().object_types["Airport"];
    assert_eq!(airport.display_name.as_deref(), Some("no"));

    let bad = object("Airport").replace("required: true", "required: yes");
    let out = mem(&[("objects.yaml", bad.as_str())]);
    assert_eq!(
        out.diagnostics[0].code,
        codes::V06,
        "{:#?}",
        out.diagnostics
    );
}

#[test]
fn duplicate_key_is_v01_with_location() {
    let out = mem(&[("objects.yaml", "kind: ObjectType\napiName: A\napiName: B\n")]);
    let d = &out.diagnostics[0];
    assert_eq!(
        (d.code.as_str(), d.line, d.column),
        (codes::V01, 3, 1),
        "{d:?}"
    );
    assert_eq!(d.message, "duplicate mapping key `apiName`");
}

#[test]
fn merge_key_is_v01() {
    let doc = "kind: ObjectType\napiName: A\nprimaryKey: code\nproperties:\n  code: &c { type: string }\n  other:\n    <<: *c\n";
    let out = mem(&[("objects.yaml", doc)]);
    let d = &out.diagnostics[0];
    assert_eq!(d.code, codes::V01, "{:#?}", out.diagnostics);
    assert_eq!(d.line, 7, "{d:?}");
}

#[test]
fn unsupported_tag_is_v01() {
    let doc = object("Airport").replace(
        "apiName: Airport",
        "apiName: Airport\ndisplayName: !custom x",
    );
    let out = mem(&[("objects.yaml", doc.as_str())]);
    let d = &out.diagnostics[0];
    assert_eq!((d.code.as_str(), d.line), (codes::V01, 3), "{d:?}");
}

#[test]
fn v01_message_has_no_parser_position_suffix() {
    let cases = [
        ("a.yaml", "kind: ObjectType\napiName: A\napiName: B\n"),
        ("b.json", "{\"a\": 1,}"),
        ("c.yaml", "a: [1\n"),
    ];
    for (name, text) in cases {
        let out = mem(&[(name, text)]);
        let d = &out.diagnostics[0];
        assert_eq!(d.code, codes::V01, "{name}: {d:?}");
        for forbidden in [" at line ", "DuplicateKeyPolicy", "Options"] {
            assert!(!d.message.contains(forbidden), "{name}: {}", d.message);
        }
    }
}

fn unknown_key_diag(name: &str, text: &str) -> (u32, u32, String) {
    let out = mem(&[(name, text)]);
    let d = out
        .diagnostics
        .iter()
        .find(|d| d.code == codes::V04)
        .unwrap_or_else(|| panic!("{name}: {:#?}", out.diagnostics));
    let slice = text[d.span.offset..d.span.offset + d.span.length].to_owned();
    (d.line, d.column, slice)
}

#[test]
fn bom_crlf_tab_and_cjk_locations_are_exact() {
    let yaml = object("Airport").replace("primaryKey", "primaryKy");

    let bom = format!("\u{feff}{yaml}");
    assert_eq!(unknown_key_diag("a.yaml", &bom), (3, 1, "primaryKy".into()));

    let crlf = yaml.replace('\n', "\r\n");
    assert_eq!(
        unknown_key_diag("a.yaml", &crlf),
        (3, 1, "primaryKy".into())
    );

    let tabbed =
        "{\n\t\"kind\": \"ObjectType\",\n\t\"apiName\": \"Airport\",\n\t\"primaryKy\": \"code\"\n}";
    assert_eq!(
        unknown_key_diag("a.json", tabbed),
        (4, 2, "\"primaryKy\"".into())
    );

    let cjk_line = "description: 机场说明\nprimaryKy: code";
    let cjk = yaml.replace("primaryKy: code", cjk_line);
    assert_eq!(unknown_key_diag("a.yaml", &cjk), (4, 1, "primaryKy".into()));

    let one_line = "{\"kind\": \"ObjectType\", \"apiName\": \"Airport\", \"description\": \"中文说明\", \"primaryKy\": \"code\"}";
    let col = one_line[..one_line.find("\"primaryKy\"").unwrap()]
        .chars()
        .count();
    let (line, column, slice) = unknown_key_diag("a.json", one_line);
    assert_eq!(
        (line, column as usize, slice.as_str()),
        (1, col + 1, "\"primaryKy\"")
    );
}

#[test]
fn json_syntax_error_column_counts_characters() {
    let text = "{\"description\": \"中文说明\", }";
    let out = mem(&[("a.json", text)]);
    let d = &out.diagnostics[0];
    let col = text
        .find('}')
        .map(|i| text[..i].chars().count() + 1)
        .unwrap();
    assert_eq!(
        (d.code.as_str(), d.column as usize),
        (codes::V01, col),
        "{d:?}"
    );
    assert_eq!(&text[d.span.offset..d.span.offset + d.span.length], "}");
}

#[test]
fn empty_yaml_documents_are_skipped() {
    let doc = format!("---\n---\n{}\n---\n# only a comment\n", object("Airport"));
    let out = mem(&[("objects.yaml", doc.as_str())]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(names(&out), ["Airport"]);
}

#[test]
fn multi_document_yaml_and_json_array_are_both_loaded() {
    let yaml = format!("{}---\n{}", object("Airport"), object("Flight"));
    let json = r#"[
      {"kind":"ObjectType","apiName":"Plane","primaryKey":"code","properties":{"code":{"type":"string","required":true}}},
      {"kind":"ObjectType","apiName":"Gate","primaryKey":"code","properties":{"code":{"type":"string","required":true}}}
    ]"#;
    let out = mem(&[("a.yaml", yaml.as_str()), ("b.json", json)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(names(&out), ["Airport", "Flight", "Gate", "Plane"]);
}

#[test]
fn properties_keep_declared_order() {
    let yaml = "kind: ObjectType\napiName: Airport\nprimaryKey: zeta\nproperties:\n  zeta: { type: string, required: true }\n  alpha: { type: string }\n  mid: { type: string }\n";
    let json = r#"{"kind":"ObjectType","apiName":"Flight","primaryKey":"zeta","properties":{"zeta":{"type":"string","required":true},"alpha":{"type":"string"},"mid":{"type":"string"}}}"#;
    let out = mem(&[("a.yaml", yaml), ("b.json", json)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.unwrap();
    for name in ["Airport", "Flight"] {
        let order: Vec<&str> = ontology.object_types[name]
            .properties
            .keys()
            .map(ontoforge_schema::ir::PropertyName::as_str)
            .collect();
        assert_eq!(order, ["zeta", "alpha", "mid"], "{name}");
    }
}

#[test]
fn file_order_does_not_change_ir() {
    let a = object("Airport");
    let b = object("Flight");
    let c = object("Gate");
    let forward = mem(&[("a.yaml", &a), ("b.yaml", &b), ("c.yaml", &c)]);
    let reverse = mem(&[("c.yaml", &c), ("b.yaml", &b), ("a.yaml", &a)]);
    assert!(forward.ontology.is_some());
    assert_eq!(forward.ontology, reverse.ontology);
}
