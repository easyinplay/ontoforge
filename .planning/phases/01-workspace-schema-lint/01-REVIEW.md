---
phase: 01-workspace-schema-lint
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 68
files_reviewed_list:
  - .github/workflows/ci.yml
  - Cargo.toml
  - crates/ontoforge-action/Cargo.toml
  - crates/ontoforge-action/src/lib.rs
  - crates/ontoforge-api/Cargo.toml
  - crates/ontoforge-api/src/lib.rs
  - crates/ontoforge-auth/Cargo.toml
  - crates/ontoforge-auth/src/lib.rs
  - crates/ontoforge-client/Cargo.toml
  - crates/ontoforge-client/src/lib.rs
  - crates/ontoforge-expr/Cargo.toml
  - crates/ontoforge-expr/src/analyze.rs
  - crates/ontoforge-expr/src/check.rs
  - crates/ontoforge-expr/src/infer.rs
  - crates/ontoforge-expr/src/lib.rs
  - crates/ontoforge-expr/src/limits.rs
  - crates/ontoforge-expr/src/walk.rs
  - crates/ontoforge-query/Cargo.toml
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-schema/Cargo.toml
  - crates/ontoforge-schema/src/diag.rs
  - crates/ontoforge-schema/src/ir/action.rs
  - crates/ontoforge-schema/src/ir/basetype.rs
  - crates/ontoforge-schema/src/ir/link.rs
  - crates/ontoforge-schema/src/ir/manifest.rs
  - crates/ontoforge-schema/src/ir/mod.rs
  - crates/ontoforge-schema/src/ir/names.rs
  - crates/ontoforge-schema/src/ir/object.rs
  - crates/ontoforge-schema/src/lib.rs
  - crates/ontoforge-schema/src/lint/config.rs
  - crates/ontoforge-schema/src/lint/e08.rs
  - crates/ontoforge-schema/src/lint/e09.rs
  - crates/ontoforge-schema/src/lint/mod.rs
  - crates/ontoforge-schema/src/lint/naming/mod.rs
  - crates/ontoforge-schema/src/lint/naming/w01.rs
  - crates/ontoforge-schema/src/lint/naming/w02.rs
  - crates/ontoforge-schema/src/lint/naming/w03.rs
  - crates/ontoforge-schema/src/lint/naming/w04.rs
  - crates/ontoforge-schema/src/lint/naming/w05.rs
  - crates/ontoforge-schema/src/lint/structure/mod.rs
  - crates/ontoforge-schema/src/lint/structure/w06.rs
  - crates/ontoforge-schema/src/lint/structure/w07.rs
  - crates/ontoforge-schema/src/lint/structure/w08.rs
  - crates/ontoforge-schema/src/lint/structure/w09.rs
  - crates/ontoforge-schema/src/lint/structure/w10.rs
  - crates/ontoforge-schema/src/lint/structure/w11.rs
  - crates/ontoforge-schema/src/lint/words.rs
  - crates/ontoforge-schema/src/loader/discover.rs
  - crates/ontoforge-schema/src/loader/dispatch.rs
  - crates/ontoforge-schema/src/loader/mod.rs
  - crates/ontoforge-schema/src/loader/parse.rs
  - crates/ontoforge-schema/src/loader/sourcemap.rs
  - crates/ontoforge-schema/src/schema_export.rs
  - crates/ontoforge-schema/src/validate/actions.rs
  - crates/ontoforge-schema/src/validate/links.rs
  - crates/ontoforge-schema/src/validate/mod.rs
  - crates/ontoforge-schema/src/validate/objects.rs
  - crates/ontoforge-server/Cargo.toml
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-storage/Cargo.toml
  - crates/ontoforge-storage/src/lib.rs
  - crates/ontoforge/Cargo.toml
  - crates/ontoforge/src/cli.rs
  - crates/ontoforge/src/lib.rs
  - crates/ontoforge/src/lint_cmd.rs
  - crates/ontoforge/src/main.rs
  - crates/ontoforge/src/pipeline.rs
  - crates/ontoforge/src/render.rs
findings:
  critical: 4
  warning: 12
  info: 10
  total: 26
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-10-16T00:00:00Z
**Depth:** standard
**Files Reviewed:** 68
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

审查范围是 Phase 1 的 68 个文件：workspace 骨架、`ontoforge-schema`（IR、loader、validate、lint）、`ontoforge-expr`（CEL 静态检查）和 `ontoforge` CLI。审查方式是逐文件通读，并对可疑点用本机构建的 `target/debug` 与 `target/release` 二进制做实测复现。复现用的 fixture 放在仓库之外的临时目录，未修改任何源文件。

整体结构清晰：IR 与 SourceMap 分离，loader 在 32 MiB 线程中运行并捕获 panic，YAML 有 depth/node/alias budget，CEL 有长度与括号深度预检。实测中，`a.a.a...`、`x[0][0]...`、`1+1+...`、`!!!!...` 等深链 CEL 表达式在 debug 构建下都没有造成栈溢出（`cel` 0.15 自带 96 层递归上限），YAML alias bomb 也被 `max_nodes` 预算拦截。

主要问题集中在以下几处：

1. 两条 structure lint 规则在不可信输入上是超线性的。实测 196 KB 的定义文件让 release 构建的 `ontoforge lint` 运行 67 秒，并输出 215 MB 的 JSON。
2. manifest 和 `ontoforge.toml` 的读取会跟随符号链接，并且可以绕过大小限制。
3. 终端转义过滤漏掉了文件名。
4. CEL 静态检查存在几处可证实的漏检（混合数值运算、下标访问）。另外，参数命名允许 CEL 关键字，这属于 one-way 契约问题。

## Critical Issues

### CR-01: ONT-W08 三重循环导致 CPU 拒绝服务（立方复杂度）

**File:** `crates/ontoforge-schema/src/lint/structure/w08.rs:42-74`
**Issue:** 第 40-41 行的注释声称用「只保留被至少三个类型持有的 key」缓解了 T-01-35，但这个过滤只排除罕见属性。只要每个类型都带有同样 3 个常见属性（`createdAt`、`updatedAt`、`ownerName` 这类审计字段在真实 ontology 中很普遍），所有类型都会进入 `candidates`，第 57-73 行的三重循环就退化为 O(n^3)，并且每次迭代都会分配一个 `BTreeSet`。实测（每个类型 5 行 YAML，主键 `id` 加 3 个共享属性）：

- debug 构建：200 个类型 3.6 s，400 个类型 27.5 s（关闭 W08 后只需 1.2 s）
- release 构建：1000 个类型（196 KB，单文件，远低于 4 MiB 上限）67 s

按立方增长，约 1 MB 的单个文件（约 5000 个类型）可以让 lint 运行数小时。对处理不可信定义文件的 CLI（例如在 CI 上对 PR 跑 `ontoforge lint`），这是可利用的可用性漏洞。
**Fix:** 改成按两两交集聚合，把复杂度降到 O(n^2) 并设工作量上限：
```rust
// shared set -> types holding it, built from pairs only
let mut found: BTreeMap<BTreeSet<Key<'_>>, BTreeSet<&str>> = BTreeMap::new();
for (i, (a, set_a)) in candidates.iter().enumerate() {
    for (b, set_b) in &candidates[i + 1..] {
        let ab: BTreeSet<Key<'_>> = set_a.intersection(set_b).copied().collect();
        if ab.len() >= MIN_SHARED {
            found.entry(ab).or_default().extend([a.api_name.as_str(), b.api_name.as_str()]);
        }
    }
}
found.retain(|_, types| types.len() >= MIN_TYPES);
```
另外加一个硬上限：例如 candidates 超过 500 个时只报告一次「too many object types share properties; rule skipped」，或只统计 `counts[key] >= MIN_TYPES` 的 key 组合并直接按 key 集合分组。

### CR-02: ONT-W07 两两报告，输出量平方级放大

**File:** `crates/ontoforge-schema/src/lint/structure/w07.rs:71-113`
**Issue:** `similar_pairs` 对同组内的每一对相似类型各报告一次。实测 release 构建、1000 个类型（196 KB 输入）产生 499,500 条 `ONT-W07` 警告，`--format json` 输出 214,736,778 字节，约放大 1000 倍。人类可读格式下还要对每条诊断做 miette 渲染。触发条件也很现实：第 81-85 行构建的属性集合包含主键本身，而分组又要求主键同名同型（第 87 行），所以主键在每一对中必然共享，相似度被抬高。5 个属性的小类型只要带 3 个审计字段，Jaccard 就是 4/6 = 0.67 ≥ 0.6，会被两两全部报告。
**Fix:** (1) 计算 Jaccard 前从两个集合中去掉主键；(2) 用 union-find 把相似类型合并成簇，每簇只报告一条（列出成员）；(3) 给单条规则设诊断数上限，超出时汇总成一条。
```rust
let set: BTreeSet<Key<'_>> = object.properties.iter()
    .filter(|(name, _)| **name != object.primary_key)
    .map(|(n, d)| (n.as_str(), d.base_type.as_str()))
    .collect();
// ... union-find over similar pairs, then one Finding per cluster
```

### CR-03: manifest 与 `ontoforge.toml` 跟随符号链接，大小检查可被绕过（OOM 与信息泄露）

**File:** `crates/ontoforge-schema/src/loader/mod.rs:137-163`；`crates/ontoforge-schema/src/lint/config.rs:325-343`
**Issue:** `discover::walk` 明确声明 "Symbolic links are neither followed nor listed"（discover.rs:47-48），但 `ontology.yaml` 不经过 walk，而是由 `read_text` 读取：先 `std::fs::metadata`（跟随链接），再 `std::fs::read`。`load_config` 读取 `<root>/ontoforge.toml` 时同样如此。git 可以提交符号链接，因此一个不可信仓库可以造成两类问题：
- `ontology.yaml -> /dev/zero`（或 `/dev/urandom`）：字符设备的 `metadata().len()` 为 0，能通过 `MAX_FILE_BYTES` 检查，随后 `std::fs::read` 无限读取直到 OOM；
- `ontology.yaml -> ~/.ssh/id_rsa`、`/proc/self/environ` 等：内容会被解析，语法错误诊断的 miette 片段会把目标文件内容打印到 CI 日志。
**Fix:** 用 `symlink_metadata` 拒绝非普通文件，并按上限读取：
```rust
let meta = std::fs::symlink_metadata(&path).map_err(|e| e.kind())?;
if !meta.file_type().is_file() {
    return Err(/* V14: `{name}` is not a regular file (symbolic links are not followed) */);
}
let mut buf = Vec::new();
std::fs::File::open(&path)?.take(MAX_FILE_BYTES + 1).read_to_end(&mut buf)?;
if buf.len() as u64 > MAX_FILE_BYTES { /* V14 over limit */ }
```
`load_config` 中对根目录 `ontoforge.toml` 做同样处理（CLI 显式传入的 `--config` 可以继续跟随链接）。

### CR-04: 文件名未经过终端控制字符过滤（终端转义注入）

**File:** `crates/ontoforge/src/render.rs:77`
**Issue:** `sanitize` 的注释说明了威胁模型（"so a definition file cannot drive the reader's terminal"），message、help 和源文本都做了过滤，但 `NamedSource::new(d.file.clone(), ...)` 的名字原样传给 miette。miette 7.6 在 `handlers/graphical.rs:683-694` 中把 `source_name` 直接写进 `╭─[name:line:col]` 头部，自身不做转义。Linux 和 macOS 的文件名可以包含 ESC、BEL 等字节，git 也允许提交这类路径，例如名为 `\x1b]52;c;<base64>\x07.yaml` 的定义文件。这样的文件只要触发任意一条带位置的诊断，就会向读者终端写入 OSC 52（写剪贴板）、改标题、清屏等序列，用来隐藏真实错误或污染剪贴板。
**Fix:**
```rust
Report::new(diag).with_source_code(NamedSource::new(sanitize(&d.file), sanitize(text)))
```
同时对 `with_code(d.code.clone())` 也做 `sanitize`（code 当前不可由输入控制，属于纵深防御）。在 `discover::walk` 中可以考虑对含控制字符的文件名直接报 V14 并跳过。

## Warnings

### WR-01: CEL 推断把 int 与 double 的混合算术视为 double，但 `cel` 0.15 运行时不支持

**File:** `crates/ontoforge-expr/src/infer.rs:295-303, 319-325`
**Issue:** `arithmetic` 对 `(Int, Double)` 返回 `Ty::Double`。`cel` 0.15 的 `impl ops::Add/Sub/Mul... for Value`（objects.rs:1903 起）只实现了同类型分支，`Int` 与 `Float` 混合会落到 `ExecutionError::UnsupportedBinaryOperator`（混合类型的比较是支持的，算术不支持）。实测条件 `params.count * 1.5 > 2.0`（`count: integer`）能通过 `validate`，但 Phase 3 求值时必然失败。ADR-0006 要求「比较两侧在可推断时类型兼容」，这里的错误推断让一个必然失败的表达式通过了检查。
**Fix:** 在 `call()` 的算术分支中，两侧都是数值但类型不同时记一条 issue（`cannot apply * to int and double; convert with double(...)`），结果类型返回 `Ty::Any`：
```rust
fn arithmetic(left: &Ty, right: &Ty) -> Result<Ty, ()> {
    match (left, right) {
        (Ty::Int, Ty::Int) => Ok(Ty::Int),
        (Ty::Double, Ty::Double) => Ok(Ty::Double),
        (l, r) if l.is_numeric() && r.is_numeric() => Err(()),
        _ => Ok(Ty::Any),
    }
}
```

### WR-02: 下标访问绕过参数和属性的存在性检查

**File:** `crates/ontoforge-expr/src/walk.rs:74-79`；`crates/ontoforge-expr/src/check.rs:135-174`
**Issue:** `walk` 只收集 `Select` 链，`params['typo']`、`objects.flight['nope']` 会被解析成 `_[_]` 的 `Call`，最终只产生根路径 `["params"]`，命中 `("params" | "objects" | "user", []) => None`。实测 `params['typo'] > 0` 能通过 validate，打错的参数名要到运行时才暴露，ADR-0006 中「`params.<name>` 必须是已声明参数」的规则可以被绕开。
**Fix:** 在 `Walker::visit` 中识别 `func_name == operators::INDEX`、目标是空闲根变量（或根变量的 Select 链）、参数是字符串字面量的情况，把它当作一个路径段：
```rust
Expr::Call(call) if call.func_name == op::INDEX => {
    if let [target, IdedExpr { expr: Expr::Literal(LiteralValue::String(key)), .. }] = call.args.as_slice() {
        // extend the member path of `target` with `key`
    }
}
```
如果下标是非字面量，就对 `params` / `objects` 根报「dynamic index cannot be checked」。

### WR-03: 参数名和属性名允许 CEL 关键字，合法定义在条件中无法引用（one-way 契约）

**File:** `crates/ontoforge-schema/src/ir/names.rs:16-26, 155-158`
**Issue:** `ParameterName` / `PropertyName` 只检查 `^[a-z][A-Za-z0-9]{0,99}$` 和 9 个保留词，没有覆盖 CEL 保留标识符（`in`、`as`、`break`、`const`、`continue`、`else`、`for`、`function`、`if`、`import`、`let`、`loop`、`package`、`namespace`、`return`、`var`、`void`、`while`、`true`、`false`、`null`）。实测参数 `in: { type: integer }` 能通过 apiName 校验，但条件 `params.in > 0` 报 `Syntax error: no viable alternative at input '.in'`。`objects.<param>.<prop>` 中的属性名也有同样问题。另外，YAML 中写 `null:` 作为参数名时会得到 `cannot deserialize null into string; use Option<String>`。按 ADR-0012，这是发布后不可变的定义格式契约，越晚收紧迁移成本越高。
**Fix:** 为 `ParameterName` 和 `PropertyName`（以及将来做 codegen 的 `LinkEndName`）增加 CEL 保留字表，报 V07，help 建议改名。如果希望保持宽松，至少在 `check.rs` 中检测到「已声明但为关键字的参数」时给出明确提示，而不是语法错误。

### WR-04: 值来源兼容性允许收窄赋值，却拒绝拓宽赋值

**File:** `crates/ontoforge-schema/src/validate/actions.rs:381, 429-439`；`crates/ontoforge-schema/src/ir/basetype.rs:85-115`
**Issue:** `compatible` 按 `value_class` 比较。`Byte | Short | Integer` 同属一类，所以 `integer` 参数赋给 `byte` 属性会被接受（运行时可能溢出或截断）；`double` 赋给 `float` 也会被接受。而 `Long` 单独成类，`integer` 参数赋给 `long` 属性（无损拓宽）却是不可关闭的 V12 hard error。规则的方向反了：接受了有数据损失风险的写法，拒绝了安全的写法。
**Fix:** 改为显式的可赋值关系：`byte ⊂ short ⊂ integer ⊂ long`、`float ⊂ double`，只允许从小到大赋值；数组按元素类型递归判断：
```rust
fn widens(from: BaseType, to: BaseType) -> bool {
    use BaseType::*;
    let rank = |t| match t { Byte => Some(0), Short => Some(1), Integer => Some(2), Long => Some(3), _ => None };
    match (rank(from), rank(to)) {
        (Some(a), Some(b)) => a <= b,
        _ => from == to || (from == Float && to == Double),
    }
}
```

### WR-05: lint 选项类型错误或取负值时静默回退到默认值

**File:** `crates/ontoforge-schema/src/lint/mod.rs:76-101`；`crates/ontoforge-schema/src/lint/config.rs:14, 259-273`
**Issue:** `parse_config` 只检查选项值是 int、float、string 或字符串数组之一，不检查它是否符合该规则选项的类型。`max_properties = "5"`、`max_properties = 5.0`、`max_properties = -1`、`silo_prefixes = 3` 都不会报 V15，而 `option_usize` / `option_str_list` 会直接返回默认值。这与模块文档「Anything unrecognized is ONT-V15」的承诺矛盾，用户以为阈值已经生效，实际没有。
**Fix:** 让 `LintRule::option_keys` 同时声明期望类型（例如 `&[("max_properties", OptionKind::NonNegativeInt)]`），在 `Parser::options` 中按类型校验，不匹配时报 V15。`option_usize` 改为 `expect` 已校验的值。

### WR-06: Windows 上大小写不同的 manifest 被同时当作定义文件解析

**File:** `crates/ontoforge-schema/src/loader/discover.rs:39-45`；`crates/ontoforge-schema/src/loader/mod.rs:42`
**Issue:** `read_text(root, "ontology.yaml")` 在 Windows 的大小写不敏感文件系统上能读到 `Ontology.yaml`，但 `is_candidate` 用 `rel != MANIFEST` 精确比较，`Ontology.yaml` 仍然是候选文件并会匹配默认的 `**/*.yaml`。实测得到一条误报 `ONT-V02 document has no 'kind' field`（file: `Ontology.yaml`）。同一目录在 Linux 上则报 "manifest not found"，跨平台行为不一致。
**Fix:** 在 `is_candidate` 中对根级文件名用 `eq_ignore_ascii_case(MANIFEST)` 排除（`LINT_CONFIG` 同理）；或者在 walk 结果中找到实际的 manifest 名，若大小写与 `ontology.yaml` 不一致，报 V14 并要求规范命名。

### WR-07: 没有跨文件的总量预算

**File:** `crates/ontoforge-schema/src/loader/mod.rs:35-36, 189-208`；`crates/ontoforge-schema/src/loader/discover.rs:58-70`
**Issue:** 只有单文件上限 4 MiB 和 serde-saphyr 的单文件 node 预算（250k）。文件数量、总字节数和 Node 树总量都没有上限，所有文本还会常驻在 `SourceMap.files` 中。一个不可信目录放上千个接近 4 MiB 的文件，就可以耗尽内存。诊断条数也没有上限（参见 CR-02）。
**Fix:** 增加 `MAX_FILES`（例如 10,000）和 `MAX_TOTAL_BYTES`（例如 64 MiB）。在 `assemble` 中累计读取的字节数，超限时报 V14 并停止读取。在 `run_rules` 或 `report` 层面给诊断总数设上限，超出部分汇总为一条。

### WR-08: 诊断消息泄露解析器内部结构，且 `clean_message` 会截坏消息

**File:** `crates/ontoforge-schema/src/loader/parse.rs:215-243`；`crates/ontoforge-expr/src/analyze.rs:75-90`
**Issue:** 实测得到以下三条消息：
- 深层三元表达式：`invalid CEL expression: Syntax error: OtherError(ParseError { source: None, pos: (672, -1), msg: "Recursion limit of 96 exceeded", expr_id: 0, source_info: None })`，把 Debug 格式直接交给了用户；
- alias bomb：`budget breached: Nodes { nodes: 250001 } at line 4, column 15 (defined at line 9, column 9) (used`。`strip_position` 用 `rfind(" at line ")` 去切尾部位置，但这条消息中间也含有 " at line "，结果把结尾截成了半句 `(used`；
- `null:` 键：`cannot deserialize null into string; use Option<String>`，直接暴露了 Rust 类型。
这些消息违背了 D-10 / specifics 中「诊断给出修复建议」的要求。
**Fix:** `clean_message` 先只剥离末尾形如 ` at line N, column M` 的后缀（用正则 `\s+at line \d+,? column \d+$`），并为 `budget breached:` 的各种类型、`cannot deserialize null` 分别映射成友好的文案。`compile_and_run` 中识别 `Recursion limit` 并映射为 `ExprError::TooDeep`。

### WR-09: 嵌套数组检查只覆盖对象属性，没有覆盖 action 参数

**File:** `crates/ontoforge-schema/src/lint/e09.rs:77-86`；`crates/ontoforge-schema/src/validate/actions.rs:92-139`
**Issue:** D-12 的 E09 是「array 元素」用途约束，但只检查了 `ObjectType.properties`。参数 `{ type: array, items: array }` 在 validate 和 lint 中都不会被报告。`check_literal(default, Array, Some(Array))` 对内层元素调用 `check_literal(e, Array, None)`，也不检查元素的元素。
**Fix:** 在 E09 中遍历 `action_types[*].parameters`，对 `param_type == Array && items == Some(BaseType::Array)` 报告，路径为 `["parameters", name, "items"]`。

### WR-10: CI 第三方 Action 使用可变引用，且未加 `--locked`

**File:** `.github/workflows/ci.yml:19-27`
**Issue:** `dtolnay/rust-toolchain@master`（分支引用）、`Swatinem/rust-cache@v2`、`actions/checkout@v7` 都是可变 tag 或分支，上游被入侵或 tag 被移动时 CI 会执行任意代码；`rust-cache` 还会写入跨运行共享的缓存。`cargo clippy` / `cargo test` 也没有加 `--locked`，`Cargo.lock` 漂移不会被发现。
**Fix:** 把三个 action 固定到 commit SHA（注释里保留版本号），所有 cargo 命令加 `--locked`。

### WR-11: `sanitize` 放行 CR 和 Unicode 双向控制字符

**File:** `crates/ontoforge/src/render.rs:84-95`
**Issue:** `'\r'` 被原样保留。消息中含用户内容（例如 YAML 键 `"x\rerror: all good"` 产生的 `unknown field` 消息）时，CR 会把光标移回行首并覆盖前面的诊断文本。U+202A..U+202E、U+2066..U+2069、U+200E/U+200F、U+061C 等双向控制和格式字符也没有处理，可以让诊断文本在显示时被重排（Trojan Source 类）。
**Fix:** 对 message 和 help 也把 `\r` 替换为 `?`（源文本可以保留 `\r\n` 中的 `\r`，miette 按行处理）。把双向控制字符（`'\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}' | '\u{61c}'`）替换为等字节长度的 `?`，这些字符在 UTF-8 中占 3 字节或 2 字节。

### WR-12: 非 UTF-8 的文件名和目录名被静默丢弃

**File:** `crates/ontoforge-schema/src/loader/discover.rs:56, 63-69`
**Issue:** `filter_entry` 中 `to_str().is_some_and(..)` 会跳过名字不是 UTF-8 的整个目录；文件路径经 `Utf8Path::from_path(..).ok()` 转换失败时，被 `files.extend(rel)` 静默丢弃。Linux 上这类目录或文件中的定义会凭空消失，没有任何诊断，ontology 少了类型却显示 `ok`。
**Fix:** 遇到非 UTF-8 路径时报 V14（`file name is not valid UTF-8; skipped`，用 `to_string_lossy` 显示），不要静默跳过。

## Info

### IN-01: 未使用或重复声明的依赖

**File:** `crates/ontoforge-schema/Cargo.toml:19`；`crates/ontoforge/Cargo.toml:25, 33`
**Issue:** `thiserror` 在 `ontoforge-schema` 中没有任何引用；`serde` 在 `ontoforge` CLI 的源码中没有使用；`serde_json` 同时出现在 CLI 的 `[dependencies]` 和 `[dev-dependencies]` 中。
**Fix:** 删除这三处声明。

### IN-02: 死代码与过时注释

**File:** `crates/ontoforge-schema/src/loader/dispatch.rs:16-28, 81-87`；`crates/ontoforge-schema/src/lint/mod.rs:84-92`
**Issue:** `IMPLEMENTED_KINDS` 与 `KINDS` 完全相同，`"kind ... is not implemented yet"` 分支不可达。`RULE_KINDS` 的注释写着 "`function` is recognised only to reject it"，但数组里并没有 `function`（拒绝逻辑在 `rewrite_rules` 中）。`LintCx::option_f64` 和 `OptionValue::Float` 没有任何规则使用，只在测试中出现。
**Fix:** 删除 `IMPLEMENTED_KINDS` 及其分支，修正 `RULE_KINDS` 的注释；`option_f64` 等到有规则需要时再加。

### IN-03: naming 与 structure 的辅助函数重复

**File:** `crates/ontoforge-schema/src/lint/naming/mod.rs:22-38, 67-113`；`crates/ontoforge-schema/src/lint/structure/mod.rs:29-46, 48-97`
**Issue:** `off_hint`、`type_finding` 和 `testing` 模块（`flagged`、`run` 等）在两处几乎逐字重复，只有提示文案略有不同。
**Fix:** 上移到 `lint/mod.rs`（或 `lint/finding.rs`），由 `off_hint(rule_id, kind)` 参数化文案。

### IN-04: W02 的修复建议写死了 `flight` 前缀

**File:** `crates/ontoforge-schema/src/lint/naming/w02.rs:54-58`
**Issue:** 无论对象类型是什么，help 都是 "qualify it, such as `flight{Word}`"，例如 `Shipment.status` 会被建议改成 `flightStatus`。
**Fix:** 用对象类型名的小驼峰作前缀：`format!("{}{}", object.api_name.as_str().to_lower_camel_case(), capitalized(&lower))`。

### IN-05: CEL 类型标识符被误报为未知变量

**File:** `crates/ontoforge-expr/src/check.rs:168-172`
**Issue:** `type(params.a) == int` 中的 `int` 是 Ident，会报 `unknown variable 'int'`（V13，hard error）。`string`、`bool`、`double`、`uint`、`bytes`、`list`、`map`、`type`、`null_type` 同理。
**Fix:** 在 `path_problem` 中把这些 CEL 内建类型名当作已知标识符放行。

### IN-06: 配置模块文档出现双重否定

**File:** `crates/ontoforge-schema/src/lint/config.rs:14`
**Issue:** "so a typo never silently disables nothing" 是双重否定，语义与意图相反。
**Fix:** 改为 "so a typo never silently disables anything"（同时参见 WR-05）。

### IN-07: W10 的版本号正则误伤 `Ipv4` / `Ipv6`

**File:** `crates/ontoforge-schema/src/lint/structure/w10.rs:19-20`
**Issue:** `[vV]\d+$` 会命中 `AddressIpv6`、`NetworkIpv4` 等合法领域名。
**Fix:** 要求 `V` 前面是词边界（例如 `(?:^|[a-z])V\d+$`，只匹配大写 `V` 开头的新词），或排除 `ipv\d`。

### IN-08: `render_report` 失败被静默忽略

**File:** `crates/ontoforge/src/render.rs:46-48`
**Issue:** `let _ = handler.render_report(..)` 的注释写着 "Writing into a String cannot fail"，但 miette 在 `read_span` 越界时会返回 `fmt::Error`（graphical.rs:676-679），这时该条诊断在人类可读输出中只剩部分内容，或者整条消失，而退出码仍然是 1。当前的 span 都经过 clamp，暂时触发不了，但这个假设没有任何断言保护。
**Fix:** 渲染失败时退回到不带源码的 `Report::new(diag)` 再渲染一次，保证每条诊断至少输出 message。

### IN-09: E08 未检查对已删除对象执行 addLink / removeLink

**File:** `crates/ontoforge-schema/src/lint/e08.rs:65-98`
**Issue:** `deleted` 只对后续的 `modifyObject` / `deleteObject` 生效。`deleteObject flight` 之后的 `addLink { from: flight }` 不会被报告，属于同一类「删除后继续使用」的组合错误。
**Fix:** 在 `AddLink | RemoveLink` 分支中检查 `r.from` / `r.to` 是否已在 `deleted` 中。

### IN-10: `limits.rs` 的模块注释夸大了保证

**File:** `crates/ontoforge-expr/src/limits.rs:1-8`；`crates/ontoforge-expr/src/analyze.rs:7-10`
**Issue:** 注释暗示括号深度预检是防止栈溢出的屏障，但 AST 深度主要来自无括号的链式结构（`a.b.c...`、`x[0][0]...`、`1+1+...`），这些不受 `MAX_EXPR_NESTING` 约束。实测这类输入之所以安全，靠的是 4096 字符上限、32 MiB 栈和 `cel` 自带的 96 层递归上限三者叠加，`walk` / `infer` 的递归深度上限并没有被文档化。
**Fix:** 在注释中写明真正的安全依据，并为 `member_paths` / `type_issues` 加一个显式深度计数（超过 512 层返回 `ExprError::TooDeep`），不要依赖上游 crate 的实现细节。

---

_Reviewed: 2026-10-16T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
