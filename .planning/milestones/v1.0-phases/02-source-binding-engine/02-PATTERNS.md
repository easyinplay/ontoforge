# Phase 2: Source Binding & Engine - Pattern Map

**Mapped:** 2026-10-16
**Files analyzed:** 27 (新建 / 修改)
**Analogs found:** 15 / 27（engine crate 的 Trino client / printer / SQLite / config 在仓库内无 analog，见 No Analog Found）

所有 analog 路径均已确认为 git 跟踪源（`git ls-files`），无 gitignored 镜像路径。

## File Classification

| 新建 / 修改文件 | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/ontoforge-schema/src/ir/binding.rs` | model (IR) | transform | `crates/ontoforge-schema/src/ir/object.rs` | exact |
| `crates/ontoforge-schema/src/ir/{object,link,manifest,basetype}.rs`（+backing / precision,scale / defaultProfile） | model (IR) | transform | 自身（`ir/object.rs`、`ir/manifest.rs`） | exact |
| `crates/ontoforge-schema/src/ir/mod.rs`（`Definition` 加 `SourceBinding`） | model | transform | 自身 `ir/mod.rs:49-55` | exact |
| `crates/ontoforge-schema/src/loader/dispatch.rs`（+SourceBinding kind） | loader | transform | 自身 `dispatch.rs:16-18,99-114` | exact |
| `crates/ontoforge-schema/src/loader/sourcemap.rs`（+`DefKind::SourceBinding`） | utility | transform | 自身 `sourcemap.rs:16-27` | exact |
| `crates/ontoforge-schema/src/loader/discover.rs`（`bindings/<profile>` 路径规则、跳过 `fixtures`） | utility | file-I/O | 自身 `discover.rs:14-36` | exact |
| `crates/ontoforge-schema/src/validate/bindings.rs`（V16.. 引用 / 完备性） | validator | transform | `crates/ontoforge-schema/src/validate/objects.rs` | exact |
| `crates/ontoforge-schema/src/diag.rs`（codes 追加） | config | transform | 自身 `diag.rs:179-210` | exact |
| `crates/ontoforge-schema/src/lint/bind_*.rs`（ONT-E13.. / W17） | lint rule | transform | `crates/ontoforge-schema/src/lint/e09.rs` | exact |
| `crates/ontoforge-schema/src/lint/mod.rs`（`KNOWN_RULE_IDS`、registry） | config | transform | 自身 `lint/mod.rs:149-161` | exact |
| `crates/ontoforge-schema/src/schema_export.rs` + `schemas/*.json` | utility | transform | 自身 | exact |
| `crates/ontoforge-engine/Cargo.toml`、根 `Cargo.toml` | config | n/a | `crates/ontoforge-storage/Cargo.toml` + 根 `Cargo.toml` | exact |
| `crates/ontoforge-engine/src/bind_check.rs`（`compare_binding`、`DriftReport`） | service (纯函数) | transform | `validate/objects.rs`（Mismatch 到定位的模式） | role-match |
| `crates/ontoforge-engine/src/config.rs`（`ontoforge-server.toml`、`*_env`/`*_file`） | config | file-I/O | `lint/config.rs` + `loader/mod.rs:166 read_limited` | role-match |
| `crates/ontoforge-engine/src/{trino,printer,sqlite,value}.rs`、`lib.rs` | service | request-response / transform | 无 | none |
| `crates/ontoforge/src/cli.rs`（`--profile --online --check-pk --server-config`） | controller (CLI) | request-response | 自身 `cli.rs:45-75` | exact |
| `crates/ontoforge/src/lib.rs`、`online.rs`（新） | controller | request-response | 自身 `lib.rs:17-84`、`pipeline.rs` | role-match |
| `crates/ontoforge/src/render.rs`（退出码 3） | utility | transform | 自身 `render.rs:114-121` | exact |
| `crates/ontoforge/tests/cli_online.rs`（新） | test | request-response | `crates/ontoforge/tests/cli.rs` | exact |
| `crates/ontoforge-engine/tests/{trino_client,printer_prop,sqlite_engine,trino_it}.rs` | test | request-response | `crates/ontoforge-schema/tests/*.rs`（仅框架） | partial |
| `crates/ontoforge-schema/tests/binding.rs`（新） | test | transform | `crates/ontoforge-schema/tests/object_types.rs` | exact |
| `examples/ontologies/aviation/{objects,links,ontology}.yaml` + `bindings/dev/*` | config | file-I/O | 自身 `objects.yaml`、`ontology.yaml` | exact |
| `examples/ontologies/invalid/*` 新样例 | test fixture | n/a | 既有 `invalid/v04-unknown-field` | exact |
| `.github/workflows/ci.yml`、`nightly.yml` | config | batch | 自身 `ci.yml` | role-match |
| `.planning/adr/0021-*.md` | doc | n/a | `.planning/adr/0015-source-binding-and-type-contract.md` | role-match |

## Pattern Assignments

### `crates/ontoforge-schema/src/ir/binding.rs`（model, transform）

**Analog:** `crates/ontoforge-schema/src/ir/object.rs`

**Imports + 派生约定**（lines 3-13）：
```rust
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::basetype::BaseType;
use super::names::{ObjectTypeName, PropertyName};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ObjectType {
    /// Unique `PascalCase` name used in code, APIs and references.
    pub api_name: ObjectTypeName,
    pub display_name: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub properties: IndexMap<PropertyName, PropertyDef>,
}
```
规则：每个字段带 `///` 文档注释（会进 JSON Schema）；`deny_unknown_fields` + camelCase；可选用 `Option<T>`，集合用 `#[serde(default)]`；属性映射用 `IndexMap`（保序）。`columns` 总是对象形式（RESEARCH Pattern 1），不要 untagged enum。

**`Definition` 加变体**（`ir/mod.rs:49-55`）：
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(tag = "kind")]
pub enum Definition {
    ObjectType(ObjectType),
    LinkType(LinkType),
    ActionType(ActionType),
}
```
追加 `SourceBinding(SourceBinding)`。profile 来自目录，不进文档字段。

### `ir/object.rs` / `ir/link.rs` 的 `backing`、`PropertyDef.precision/scale`、`ir/manifest.rs` 的 `defaultProfile`

**Analog:** 自身。`Manifest`（`ir/manifest.rs:8-15`）直接加 `pub default_profile: Option<String>`；`PropertyDef`（`object.rs:44-59`）加 `precision: Option<u32>`、`scale: Option<u32>`，格式照 `items: Option<BaseType>` 的写法（文档注释 + Option）。`backing` 用 `enum Backing { Source, EditOnly }`，`#[serde(rename_all = "camelCase")]`，字段必填（无 `#[serde(default)]`）。注意：必填会让 24 个 invalid 样例、aviation / aviation-json、13 个 Rust 文件的内联 YAML 出现 `ONT-V05`，由 RESEARCH 的 Plan A 单独迁移。

### `loader/dispatch.rs` + `loader/sourcemap.rs` + `loader/discover.rs`（新 kind 接入）

**Analog:** 自身。

**kind 表**（`dispatch.rs:15-18`）：
```rust
const KINDS: [&str; 3] = ["ObjectType", "LinkType", "ActionType"];
const IMPLEMENTED_KINDS: [&str; 3] = ["ObjectType", "LinkType", "ActionType"];
```
两个数组长度都要从 3 改 4。

**kind 重写为外部 tag 再反序列化**（`dispatch.rs:99-113`）：
```rust
let Value::Object(mut body) = doc.to_value() else { /* V02 */ };
body.shift_remove("kind");
if kind == "ActionType" { rewrite_rules(&mut body, cx, doc)?; }
let mut wrapped = serde_json::Map::new();
wrapped.insert(kind.clone(), Value::Object(body));
serde_path_to_error::deserialize::<_, Definition>(Value::Object(wrapped))
    .map_err(|e| map_error(&e, doc, cx))
```
`SourceBinding` 不需要 `rewrite_rules`，原样走通用路径即可，错误路径与 line:col 自动保留。

**DefKind / DefKey**（`sourcemap.rs:16-27`）：
```rust
pub enum DefKind { ObjectType, LinkType, ActionType }
pub struct DefKey { pub kind: DefKind, pub api_name: String }
```
加 `DefKind::SourceBinding`，`api_name = "{profile}/{target}"`（如 `dev/Flight`）；`register_def` 是 `pub(crate)`。

**discover**（`discover.rs:14-17`）：`DEFAULT_INCLUDE = ["**/*.yaml","**/*.yml","**/*.json"]`、`SKIPPED_DIRS = ["target","node_modules"]`。加入 `"fixtures"` 到跳过集合（否则 `fixtures/*.json` 被当定义文档报 V02）；`bindings/<profile>/` 路径解析 profile 名，限制 `[a-z][a-z0-9_-]{0,31}`，profile 只作 map key，不拼路径。

### `validate/bindings.rs`（validator, transform）

**Analog:** `crates/ontoforge-schema/src/validate/objects.rs`

**定位闭包 + 诊断构造**（lines 12-36, 38-61）：
```rust
pub fn check(ontology: &Ontology, sources: &SourceMap, out: &mut Vec<Diagnostic>) {
    for object in ontology.object_types.values() {
        let key = DefKey { kind: DefKind::ObjectType, api_name: object.api_name.to_string() };
        let at = |path: &[&str], part: Part| {
            let path: Vec<Seg> = path.iter().map(|s| Seg::Key((*s).to_owned())).collect();
            sources.locate(&key, &path, part)
        };
        primary_key(object, &at, out);
    }
}
type At<'a> = dyn Fn(&[&str], Part) -> SourceLocation + 'a;

out.push(Diagnostic::error(
    codes::V10,
    format!("primaryKey `{pk}` is not a property of `{}`", object.api_name),
    &at(&["primaryKey"], Part::Value),
));
// 带建议：d.with_help(format!("did you mean `{s}`?")) via did_you_mean(name, candidates)
```
在 `validate/mod.rs` 注册新 `bindings::check`。引用不存在用 `codes::V09`，重复绑定用 `V08`，新增码接在 `diag.rs` 的 V15 之后（见下）。

### `diag.rs` 诊断码

**Analog:** `diag.rs:179-210`，格式 `/// 说明` + `pub const V16: &str = "ONT-V16";`，码永不复用。在线诊断码（D-14）同样从 V16 之后分配。

### `lint/bind_*.rs`（lint rule, transform）

**Analog:** `crates/ontoforge-schema/src/lint/e09.rs`

**规则骨架**（lines 14-48）：
```rust
use super::{Finding, Level, LintCx, LintRule};
pub(super) struct E09;
pub(super) static RULE: E09 = E09;

impl LintRule for E09 {
    fn id(&self) -> &'static str { "ONT-E09" }
    fn default_level(&self) -> Level { Level::Error }
    fn check(&self, cx: &LintCx<'_>, out: &mut Vec<Finding>) {
        for object in cx.ontology.object_types.values() {
            let def = DefKey { kind: DefKind::ObjectType, api_name: object.api_name.to_string() };
            // Finding { def, path: vec![Seg::Key(..)], part: Part::Value, message, help, severity }
        }
    }
}
```
一个规则 id 可同时带两种 severity（`Some(Severity::Warning)` 覆盖默认）。绑定类 lint 的 `DefKey` 用 `DefKind::SourceBinding`。

**注册**（`lint/mod.rs:149-161`）：
```rust
pub const KNOWN_RULE_IDS: [&str; 13] = [ "ONT-E08", "ONT-E09", "ONT-W01", /* ... */ "ONT-W11" ];
pub fn registry() -> Vec<&'static dyn LintRule> {
    let mut rules: Vec<&'static dyn LintRule> = vec![&e08::RULE, &e09::RULE];
    rules.extend(naming::rules());
    rules.extend(structure::rules());
    rules
}
```
新增 id 从 `ONT-E13`、`ONT-W17` 起（E10..E12、W12..W16 被 foundry spec 占用），同步改数组长度和 `registry_ids_are_known_and_unique` 测试。每条规则配 `snapshots/*__clean.snap` / `__triggers.snap` insta 快照，沿用 `lint/snapshots/` 命名。

### `ontoforge-engine/Cargo.toml` 与根 `Cargo.toml`

**Analog:** `crates/ontoforge-storage/Cargo.toml`
```toml
[package]
name = "ontoforge-storage"
description = "StorageProvider trait and backends"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
ontoforge-schema = { path = "../ontoforge-schema" }
```
engine 的 `[dependencies]` 用 `xxx.workspace = true`，新依赖加到根 `Cargo.toml` 的 `[workspace.dependencies]`（`members = ["crates/*"]` 自动纳入新 crate）。engine 依赖 `ontoforge-schema`，不依赖 `ontoforge-query`（CONTEXT）。workspace lints 有 `unsafe_code = "forbid"` 与 clippy pedantic，新 crate 必须 `[lints] workspace = true`。

### `ontoforge-engine/src/config.rs` + 密钥 / CA 文件读取

**Analog（读文件）:** `loader/mod.rs:166-192 read_limited`
```rust
pub(crate) fn read_limited(path: &std::path::Path, follow_links: bool) -> Result<Vec<u8>, ReadFailure> {
    let meta = if follow_links { std::fs::metadata(path) } else { std::fs::symlink_metadata(path) }
        .map_err(|e| ReadFailure::Io(e.kind()))?;
    if !meta.file_type().is_file() { return Err(ReadFailure::NotRegular); }
    if meta.len() > MAX_FILE_BYTES { return Err(ReadFailure::TooLarge(Some(meta.len()))); }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|e| ReadFailure::Io(e.kind()))?;
    if bytes.len() as u64 > MAX_FILE_BYTES { return Err(ReadFailure::TooLarge(None)); }
    Ok(bytes)
}
```
是 `pub(crate)`，engine 不能直接用。engine 里写同形的 `read_secret_file`：`follow_links = true`（`*_file` / `ca_file` 常为 k8s secret 符号链接，RESEARCH Pitfall 11）、上限 64 KiB、去尾换行、错误只含路径不含内容。toml 解析与未知键校验参照 `lint/config.rs`（`parse_config` / `load_config`）；配置值错误用有位置的诊断而非 panic。环境变量值由调用方以参数传入（`set_var` 被禁，库内不得读 `ONTOFORGE_PROFILE`）。

### `ontoforge-engine/src/bind_check.rs`（纯函数 `compare_binding`）

**Analog:** `validate/objects.rs` 的 “IR + 定位闭包 -> `Vec<Diagnostic>`” 形态，加 `did_you_mean` 建议。`Mismatch` 带 `Seg` 路径，CLI 侧用 `SourceMap::locate(&DefKey{kind: SourceBinding,..}, &path, Part::Value)` 映射成 `Diagnostic`；类型侧 `BaseType` 来自 `ir/basetype.rs`（`pk_suitability` 等函数的 match 风格可参考）。远端字符串（列名、`error.message`）必须经 `render::sanitize`（`render.rs`）输出，不得 `println!` 直出。

### `crates/ontoforge/src/cli.rs`（CLI args）

**Analog:** 自身 `cli.rs:56-75`
```rust
#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Ontology directory containing `ontology.yaml`.
    pub dir: Utf8PathBuf,
    #[command(flatten)]
    pub output: OutputArgs,
}
```
在 `ValidateArgs` 上加 `#[arg(long)] pub profile: Option<String>`、`#[arg(long)] pub online: bool`、`#[arg(long, requires = "online")] pub check_pk: bool`、`#[arg(long)] pub server_config: Option<Utf8PathBuf>`。`--config` 在 `LintArgs`（`cli.rs:49-51`）的 `Option<Utf8PathBuf>` 写法可抄。

### `crates/ontoforge/src/lib.rs` + `online.rs`（编排）与退出码

**Analog:** `lib.rs:60-75` 与 `pipeline.rs:9-23`
```rust
fn validate(args: &ValidateArgs) -> u8 {
    let out = pipeline::check(&args.dir);
    let code = report(&out, &args.output);
    if args.output.format == Format::Human { /* eprintln summary */ }
    code
}
// pipeline::check: load_dir -> ontoforge_expr::check_ontology -> extend + sort_diagnostics -> has_errors => ontology=None
```
online 路径沿用 `pipeline::check` 之后追加在线诊断，再 `sort_diagnostics`；`report()`（`lib.rs:46-58`）渲染 human / JSON 不变。异步 runtime 只在 online 分支内建 `current_thread`。`run()` 返回 `u8`：退出码 3（EngineUnavailable / 认证失败）需要在 `render::exit_code`（`render.rs:114-121`，当前 `u8::from(failed)`）之外单独产生，不要改动该函数既有语义，新增一个独立函数或在 `online.rs` 中直接返回 3。`profile` 解析写成纯函数 `resolve_profile(flag, env, manifest) -> Option<&str>`，env 值由 `main.rs` 读取后传入。

### `crates/ontoforge/tests/cli_online.rs`（test）

**Analog:** `crates/ontoforge/tests/cli.rs:1-40`
```rust
const AVIATION: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation");
fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge")).args(args).env("NO_COLOR", "1").output().expect("spawn ontoforge")
}
fn json_diagnostics(dir: &str) -> (Output, Vec<serde_json::Value>) { /* --format json, parse stdout */ }
// 断言: out.status.code(), first["code"], first["file"], first["line"], first["column"]
```
环境变量只经 `.env("ONTOFORGE_PROFILE", ..)` 传给子进程，禁止 `set_var`。

### `crates/ontoforge-schema/tests/binding.rs` 与 `examples/ontologies/invalid/*`

**Analog:** `crates/ontoforge-schema/tests/object_types.rs`、`crates/ontoforge/tests/invalid_examples.rs`（一目录一错误码的快照模式）；新增绑定错误样例按 `invalid/vNN-description/` 命名。

### `examples/ontologies/aviation/*`

**Analog:** 自身。`ontology.yaml` 现为：
```yaml
# yaml-language-server: $schema=../../../schemas/ontoforge-manifest.schema.json
apiName: aviation
displayName: Aviation
description: ...
```
加 `defaultProfile: dev`。`objects.yaml` 每个 `kind: ObjectType` 文档加 `backing: source`，并保留 `# yaml-language-server: $schema=` 头。`bindings/dev/*.yaml` 同样首行加 `$schema=../../../../schemas/ontoforge-definition.schema.json`（深一层目录）。

风险：aviation 的 `Airport.location` 是 `geopoint`，而 D-13 / BIND-02 规定 geopoint 不支持 source 绑定（报 E）；planner 需决定 Airport 为 `editOnly`，或绑定里不映射 geopoint 列（RESEARCH Open Question 4）。

### `.github/workflows/ci.yml`

**Analog:** 自身（现有 job）
```yaml
- uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
- uses: dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38b36d230d1e12f # v1
  with: { toolchain: "1.98.0", components: "clippy, rustfmt" }
- uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
- run: cargo test --workspace --locked
```
新增 Linux Trino 作业：第三方 action 一律钉完整 commit SHA + tag 注释；`permissions: contents: read`；命令带 `--locked`。Trino 用 checkout 后 `docker run -v "$PWD/ci/trino/catalog:/etc/trino/catalog"`（services 不能挂仓库文件），PG 用 `services:`；集成测试 `-- --ignored`，缺环境变量时 `panic!`。

## Shared Patterns

### 诊断与位置
**Source:** `crates/ontoforge-schema/src/diag.rs`（`Diagnostic::error(code, msg, &loc)`、`.with_help`、`did_you_mean`、`sort_diagnostics`、`has_errors`）
**Apply to:** 所有新增 validate / lint / online 诊断。诊断码永不复用，错误不 `println!`，经 `render.rs` 的 human / JSON 渲染与 `sanitize`。

### IR 唯一契约 + 快照
**Source:** `ir/*.rs` 的 `deny_unknown_fields, rename_all = "camelCase"` + `schema_export.rs`
**Apply to:** 所有新字段 / 新 kind。必须进 IR 与导出的 JSON Schema（`schemas/*.json` 入库、`tests/schema_export.rs` 校验），并有 insta 快照；`Definition` 的 `#[schemars(tag = "kind")]` 保证编辑器 `oneOf`。

### 文件读取安全
**Source:** `loader/mod.rs:166 read_limited`（4 MiB 上限、拒绝符号链接与非常规文件、有界读取）
**Apply to:** 绑定 / fixture / 服务配置读取。定义与绑定文件沿用 `follow_links=false`；`*_file` 密钥与 `ca_file` 例外允许跟随链接（Pitfall 11）。

### 测试环境变量
**Source:** `CONTRIBUTING.md:27` 与 `tests/cli.rs`（`Command::env`）
**Apply to:** 所有涉及 `ONTOFORGE_PROFILE` 或密钥环境变量的测试。库函数以参数接收环境变量值。

### Workspace lint 基线
**Source:** 根 `Cargo.toml` `[workspace.lints]`（`unsafe_code = "forbid"`、clippy pedantic，`must_use_candidate` 等已 allow）
**Apply to:** 新 crate 必须 `[lints] workspace = true`；`cargo clippy --workspace --all-targets --locked -- -D warnings` 是 CI 门。

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/ontoforge-engine/src/trino/*`（client、headers、decode、error） | service | request-response / streaming | 仓库内无 HTTP / 异步代码；用 RESEARCH Pattern 4 与 `.planning/research/spike-01-trino-client.md`，wiremock 做测试 |
| `crates/ontoforge-engine/src/printer/*`（ast、escape、dialect、`PrintedSql`） | utility | transform | 仓库内无 SQL 生成；用 RESEARCH Pattern 3 / 5 与 `spike-02-sqlparser-l2.md`，proptest 做性质测试 |
| `crates/ontoforge-engine/src/sqlite/*`（ATTACH、fixture 加载） | service | CRUD / file-I/O | 无 sqlx 使用先例（`ontoforge-storage` 当前仅依赖 schema / query）；用 RESEARCH Pitfall 1、2（必须 `sqlx`，`max_connections(1)`，`PRAGMA query_only`） |
| `crates/ontoforge-engine/src/{value,lib}.rs`（`Engine` trait、`EngineError`、`Value`、`PhysicalType`） | model | request-response | 无 trait 抽象先例；用 RESEARCH「Engine trait 与错误分类」代码示例。错误类型惯例用 `thiserror`（已在 workspace） |
| `.github/workflows/nightly.yml` | config | batch | 现有仅一个 `ci.yml`，无定时作业 |
| `ci/trino/catalog/*.properties`、`ci/pg/seed.sql` | config | batch | 无先例 |

## Metadata

**Analog search scope:** `crates/ontoforge-schema/src/{ir,loader,validate,lint}`、`crates/ontoforge/{src,tests}`、`crates/ontoforge-storage`、根 `Cargo.toml`、`.github/workflows`、`examples/ontologies/aviation`
**Files scanned:** 约 25（`git ls-files` 全量清单 + 逐一读取关键 analog）
**Pattern extraction date:** 2026-10-16
