---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T07:19:35Z
depth: deep
review_axis: spec
review_base: c9f823cf7b0cf01a382a3e3c21c912887bdabb94
review_head: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
files_reviewed: 23
files_reviewed_list:
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/mcp_actions.rs
  - crates/ontoforge-server/src/mcp_catalog.rs
  - crates/ontoforge-server/src/mcp_l2.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge-server/src/health.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge/src/serve.rs
  - crates/ontoforge/src/dev.rs
  - crates/ontoforge-api/src/dto.rs
  - crates/ontoforge-api/src/openapi.rs
  - crates/ontoforge-client/src/lib.rs
  - clients/typescript/src/index.ts
  - clients/typescript/src/codec.ts
  - crates/ontoforge-action/src/lib.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-storage/src/history.rs
  - crates/ontoforge-server/tests/health_lifecycle_it.rs
  - crates/ontoforge-server/tests/http_actions.rs
findings:
  critical: 10
  warning: 0
  info: 0
  total: 10
status: issues_found
phase_acceptance: pending
---

# Phase 7 Spec Review

固定比较 `git diff gsd/phase-06-l2-toolsets-audit...fd24beb`，已读取对应 commit log；merge-base 等于上述 base。范围为已实施 07-01..13/15/16。以下全部为静态调用链证据，confidence=high；没有新增动态复现。路径均相对于 `D:/DataSpellProjects/ontoforge`；`P` 表示 `.planning/phases/07-rest-mcp-shell`。

## Narrative Findings (AI reviewer)

### CR-01 [BLOCKER] 未接入 drift

**规格：** `P/07-06-PLAN.md:120`：“复用check_drift做启动与周期检测”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge/src/serve.rs:153-212` → `crates/ontoforge-server/src/lib.rs:142-245` 只安装 L2 monitor；`state.rs:16-24` 无 drift owner。全 crates 搜索确认 `engine/src/bind_check.rs:770` 仅被测试调用。ADR-0021:32/44 未撤销此要求。**影响/修正：** source schema 改变不会触发规定的 target 降级；接入启动、周期检查及目标状态消费。

### CR-02 [BLOCKER] describe 缺 links 与引擎状态

**规格：** `.planning/REQUIREMENTS.md:107`：“object types、属性、links、actions”；`:33`：“describe 暴露引擎状态”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp.rs:170-198` 仅投影 types/actions；REST `rest.rs:71-115` 有 links/status。**修正：** 复用受 scope 过滤的 metadata 投影并补两字段。

### CR-03 [BLOCKER] REST 丢失 Action 诊断

**规格：** `.planning/REQUIREMENTS.md:75/77`：“逐参数的错误”“配置的失败信息”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/rest.rs:702-708` → `error.rs:8-13` 只输出三个字段，丢弃 `action/src/lib.rs:116/122/134-137` 的详情。**修正：** 显式输出安全参数问题与 criteria 消息，同步 DTO/client。

### CR-04 [BLOCKER] Action schema 拒绝合法类型

**规格：** `P/07-07-PLAN.md:155`：“schema覆盖真实参数类型”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp_actions.rs:112` 将所有 ObjectReference 设为 string，`:79-90` 拒绝整数/Boolean PK；而 `action/src/params.rs:39-49` 按实际 PK 类型验证。`:123` 还将 Float/Double 数组声明为 string items。**修正：** 使用固定 ontology 的 PK/元素类型生成 schema。

### CR-05 [BLOCKER] 控制字段覆盖合法参数

**规格：** `P/07-07-PLAN.md:155`：“重名…拒绝而非静默覆盖”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp_actions.rs:25-34` 覆盖合法 `commandId`/`validateOnly`；`mcp.rs:507-511` 又删除它们，导致 required 业务参数永远缺失。`schema/src/ir/names.rs:17-27/206` 未保留这两个名称。**修正：** 明确无冲突控制参数映射并保留业务参数；合同选择交主会话。

### CR-06 [BLOCKER] MCP 错误缺结构且泄露 storage 信息

**规格：** `P/07-07-PLAN.md:155`：“code/apiName/request_id，L0不泄露物理信息”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp.rs:413/546/569` 直接 `to_string()` → `invalid_params(..., None)`；`storage/src/error.rs:38-46` 保留数据库原始消息，经透明 HistoryError/ActionError 可暴露物理 relation。**修正：** 穷举映射安全领域错误，将 envelope 放入 MCP error data，保留请求 ID。

### CR-07 [BLOCKER] history 上限与截断标志不成立

**规格：** `P/07-04-PLAN.md:28/116`：“超限显式 truncated/继续方式”“字节上限”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/rest.rs:637-648` 和 `mcp.rs:393-416` 无历史字节预算；storage `history.rs:147-167/214-216` 仅限条数并扩展完整记录。MCP 永远 `truncated:false`；REST 用用户 limit 比较，limit>500 时忽略 storage 的500 clamp。**修正：** 同一分页适配器按完整记录施加 bytes/条数预算并准确报告继续状态。

### CR-08 [BLOCKER] dev 锁存在双 owner 窗口

**规格：** `P/07-12-PLAN.md:30`：“活跃实例持独占dev锁…并发reset拒绝”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge/src/dev.rs:252-268` 创建空锁后才写 PID；第二进程可读空、判为 stale、删除并重建，双方随后均进入 `prepare:64-67`，reset 可删除活跃数据。**修正：** 使用跨进程独占锁，损坏/未完成锁不得当作可回收凭据；补 barrier 竞争测试。

### CR-09 [BLOCKER] MCP PK 截断伪装为空结果

**规格：** `.planning/REQUIREMENTS.md:70`：“截断并标 truncated”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp.rs:227-237` 丢弃 `QueryPage.truncated`；`query/src/lib.rs:205-215/394-398` 单行超预算返回空 rows + true，最终 MCP 返回成功 null。**修正：** 与 REST PK 同样返回明确预算错误，保留真实不存在语义。

### CR-10 [BLOCKER] typed health 丢失 L2 故障原因

**规格：** `P/07-CONTEXT.md:29`：“正文…暴露脱敏的 L2 禁用原因”；`.planning/REQUIREMENTS.md:101-102`：“全部 REST 能力”。**实现：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-api/src/dto.rs:373-376` 没有 l2，Rust `client/src/lib.rs:435-440` 丢弃 `server/src/health.rs:39-51` 生成的原因，TS生成类型同样不可访问。**修正：** DTO/OpenAPI/client 保留可选 L2 状态与原因。

## Coverage And Limits

检查上述 23 个变更文件的相关实现/调用点，补读 params、basetype、names、storage error、bind_check 等被调用模块；并非全部 399 个变更文件或全部函数的完整覆盖。规格覆盖来自 REQUIREMENTS、CONTEXT、各已实施 PLAN 的合同条款与 findings/ADR；未重新证明所有验收项。

未重复报告 Staff 五项已关闭问题、Windows helper 两轮已关闭问题，未将未实施 07-14 或已交给它的 artifact smoke 缺口视为新发现。未运行 Cargo、Docker、安装、网络查询或新测试；既有通过日志不作为本 lane 动态证明。无源代码、共享 planning 修改或提交。
