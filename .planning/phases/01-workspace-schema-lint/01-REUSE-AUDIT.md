---
phase: 01-workspace-schema-lint
type: audit
date: 2026-10-15
scope: crates/**/src/**/*.rs 中的手写逻辑 vs 成熟 crate（"不重复造轮子"）
status: advisory（只读审计，未改代码）
---

# Phase 1 手写逻辑复用审计

数据来源：crates.io API（`max_stable_version` / `updated_at` / `recent_downloads` / license，2026-10-15 查询）、`gh api repos/...`（`pushed_at` / archived / stars）、`cargo tree -i`（是否已是传递依赖）、`~/.cargo/registry/src` 中已下载 crate 的源码。下文 "DL90" = crates.io `recent_downloads`（近 90 天）。

结论分布：REPLACE 3 项，CONSIDER-LATER 2 项，其余 KEEP。研究阶段（01-RESEARCH.md "Don't Hand-Roll"）已把 YAML/JSON 解析、路径回查、schema、渲染、编辑距离、glob、TOML 都交给了现成 crate；剩余手写部分大多是 crate 缺能力（spike 已证）或 <30 行胶水。

## 审计表

| 组件 | 文件 | 行数约 | 现成候选 | 核实数据 | 结论 | 理由/迁移成本 |
|---|---|---|---|---|---|---|
| apiName 规范化建议 `canonical()` | `ontoforge-schema/src/ir/names.rs` | 25 | `heck` 0.5.0 | MIT/Apache-2.0；DL90 3.2 亿；repo pushed 2025-08-09，未归档；**已在 Cargo.lock**（`clap_derive`、`strum_macros` 的依赖） | **REPLACE** | `to_lower_camel_case` / `to_upper_camel_case` 正好对应；现实现对 `HTTPServer` 给出 `hTTPServer`，heck 给 `httpServer`；现实现把非 ASCII 字母当分隔符（`café_name` 建议成 `cafName`，误导），heck 保留后经 `well_formed` 过滤为无建议。零新下载，零新审计 |
| `ontoforge.toml` 的 `Raw` 树 + `RawVisitor` + `DATETIME_KEY` | `ontoforge-schema/src/lint/config.rs` | 70 | `toml::de::DeTable::parse` / `parse_recoverable`（toml 1.1.8 自带） | toml 1.1.8，MIT/Apache-2.0，DL90 2.6 亿，pushed 2026-10-09；源码 `src/de/parser/detable.rs:16-32` 确认 `pub type DeTable = Map<Spanned<DeString>, Spanned<DeValue>>` 且 `parse` 为 pub | **REPLACE** | 现实现依赖 toml 内部私有键 `$__toml_private_datetime` 识别 datetime，属于上游实现细节，升级即可能静默失效；`DeValue::Datetime` 是显式变体。`DeInteger::as_str()+radix()` 文档写明可直接 `i64::from_str_radix`。`parse_recoverable` 还能一次报多个语法错误 |
| `--color auto` 判定 + 自定义 `ColorChoice` | `ontoforge/src/render.rs`、`ontoforge/src/lib.rs::report` | 15 | `anstream` 1.0.0 `AutoStream::choice`（底层 `anstyle-query` 1.1.5）；备选 `supports-color` 3.0.2 | anstream MIT/Apache-2.0，DL90 2.0 亿，2026-02-11；**已是传递依赖**（`clap_builder`）；源码 `anstream-1.0.0/src/auto.rs:198-209` 处理 `NO_COLOR`、`CLICOLOR`、`CLICOLOR_FORCE`、TERM。supports-color 已经 miette 传递引入，但 repo 最后 push 2024-11-26 | **REPLACE** | 现实现只看 `is_terminal()` 与 `NO_COLOR` 是否存在（`NO_COLOR=""` 也会关色，不符合 no-color.org 的"非空"约定），不支持 `CLICOLOR_FORCE`、`TERM=dumb`。优先 anstream（维护更活跃） |
| 驼峰分词 `split_camel` | `ontoforge-schema/src/lint/words.rs` | 30 | `heck` 0.5.0；`convert_case` 0.12.0 | heck 源码 `transform()` 只在大小写边界切词，不在字母/数字边界切（`code01` 为一个词、`FlightV2` 得 `flight`,`v2`），与本函数测试固化的语义不同，且无"返回词列表"API；convert_case MIT，DL90 1.4 亿，pushed 2026-08-27，边界可配置 | CONSIDER-LATER | grep 证实当前**没有生产调用方**（只有自身单测）；W 系列命名规则落地时，若需要可配置边界再评估 convert_case，否则保留 30 行 |
| 字节偏移 ↔ 行:列（字符列），三份副本 | `loader/parse.rs::offset_of`+`precheck_json`、`lint/config.rs::Parser::at`、`ontoforge-expr/src/walk.rs::offset_to_line_col` | 45 | `line-index` 0.1.2（rust-analyzer）；miette `SourceCode::read_span` | line-index MIT/Apache-2.0，DL90 170 万，2026-10-08；runtime 依赖 `text-size`、`nohash-hasher`；列为 UTF-8 字节，需 `WideEncoding::Utf32` 转字符列。schema crate 不依赖 miette | CONSIDER-LATER（内部去重，不引 crate） | 每份 <15 行，引 crate 换不来什么。真正问题是三份逻辑各写一遍：建议合并为 `diag` 里一个 `line_col(text, byte_offset)`，expr 复用 |
| 「你是否想写 X」`did_you_mean` | `ontoforge-schema/src/diag.rs` | 12 | `strsim` 0.11.1 | 已在用 `strsim::jaro_winkler`；MIT，DL90 2.5 亿，repo pushed 2025-11-27 | KEEP | 已复用，只有阈值与大小写折叠是自写 |
| 规则 ID 补零建议 `zero_padded` | `lint/config.rs` | 10 | 同上 | 编辑距离无法区分 `ONT-W1` 应为 `W01` 还是 `W10`（注释已说明） | KEEP | 领域规则，非通用能力 |
| apiName 格式与保留字校验 | `ir/names.rs::well_formed/reserved` | 15 | `regex` 1.13.1（已是依赖） | — | KEEP | ASCII 判定 5 行，比正则更直接；JSON Schema 侧已用 pattern 字符串 |
| 目录遍历 + 跳过隐藏/`target`/`node_modules` | `loader/discover.rs::walk/is_candidate` | 40 | `walkdir`、`globset`（已在用）；`ignore` 0.4.33 | ignore Unlicense/MIT，DL90 4400 万，pushed 2026-08-04 | KEEP | ignore 会引入 `.gitignore` 语义，定义文件可能被静默忽略，不是想要的行为；现实现已是 walkdir+globset 组合 |
| include 越界检查 `escapes_root` | `loader/discover.rs` | 5 | `path-clean` 1.0.1、`normpath` 1.5.2 | path-clean 最后发布 2023-02-24；normpath 2026-09-19 | KEEP | 二者规范化的是文件路径，不判断 glob 模式是否越界；5 行足够 |
| 路径分隔符归一 `normalize` | `loader/discover.rs` | 1 | `path-slash` 0.2.1 | 最后发布 2022-08-06 | KEEP | 一行 `replace`，不值得加依赖 |
| Spanned 节点树、`to_value`、路径回查 span | `loader/parse.rs`、`loader/sourcemap.rs` | 200 | `serde-saphyr` `Spanned` 直接反序列化；`saphyr` 0.2.2 `MarkedYaml`；`json-spanned-value` 0.2.2 | spike S1：tagged/untagged/flatten 内丢 span；saphyr 2026-10-15 发布（Plan B）；json-spanned-value 停在 2020-10-10 | KEEP | ADR-0003 已定，能力缺口由 spike 证实 |
| JSON 严格预检 | `loader/parse.rs::precheck_json` | 20 | `serde_json`（已在用） | — | KEEP | 只有偏移换算是自写，并入上面的行列去重即可 |
| 解析器报错清洗 `strip_position/clean_message/json_message` | `loader/parse.rs` | 30 | 无 | — | KEEP | 依赖上游报错文本格式（风险见下），已有单测守护 |
| serde 报错映射 `map_error/ticks/to_segs` | `loader/dispatch.rs` | 80 | `serde_path_to_error`（已在用） | — | KEEP | 解析 serde 的 `unknown field`/`missing field`/`unknown variant` 文本；serde core 这些格式多年未变，有测试兜底 |
| `custom_de_error`（`\u{1f}` 编码 code+help） | `diag.rs` | 30 | 无 | serde `Error::custom` 只携带 Display 文本 | KEEP | 没有别的结构化通道 |
| Diagnostic 模型、排序、JSON 输出 | `diag.rs` | 100 | miette `Diagnostic` derive；`codespan-reporting` 0.13.1 | codespan Apache-2.0，DL90 3100 万，2025-10-22 | KEEP | D-10 规定 camelCase JSON 线格式；schema crate 保持不依赖 miette |
| 人类可读渲染 | `ontoforge/src/render.rs` | 40 | miette（已在用） | — | KEEP | 已复用 GraphicalReportHandler |
| 终端控制字符清洗 `sanitize` | `ontoforge/src/render.rs` | 15 | `strip-ansi-escapes` 0.2.1 | Apache-2.0/MIT，DL90 1150 万，repo pushed 2025-01-14 | KEEP | 该 crate 删除转义序列，会改变字节长度、打乱 span 偏移；现实现刻意保持字节长度 |
| 大栈线程（3 处） | `ontoforge/src/main.rs`、`loader/mod.rs::isolated`、`ontoforge-expr/src/analyze.rs` | 30 | `stacker` 0.1.25 | MIT/Apache-2.0，DL90 3000 万，2026-08-02 | KEEP | 递归发生在 cel/serde-saphyr 内部，`maybe_grow` 插不进去；`grow()` 不会把 panic 转成诊断；std 线程零依赖 |
| CEL 词法预检 `limits.rs` | `ontoforge-expr/src/limits.rs` | 200 | `cel` 0.15.0 自带词法器 | 源码 `cel-0.15.0/src/parser/mod.rs:3` 为私有 `mod gen;`，词法器不对外；无独立 CEL lexer crate | KEEP | 必须在调用 `compile` 前限深（Pitfall 1），上游无 API |
| 作用域感知 AST 遍历 | `ontoforge-expr/src/walk.rs` | 150 | `cel` `references()` | spike S2：把 comprehension 变量算成自由变量，也给不出 `params.x` 路径 | KEEP | 能力缺口已证 |
| 保守类型推断 | `ontoforge-expr/src/infer.rs`、`check.rs` | 550 | `cel` 0.15.0 checker | cel 0.15.0 为最新稳定版（2026-10-06），无变量类型声明与 checker（spike S2）；repo pushed 2026-10-13 | KEEP | ADR-0006。每次升级 cel 时复查上游是否新增 checker |
| lint 级别 / 选项访问器 | `lint/mod.rs` | 60 | `strum`、serde derive | strum 只经 dev 依赖 jsonschema 传递引入 | KEEP | 需要带 span 的 ONT-V15 报错，derive 拿不到 span；match 只有 4 臂 |
| CLI 集成测试 helper | `ontoforge/tests/*.rs` | 20 | `assert_cmd` 2.2.2 | MIT/Apache-2.0，2026-05-11 | KEEP | 研究阶段已选 `std::process::Command`+`CARGO_BIN_EXE_*`，零依赖 |
| schema 导出后处理 `with_header/describe_def` | `ontoforge-schema/src/schema_export.rs` | 20 | schemars `transform` | — | KEEP | 01-09 正在进行中，不建议本轮动 |

## 推荐 REPLACE（按优先级）

### 1. `lint/config.rs`：`Raw` 树改用 `toml::de::DeTable::parse`（优先级最高：消除对上游私有键的依赖）
- 删除：`enum Raw`、`impl Deserialize for Raw`、`RawVisitor`、`DATETIME_KEY` 常量，以及 `serde::de::{Deserializer, MapAccess, SeqAccess, Visitor}` 的导入。
- `parse_config`：`toml::from_str::<Raw>(text)` 改为 `toml::de::DeTable::parse(text)`（或用 `parse_recoverable` 一次报出全部语法错误，两种做法任选一种，选后者要确认 `toml_syntax_error_is_v15_at_its_span` 的期望数量）。
- `Parser::lint/level/options/option_value`：模式匹配从 `Raw::*` 改为 `DeValue::{String, Integer, Float, Array, Table, Boolean, Datetime}`；整数用 `i64::from_str_radix(i.as_str(), i.radix())`，溢出时归入原来的 `Raw::Other` 分支；浮点 `f.as_str().parse::<f64>()`。`Spanned` 仍是 `serde_spanned::Spanned`，`.span()` / `.get_ref()` 用法不变。
- 注意：`DeTable` 默认按 key 字典序存储（toml 未开 `preserve_order`）；诊断最终按 `(file, line, column, code)` 排序，输出不受影响。
- 受影响测试：`lint/config.rs` 内 17 个单测（`config_levels_parse` … `missing_override_path_is_error`），`ontoforge/tests/cli_lint.rs`。预期全部不改断言即可通过。

### 2. `ir/names.rs::canonical()` 改用 `heck`
- 根 `Cargo.toml` `[workspace.dependencies]` 加 `heck = "0.5.0"`，`ontoforge-schema/Cargo.toml` 加 `heck.workspace = true`（锁文件中已有 0.5.0，不新增下载）。
- `canonical` 函数体换成 `match initial { Upper => raw.to_upper_camel_case(), Lower => raw.to_lower_camel_case() }`，保留末尾的 `well_formed && reserved.is_none() && out != raw` 过滤。
- 受影响测试：`names.rs` 的 `lowercase_object_type_gets_a_canonical_suggestion`、`separators_are_folded_into_camel_case`、`unfixable_names_get_no_help`；`tests/link_types.rs:167`（`FlightCrew` → `flightCrew`）；快照 `ontoforge/tests/snapshots/invalid_examples__v07-bad-object-type-name.snap`（`flight` → `Flight`）。按 heck 源码推导，这 5 处结果不变；新增一个 `HTTPServer` → `httpServer` 用例固化改进。

### 3. `--color auto` 改用 `anstream`
- `ontoforge/Cargo.toml` 加 `anstream`（工作区加 `anstream = "1.0.0"`，锁文件已有）。
- `lib.rs::report` 的 `ColorChoice::Auto` 分支换成 `anstream::AutoStream::choice(&std::io::stdout()) != anstream::ColorChoice::Never`。自定义 `render::ColorChoice` 可以保留（clap `ValueEnum`、帮助文本不变），也可以换成 `clap::ColorChoice`，两种都行，保持 CLI 参数不变即可。
- 行为变化：`NO_COLOR=""` 不再关色（符合规范）；新支持 `CLICOLOR=0`、`CLICOLOR_FORCE`、`TERM=dumb`。
- 受影响测试：`ontoforge/tests/cli.rs`、`cli_lint.rs`、`cli_schema.rs` 均用 `NO_COLOR=1` 或 `--color never/always`，预期不受影响。风险：CI 若设了 `CLICOLOR_FORCE`，未带 `--color` 的人类输出用例会出现 ANSI；`NO_COLOR` 在 anstream 中优先级更高，测试已设，可兜住。

## 风险与备注
- 解析器报错文本清洗（`strip_position`、`clean_message` 匹配 `"duplicate mapping key: "` / `"budget breached: Depth"`）与 `map_error` 都依赖上游错误文本。`serde-saphyr` 2.0.0 是 2026-10-13 刚发的 major，升级时这里最容易坏；现有单测会先报警。建议在 `Cargo.toml` 注释中注明。
- `supports-color` 3.0.2 经 miette 传递引入，上游 2024-11 以后没有 push。这不阻塞 Phase 1，记录在案。
- 行列换算去重（CONSIDER-LATER）是纯内部重构，不引依赖，可以并入下一次触碰这三个文件的 plan。
