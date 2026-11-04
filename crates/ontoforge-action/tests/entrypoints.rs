//! Regression guard for production write callers and the audited import boundary.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::ActionRequest;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::import::{ImportOptions, ImportTarget, import};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use regex::Regex;
use sha2::{Digest, Sha256};

fn write_calls(source: &str) -> Vec<(usize, String, String)> {
    let pattern = Regex::new(
        r"([A-Za-z_][A-Za-z0-9_]*)\s*(?:\.|::)\s*(commit_action_guarded|commit_action_with_fault|commit_action|commit)\s*\(",
    )
    .unwrap();
    let without_comments = source
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("//") {
                " ".repeat(line.len())
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    pattern
        .captures_iter(&without_comments)
        .map(|capture| {
            let whole = capture.get(0).unwrap();
            let line = without_comments[..whole.start()]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            (line, capture[1].to_owned(), capture[2].to_owned())
        })
        .collect()
}

#[test]
fn source_guard_recognizes_split_and_ufcs_calls() {
    let sample = "store .\n commit_action_guarded (input);\nEditStore::commit(&store, input);";
    assert_eq!(
        write_calls(sample),
        [
            (1, "store".to_owned(), "commit_action_guarded".to_owned()),
            (3, "EditStore".to_owned(), "commit".to_owned()),
        ]
    );
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn production_mutation_callers_are_action_import_and_read_audit() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates root");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(crates).expect("workspace crates") {
        let src = entry.expect("crate entry").path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    assert!(!files.is_empty(), "no production source discovered");
    let mut callers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let rel = file
            .strip_prefix(crates)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read_to_string(&file).expect("production source");
        let production = source.split("\n#[cfg(test)]").next().unwrap();
        for (line, receiver, method) in write_calls(production) {
            if method == "commit" && receiver == "sql_exit" {
                continue;
            }
            if rel == "ontoforge-storage/src/sql_exit.rs"
                && method == "commit"
                && (receiver == "t" || receiver == "tx")
            {
                continue;
            }
            if rel == "ontoforge-storage/src/commit.rs" && method == "commit_action_guarded" {
                continue; // storage's own delegating entry point
            }
            callers
                .entry(format!("{rel}:{method}"))
                .or_default()
                .push(format!(
                    "{rel}:{line}: {}",
                    production.lines().nth(line - 1).unwrap().trim()
                ));
        }
    }
    let expected = BTreeMap::from([
        (
            "ontoforge-action/src/lib.rs:commit_action_guarded".to_owned(),
            1,
        ),
        ("ontoforge-storage/src/import.rs:commit".to_owned(), 1),
        ("ontoforge-storage/src/read_audit.rs:commit".to_owned(), 2),
    ]);
    let counts: BTreeMap<_, _> = callers
        .iter()
        .map(|(key, sites)| (key.clone(), sites.len()))
        .collect();
    assert_eq!(
        counts, expected,
        "unexpected production write callers: {callers:#?}"
    );
}

#[test]
fn action_request_rejects_privileged_write_fields() {
    for field in [
        "commit",
        "edits",
        "principal",
        "scope",
        "limits",
        "webhookUrl",
        "headers",
    ] {
        let mut value = serde_json::json!({
            "action": "createGate", "commandId": "test", "items": [], "validateOnly": false
        });
        value[field] = serde_json::json!({});
        assert!(
            serde_json::from_value::<ActionRequest>(value).is_err(),
            "accepted {field}"
        );
    }
}

#[tokio::test]
async fn import_log_has_reserved_identity_and_source_digest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let file = Utf8PathBuf::from_path_buf(dir.path().join("gates.csv")).unwrap();
    let contents = b"code,name\nG1,Private Name\n";
    std::fs::write(&file, contents).expect("fixture file");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("store");
    let defs = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
";
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", defs)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    reload(&store, &loaded.ontology.unwrap(), &ReloadOptions::default())
        .await
        .expect("reload");
    let report = import(
        &store,
        ImportTarget::Object("Gate".to_owned().try_into().unwrap()),
        &file,
        &ImportOptions::default(),
    )
    .await
    .expect("import");
    assert_eq!(report.rows, 1);
    let log = store.read_log(0, 10).await.expect("log");
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].actor, "system:import");
    assert_eq!(log[0].action, "system:import");
    assert_eq!(log[0].params["file"], "gates.csv");
    assert_eq!(
        log[0].params["source_digest"],
        format!("{:x}", Sha256::digest(contents))
    );
}
