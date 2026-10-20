//! End-to-end tests of `ontoforge scope plan|apply`: roles-as-code from a scope directory into the
//! edit store, with an offline check first (plan 03-16: SCOPE-01, SCOPE-07, D-01).

use std::path::Path;
use std::process::{Command, Output};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);
const DEV_CONFIG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/server/ontoforge-server.dev.toml"
);
const DEV_STORE_PATH: &str = "../../target/ontoforge-dev/editstore.db";

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

/// A temporary directory with the example server configuration, its store moved into the
/// directory.
struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
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
        Self { dir }
    }

    fn config(&self) -> String {
        self.dir
            .path()
            .join("ontoforge-server.toml")
            .to_str()
            .expect("utf-8 path")
            .to_owned()
    }

    fn db(&self) -> std::path::PathBuf {
        self.dir.path().join("edit.db")
    }

    fn reload(&self, ontology: &str) -> Output {
        ontoforge(&["reload", ontology, "--server-config", &self.config()])
    }

    fn plan(&self, scope: &str, extra: &[&str]) -> Output {
        let config = self.config();
        let mut args = vec![
            "scope",
            "plan",
            scope,
            "--ontology",
            AVIATION,
            "--server-config",
            &config,
        ];
        args.extend_from_slice(extra);
        ontoforge(&args)
    }

    fn apply(&self, scope: &str, hash: &str, extra: &[&str]) -> Output {
        let config = self.config();
        let mut args = vec![
            "scope",
            "apply",
            scope,
            "--ontology",
            AVIATION,
            "--server-config",
            &config,
            "--plan-hash",
            hash,
        ];
        args.extend_from_slice(extra);
        ontoforge(&args)
    }

    /// Rows of `table` in the edit store.
    fn count(&self, table: &str) -> i64 {
        rows(&self.db(), &format!("SELECT count(*) FROM {table}"))
    }
}

fn rows(db: &Path, sql: &str) -> i64 {
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
            .expect("count")
    })
}

/// The 16 hex digits after `plan hash: `.
fn plan_hash(out: &Output) -> String {
    let text = stdout(out);
    let hash = text
        .lines()
        .find_map(|line| line.strip_prefix("plan hash: "))
        .unwrap_or_else(|| panic!("no plan hash in:\n{text}"))
        .trim()
        .to_owned();
    assert_eq!(hash.len(), 16, "{hash}");
    assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()), "{hash}");
    hash
}

fn lines_with(out: &Output, prefix: &str) -> usize {
    stdout(out)
        .lines()
        .filter(|line| line.starts_with(prefix))
        .count()
}

#[test]
fn plan_then_apply_aviation_scope() {
    let env = Env::new();
    let reload = env.reload(AVIATION);
    assert_eq!(reload.status.code(), Some(0), "stderr: {}", stderr(&reload));

    let plan = env.plan(AVIATION_SCOPE, &[]);
    assert_eq!(plan.status.code(), Some(0), "stderr: {}", stderr(&plan));
    assert_eq!(lines_with(&plan, "add role "), 3, "{}", stdout(&plan));
    assert_eq!(lines_with(&plan, "add binding "), 3, "{}", stdout(&plan));
    assert_eq!(lines_with(&plan, "create index "), 1, "{}", stdout(&plan));
    let hash = plan_hash(&plan);
    assert_eq!(env.count("of_role"), 0, "a plan writes nothing");

    let apply = env.apply(AVIATION_SCOPE, &hash, &[]);
    assert_eq!(apply.status.code(), Some(0), "stderr: {}", stderr(&apply));
    assert_eq!(env.count("of_role"), 3);
    assert_eq!(env.count("of_role_binding"), 3);
    assert_eq!(env.count("of_admin_audit"), 1);
    assert_eq!(
        rows(
            &env.db(),
            "SELECT count(*) FROM of_admin_audit WHERE actor LIKE '%@%'"
        ),
        1,
        "the actor is <user>@<host>"
    );

    let again = env.plan(AVIATION_SCOPE, &[]);
    assert_eq!(again.status.code(), Some(0), "stderr: {}", stderr(&again));
    assert_eq!(lines_with(&again, "add "), 0, "{}", stdout(&again));
    assert_eq!(lines_with(&again, "remove "), 0, "{}", stdout(&again));
    assert!(stdout(&again).contains("no changes"), "{}", stdout(&again));
}

#[test]
fn invalid_scope_writes_nothing() {
    let env = Env::new();
    let reload = env.reload(AVIATION);
    assert_eq!(reload.status.code(), Some(0), "stderr: {}", stderr(&reload));

    let scope = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        scope.path().join("roles.yaml"),
        "kind: Role\napiName: broken\nobjectTypes:\n  Flight:\n    properties: [flightId, status]\n    rowFilter: \"object.status == principal.attrs.tenantId\"\n---\nkind: RoleBinding\nrole: broken\nsubjects:\n  - group: ops\n",
    )
    .expect("scope file");
    let scope = scope.path().to_str().expect("utf-8 path");

    let plan = env.plan(scope, &[]);
    assert_eq!(plan.status.code(), Some(1), "stderr: {}", stderr(&plan));
    assert!(stdout(&plan).contains("ONT-V22"), "{}", stdout(&plan));

    let apply = env.apply(scope, "0000000000000000", &[]);
    assert_eq!(apply.status.code(), Some(1), "stderr: {}", stderr(&apply));
    assert!(stdout(&apply).contains("ONT-V22"), "{}", stdout(&apply));
    assert_eq!(env.count("of_role"), 0);
    assert_eq!(env.count("of_role_binding"), 0);
    assert_eq!(env.count("of_admin_audit"), 0);
}
