---
phase: 01-workspace-schema-lint
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 33
files_reviewed_list:
  - .github/workflows/ci.yml
  - .vscode/settings.json
  - .zed/settings.json
  - CONTRIBUTING.md
  - crates/ontoforge-expr/src/analyze.rs
  - crates/ontoforge-expr/src/check.rs
  - crates/ontoforge-expr/src/infer.rs
  - crates/ontoforge-expr/src/walk.rs
  - crates/ontoforge-expr/tests/analyze.rs
  - crates/ontoforge-expr/tests/criteria.rs
  - crates/ontoforge-schema/src/ir/basetype.rs
  - crates/ontoforge-schema/src/ir/names.rs
  - crates/ontoforge-schema/src/lint/config.rs
  - crates/ontoforge-schema/src/lint/e09.rs
  - crates/ontoforge-schema/src/lint/mod.rs
  - crates/ontoforge-schema/src/lint/structure/mod.rs
  - crates/ontoforge-schema/src/lint/structure/w06.rs
  - crates/ontoforge-schema/src/lint/structure/w07.rs
  - crates/ontoforge-schema/src/lint/structure/w08.rs
  - crates/ontoforge-schema/src/lint/structure/w09.rs
  - crates/ontoforge-schema/src/loader/discover.rs
  - crates/ontoforge-schema/src/loader/mod.rs
  - crates/ontoforge-schema/src/loader/parse.rs
  - crates/ontoforge-schema/src/validate/actions.rs
  - crates/ontoforge-schema/tests/action_types.rs
  - crates/ontoforge-schema/tests/diagnostics.rs
  - crates/ontoforge-schema/tests/loader.rs
  - crates/ontoforge-schema/tests/schema_export.rs
  - crates/ontoforge/src/render.rs
  - examples/ontologies/aviation/actions.yaml
  - examples/ontologies/aviation/links.yaml
  - examples/ontologies/aviation/objects.yaml
  - crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap
findings:
  critical: 0
  warning: 7
  info: 3
  total: 10
status: issues_found
---

# Phase 1: Code Review Report (incremental re-review)

**Reviewed:** 2026-10-16T00:00:00Z
**Depth:** standard
**Files Reviewed:** 33
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

这是增量复审：对象是 `64b5f8b..HEAD` 的改动，即 `01-REVIEW-FIX.md` 记录的 16 个修复，加上 gap-closure plan 01-14（编辑器 `yaml.schemas` 配置、aviation 示例的逐 document modeline、CONTRIBUTING 编辑器一节、`schema_export.rs` 中的两条回归测试）。已有的 `01-REVIEW.md` 内容（CR-01..CR-04、WR-01..WR-12）和 `01-REVIEW-FIX.md` 中已声明的取舍（TOCTOU 残留、W08 改为两两交集的语义变化、诊断总数不设上限、左嵌套三元式的 `Incomplete ExprContext` 消息、ADR 补录建议）没有重复报告。

方法：逐个读 diff，并用 `cargo test --workspace --locked`（全绿）和本机 debug 二进制对可疑点做实测。复现用的 fixture 在 `/tmp` 下，没有改动任何受审文件。

结论：16 个原有发现的修复整体正确，没有发现回归性的 BLOCKER。实测确认下列修复仍有缺口：

1. WR-08 的消息映射漏了 `Documents` 预算，单文件超过 1024 个 document 时用户看到的是 `document exceeds a parser limit (Documents)`。
2. CR-01 / CR-02 的修复把 W07 压成一条，但 W08 仍然是「输出量 = 候选类型数 x 组合数」，而且 `MAX_COMBINATIONS` 不限制输出。
3. WR-12 的修复把「任意位置的非 UTF-8 文件名」都变成了让整个加载失败的 error，包括根本不是定义文件的文件。
4. CR-04 / WR-11 的 `sanitize` 仍放行 `\n`，文件名可以靠它在报告头部伪造行。
5. WR-08 的 `Recursion limit of` 子串匹配会把用户输入回显的普通语法错误误判成嵌套过深。

01-14 的改动本身（两份编辑器配置、modeline、两条测试）逻辑正确，没有发现缺陷；只有两条测试可靠性方面的提示（IN-01、IN-02）。

## Warnings

### WR-01: 单文件超过 1024 个 document 时报 `document exceeds a parser limit (Documents)`，WR-08 的消息映射不完整

**File:** `crates/ontoforge-schema/src/loader/parse.rs:224-246`
**Issue:** `budget_message` 只映射了 `Depth`、`Nodes`、`Aliases`、`Anchors`、`Events`。`serde-saphyr` 2.0.0 的 `Budget::max_documents` 默认是 1024（`budget.rs:292`），超出时 breach 名为 `Documents`，落进 `other =>` 分支。实测：生成一个含 3400 个 object type 的单文件 `defs.yaml`，`ontoforge lint` 得到

```
ONT-V01  document exceeds a parser limit (Documents)   (defs.yaml:19456:1)
```

消息没有说明是 document 数量超限，没有给出上限，也没有修复建议（拆分文件），并且把 Debug 变体名暴露给用户。这正是 WR-08 要消除的那类消息。同样的兜底分支还会对 `ScalarBytes`、`MergeKeys`、`AliasAnchorRatio`、`InclusionDepth` 等其余变体输出 `(ScalarBytes)` 之类的内部名。更实际的影响：一个生成式的大 ontology 放在单个文件里，1025 个 definition 就硬失败。
**Fix:**
```rust
"Documents" => format!(
    "file has more than {} YAML documents; split the definitions over several files",
    budget.max_documents
),
other => "document exceeds a parser limit; simplify or split the file".to_owned(),
```
兜底分支不要再回显 breach 名。并加一条测试：1025 个 `---` 分隔的 document 得到包含 "documents" 且不含 `(Documents)` 的消息。

### WR-02: W08 的输出量仍然是平方级放大，`MAX_COMBINATIONS` 不限制输出

**File:** `crates/ontoforge-schema/src/lint/structure/w08.rs:72-116`
**Issue:** 修复 CR-01 后 CPU 开销受 `MAX_COMBINATIONS = 500` 约束，但每条 finding 的 message 会列出整个类型集合（第 108 行）。当一个大组合（很多类型持有同一组共享属性）与多个小组合相交时，`found` 的每个键都带着大组合的全部类型。实测（`/tmp/rv_gen/gen_a.py 600 400`）：600 个 `Big` 类型各持有 12 个共享属性，400 个 `Sub` 类型各持有其中一个 4 子集，共 1000 个类型、341 KB 输入，得到 621 条 `ONT-W08`，每条列出约 600 个类型名，`--format json` 输出 3.67 MB（W08 消息合计 2.41 MB）。输出量 = 大组合类型数 x 子集数。受 `MAX_FILES x 1024 documents` 约束，输入可以放大到数十万个类型，输出可达数百 MB，这与 CR-02 同类，只是规则换成了 W08。同一个 `found` 的内存占用也随之按「类型数 x 键数」增长。
**Fix:** 与 W07 一致，限制每条 message 列出的类型数并汇总其余：
```rust
const MAX_LISTED: usize = 20;
let shown: Vec<_> = types.iter().take(MAX_LISTED).copied().collect();
// "... and N more" when types.len() > MAX_LISTED
```
同时给 `found` 设上限（例如键数超过 `MAX_COMBINATIONS` 时按「未检查」处理），并避免对每个 `shared` 重复 `extend` 整个大组合：对包含关系用类型计数而不是类型集合表示。

### WR-03: 任意位置的非 UTF-8 名称都会变成 V14 error，即使该文件不可能是定义文件

**File:** `crates/ontoforge-schema/src/loader/discover.rs:68-100`
**Issue:** WR-12 的修复在 `walk` 里对每一个深度大于 0 的文件和目录调用 `Utf8Path::from_path`，失败即报 V14 error（第 89-92 行），不管该文件会不会被 `include` 选中。V14 是 error，加载因此不产出 ontology（`loader.rs` 里的预算测试同样断言 `ontology.is_none()`）。所以 ontology 目录下任何一个 Latin-1 文件名的 PDF、数据文件或截图，都会让 `ontoforge lint` 整体失败；修复前这些文件是被忽略的，不影响结果。另一个副作用：以 `.` 开头且非 UTF-8 的目录（例如 `.\xff-cache`）原本属于隐藏目录应被跳过，现在 `filter_entry` 用的 `is_none_or`（第 75 行）让它通过过滤并被报错。
**Fix:** 只对「可能成为定义文件」的名称报错：比较 `to_string_lossy()` 后的扩展名是否属于 `.yaml`、`.yml`、`.json`，其余非 UTF-8 文件静默忽略；对目录，非 UTF-8 的隐藏目录仍按隐藏目录跳过（用 `to_string_lossy().starts_with('.')` 判断）。如果坚持全部报告，把严重度降为 warning，不要阻断加载。

### WR-04: 按 `Recursion limit of` 子串判断 `ParserLimit`，用户输入回显会造成误判并丢失位置

**File:** `crates/ontoforge-expr/src/analyze.rs:87-94`
**Issue:** `errors.iter().any(|e| e.msg.contains("Recursion limit of"))` 匹配的是 `cel` 错误消息的任意位置，而 `cel` 的语法错误消息会回显出错的 token。实测条件 `params.delayMinutes > 0 && true 'Recursion limit of'`（一个普通的多余 token）得到

```
ONT-V13  invalid CEL expression: expression nests too deeply for the CEL parser; split it into several conditions
```

真实的 `extraneous input` 错误和其 `at 1:N` 位置都被替换掉了，用户会去拆分一个根本不深的表达式。输入内容来自不可信定义文件，所以这是可以由输入诱导的误报，虽然后果只是消息错误。
**Fix:** 锚定到 `cel` 实际的格式，而不是任意子串，例如要求消息以 `Syntax error: OtherError(ParseError` 开头并包含 `msg: "Recursion limit of`：
```rust
fn is_recursion_limit(msg: &str) -> bool {
    msg.starts_with("Syntax error: OtherError(") && msg.contains("msg: \"Recursion limit of")
}
```
并加一条测试：用字符串字面量里含该短语的多余 token 的表达式，期望仍是带位置的 `Syntax` 错误。

### WR-05: `sanitize` 保留 `\n`，文件名可以在报告头部伪造行（CR-04 修复不完整）

**File:** `crates/ontoforge/src/render.rs:79`，`crates/ontoforge/src/render.rs:88-109`
**Issue:** CR-04 把 `sanitize(&d.file)` 传给 `NamedSource::new`，但 `sanitize` 为了源文本保留了 `\t` 和 `\n`（第 99 行的 `'\t' | '\n' => true`）。Linux 与 macOS 的文件名可以含换行，git 也允许提交，所以文件名 `a.yaml\nerror[ONT-V00]: all good` 会被原样放进 miette 的 `╭─[name:line:col]` 头部，形成多行输出，可以伪造一条诊断行或把真实头部挤走。ESC、BEL 已经被替换，但换行和制表符对「一行一个文件名」的场景同样应当被替换。`message` 与 `help` 允许多行是合理的，文件名不是。
**Fix:** 给 `sanitize` 加一个只用于单行字段的变体，文件名和 `code` 用它：
```rust
fn sanitize_line(text: &str) -> String {
    sanitize(text).chars().map(|c| if matches!(c, '\n' | '\r' | '\t') { '?' } else { c }).collect()
}
// NamedSource::new(sanitize_line(&d.file), sanitize(text))
```
`\n`、`\r`、`\t` 都是单字节，替换成 `?` 不改变字节长度。

### WR-06: V07 的 `write it as` 建议可能是 CEL 关键字，下一次运行又被拒绝

**File:** `crates/ontoforge-schema/src/ir/names.rs:79-85`，`crates/ontoforge-schema/src/ir/names.rs:96-118`
**Issue:** WR-03 新增 CEL 关键字检查在 `check` 里，但 `canonical` 生成建议名时只排除了 `RESERVED_API_NAMES`，没有排除 `keywords`。实测属性名 `True` 得到

```
ONT-V07  invalid property apiName `True`: must match ^[a-z][A-Za-z0-9]{0,99}$
  help: write it as `true`
```

用户照做后得到第二条 V07：`true is a CEL keyword`。`Null`、`In`、`For`、`IF` 等同理。
**Fix:** 让 `canonical` 接收 `keywords` 并排除命中的建议；命中时改为不给 `write it as`，或给出 `{out}Value`：
```rust
fn canonical(raw: &str, initial: Initial, keywords: &[&str]) -> Option<String> {
    // ...
    (well_formed(&out, initial)
        && reserved(&out).is_none()
        && !keywords.contains(&out.as_str())
        && out != raw).then_some(out)
}
```

### WR-07: 人类可读输出的渲染开销是「诊断数 x 文件大小」

**File:** `crates/ontoforge/src/render.rs:79`
**Issue:** `to_report` 对每一条诊断都执行 `sanitize(text)`（遍历并重新分配整个文件文本）并把结果交给 `NamedSource::new`。WR-11 把 `sanitize` 改成逐字符的 `peekable` 循环后，每条诊断的固定开销更高。实测：上面 WR-02 的 341 KB fixture 产生 3036 条诊断，debug 构建 `--format json` 用时 1.1 s，默认的人类可读格式用时 34 s。允许的输入可以到 4 MiB 的文件、数万条诊断，渲染时间随两者的乘积增长，而 CR-01 / CR-02 的修复只解决了规则本身的开销。
**Fix:** 每个文件只 sanitize 一次并缓存，每条诊断复用（例如 `HashMap<&str, Arc<String>>`，或者在 `SourceMap` 里存放已 sanitize 的文本）；同样缓存 `NamedSource` 所需的文本克隆，改用 `Arc<str>`。

## Info

### IN-01: 依赖环境能力的测试在无法创建符号链接或非 UTF-8 文件名时静默通过

**File:** `crates/ontoforge-schema/tests/loader.rs:229-233`、`crates/ontoforge-schema/tests/loader.rs:285-291`，`crates/ontoforge-schema/src/lint/config.rs:633-641`
**Issue:** 三条测试在创建符号链接或特殊文件名失败时直接 `return`，测试报告为 ok。CR-03、WR-12 的防护因此在没有权限的环境（没开 Developer Mode 的 Windows 开发机、部分容器）里没有任何覆盖，且看不出被跳过。CI 的 `ubuntu-latest` 和 `windows-latest` 通常能创建，但没有任何机制保证。
**Fix:** 在 CI 中通过环境变量（例如 `ONTOFORGE_REQUIRE_SYMLINK_TESTS=1`）把「创建失败」变成 panic；本地仍然跳过，并用 `eprintln!` 打印一行 `skipped: cannot create symlinks`。

### IN-02: 编辑器配置测试用 `globset` 验证 glob，不等于 yaml-language-server 的匹配语义

**File:** `crates/ontoforge-schema/tests/schema_export.rs:197-215`、`crates/ontoforge-schema/tests/schema_export.rs:259-330`
**Issue:** `editor_configs_associate_definition_schema_with_examples` 用 `globset`（`literal_separator(true)`）判断 `examples/ontologies/**/*.yaml` 与 `!examples/ontologies/**/ontology.yaml` 的覆盖范围。yaml-language-server 使用自己的 glob 实现与「后者覆盖前者」的取反规则，两者对相对模式和 `!` 的处理并不保证一致，所以这条测试证明的是「配置与 globset 一致」，而不是「manifest 在编辑器里真的被排除」。另外 `only_key` 要求每一层对象恰好只有一个键，之后任何人在 `.vscode/settings.json` 或 `.zed/settings.json` 里加别的项目级设置（格式化、`files.associations`）都会让测试失败；CONTRIBUTING 说明了意图，但失败信息只指向「nothing executable」。这是刻意取舍，不是缺陷，仅提示其脆弱性和覆盖范围。
**Fix:** 在 CONTRIBUTING 或测试注释里写明「排除 manifest 的行为由 manifest 自带的 modeline 兜底（modeline 优先），glob 排除只是第一层」，并把 `only_key` 的失败信息改为说明如何有意扩展这两份配置。

### IN-03: W07 组内两两比较没有上限，W08 才有 `MAX_COMBINATIONS`

**File:** `crates/ontoforge-schema/src/lint/structure/w07.rs:103-113`
**Issue:** 同一主键组内对所有类型对做集合交集，规则本身没有类似 W08 的工作量上限。CR-02 修复后输出被压到每簇一条，但比较次数仍是 `n^2/2`。单文件受 1024 个 document 限制时可以忽略；跨文件时受 `MAX_FILES` 与 `MAX_TOTAL_BYTES` 约束，同组类型数理论上可到数十万，比较次数可达 10^10 到 10^11。属于性能类问题，不在 v1 范围，只作为与 W08 处理不一致的提示。
**Fix:** 与 W08 相同，组内成员数超过某个上限（例如 2000）时只报一条「未检查」诊断；或者先按「属性集合的最小哈希」分桶再比较。

---

_Reviewed: 2026-10-16T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard (incremental re-review after 01-REVIEW-FIX + plan 01-14)_
