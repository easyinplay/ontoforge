---
phase: 01-workspace-schema-lint
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/phases/01-workspace-schema-lint/01-REVIEW.md
iteration: 1
findings_in_scope: 16
fixed: 16
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** `.planning/phases/01-workspace-schema-lint/01-REVIEW.md`
**Iteration:** 1
**Scope:** `critical_warning`（CR-01..CR-04、WR-01..WR-12；IN-* 不在范围内）

**Summary:**
- Findings in scope: 16
- Fixed: 16（其中 5 条为逻辑或语义改动，标为 `fixed: requires human verification`：CR-01、CR-02、WR-01、WR-02、WR-04）
- Skipped: 0

**工作位置：** 主 checkout `D:/DataSpellProjects/ontoforge`，分支 `gsd/phase-01-workspace-schema-lint`。orchestrator 的约束要求停留在该分支、只暂存自己改动的文件、`docs/` 保持 untracked，因此没有创建 worktree（按 `wt="."` 处理，没有临时分支，也没有 recovery sentinel）。本文件中所有门禁结果都在主 checkout 上运行，可直接复现。

**新依赖：** 无。全部修复复用现有 workspace 依赖（`regex`、`walkdir`、`tempfile` 等）和标准库。

## Fixed Issues

### CR-01: ONT-W08 三重循环导致 CPU 拒绝服务（立方复杂度）

**Status:** fixed: requires human verification（算法与规则语义都有变化）
**Files modified:** `crates/ontoforge-schema/src/lint/structure/w08.rs`、`crates/ontoforge-schema/src/lint/structure/mod.rs`
**Commits:** `afa6a4d`（先提交失败测试）、`2c40911`（修复）
**Applied fix:**
- 先按「被至少 3 个类型持有的属性」组成候选集合，把候选集合相同的类型归为一组，每种组合只比较一次。
- 三重循环改为对不同组合做两两交集：交集至少 3 个属性时登记，最终保留至少有 3 个类型的集合。可以证明每个被报告的集合都确实被列出的每个类型完整持有，且任意 3 个列出的类型的三方交集正好是该集合。
- 不同组合数超过 `MAX_COMBINATIONS = 500` 时，规则不做比较，在第一个候选类型上报一条「未检查」诊断。
- 语义变化已写入模块文档：共享集合按两两交集定义。旧实现能报告「三方交集严格小于任何两两交集」的集合，新实现不报这种情况。审计字段这类真实场景不受影响，现有 `triggers` / `clean` 快照不变。
- 测试：新增 `run_within`（另起线程加超时）测试辅助函数，以及 3 条测试。1000 个相同共享集合的类型、400 个共享集合各不相同的类型都必须在 5 s 内完成（修复前两条都超时）；600 种组合时只输出一条「未检查」诊断。

### CR-02: ONT-W07 两两报告，输出量平方级放大

**Status:** fixed: requires human verification（报告形状与相似度计算都有变化）
**Files modified:** `crates/ontoforge-schema/src/lint/structure/w07.rs`、`crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap`
**Commits:** `ac7dd19`（先提交失败测试，当时输出 499500 条）、`75bb116`（修复）
**Applied fix:**
- 计算 Jaccard 前从两个集合中去掉主键。分组本来就要求主键同名同型，主键会人为抬高相似度。
- 相似的类型用 union-find 合并成簇，每簇只报告一次。报告挂在簇内 `apiName` 最后的类型上，消息列出其他成员（最多 20 个，其余写 `N more`），相似度给出簇内最高值。两个成员的簇沿用原来的消息格式。
- 语义变化已写入模块文档。`MIN_PROPERTIES` 仍按包含主键的属性总数计算，没有改动。
- 测试：1000 个相似类型只输出 1 条，并在 5 s 内完成；新增 3 个成员的簇的消息测试。

### CR-03: manifest 与 `ontoforge.toml` 跟随符号链接，大小检查可被绕过

**Files modified:** `crates/ontoforge-schema/src/loader/mod.rs`、`crates/ontoforge-schema/src/lint/config.rs`、`crates/ontoforge-schema/tests/loader.rs`
**Commit:** `98373f5`
**Applied fix:**
- 新增 `loader::read_limited(path, follow_links)`：用 `symlink_metadata`（或在 `follow_links` 为 true 时用 `metadata`）拒绝非普通文件，然后通过 `File::take(MAX_FILE_BYTES + 1)` 做有界读取。即使文件大小在检查后变化，或元数据报告的大小不准（例如字符设备），也无法超过 4 MiB 上限。
- `read_text`（manifest 和所有定义文件）不跟随链接，非普通文件报 V14：``` `{name}` is not a regular file (symbolic links are not followed); skipped ```。
- `load_config`：根目录的 `ontoforge.toml` 不跟随链接，报 V15。CLI 显式传入的 `--config` 可以是链接，但必须解析到普通文件。两种情况都有读取上限。
- 测试：manifest 是符号链接、manifest 是目录、根配置是目录、根配置是符号链接（同时验证显式路径仍可跟随）。本机开启了 Windows Developer Mode，符号链接测试实际执行了，没有走跳过分支。
- 残留：`symlink_metadata` 与 `open` 之间存在 TOCTOU 窗口。有界读取保证了内存不会被耗尽；要彻底消除，需要在 Unix 上使用 `O_NOFOLLOW`，这会引入平台相关代码，本次未做。

### CR-04: 文件名未经过终端控制字符过滤

**Files modified:** `crates/ontoforge/src/render.rs`
**Commit:** `1c33cca`
**Applied fix:** `NamedSource::new(sanitize(&d.file), sanitize(text))`，`with_code(sanitize(&d.code))` 作为纵深防御。测试：名为 `\x1b]52;c;aGk=\x07.yaml` 的文件被渲染为 `?]52;c;aGk=?.yaml`，输出中不含 ESC 和 BEL。

### WR-01: CEL 推断把 int 与 double 的混合算术视为 double

**Status:** fixed: requires human verification（类型规则改动）
**Files modified:** `crates/ontoforge-expr/src/infer.rs`、`crates/ontoforge-expr/tests/criteria.rs`
**Commit:** `2298273`
**Applied fix:** 已核实 `cel-0.15.0/src/objects.rs:1903` 起的 `ops::Add` / `ops::Mul` 只实现了同类型分支。`arithmetic` 改为返回 `Result`：`int op int` 得 int，`double op double` 得 double，int 与 double 混合时报 V13：``` cannot apply * to int and double; convert one side with int(...) or double(...) ```，结果类型为 `Any`。混合类型的比较仍然允许。

### WR-02: 下标访问绕过参数和属性的存在性检查

**Status:** fixed: requires human verification（引用检查范围变化）
**Files modified:** `crates/ontoforge-expr/src/walk.rs`、`crates/ontoforge-expr/src/check.rs`、`crates/ontoforge-expr/tests/criteria.rs`
**Commit:** `6a6d7b7`
**Applied fix:**
- `walk` 把 `_[_]` 当作路径的一步：字符串字面量下标等同于字段，所以 `params['typo']` 会报 unknown parameter，`objects.flight['nope']` 会报属性不存在。
- 非字面量下标会截断路径，下标表达式本身单独遍历，`MemberPath` 新增 `dynamic` 字段。如果截断发生在仍需要名字的位置（`params[x]`、`user[x]`、`objects[x]`、`objects.flight[x]`），报 ``` dynamic index on `...` cannot be checked ```。`params.tags[0]` 这类列表下标不受影响。
- 先做名字检查，再做动态下标检查，所以 `objects.nope[x]` 仍然报 unknown parameter。
- `MemberPath` 是 `ontoforge-expr` 的公开类型。新增字段只影响 workspace 内部，没有外部使用者。

### WR-03: 参数名和属性名允许 CEL 关键字

**Status:** fixed（这是对 ADR-0012 契约的收紧，见下）
**Files modified:** `crates/ontoforge-schema/src/ir/names.rs`、`crates/ontoforge-schema/tests/diagnostics.rs`
**Commit:** `5e512e4`
**Applied fix:**
- 新增 `CEL_RESERVED_WORDS`，共 21 个词，大小写敏感：`as break const continue else false for function if import in let loop namespace null package return true var void while`。`ParameterName` 和 `PropertyName` 命中时报 **ONT-V07**（复用现有的 apiName 校验码，没有新增错误码）。消息为 ``` `in` is a CEL keyword and cannot be used as parameter apiName: submission criteria could not refer to it ```，help 建议 `inValue` 这类名字。
- 覆盖范围：finding 点名的是参数和属性两类，属性名会以 `objects.<param>.<prop>` 出现在条件中，所以两类都覆盖。`LinkEndName`（reviewer 提到的将来 codegen 场景）本次没有改动。
- **契约收紧说明：** ADR-0012 规定的定义格式是 one-way 契约。本修复让过去合法的参数名和属性名（例如 `in`、`for`、`null`）变为 V07 hard error。按 ADR-0013 的原则，更严格的规则以后可以向后兼容地放宽，反方向则会让已有定义文件失效，所以在发布前收紧。仓库内的示例和 fixture 都没有用到这些名字，全部测试通过。建议另写一条 ADR（或在 ADR-0012 的后续 ADR 中）记录这一条，本次修复没有修改 ADR 文件。
- JSON Schema 的 `pattern` 无法表达「排除关键字」。这与现有的 `RESERVED_API_NAMES` 一致（同样不在 schema 中），schema 快照没有变化。
- `null:` 作为键时的 Rust 类型泄露在 WR-08 中处理。

### WR-04: 值来源兼容性允许收窄赋值，却拒绝拓宽赋值

**Status:** fixed: requires human verification（赋值规则反转）
**Files modified:** `crates/ontoforge-schema/src/ir/basetype.rs`、`crates/ontoforge-schema/src/validate/actions.rs`、`crates/ontoforge-schema/tests/action_types.rs`
**Commit:** `12c3a79`
**Applied fix:**
- 新增 `basetype::widens_to(from, to)`：只允许沿 `byte ⊂ short ⊂ integer ⊂ long`、`float ⊂ double` 从小到大赋值，其余类型必须相同。数组按元素类型递归判断。
- 删除了 `ValueClass` / `value_class`：它只被 `compatible` 和一条测试使用，保留下来会留下一个与新规则矛盾的公开 API。
- 行为变化：`integer` 参数赋给 `long` 属性现在合法；`long → integer`、`integer → byte`、`double → float` 都报 V12。
- `01-06-SUMMARY.md` 中关于 `ValueClass` 的描述是当时的交付记录，属于历史，没有修改。

### WR-05: lint 选项类型错误或取负值时静默回退到默认值

**Status:** fixed（`LintRule::option_keys` 的签名变化）
**Files modified:** `crates/ontoforge-schema/src/lint/mod.rs`、`crates/ontoforge-schema/src/lint/config.rs`、`crates/ontoforge-schema/src/lint/structure/w06.rs`、`w07.rs`、`w09.rs`
**Commit:** `b61af15`
**Applied fix:** 新增 `OptionKind { Count, Number, Words }`，`option_keys` 改为返回 `&[(&str, OptionKind)]`。`Parser::options` 校验值的种类，不匹配时报 V15：``` option `max_properties` of ONT-W06 must be a non-negative integer ```，位置指向该值。`max_properties = "5"`、`5.0`、`-1`、`["a"]`，以及 `silo_prefixes = 3` 现在都会报错。IN-06 的双重否定注释不在范围内，没有改。

### WR-06: Windows 上大小写不同的 manifest 被同时当作定义文件解析

**Files modified:** `crates/ontoforge-schema/src/loader/discover.rs`、`crates/ontoforge-schema/tests/loader.rs`
**Commit:** `973d515`
**Applied fix:** `is_candidate` 对 manifest 和 `ontoforge.toml` 的文件名做忽略 ASCII 大小写的比较，所以 `Ontology.yaml` 不再是定义文件（误报的 V02 消失）。`walk` 对根目录下大小写不规范的 manifest 报 V14：``` the manifest must be named `ontology.yaml`, not `Ontology.yaml` ```，Windows 和 Linux 都会报。大小写不同的 `ontoforge.toml` 只从候选文件中排除，没有单独报错。

### WR-07: 没有跨文件的总量预算

**Files modified:** `crates/ontoforge-schema/src/loader/mod.rs`、`crates/ontoforge-schema/tests/loader.rs`
**Commit:** `ab230e5`
**Applied fix:**
- 新增 `MAX_FILES = 10_000` 和 `MAX_TOTAL_BYTES = 64 MiB`，两者都是 `pub const`。`assemble` 只读取前 `MAX_FILES` 个选中的文件，并累计字节数，超限时报 V14 并停止读取。这两种情况都是 error，所以不会产出不完整的 ontology。磁盘加载和内存加载都经过这条路径。
- 部分处理：没有给诊断总数设上限。CR-01 / CR-02 修复后，各规则的输出与输入成线性关系，而输入已受上述预算约束。如需硬性的输出上限，可以在后续迭代中加。

### WR-08: 诊断消息泄露解析器内部结构，且 `clean_message` 会截坏消息

**Files modified:** `crates/ontoforge-schema/src/loader/parse.rs`、`crates/ontoforge-expr/src/analyze.rs`、`crates/ontoforge-expr/tests/analyze.rs`、`crates/ontoforge-expr/tests/criteria.rs`
**Commit:** `b47c79b`
**Applied fix:**
- `strip_position` 改为只去掉末尾的 `\s+at line \d+,? column \d+$`。
- `budget breached: X` 按类型映射为可读文案，例如 Nodes 变为 ``` document has more than 250000 nodes; each use of a YAML alias counts again ```。Depth、Aliases、Anchors、Events 各有对应文案，其他类型使用通用文案。
- `cannot deserialize null into string; use Option<String>` 映射为 ``` a mapping key is null; write a name, or quote it (`"null"`) if the text null is meant ```。
- CEL：任意一条编译错误含 `Recursion limit of` 时，返回新增的 `ExprError::ParserLimit`（``` expression nests too deeply for the CEL parser; split it into several conditions ```）。`cel` 有时会先报连带错误，所以检查全部错误，而不只是第一条。`analyze.rs` 中已有的小栈测试也相应接受 `ParserLimit`。
- 未处理：左侧嵌套的三元式会触发 `cel` 的 `Incomplete ExprContext for _?_:_ expression!`。这条消息不含 Debug 输出，保持原样。

### WR-09: 嵌套数组检查只覆盖对象属性，没有覆盖 action 参数

**Files modified:** `crates/ontoforge-schema/src/lint/e09.rs`
**Commit:** `cb6b22c`
**Applied fix:** E09 遍历 `action_types[*].parameters`，对 `type: array, items: array` 报 error，路径为 `["parameters", name, "items"]`，span 指向 `items` 的值。新增测试，现有 E09 快照不变。

### WR-10: CI 第三方 Action 使用可变引用，且未加 `--locked`

**Files modified:** `.github/workflows/ci.yml`
**Commit:** `f3f3826`
**Applied fix:** 三个 action 都固定到完整 commit SHA，并通过 `gh api` 核实：
- `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1`（`git/ref/tags/v7.0.1` → commit）
- `dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38b36d230d1e12f # v1`（该仓库没有 semver tag，`v1` tag 与 `master` 指向同一个 commit，即 2026-10-09 的 master HEAD）
- `Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2`（annotated tag `63fed3e…` 解引用后的 commit）

`cargo clippy` 与 `cargo test` 加了 `--locked`，本机 `cargo metadata --locked` 通过（`Cargo.lock` 已入库且是最新的）。YAML 没有语法检查工具，只做了 Tier 1 复读。

### WR-11: `sanitize` 放行 CR 和 Unicode 双向控制字符

**Files modified:** `crates/ontoforge/src/render.rs`
**Commit:** `5adebef`
**Applied fix:** 后面不跟 `\n` 的 `\r` 替换为 `?`（`\r\n` 保留）；U+061C、U+200E、U+200F、U+202A..U+202E、U+2066..U+2069 替换为与其 UTF-8 字节数相同个数的 `?`，span 偏移保持有效。同一个函数用于 message、help、源文本和文件名。

### WR-12: 非 UTF-8 的文件名和目录名被静默丢弃

**Files modified:** `crates/ontoforge-schema/src/loader/discover.rs`、`crates/ontoforge-schema/tests/loader.rs`
**Commit:** `c692170`
**Applied fix:** `filter_entry` 不再跳过名字不是 UTF-8 的目录。遍历时，名字不是 UTF-8 的文件或目录报 V14（``` the name of `...` is not valid UTF-8; skipped ```，用 `to_string_lossy` 显示）；目录用 `skip_current_dir` 跳过其内容。测试在 Windows 上用不成对的 surrogate 构造这类名字，在 Unix 上用 `0xff` 字节构造；文件系统不支持时跳过。本机 NTFS 上测试实际执行了。

## 快照变更

| 快照 | 原因 |
|------|------|
| `crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap` | CR-02 计算相似度时不再包含主键：`Contact` 与 `CrmContact` 的相似度从 `0.67` 变为 `0.60`（共享 5 个非主键属性中的 3 个）。只有 message 一行变化。 |

没有其他快照变化，也没有遗留 `.snap.new` 文件（`fd -H -I "\.snap\.new$" .` 无输出）。

## Verification

所有门禁都在**主 checkout** `D:/DataSpellProjects/ontoforge` 上运行（没有使用 worktree），环境为 `rustc 1.98.0 (88d9e12ae 2026-08-18)`、Windows 11。

- `cargo test --workspace`：exit 0，所有 test target 合计 **297 passed, 0 failed**。主要 target：`ontoforge_schema` lib 101、`loader` 34、`action_types` 26、`analyze` 21、`criteria` 20、`link_types` 17、`diagnostics` 16、`cli_lint` 14、`object_types` 14。
- `cargo clippy --workspace --all-targets -- -D warnings`：exit 0（`Finished dev profile`）。
- `cargo fmt --all --check`：exit 0。
- 端到端复现 CR-01 / CR-02（reviewer 的场景：1000 个类型，每个类型有主键 `id` 和 3 个共享属性，`defs.yaml` 共 200,890 字节）：debug 构建的 `ontoforge lint --format json` 用时 **547 ms**，exit 0，输出 **2 条诊断**（ONT-W07 1 条、ONT-W08 1 条），共 12,995 字节。审查时 release 构建需要 67 s，输出 499,500 条 W07 诊断、215 MB JSON。

## 需要人工确认

1. CR-01 / CR-02 的规则语义变化（W08 改为两两交集，W07 去掉主键并按簇报告），以及 W07 消息格式。
2. WR-01、WR-02、WR-04 的类型与引用规则变化。
3. WR-03 是对 ADR-0012 定义格式契约的收紧，建议补一条 ADR 记录。

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
