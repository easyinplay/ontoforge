---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T07:20:53Z
depth: standard
review_axis: standards
review_base: c9f823cf7b0cf01a382a3e3c21c912887bdabb94
review_merge_base: c9f823cf7b0cf01a382a3e3c21c912887bdabb94
review_head: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
files_reviewed: 53
files_reviewed_list:
  - .github/workflows/ci.yml
  - ci/query-services.sh
  - ci/server-services.sh
  - ci/windows-server.py
  - clients/typescript/package.json
  - clients/typescript/src/codec.ts
  - clients/typescript/src/index.ts
  - crates/ontoforge-action/src/lib.rs
  - crates/ontoforge-api/Cargo.toml
  - crates/ontoforge-api/src/dto.rs
  - crates/ontoforge-api/src/lib.rs
  - crates/ontoforge-api/src/openapi.rs
  - crates/ontoforge-client/Cargo.toml
  - crates/ontoforge-client/src/lib.rs
  - crates/ontoforge-engine/src/config.rs
  - crates/ontoforge-engine/src/lib.rs
  - crates/ontoforge-engine/src/printer/l2.rs
  - crates/ontoforge-engine/src/trino/cancel.rs
  - crates/ontoforge-engine/src/trino/l2_monitor.rs
  - crates/ontoforge-engine/src/trino/mod.rs
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/l2.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-server/Cargo.toml
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/src/health.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/mcp_actions.rs
  - crates/ontoforge-server/src/mcp_catalog.rs
  - crates/ontoforge-server/src/mcp_l2.rs
  - crates/ontoforge-server/src/mcp_sessions.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge-server/tests/mcp_actions.rs
  - crates/ontoforge-storage/src/history.rs
  - crates/ontoforge-storage/src/lib.rs
  - crates/ontoforge-storage/src/request_snapshot.rs
  - crates/ontoforge/Cargo.toml
  - crates/ontoforge/src/cli.rs
  - crates/ontoforge/src/dev.rs
  - crates/ontoforge/src/dev_assets.rs
  - crates/ontoforge/src/lib.rs
  - crates/ontoforge/src/mcp.rs
  - crates/ontoforge/src/serve.rs
  - crates/ontoforge/tests/dev_lifecycle.rs
  - docs/api.md
  - docs/dev.md
  - tools/openapi-gen/generate.mjs
  - tools/openapi-gen/package.json
coverage:
  full_content_count: 29
  diff_hunks_count: 24
  complete_phase_source_review: false
diff_hunks_only:
  - .github/workflows/ci.yml
  - ci/query-services.sh
  - clients/typescript/package.json
  - crates/ontoforge-action/src/lib.rs
  - crates/ontoforge-api/Cargo.toml
  - crates/ontoforge-api/src/lib.rs
  - crates/ontoforge-client/Cargo.toml
  - crates/ontoforge-engine/src/config.rs
  - crates/ontoforge-engine/src/lib.rs
  - crates/ontoforge-engine/src/printer/l2.rs
  - crates/ontoforge-engine/src/trino/cancel.rs
  - crates/ontoforge-engine/src/trino/l2_monitor.rs
  - crates/ontoforge-engine/src/trino/mod.rs
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/l2.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-server/Cargo.toml
  - crates/ontoforge-storage/src/history.rs
  - crates/ontoforge-storage/src/lib.rs
  - crates/ontoforge/Cargo.toml
  - crates/ontoforge/src/cli.rs
  - crates/ontoforge/src/lib.rs
  - tools/openapi-gen/package.json
findings:
  critical: 5
  warning: 0
  info: 0
  total: 5
status: issues_found
phase_acceptance: pending
---

# Phase 7 Standards Review

## Scope And Limits

固定 `git diff gsd/phase-06-l2-toolsets-audit...fd24beb`，范围为 `crates/ clients/ tools/ ci/ .github/ docs/api.md docs/dev.md`。上列 53 文件中 29 个读取全文，24 个仅检查变更 hunk；另追踪 auth key、Action 参数和名称校验的既有 caller/callee。未逐读其余 37 个测试文件；2 个 lockfile 与生成的 `generated.ts` 排除。不宣称全阶段源码审查完成。

规范来源：会话 AGENTS、`Cargo.toml`、`CONTRIBUTING.md`、接口文档、code-review smell baseline。工作目录无 AGENTS、CodeGraph、项目 skills；GSD reviewer 没有额外 skill。跳过 tooling 已验证项；未运行 Cargo、Docker、安装或动态复现。Staff/Windows 已关闭发现不重报，07-14 不计缺陷。当前实现与冻结 head 一致；`docs/dev.md` 有其他执行者后续改动，本报告引用其冻结版本已有条款。这里只写报告，不修改实现、不提交。

## Narrative Findings (AI reviewer)

### CR-01: [BLOCKER] dev key 创建丢失 Unix 私有权限

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge/src/dev.rs:168-175`
**Confidence:** high；静态确认。
**Standard:** `docs/api.md:37-38` 要求私有目录、0600；已有 `crates/ontoforge-auth/src/authn/dev.rs:155-160` 实现该约束。
**Issue:** 新的 `ensure_key` 默认权限创建两把 key；可遍历目录、umask 022 下得到 0644，其他本机账户可读 HMAC key 并签发任意 dev 身份。随后 `serve.rs:122` 读取已有文件，不补权限。
**Fix:** 复用安全 key 创建边界；Unix 新目录 0700、新文件 0600，补权限回归。Windows 既有 ACL 限制不在本发现内。

### CR-02: [BLOCKER] dev 锁恢复可删除活跃实例的锁

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge/src/dev.rs:250-270`
**Confidence:** high；静态并发交错，未动态复现。
**Standard:** `docs/dev.md:18` 承诺拒绝活跃实例。
**Issue:** A 的 create_new 成功后、写 PID 前，B 读到空文件，将其当 stale 删除并重新获取；A/B 均进入同一目录。两个 stale 恢复者也能删除对方新锁；并发 reset 可删除另一实例的数据文件。
**Fix:** 用持有整个生命周期的 OS 文件锁；无效 PID 必须 fail closed，不能充当删除授权。增加 barrier 控制的并发初始化/reset 测试。

### CR-03: [BLOCKER] MCP 重复类型映射偏离领域校验

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp_actions.rs:107-123`
**Confidence:** high；静态确认。
**Standard:** Repeated Switches / Duplicated Code 启发式，已出现实际行为分叉。
**Issue:** ObjectReference 固定为 string，数字/boolean PK 被 `:80` 拒绝；`ontoforge-action/src/params.rs:45-55` 实际按目标 PK 校验。Float/Double 数组又发布成 string[]，与 `basetype.rs:116-119,148-155` 冲突。
**Fix:** 从同一领域类型映射生成 schema，并给引用解析目标 PK；覆盖非字符串 PK 与浮点数组。

### CR-04: [BLOCKER] MCP 控制字段覆盖合法 Action 参数

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp_actions.rs:30-34`
**Confidence:** high；静态确认。
**Standard:** 参数命名空间隔离启发式，非风格偏好。
**Issue:** `crates/ontoforge-schema/src/ir/names.rs:206` 允许 commandId/validateOnly；schema 覆盖同名业务参数，`mcp.rs:507-511` 又从 params 删除它们。required 参数永远缺失，optional 参数静默丢失。
**Fix:** 使用互不冲突的控制/业务命名空间，或在定义准入时明确拒绝冲突；增加同名参数回归。

### CR-05: [BLOCKER] history 丢失审计关联和查询失败语义

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/rest.rs:593-602,648-654`
**Confidence:** high；静态确认。
**Standard:** `docs/api.md:46-48` 审计 ID 合同；错误传播使用 Duplicated Code 启发式。
**Issue:** `query/src/lib.rs:287-299` 返回真实 audit ID 和错误；history 忽略该 ID，响应仍用 transport ID，并把 Source/Store/AuditUnavailable 全转换为 404。客户端无法关联审计，也无法区分暂时失败和对象不存在。
**Fix:** 保留 `page.request_id`，复用 `rest.rs:776-787` 的 query_failure；storage history 错误仅将真实不可见映射为 404。补成功 ID 对账与 source/audit 故障用例。
