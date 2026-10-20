//! End-to-end tests of `ontoforge migrate`: the way out of the destructive changes `reload`
//! refuses (STORE-02, D-14, D-16). A definition loses a property, `reload` stops with ONT-V25,
//! `migrate --plan` shows the steps and their hash, `migrate --apply --plan-hash` runs them.

use std::path::Path;
use std::process::{Command, Output};

const SQLITE_CONFIG: &str = "[store]\nkind = \"sqlite\"\npath = \"edit.db\"\n";

const GATE_WITH_CAPACITY: &str = "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  name: { type: string }\n  capacity: { type: integer }\n";

const GATE_WITHOUT_CAPACITY: &str = "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n  name: { type: string }\n";

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A temporary workspace: a server configuration with an SQLite store and an ontology directory.
struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new(gate: &str) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("ontoforge-server.toml"), SQLITE_CONFIG).expect("config");
        std::fs::create_dir(dir.path().join("defs")).expect("defs dir");
        std::fs::write(dir.path().join("defs/ontology.yaml"), "apiName: demo\n").expect("manifest");
        let workspace = Self { dir };
        workspace.set_gate(gate);
        workspace
    }

    fn set_gate(&self, gate: &str) {
        std::fs::write(self.dir.path().join("defs/defs.yaml"), gate).expect("defs");
    }

    fn defs(&self) -> String {
        self.dir
            .path()
            .join("defs")
            .to_str()
            .expect("utf-8")
            .to_owned()
    }

    fn config(&self) -> String {
        self.dir
            .path()
            .join("ontoforge-server.toml")
            .to_str()
            .expect("utf-8")
            .to_owned()
    }

    fn run(&self, command: &str, extra: &[&str]) -> Output {
        let defs = self.defs();
        let config = self.config();
        let mut args = vec![command, defs.as_str(), "--server-config", config.as_str()];
        args.extend_from_slice(extra);
        ontoforge(&args)
    }

    fn db(&self) -> &Path {
        self.dir.path()
    }
}

/// The 16 hex digits after `plan hash: `.
fn plan_hash(out: &Output) -> String {
    let text = stdout(out);
    let hash = text
        .lines()
        .find_map(|line| line.strip_prefix("plan hash: "))
        .unwrap_or_else(|| panic!("no `plan hash:` line in:\n{text}"))
        .trim()
        .to_owned();
    assert_eq!(hash.len(), 16, "{hash}");
    assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()), "{hash}");
    hash
}

#[test]
fn migrate_plan_then_apply() {
    let ws = Workspace::new(GATE_WITH_CAPACITY);
    let first = ws.run("reload", &[]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));

    ws.set_gate(GATE_WITHOUT_CAPACITY);
    let refused = ws.run("reload", &[]);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "stderr: {}",
        stderr(&refused)
    );
    assert!(stdout(&refused).contains("ONT-V25"), "{}", stdout(&refused));

    let plan = ws.run("migrate", &["--plan"]);
    assert_eq!(plan.status.code(), Some(0), "stderr: {}", stderr(&plan));
    assert!(
        stdout(&plan).contains("archive column"),
        "{}",
        stdout(&plan)
    );
    let hash = plan_hash(&plan);

    let apply = ws.run("migrate", &["--apply", "--plan-hash", &hash]);
    assert_eq!(apply.status.code(), Some(0), "stderr: {}", stderr(&apply));
    assert!(
        stderr(&apply).contains("def_version 2"),
        "{}",
        stderr(&apply)
    );
    assert!(ws.db().join("edit.db").is_file());

    let reload = ws.run("reload", &[]);
    assert_eq!(reload.status.code(), Some(0), "stderr: {}", stderr(&reload));
    assert!(
        stderr(&reload).contains("definitions unchanged"),
        "{}",
        stderr(&reload)
    );
}

#[test]
fn apply_with_a_wrong_hash_exits_1() {
    let ws = Workspace::new(GATE_WITH_CAPACITY);
    assert_eq!(ws.run("reload", &[]).status.code(), Some(0));
    ws.set_gate(GATE_WITHOUT_CAPACITY);

    let apply = ws.run("migrate", &["--apply", "--plan-hash", "0000000000000000"]);
    assert_eq!(apply.status.code(), Some(1), "stderr: {}", stderr(&apply));
    assert!(
        stderr(&apply).contains("plan changed") && stderr(&apply).contains("0000000000000000"),
        "{}",
        stderr(&apply)
    );
}

#[test]
fn apply_requires_plan_hash() {
    let ws = Workspace::new(GATE_WITH_CAPACITY);
    let out = ws.run("migrate", &["--apply"]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("--plan-hash"), "{}", stderr(&out));
}

#[test]
fn plan_and_apply_are_exclusive_and_one_is_required() {
    let ws = Workspace::new(GATE_WITH_CAPACITY);
    let neither = ws.run("migrate", &[]);
    assert_eq!(neither.status.code(), Some(2), "{}", stderr(&neither));
    assert!(stderr(&neither).contains("--plan"), "{}", stderr(&neither));
    let both = ws.run("migrate", &["--plan", "--apply", "--plan-hash", "x"]);
    assert_eq!(both.status.code(), Some(2), "{}", stderr(&both));
}
