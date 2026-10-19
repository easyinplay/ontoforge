use std::process::{Command, Output};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas");
const FILES: [&str; 2] = [
    "ontoforge-definition.schema.json",
    "ontoforge-manifest.schema.json",
];

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

#[test]
fn schema_export_writes_committed_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_dir = dir.path().to_str().expect("utf-8 path");
    let out = ontoforge(&["schema", "export", "--out-dir", out_dir]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for name in FILES {
        let written = std::fs::read(dir.path().join(name)).expect("exported file exists");
        let committed = std::fs::read(format!("{COMMITTED}/{name}")).expect("committed file");
        assert_eq!(written, committed, "{name} differs from schemas/");
    }
}

#[test]
fn schema_export_creates_missing_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let nested = dir.path().join("a").join("b");
    let out = ontoforge(&[
        "schema",
        "export",
        "--out-dir",
        nested.to_str().expect("utf-8"),
    ]);
    assert_eq!(out.status.code(), Some(0));
    for name in FILES {
        assert!(nested.join(name).is_file(), "{name} was not written");
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("ontoforge-definition.schema.json"),
        "stderr: {stderr}"
    );
}

#[test]
fn schema_export_reports_io_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, "x").expect("write blocker");
    let out = ontoforge(&[
        "schema",
        "export",
        "--out-dir",
        blocker.join("sub").to_str().expect("utf-8"),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!out.stderr.is_empty());
}

#[test]
fn aviation_still_validates_with_modelines() {
    let out = ontoforge(&["validate", AVIATION, "--color", "never"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
}
