//! End-to-end tests of `ontoforge reload`: edit tables from a definition, dry run, exit codes
//! and the rule that a broken definition never opens the store (STORE-02, D-15).

use std::path::Path;
use std::process::{Command, Output};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

const SQLITE_CONFIG: &str = "[store]\nkind = \"sqlite\"\npath = \"edit.db\"\n";

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

fn reload(dir: &str, config: &Path, extra: &[&str]) -> Output {
    let config = config.to_str().expect("utf-8 path");
    let mut args = vec!["reload", dir, "--server-config", config];
    args.extend_from_slice(extra);
    ontoforge(&args)
}

/// A temporary directory holding `ontoforge-server.toml` with `text`.
fn server_dir(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("ontoforge-server.toml"), text).expect("write config");
    dir
}

fn config_of(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join("ontoforge-server.toml")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn reload_creates_tables() {
    let dir = server_dir(SQLITE_CONFIG);
    let first = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    assert!(
        stderr(&first).contains("def_version 1"),
        "{}",
        stderr(&first)
    );
    assert!(dir.path().join("edit.db").is_file());

    let second = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(second.status.code(), Some(0), "stderr: {}", stderr(&second));
    assert!(
        stderr(&second).contains("definitions unchanged"),
        "{}",
        stderr(&second)
    );
}

#[test]
fn dry_run_prints_ddl_only() {
    let dir = server_dir(SQLITE_CONFIG);
    let dry = reload(AVIATION, &config_of(&dir), &["--dry-run"]);
    assert_eq!(dry.status.code(), Some(0), "stderr: {}", stderr(&dry));
    assert!(stdout(&dry).contains("CREATE TABLE"), "{}", stdout(&dry));
    assert!(stderr(&dry).contains("nothing applied"), "{}", stderr(&dry));

    let real = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(real.status.code(), Some(0), "stderr: {}", stderr(&real));
    assert!(stderr(&real).contains("def_version 1"), "{}", stderr(&real));
}

#[test]
fn missing_store_section() {
    let dir = server_dir("");
    let out = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ONT-V20"), "{}", stdout(&out));
}

#[test]
fn store_unavailable() {
    // A regular file where the store's directory should be: the directory cannot be created.
    let dir = server_dir("[store]\nkind = \"sqlite\"\npath = \"not-a-dir/sub/edit.db\"\n");
    std::fs::write(dir.path().join("not-a-dir"), "").expect("blocker file");
    let out = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(out.status.code(), Some(3), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ONT-V26"), "{}", stdout(&out));
}

#[test]
fn invalid_definitions_never_open_store() {
    let dir = server_dir(SQLITE_CONFIG);
    let defs = tempfile::tempdir().expect("tempdir");
    std::fs::write(defs.path().join("ontology.yaml"), "apiName: demo\n").expect("manifest");
    std::fs::write(
        defs.path().join("defs.yaml"),
        "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n---\nkind: LinkType\napiName: gateStands\ncardinality: manyToMany\nfrom: { objectType: Gate, apiName: stands }\nto: { objectType: Missing, apiName: gates }\nbacking: editOnly\n",
    )
    .expect("defs");
    let out = reload(
        defs.path().to_str().expect("utf-8 path"),
        &config_of(&dir),
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(
        !dir.path().join("edit.db").exists(),
        "the store file must not be created for an invalid definition"
    );
}

#[test]
fn json_output() {
    let dir = server_dir(SQLITE_CONFIG);
    let out = reload(AVIATION, &config_of(&dir), &["--format", "json"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    let result = &value["result"];
    assert_eq!(result["defVersion"], 1);
    assert_eq!(result["changed"], true);
    assert_eq!(result["dryRun"], false);
    assert!(
        result["ddl"]
            .as_array()
            .expect("ddl")
            .iter()
            .any(|s| s.as_str().is_some_and(|s| s.contains("CREATE TABLE"))),
        "{result}"
    );
}

// ---------------------------------------------------------------------------------------------
// Roles that a new definition would break (plan 03-18, D-18, SCOPE-06)
// ---------------------------------------------------------------------------------------------

const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);
const DEV_CONFIG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/server/ontoforge-server.dev.toml"
);
const DEV_STORE_PATH: &str = "../../target/ontoforge-dev/editstore.db";

/// A copy of `from` below `to`.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create dir");
    for entry in std::fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("copy");
        }
    }
}

/// The number the one-column query `sql` returns from the SQLite file `db`.
fn scalar(db: &Path, sql: &str) -> i64 {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let options = sqlx::sqlite::SqliteConnectOptions::new().filename(db);
        let pool = sqlx::SqlitePool::connect_with(options).await.expect("pool");
        sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql.to_owned()))
            .fetch_one(&pool)
            .await
            .expect("scalar")
    })
}

#[test]
fn reload_reports_role_breakage() {
    // The example server configuration declares `homeAirports`, which crewScheduler reads.
    let dir = tempfile::tempdir().expect("tempdir");
    let text = std::fs::read_to_string(DEV_CONFIG).expect("example server config");
    assert!(
        text.contains(DEV_STORE_PATH),
        "the example store path moved"
    );
    std::fs::write(
        dir.path().join("ontoforge-server.toml"),
        text.replace(DEV_STORE_PATH, "edit.db"),
    )
    .expect("write config");
    let config_path = config_of(&dir);
    let config = config_path.to_str().expect("utf-8 path");
    let db = dir.path().join("edit.db");

    let first = reload(AVIATION, &config_path, &[]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    let plan = ontoforge(&[
        "scope",
        "plan",
        AVIATION_SCOPE,
        "--ontology",
        AVIATION,
        "--server-config",
        config,
    ]);
    assert_eq!(plan.status.code(), Some(0), "stderr: {}", stderr(&plan));
    let hash = stdout(&plan)
        .lines()
        .find_map(|line| line.strip_prefix("plan hash: "))
        .expect("a plan hash")
        .trim()
        .to_owned();
    let apply = ontoforge(&[
        "scope",
        "apply",
        AVIATION_SCOPE,
        "--ontology",
        AVIATION,
        "--server-config",
        config,
        "--plan-hash",
        &hash,
    ]);
    assert_eq!(apply.status.code(), Some(0), "stderr: {}", stderr(&apply));

    // A definition under which `delayFlight` reads `Flight.gate`, a column the dispatcher role
    // does not grant.
    let changed = tempfile::tempdir().expect("tempdir");
    copy_dir(Path::new(AVIATION), changed.path());
    for (file, from, to) in [
        (
            "objects.yaml",
            "  delayMinutes: { type: integer, displayName: Delay in minutes }",
            "  delayMinutes: { type: integer, displayName: Delay in minutes }\n  gate: { type: string, displayName: Gate }",
        ),
        (
            "actions.yaml",
            "objects.flight.status != 'CANCELLED'",
            "objects.flight.gate != 'X'",
        ),
    ] {
        let path = changed.path().join(file);
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains(from), "{file} lost `{from}`");
        std::fs::write(&path, text.replace(from, to)).expect("write");
    }
    let broken = reload(
        changed.path().to_str().expect("utf-8 path"),
        &config_path,
        &[],
    );
    assert_eq!(broken.status.code(), Some(1), "stderr: {}", stderr(&broken));
    assert!(stdout(&broken).contains("ONT-V23"), "{}", stdout(&broken));
    assert!(
        stdout(&broken).contains("dispatcher"),
        "the role is named: {}",
        stdout(&broken)
    );
    assert_eq!(
        scalar(&db, "SELECT count(*) FROM of_def_version"),
        1,
        "no definition version was recorded"
    );
    assert_eq!(
        scalar(
            &db,
            "SELECT count(*) FROM pragma_table_info('of_obj_flight') WHERE name = 'gate'"
        ),
        0,
        "no DDL ran"
    );

    // The unchanged example definition still reloads against the stored roles.
    let again = reload(AVIATION, &config_path, &[]);
    assert_eq!(again.status.code(), Some(0), "stderr: {}", stderr(&again));
    assert!(
        stderr(&again).contains("definitions unchanged"),
        "{}",
        stderr(&again)
    );
}

// ---------------------------------------------------------------------------------------------
// The documented quickstart on a fresh checkout (plan 03-23, CR-D01)
// ---------------------------------------------------------------------------------------------

#[test]
fn example_config_reload_works_from_a_fresh_tree() {
    // A checkout has no `target/ontoforge-dev/`: the store must create its own directory.
    let tree = tempfile::tempdir().expect("tempdir");
    let server_dir = tree.path().join("examples").join("server");
    std::fs::create_dir_all(&server_dir).expect("create examples/server");
    let config_path = server_dir.join("ontoforge-server.dev.toml");
    std::fs::copy(DEV_CONFIG, &config_path).expect("copy the example config");
    assert!(!tree.path().join("target").exists(), "a fresh tree");

    let first = reload(AVIATION, &config_path, &[]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    assert!(
        stderr(&first).contains("def_version 1"),
        "{}",
        stderr(&first)
    );
    assert!(
        tree.path()
            .join("target")
            .join("ontoforge-dev")
            .join("editstore.db")
            .is_file()
    );

    let plan = ontoforge(&[
        "scope",
        "plan",
        AVIATION_SCOPE,
        "--ontology",
        AVIATION,
        "--server-config",
        config_path.to_str().expect("utf-8 path"),
    ]);
    assert_eq!(plan.status.code(), Some(0), "stderr: {}", stderr(&plan));
    assert!(stdout(&plan).contains("plan hash: "), "{}", stdout(&plan));
}
