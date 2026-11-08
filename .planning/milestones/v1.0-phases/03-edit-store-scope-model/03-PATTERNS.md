# Phase 3: Edit Store & Scope Model - Pattern Map

**Mapped:** 2026-10-19
**Files analyzed:** 34 (new/modified)
**Analogs found:** 28 exact/role-match / 34 (6 have no in-repo analog)

All analog paths below were listed by `git ls-files` (tracked). `ontoforge-storage`, `ontoforge-auth` are 14-line shells today; nothing to copy from them except the Cargo.toml layout.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/ontoforge-schema/src/ir/scope.rs` (Role, RoleBinding, ScopeDefinition) | model (IR) | transform | `crates/ontoforge-schema/src/ir/mod.rs` (`Definition`) + `ir/binding.rs` | exact |
| `crates/ontoforge-schema/src/ir/object.rs` (+`indexed`) | model (IR) | transform | itself (`PropertyDef`) | modify |
| `crates/ontoforge-schema/src/loader/scope.rs` + generalise `loader/dispatch.rs` | loader | request-response | `crates/ontoforge-schema/src/loader/dispatch.rs` | exact |
| `crates/ontoforge-schema/src/schema_export.rs` (+`ontoforge-scope.schema.json`) | utility | transform | itself (`rendered_schemas`) | modify |
| `crates/ontoforge-schema/src/diag.rs` (`codes::V*`, new E codes) | config | - | itself (`codes` module) | modify |
| `crates/ontoforge-schema/tests/scope_*.rs`, schema_export snapshot | test | transform | `tests/schema_export.rs`, `tests/bindings.rs` | exact |
| `crates/ontoforge-expr/src/rowpred/{ir,parse,typecheck,eval}.rs` | service | transform | `ontoforge-expr/src/analyze.rs`, `walk.rs`, `infer.rs` | role-match |
| `crates/ontoforge-expr/tests/rowpred_*.rs` | test | transform | `crates/ontoforge-expr/tests/analyze.rs`, `criteria.rs` | role-match |
| `crates/ontoforge-engine/src/printer/ast.rs` (+`Expr::StringMatch`) | model | transform | itself (`Expr::InList`) | modify |
| `crates/ontoforge-engine/src/printer/escape.rs` (+`glob_escape`, `like_escape`) | utility | transform | itself (`quote_string`) | modify |
| `crates/ontoforge-engine/src/printer/call.rs` (`CALL ...flush_metadata_cache`) | utility | request-response | `printer/dialect.rs` | role-match |
| `crates/ontoforge-engine/src/config.rs` (+`[auth]`, `[store]`) | config | transform | itself (`ServerConfig`, `TrinoConfig`) | modify |
| `crates/ontoforge-engine/src/printer/sqlite_ddl.rs`, `sqlite/mod.rs` (export collations) | utility | transform | itself | modify |
| `crates/ontoforge-auth/src/scope/{merge,lower,lint}.rs` | service | transform | `ontoforge-schema/src/lint/binding/e13.rs` (lint shape); `ontoforge-expr/src/walk.rs` (`member_paths`) | role-match |
| `crates/ontoforge-auth/src/authn/{oidc,jwks,dev,principal}.rs` | service | request-response | `ontoforge-engine/src/trino/client.rs` + `config.rs` (reqwest, `*_env`/`*_file` secrets) | partial |
| `crates/ontoforge-auth/tests/*` (differential proptest) | test | transform | `ontoforge-engine/tests/printer_prop.rs` | exact |
| `crates/ontoforge-storage/src/{naming,physical,diff,ddl}.rs` | utility | transform | `ontoforge-engine/src/printer/sqlite_ddl.rs` | role-match |
| `crates/ontoforge-storage/src/{state,commit,log,history}.rs` | service | CRUD | none | no analog |
| `crates/ontoforge-storage/src/{reload,migrate,import,scope_store}.rs` | service | batch | `ontoforge-engine/src/sqlite/fixture.rs` (import parsing) | partial |
| `crates/ontoforge-storage/src/sql_exit.rs` | utility | request-response | `ontoforge-engine/src/printer/escape.rs` + `sqlite/mod.rs` (`AssertSqlSafe`) | role-match |
| `crates/ontoforge-storage/migrations/{postgres,sqlite}/*.sql` | migration | CRUD | none | no analog |
| `crates/ontoforge-storage/tests/sql_exit_guard.rs` | test | transform | `ontoforge-engine/tests/sql_text_only_from_printer.rs` | exact |
| `crates/ontoforge-storage/tests/{conformance,model_prop,replay,append_only}.rs` | test | CRUD | `printer_prop.rs` (proptest), `trino_it.rs` (ignored env-gated) | role-match |
| `crates/ontoforge/src/{reload,migrate,import,scope,dev_token}.rs` + `cli.rs` + `lib.rs` | controller (CLI) | request-response | `crates/ontoforge/src/{cli,lib,pipeline,lint_cmd,online}.rs` | exact |
| `crates/ontoforge/tests/cli_{reload,migrate,import,scope}.rs` | test | request-response | `crates/ontoforge/tests/cli_schema.rs`, `cli_lint.rs` | exact |
| `crates/ontoforge-{storage,auth}/Cargo.toml`, workspace `Cargo.toml` | config | - | `crates/ontoforge-engine/Cargo.toml` | role-match |
| `.github/workflows/ci.yml` (PG conformance step) | config | - | itself, `integration` job | exact |
| `ci/trino/catalog/editstore.properties` | config | - | itself | modify |
| `examples/ontologies/aviation/` roles + import data | config | - | existing aviation ontology files | role-match |
| `schemas/ontoforge-scope.schema.json` | config | - | `schemas/ontoforge-definition.schema.json` | exact |

## Pattern Assignments

### `ir/scope.rs` + `loader/scope.rs` + `dispatch.rs` generalisation (loader, kind dispatch)

**Analog:** `crates/ontoforge-schema/src/loader/dispatch.rs`

The kind whitelist is hard-coded constants and `Definition` is the deserialization target. For scope, parameterise `dispatch` over a "document set" (kinds list + target enum) rather than adding `Role`/`RoleBinding` to `KINDS` (RESEARCH: ontology dir must reject them with ONT-V03 and a hint).

**Kind constants** (lines 15-18, 28):
```rust
const KINDS: [&str; 4] = ["ObjectType", "LinkType", "ActionType", "SourceBinding"];
const IMPLEMENTED_KINDS: [&str; 4] = ["ObjectType", "LinkType", "ActionType", "SourceBinding"];
const RESERVED_KINDS: [&str; 3] = ["Interface", "ValueType", "SharedProperty"];
```

**Core rewrite + deserialize** (lines 99-113): copy as is for the scope enum.
```rust
let Value::Object(mut body) = doc.to_value() else { ... };
body.shift_remove("kind");
let mut wrapped = serde_json::Map::new();
wrapped.insert(kind.clone(), Value::Object(body));
serde_path_to_error::deserialize::<_, Definition>(Value::Object(wrapped))
    .map_err(|e| map_error(&e, doc, cx))
```

**Error mapping** (lines 208-286): `map_error` (unknown field -> V04 with `did_you_mean`, missing -> V05, unknown variant -> V03/V06) is reusable unchanged; make it generic over nothing (it only uses `Node`/`Ctx`), so scope loader calls it directly. Diagnostics API: `Diagnostic::error(codes::V03, msg, &loc).with_help(..)` (diag.rs:85,111), codes in `diag.rs::codes` (line 179+, last is `V20`).

IR struct conventions: `#[serde(deny_unknown_fields)]`, camelCase, schemars `JsonSchema`, see `ir/mod.rs` `Definition` doc comment (line ~58: externally tagged after kind rewrite). Follow `ir/binding.rs` for a sibling kind with maps keyed by newtype names (`names.rs` validated newtypes, 100-char alnum apiName).

### `schema_export.rs` + JSON Schema snapshots

**Analog:** `crates/ontoforge-schema/src/schema_export.rs` (lines 12-20, 74-87)

Export-only untagged wrapper plus `rendered_schemas()` returning `(file_name, content)`:
```rust
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum DefinitionFile { One(Box<Definition>), Many(Vec<Definition>) }
...
vec![
    ("ontoforge-definition.schema.json", render(&definition_schema())),
    ("ontoforge-manifest.schema.json", render(&manifest_schema())),
]
```
Add `ScopeFile` + `scope_schema()` + third tuple `ontoforge-scope.schema.json`; extend `FILES` in `crates/ontoforge/tests/cli_schema.rs:6-9` (`const FILES: [&str; 2]`) to 3; commit regenerated `schemas/*.json` (also `PropertyDef.indexed` changes the definition schema). Drift test: `crates/ontoforge-schema/tests/schema_export.rs` (byte compare against `schemas/` plus `jsonschema::validator_for` accepting examples; YAML loaded with `serde_saphyr` `strict_booleans`).

### `ontoforge-expr/src/rowpred/*` (parse -> RowPred IR, typecheck, Kleene eval)

**Analog:** `crates/ontoforge-expr/src/analyze.rs`, `walk.rs`, `infer.rs`

All CEL access must go through `analyze` (prechecks length/nesting, runs on a 32 MiB thread, panics become `ExprError::Internal`). Do not call `cel::Program::compile` directly.

**Entry point** (analyze.rs:65-80):
```rust
pub fn analyze<R, F>(src: &str, f: F) -> Result<R, ExprError>
where F: FnOnce(&cel::Program) -> R + Send, R: Send,
```
Lower the AST to `RowPred` inside the closure and return the owned IR (the program must not escape the thread). Errors: `ExprError::{Syntax{line,column,message}, ParserLimit, Internal}`.

**AST matching** (walk.rs:1-12, 25-40; infer.rs:277-287 per RESEARCH): imports are `use cel::common::ast::operators as op; use cel::common::ast::{EntryExpr, Expr, IdedExpr, LiteralValue, SourceInfo};` Match `op::LOGICAL_AND | op::LOGICAL_OR | op::LOGICAL_NOT | op::IN`, `op::EQUALS | NOT_EQUALS | LESS | LESS_EQUALS | GREATER | GREATER_EQUALS`, method names `"startsWith" | "endsWith" | "contains"`. `walk.rs::step` shows `Expr::Select(sel)` (`sel.field`, `sel.operand`) and `Expr::Call(call)` (`call.func_name`, `call.target`, `call.args`). Reject `Expr::Comprehension`, `Select.test`, `NOT_STRICTLY_FALSE`. Use `member_paths` (walk.rs `MemberPath { segments, pos, dynamic }`) to collect `object.<prop>` references for the criteria coverage lint.

**Do not** reuse `infer.rs::compatible` for typing (it allows int/double mixing; RESEARCH Pattern 6 requires stricter families).

Kleene eval and `Tri` code: take verbatim from RESEARCH "Code Examples / Kleene" (no in-repo analog).

### `printer/ast.rs` + `escape.rs` + `dialect.rs` (StringMatch node, two escape functions)

**Analog:** `crates/ontoforge-engine/src/printer/{ast,escape,dialect}.rs`

ast.rs:6-7 already reserves this: "a `LIKE` node must route its pattern wildcard escaping through the same `escape` exit". Add beside `InList` (ast.rs:44-48):
```rust
StringMatch { expr: Box<Expr>, needle: String, kind: MatchKind },
```
Nodes carry raw values; escaping only at print time.

**Escape exit shape** (escape.rs:9-26) -- new functions must be `pub(super)` here, return `Result<String, PrintError>`, reject NUL the same way:
```rust
pub(super) fn quote_string(s: &str) -> Result<String, PrintError> {
    if s.contains('\0') { return Err(PrintError::NulInString); }
    Ok(format!("'{}'", s.replace('\'', "''")))
}
```
Add `like_escape` (Trino: escape `\ % _`, print `LIKE '<p>' ESCAPE '\'`) and `glob_escape` (SQLite: `[`->`[[]`, `*`->`[*]`, `?`->`[?]`, print `GLOB`); final literal still passes `quote_string`. Code in RESEARCH "Code Examples".

**Printer dispatch** (dialect.rs:110-140) -- add an arm next to `Expr::InList`; note the existing empty-list guard returns `PrintError::EmptyInList` (dialect.rs:129), so `lower` must map empty lists to a constant FALSE before printing.

Tests: extend `crates/ontoforge-engine/src/printer/tests.rs` and the proptest strategy `tricky_string()` in `tests/printer_prop.rs:31-63` (already contains `%`, `_`, `\`) plus add `[`, `*`, `?`, empty, mixed case.

### Differential proptest (Rust eval vs SQLite)

**Analog:** `crates/ontoforge-engine/tests/printer_prop.rs`

**Config with persisted regressions** (lines 21-28):
```rust
fn config() -> ProptestConfig {
    ProptestConfig {
        failure_persistence: Some(Box::new(FileFailurePersistence::WithSource("proptest-regressions"))),
        ..ProptestConfig::default()
    }
}
```
**Runtime + engine** (lines 72-80): `tokio::runtime::Builder::new_current_thread().enable_all().build()`; engine via `SqliteEngine::in_memory()` and `QueryContext::new("prop".to_owned(), Vec::new())`. Regression file convention: `tests/printer_prop.proptest-regressions` committed. Trino side (D-09): fixed corpus + historical counterexamples, `#[ignore]`, env-gated -- copy `trino_it.rs` below.

### Ignored, env-gated integration tests (PG conformance, Trino probe)

**Analog:** `crates/ontoforge-engine/tests/trino_it.rs:1-60`

```rust
const URL_VAR: &str = "ONTOFORGE_TRINO_URL";
fn trino_url(value: Option<String>) -> String {
    match value {
        Some(url) if !url.trim().is_empty() => url,
        _ => panic!("{URL_VAR} is not set: ... (see CONTRIBUTING.md)"),
    }
}
```
Every test `#[ignore]`; ignored test panics (does not silently pass) when the variable is absent. Mirror with `ONTOFORGE_PG_URL` for `conformance_pg.rs`. Write conformance as generic functions in `tests/conformance.rs` called from `conformance_sqlite.rs` (default) and `conformance_pg.rs` (ignored).

### `config.rs` (`[auth]`, `[store]` sections)

**Analog:** `crates/ontoforge-engine/src/config.rs`

**Struct convention** (lines 29-54): `#[derive(Debug, Clone, PartialEq, Eq, Deserialize)] #[serde(deny_unknown_fields)]`, optional sections `#[serde(default)] pub trino: Option<TrinoConfig>`, defaults through named fns (`default_connect_timeout_ms`). Secrets only via `*_env` / `*_file` (file header lines 1-4; `MAX_SECRET_FILE_BYTES` 64 KiB; `SecretString`). Parse errors via `ontoforge_schema::diag::parse_custom_de_error`; file read via `read_limited`/`MAX_FILE_BYTES`. Add `pub auth: Option<AuthConfig>` and `pub store: Option<StoreConfig>` to `ServerConfig` (lines 32-38) and extend `tests/server_config.rs`; CI file `ci/ontoforge-server.ci.toml`.

### `sql_exit.rs` + `sql_exit_guard.rs` (single SQL exit in storage)

**Analog:** `crates/ontoforge-engine/tests/sql_text_only_from_printer.rs` (whole file, 63 lines)

Copy the scanner pattern: self-check sample proving the regex sees what it hunts, `WalkDir` over `src`, skip the exit module, assert `scanned > 0`:
```rust
const KEYWORDS: &str = "SELECT|WITH|INSERT|UPDATE|DELETE|CREATE|DROP|ATTACH|PRAGMA";
let sql_literal = Regex::new(&format!(r#""(?:{KEYWORDS})\b"#)).unwrap();
...
if path.extension().is_none_or(|e| e != "rs") || path.starts_with(&printer) { continue; }
```
Storage version: skip `src/sql_exit.rs`; additionally forbid `AssertSqlSafe`, `OR REPLACE`, `REPLACE INTO` outside/inside (RESEARCH Pitfalls 1 and 9). Storage DDL/DML comes from sea-query, not printer; this file is the only `AssertSqlSafe` wrapper. Existing `AssertSqlSafe` use to imitate: `crates/ontoforge-engine/src/sqlite/mod.rs` (sqlx 0.9 pattern; lines 36-60 show `SqliteConnectOptions::from_str(...).collation(DECIMAL_COLLATION, compare_decimal).collation(TIMESTAMP_COLLATION, compare_timestamp)`).

SQLite connection options for storage: same builder plus `.pragma("recursive_triggers", "ON")`. Wave 0: make `DECIMAL_COLLATION`, `TIMESTAMP_COLLATION` (`printer/sqlite_ddl.rs`) and `compare_decimal`/`compare_timestamp` (`sqlite/mod.rs:209,245`) `pub` and re-export.

### `naming.rs` / `physical.rs` / `ddl.rs` (pure physical schema model)

**Analog:** `crates/ontoforge-engine/src/printer/sqlite_ddl.rs` (type mapping for SQLite fixture tables; exports collation names) and `crates/ontoforge-engine/src/bind_check.rs` (`expected_columns` from IR; drift compare). No existing analog for PG column-type mapping or diff; use RESEARCH Pattern 1/5 type table. Property names validated newtypes from `ontoforge-schema/src/ir/names.rs` (max 100, ASCII alnum); iterate `cx.ontology.object_types.values()` and `object.properties` the way `e13.rs` does:
```rust
for object in cx.ontology.object_types.values() {
    for (name, property) in &object.properties { ... }
}
```

### `auth/scope/lint.rs` (criteria coverage SCOPE-06, collation D-08)

**Analog:** `crates/ontoforge-schema/src/lint/binding/e13.rs` (lines 1-60)

Shape of a rule: unit struct + `pub(super) static RULE`, `impl LintRule { id, default_level, check(&self, cx: &LintCx, out: &mut Vec<Finding>) }`, `Finding { def: DefKey{kind, api_name}, path: Vec<Seg>, part: Part, message, help, severity }`. Registry: `lint/mod.rs:156-164` (`KNOWN_RULE_IDS: [&str; 18]`, `registry()`). Because scope lint needs a Role set, which `LintCx` does not carry, copy the `Finding`/diagnostic structure but expose a standalone function `validate_scope(ontology, profile, server_cfg, roles) -> Vec<Diagnostic>` in auth (RESEARCH tier map). Use `ColumnBinding.collation` (`ir/binding.rs:94-98`) and `ontoforge_expr::member_paths`.

### `authn/jwks.rs` (HTTP fetch with limits)

**Analog:** `crates/ontoforge-engine/src/trino/client.rs` + `trino/headers.rs` (reqwest 0.13 `rustls`, connect timeout, body size cap, `extra_origins`, https-or-loopback policy). Read these before implementing; workspace has no reqwest `json` feature, so use `bytes()` + `serde_json`. Test stubs: `wiremock` as in `crates/ontoforge-engine/tests/trino_client.rs`, `trino_resilience.rs`. JWT code skeleton: RESEARCH "JWT validation skeleton" (algorithm whitelist excluding HS*).

### CLI subcommands (`reload`, `migrate`, `import`, `scope plan|apply`, `dev-token`)

**Analog:** `crates/ontoforge/src/cli.rs`, `lib.rs`, `pipeline.rs`, `online.rs`, `main.rs`

**Clap shape** (cli.rs:19-55): `#[derive(Debug, Subcommand)] pub enum Command { Validate(ValidateArgs), Lint(..), Schema(SchemaArgs) }`; nested subcommand pattern for `scope plan|apply`/`migrate` copies `SchemaArgs`/`SchemaCommand::Export { out_dir: Utf8PathBuf }`. Reuse flatten groups `ProfileArgs` (`--profile`, env `ONTOFORGE_PROFILE`) and `OutputArgs` (`--format`, `--color`, `--deny-warnings`); `--server-config` as in `ValidateArgs` (cli.rs: `#[arg(long, value_name = "FILE")] Option<Utf8PathBuf>`). `--dry-run` for reload; `--rename <Type>.<old>=<new>` as repeatable `Vec<String>` with a value parser.

**Dispatch** (lib.rs:18-24):
```rust
pub fn run(cli: Cli) -> u8 {
    match cli.command {
        Command::Validate(args) => validate(&args),
        Command::Lint(args) => lint(&args),
        Command::Schema(args) => match args.command { SchemaCommand::Export { out_dir } => export_schemas(&out_dir) },
```
Returns `u8` exit code; main wraps in 32 MiB thread (main.rs:10-26). Pre-apply validation reuses `pipeline::check_with_profile(root, profile) -> Checked { out, profile }` (pipeline.rs:25-50): load, profile check, `ontoforge_expr::check_ontology`, `sort_diagnostics`, `has_errors` -> `out.ontology = None` on error. Rendering through `render.rs` (`Format`, `ColorChoice`). Async work (sqlx, Trino) is run in `online.rs`; copy how it builds the runtime and maps engine errors to exit codes.

Tests: `crates/ontoforge/tests/cli_schema.rs` pattern -- `Command::new(env!("CARGO_BIN_EXE_ontoforge")).args(args).env("NO_COLOR","1").output()`, `tempfile::tempdir()`, assert `out.status.code()`; snapshots with `insta::assert_snapshot!("name", stdout)` (cli.rs:88, cli_lint.rs:292). Example dir const: `concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation")`.

### `.github/workflows/ci.yml` (PG conformance step)

**Analog:** `ci.yml` job `integration` (lines 36-end)

The `editstore` database is already created (`CREATE DATABASE demo` + `editstore`), PG service pinned by digest, actions pinned by SHA with tag comment. Add steps:
```yaml
- name: Edit store conformance (PostgreSQL)
  env:
    ONTOFORGE_PG_URL: postgres://postgres:ontoforge-ci-only@localhost:5432/editstore
  run: cargo test -p ontoforge-storage --test conformance_pg --locked -- --ignored
```
and extend the "Ignored tests are actually there" loop (`for target in "ontoforge-engine trino_it" "ontoforge cli_trino_it"`) with `"ontoforge-storage conformance_pg"` so a green run cannot mean nothing ran. Trino reload-probe tests join `trino_it`-style job; add `postgresql.array-mapping=AS_ARRAY` to `ci/trino/catalog/editstore.properties` (RESEARCH Pitfall 7; `demo.properties` already has it). Windows `test` job (`cargo test --workspace --locked`) stays SQLite-only; gate `postgresql_embedded` behind the `embedded-pg` feature. Add a Wave 0 guard step `cargo tree -i openssl` expecting failure.

### Cargo manifests

**Analog:** `crates/ontoforge-engine/Cargo.toml` (workspace-inherited fields) and current `ontoforge-storage/Cargo.toml`:
```toml
[package]
name = "ontoforge-storage"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true
[lints]
workspace = true
[dependencies]
ontoforge-schema = { path = "../ontoforge-schema" }
ontoforge-query = { path = "../ontoforge-query" }   # delete: RESEARCH dependency fix
```
New storage deps: schema, expr, auth, engine. Auth deps: schema, expr, engine. Dependencies versions go in workspace `[workspace.dependencies]` (see RESEARCH Installation block); all CI uses `--locked`, commit `Cargo.lock`.

## Shared Patterns

### Diagnostics
**Source:** `crates/ontoforge-schema/src/diag.rs` (`Diagnostic::error/with_help`, `codes::*`, `sort_diagnostics`, `has_errors`)
**Apply to:** scope loader, `validate_scope`, reload/migrate lint outputs. New codes are added to `diag::codes` and the rule-id list in `lint/mod.rs` (`KNOWN_RULE_IDS`), each with an insta/CLI snapshot.

### Source-located errors
**Source:** `loader/sourcemap.rs` (`DefKey`, `DefKind`, `Seg`, `Part`, `FileId`) and `loader/parse.rs` (`Node`, `Kind`). Scope documents must register in the same source map so `Finding` can point at `rowFilter` lines.

### IR contract
**Source:** `ir/*.rs`: `deny_unknown_fields`, camelCase, `JsonSchema`; every new field lands in JSON Schema + insta snapshot + `schemas/` copy.

### Single SQL exit
**Source:** `printer/escape.rs` (values/identifiers), `tests/sql_text_only_from_printer.rs` (guard). Storage mirrors with `sql_exit.rs`; role predicates only reach SQL through the printer AST (never string concatenation).

### Env/secret references
**Source:** `engine/src/config.rs` (`*_env`, `*_file`, `SecretString`, size caps). Apply to JWKS config, dev key file, PG URL (`url_env`).

### Ignored-but-loud tests
**Source:** `trino_it.rs` `trino_url` panic-if-unset. Apply to every PG and Trino test.

### Proptest persistence
**Source:** `printer_prop.rs:21-28` `FileFailurePersistence::WithSource("proptest-regressions")`, commit the `.proptest-regressions` files.

### Dependency legitimacy
New crates (sea-query, sea-query-sqlx, jsonwebtoken, getrandom, postgresql_embedded) follow RESEARCH Package Legitimacy Audit; add `checkpoint:human-verify` for `sea-query-sqlx` and `getrandom`.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `ontoforge-storage/src/{state,commit,log,history}.rs` | service | CRUD | No write-path/DB-transaction code in repo; use RESEARCH Patterns 2, 3, 10 (`next_state` pure fn + naive reference model) |
| `ontoforge-storage/migrations/{postgres,sqlite}/*.sql` | migration | CRUD | No sqlx migrations exist; use RESEARCH Pattern 4 (triggers, TRUNCATE trigger, `recursive_triggers`) |
| `ontoforge-storage/src/diff.rs` | utility | transform | No schema diff exists; RESEARCH Pattern 5 additive/destructive split |
| `ontoforge-storage/src/reload.rs` (PG txn, `lock_timeout`, advisory lock, fence) | service | batch | First PG code in repo; RESEARCH Pattern 5 and "PG DDL transaction skeleton" |
| `ontoforge-auth/src/scope/merge.rs` (`TypeScope`) | service | transform | New concept; RESEARCH Pattern 7 |
| `ontoforge-auth/src/authn/dev.rs` | service | request-response | No token code; RESEARCH Pattern 8 |

## Metadata

**Analog search scope:** `crates/*`, `.github/workflows`, `ci/`, `schemas/`, `examples/ontologies/aviation`
**Files scanned:** ~110 tracked sources listed; 17 read in detail (`dispatch.rs`, `schema_export.rs`, `escape.rs`, `ast.rs`, `dialect.rs` excerpt, `sql_text_only_from_printer.rs`, `printer_prop.rs` excerpt, `e13.rs`, `config.rs`, `trino_it.rs`, `ci.yml`, `cli.rs`, `lib.rs` excerpt, `pipeline.rs`, `main.rs`, `analyze.rs`, `walk.rs`)
**Pattern extraction date:** 2026-10-19
