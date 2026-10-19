//! The exported JSON Schemas stay in step with the IR and accept the examples.

use ontoforge_schema::schema_export::{definition_schema, manifest_schema, rendered_schemas};
use serde_json::Value;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn example_dir(name: &str) -> std::path::PathBuf {
    std::path::Path::new(ROOT)
        .join("examples/ontologies")
        .join(name)
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every YAML document of a file, parsed as loosely typed JSON with the loader's
/// boolean strictness.
fn yaml_documents(path: &std::path::Path) -> Vec<Value> {
    let mut options = serde_saphyr::Options::default();
    options.strict_booleans = true;
    serde_saphyr::from_multiple_with_options::<Value>(&read(path), options)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn json_document(path: &std::path::Path) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn validator(schema: &Value) -> jsonschema::Validator {
    jsonschema::validator_for(schema).expect("the exported schema is itself valid")
}

/// Collect all `const` values anywhere inside `value`.
fn consts(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if key == "const"
                    && let Some(s) = inner.as_str()
                {
                    out.push(s.to_owned());
                }
                consts(inner, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| consts(i, out)),
        _ => {}
    }
}

#[test]
fn definition_schema_is_draft_2020_12() {
    for schema in [definition_schema(), manifest_schema()] {
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }
}

#[test]
fn definition_variants_are_discriminated_by_kind() {
    let schema = definition_schema();
    let mut found = Vec::new();
    for key in ["$defs", "definitions"] {
        if let Some(defs) = schema.get(key).and_then(Value::as_object) {
            let one_of = defs
                .get("Definition")
                .and_then(|d| d.get("oneOf"))
                .and_then(Value::as_array);
            if let Some(branches) = one_of {
                assert_eq!(branches.len(), 4, "Definition has four branches");
                for branch in branches {
                    let kind = branch["properties"]["kind"]["const"]
                        .as_str()
                        .expect("every branch pins `kind` with a const");
                    found.push(kind.to_owned());
                    assert_eq!(
                        branch["additionalProperties"],
                        Value::Bool(false),
                        "branch {kind} rejects unknown fields"
                    );
                }
            }
        }
    }
    found.sort();
    assert_eq!(
        found,
        ["ActionType", "LinkType", "ObjectType", "SourceBinding"]
    );
}

#[test]
fn rule_variants_are_discriminated_by_kind() {
    let mut all = Vec::new();
    consts(&definition_schema(), &mut all);
    for rule in [
        "createObject",
        "modifyObject",
        "deleteObject",
        "addLink",
        "removeLink",
    ] {
        assert!(all.iter().any(|c| c == rule), "missing const `{rule}`");
    }
}

#[test]
fn committed_schemas_are_up_to_date() {
    let update = std::env::var("ONTOFORGE_UPDATE_SCHEMAS").as_deref() == Ok("1");
    let dir = std::path::Path::new(ROOT).join("schemas");
    for (name, rendered) in rendered_schemas() {
        let path = dir.join(name);
        if update {
            std::fs::create_dir_all(&dir).expect("create schemas dir");
            std::fs::write(&path, &rendered).expect("write schema");
            continue;
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == rendered,
            "schemas/{name} is out of date with the IR; run \
             `cargo run -p ontoforge -- schema export` and commit the result"
        );
    }
}

#[test]
fn rendering_is_deterministic() {
    assert_eq!(rendered_schemas(), rendered_schemas());
}

#[test]
fn aviation_documents_validate() {
    let definition = validator(&definition_schema());
    let manifest = validator(&manifest_schema());

    for name in ["objects.yaml", "links.yaml", "actions.yaml"] {
        let path = example_dir("aviation").join(name);
        for (i, doc) in yaml_documents(&path).iter().enumerate() {
            let errors: Vec<String> = definition
                .iter_errors(doc)
                .map(|e| format!("{e} at {}", e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{name} document {i}: {errors:#?}");
        }
    }
    for name in ["objects.json", "links.json", "actions.json"] {
        let path = example_dir("aviation-json").join(name);
        let doc = json_document(&path);
        let errors: Vec<String> = definition
            .iter_errors(&doc)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:#?}");
    }
    for dir in ["aviation", "aviation-json"] {
        let path = example_dir(dir).join("ontology.yaml");
        for doc in yaml_documents(&path) {
            let errors: Vec<String> = manifest
                .iter_errors(&doc)
                .map(|e| format!("{e} at {}", e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{dir}/ontology.yaml: {errors:#?}");
        }
    }
}

#[test]
fn invalid_examples_fail_schema() {
    let definition = validator(&definition_schema());
    for name in [
        "v04-unknown-field",
        "v06-unknown-base-type",
        "v07-bad-object-type-name",
    ] {
        let path = example_dir("invalid").join(name).join("objects.yaml");
        let rejected = yaml_documents(&path)
            .iter()
            .any(|d| !definition.is_valid(d));
        assert!(rejected, "{name} unexpectedly passes the definition schema");
    }
}

const DEFINITION_SCHEMA_KEY: &str = "./schemas/ontoforge-definition.schema.json";

fn repo_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(ROOT).join(relative)
}

/// Parse an editor settings file as strict JSON (no comments, no trailing commas).
fn strict_json(relative: &str) -> Value {
    serde_json::from_str(&read(&repo_path(relative)))
        .unwrap_or_else(|e| panic!("{relative} must be strict JSON: {e}"))
}

fn only_key<'a>(value: &'a Value, expected: &str, at: &str) -> &'a Value {
    let map = value
        .as_object()
        .unwrap_or_else(|| panic!("{at} must be a JSON object"));
    let keys: Vec<&String> = map.keys().collect();
    assert_eq!(
        keys,
        [expected],
        "{at} must contain exactly the key `{expected}`: the editor configs only carry the \
         schema mapping, nothing executable"
    );
    &map[expected]
}

fn glob(pattern: &str, at: &str) -> globset::GlobMatcher {
    globset::GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .unwrap_or_else(|e| panic!("{at}: invalid glob `{pattern}`: {e}"))
        .compile_matcher()
}

#[test]
fn editor_configs_associate_definition_schema_with_examples() {
    let zed = strict_json(".zed/settings.json");
    let lsp = only_key(&zed, "lsp", ".zed/settings.json");
    let server = only_key(lsp, "yaml-language-server", ".zed/settings.json lsp");
    let settings = only_key(
        server,
        "settings",
        ".zed/settings.json lsp.yaml-language-server",
    );
    let yaml = only_key(settings, "yaml", ".zed/settings.json ...settings");
    let zed_schemas = only_key(yaml, "schemas", ".zed/settings.json ...settings.yaml");

    let vscode = strict_json(".vscode/settings.json");
    let vscode_schemas = only_key(&vscode, "yaml.schemas", ".vscode/settings.json");

    assert_eq!(
        zed_schemas, vscode_schemas,
        ".zed/settings.json and .vscode/settings.json must map the same schemas"
    );

    // Every key is a schema file inside the repository, never a remote URL.
    let exported: Vec<&str> = rendered_schemas().iter().map(|(name, _)| *name).collect();
    let mappings = zed_schemas
        .as_object()
        .expect("yaml.schemas must be a JSON object");
    for key in mappings.keys() {
        let relative = key.strip_prefix("./schemas/").unwrap_or_else(|| {
            panic!("yaml.schemas key `{key}` must be a local `./schemas/...` path")
        });
        assert!(
            repo_path(&format!("schemas/{relative}")).is_file(),
            "yaml.schemas key `{key}` points at a file that does not exist"
        );
        assert!(
            exported.contains(&relative),
            "yaml.schemas key `{key}` is not one of the exported schemas {exported:?}"
        );
    }

    let globs: Vec<&str> = mappings
        .get(DEFINITION_SCHEMA_KEY)
        .unwrap_or_else(|| panic!("yaml.schemas must contain the key `{DEFINITION_SCHEMA_KEY}`"))
        .as_array()
        .expect("the definition schema maps to an array of globs")
        .iter()
        .map(|g| g.as_str().expect("globs are strings"))
        .collect();
    assert_eq!(
        globs.first().copied(),
        Some("examples/ontologies/**/*.yaml"),
        "the first glob must cover every example definition file"
    );
    let include: Vec<_> = globs
        .iter()
        .filter(|g| !g.starts_with('!'))
        .map(|g| glob(g, "include"))
        .collect();
    let exclude: Vec<_> = globs
        .iter()
        .filter_map(|g| g.strip_prefix('!'))
        .map(|g| glob(g, "exclude"))
        .collect();

    let mut covered = Vec::new();
    let examples = repo_path("examples/ontologies");
    for entry in walkdir::WalkDir::new(&examples) {
        let entry = entry.expect("walk examples/ontologies");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let relative = path
            .strip_prefix(std::path::Path::new(ROOT))
            .expect("example under repo root")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let is_covered = include.iter().any(|g| g.is_match(&relative))
            && !exclude.iter().any(|g| g.is_match(&relative));
        if path.file_name().and_then(|n| n.to_str()) == Some("ontology.yaml") {
            assert!(
                !is_covered,
                "{relative} must not be covered by the definition schema: a manifest relies on \
                 its own modeline"
            );
        } else {
            assert!(
                is_covered,
                "{relative} is not covered by the definition schema globs {globs:?}"
            );
            covered.push(relative);
        }
    }
    for name in ["objects.yaml", "links.yaml", "actions.yaml"] {
        let expected = format!("examples/ontologies/aviation/{name}");
        assert!(
            covered.contains(&expected),
            "{expected} missing from the covered set (empty walk?)"
        );
    }
}

const DEFINITION_MODELINE: &str =
    "# yaml-language-server: $schema=../../../schemas/ontoforge-definition.schema.json";
const MANIFEST_MODELINE: &str =
    "# yaml-language-server: $schema=../../../schemas/ontoforge-manifest.schema.json";

/// yaml-language-server applies a modeline only to the YAML document it sits in
/// (redhat-developer/vscode-yaml#1066), so every document of a multi-document
/// definition file carries its own.
#[test]
fn aviation_definition_documents_start_with_modeline() {
    let dir = example_dir("aviation");

    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .filter(|p| p.file_name().and_then(|n| n.to_str()) != Some("ontology.yaml"))
        .collect();
    files.sort();
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    for expected in ["actions.yaml", "links.yaml", "objects.yaml"] {
        assert!(
            names.iter().any(|n| n == expected),
            "{expected} missing from aviation definition files {names:?}"
        );
    }

    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy();
        let text = read(path);
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(
            lines.first().copied(),
            Some(DEFINITION_MODELINE),
            "{name}:1 must be the definition modeline"
        );
        for (i, line) in lines.iter().enumerate() {
            if *line == "---" {
                assert_eq!(
                    lines.get(i + 1).copied(),
                    Some(DEFINITION_MODELINE),
                    "{name}:{} must be the definition modeline (the line after `---`)",
                    i + 2
                );
            }
        }
        let modelines = lines.iter().filter(|l| **l == DEFINITION_MODELINE).count();
        let documents = yaml_documents(path).len();
        assert_eq!(
            modelines, documents,
            "{name} has {modelines} definition modelines for {documents} YAML documents"
        );
    }

    let manifest = read(&dir.join("ontology.yaml"));
    assert_eq!(
        manifest.lines().next(),
        Some(MANIFEST_MODELINE),
        "aviation/ontology.yaml:1 must be the manifest modeline: the editor configs exclude it"
    );

    for modeline in [DEFINITION_MODELINE, MANIFEST_MODELINE] {
        let relative = modeline
            .split_once("$schema=")
            .expect("modeline has $schema=")
            .1;
        assert!(
            dir.join(relative).is_file(),
            "modeline schema path `{relative}` does not resolve from examples/ontologies/aviation"
        );
    }
}

#[test]
fn aviation_bindings_validate() {
    let definition = validator(&definition_schema());
    let dir = example_dir("aviation").join("bindings/dev");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("aviation bindings/dev") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        for (i, doc) in yaml_documents(&path).iter().enumerate() {
            let errors: Vec<String> = definition
                .iter_errors(doc)
                .map(|e| format!("{e} at {}", e.instance_path()))
                .collect();
            assert!(
                errors.is_empty(),
                "{} document {i}: {errors:#?}",
                path.display()
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 5, "one binding per source type and link");
    let json = json_document(&example_dir("aviation-json").join("bindings/dev/bindings.json"));
    let errors: Vec<String> = definition
        .iter_errors(&json)
        .map(|e| format!("{e} at {}", e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "bindings.json: {errors:#?}");
}
