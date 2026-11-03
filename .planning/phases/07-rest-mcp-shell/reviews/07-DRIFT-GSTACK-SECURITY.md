---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T11:38:46Z
review_base: 4d203bf
review_target: working-tree-source-drift-increment
specialist: gstack-security
depth: deep
files_reviewed: 22
files_reviewed_list:
  - crates/ontoforge-action/src/lib.rs
  - crates/ontoforge-action/src/source.rs
  - crates/ontoforge-engine/Cargo.toml
  - crates/ontoforge-engine/src/bind_check.rs
  - crates/ontoforge-engine/src/lib.rs
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-query/src/link.rs
  - crates/ontoforge-server/Cargo.toml
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/drift.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/src/health.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge-server/tests/source_drift.rs
  - crates/ontoforge-server/tests/unit/drift.test.rs
  - crates/ontoforge/src/serve.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
tests_rerun: false
real_services_verified: false
phase_acceptance: pending
---

# RF-01 gstack Security 专项审查

## Narrative Findings (AI reviewer)

NO FINDINGS

本轮未证实 RF-01 source drift 增量引入新的可达安全缺陷。结论仅覆盖下述专项范围，不代表 Phase 7 或 RF-01 动态验收通过。DS01、CR-01 已有修复，不重复列为未解决 finding。

## 范围与方法

应用 `C:/Users/easyi/.agents/skills/gstack/review/specialists/security.md`，审查 `git diff 4d203bf -- crates` 中 source drift 相关生产增量、新 `server/src/drift.rs`，并沿授权、source 访问、错误输出、monitor 状态和生命周期调用链读取上下文。两个 Cargo.toml 仅增加 dev-dependency 的 `test-util` feature。测试文件只核对与安全断言和状态隔离相关的内容，不宣称审查整个仓库。

工作目录不存在磁盘 `AGENTS.md`、`.codegraph/`、`.codex/skills/` 或 `.agents/skills/`；遵守任务提供的 AGENTS 指令。已执行 GSD `query agent-skills gsd-code-reviewer --skills-only`，未返回额外 skill。已读取 `.gitignore`；本轮 source 路径不在其忽略范围内。既有 `target/` 日志仅作为主会话明确指定的证据读取，不作为 source 审查范围。

## 关键审查路径

- **授权顺序**：`context.rs:246-280` 完成认证与 scope 编译。PK 的 type grant 检查在 `query/src/lib.rs:321-325`，列表在 `execute.rs:588-592`，聚合在 `aggregate.rs:962-967`；`link.rs:105-114` 在 M:N source gate 前取得两端 type scope。CR-01 修复 `action/src/source.rs:56-65` 在 edit/source 读取前拒绝缺失 type grant，Action 自身 grant 仍在 `action/src/lib.rs:298-300` 检查。需要实际行数据的授权仍由原有后续流程执行；本轮没有把 type grant 等同于完整行/列授权。
- **source gate 接线**：PK、列表首批/后续、aggregate 和 M:N 对应 `query/src/lib.rs:520`、`execute.rs:747/900`、`aggregate.rs:787`、`link.rs:304`。REST Action 正式与 validateOnly 使用固定 drift view 的 wrapper (`rest.rs:443-476`)，MCP Action 同样使用 wrapper (`mcp.rs:614-648`)。本轮读取的 `context.rs:195-196` 同时执行 drift 检查和 inner hook，未读到临时 bypass mutation。
- **跨版本隔离与默认拒绝**：`drift.rs` 的 key 同时包含定义版本和 profile；capture 要求报告 key 完全匹配，未知 target 与不完整状态均返回固定错误。publish 要求 observed key 相符；任务退出设置 stopped。未发现请求参数可替换服务端捕获的 key/view 的路径。
- **错误信息边界**：`SourceTargetError` 仅含四个无 payload variant；`drift.rs:44-65` 将含 operator detail 的内部报告映射到安全 variant。`error.rs:15-20/83-112/122` 返回固定 code、调用方 apiName、request_id，不序列化报告或 engine error。`health.rs` 保留 status/engineStatus/可选 l2，没有新增对象名、物理列名、catalog、schema、mismatch 或 detail。
- **输入与攻击面**：增量未新增用户可控 URL、路径、shell 命令、模板执行或反序列化执行入口；metadata probe 读取已加载定义的绑定。source SQL 继续经过既有 AST/printer，新增 gate 不拼接输入为 SQL。增量未改 token 签名、密码哈希或 secret 比较，也未发现新增凭据字面值。

## 已读取证据与局限

已读 `07-DRIFT-SAFETY-RECHECK.md` 和 `07-DRIFT-SPEC-RECHECK.md`，并独立核对当前代码；其意见不直接充当本报告 finding。CR-01 的当前 Action blob 为 `33af0bc044235488f725d4835625388137583c20`。

已亲读 `source_drift.rs:938-969`：健康/漂移两种 metadata × REST/MCP × 正式/validateOnly，共八个拒绝路径断言无 type grant 返回 NOT_FOUND，并检查 source SQL 次数为零。`:974-1001` 检查有授权的 MCP 两模式保留 SOURCE_SCHEMA_DRIFT、apiName、request_id，并检查无 source SQL 或额外写入。`:200-233` 核对匿名 health 的精确安全形状及 engine secret 不泄露。现有 `target/phase7-drift-action-auth-red.log` 为真实断言失败；`target/phase7-drift-action-auth-green.log` 记录 19 passed / 0 failed / 0 ignored。本 reviewer 没有运行这些测试，也没有把这些日志当成当前全部工作树的最终测试结果。

动态验收未代跑：未运行 Cargo、真实 Trino/PG、网络探测或 mutation 测试。只写本报告，未改实现、未 commit。真实服务验收与最终冻结结果由主会话汇总。

## 审查快照

结束前核对的 18 个生产 Rust 文件 blob 与 `07-DRIFT-SAFETY-RECHECK.md` 记录一致，唯一预期差异为已修复的 Action source；当前关键 blob：

```text
crates/ontoforge-action/src/source.rs 33af0bc044235488f725d4835625388137583c20
crates/ontoforge-server/src/context.rs bfc178c59d5e8de31a0d7f36197bcc5b31be82ec
crates/ontoforge-server/src/drift.rs e7eacbb94dadf727e3d0bd1ad9e8a9c60c74b568
crates/ontoforge-server/src/lib.rs 38005f3943f73da00026c8fa9ff3c4b08f12c7d8
```

后续文件发生变化时，本报告不自动覆盖新的内容。
