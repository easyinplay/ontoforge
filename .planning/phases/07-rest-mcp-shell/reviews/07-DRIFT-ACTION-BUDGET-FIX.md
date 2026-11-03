---
status: frozen_for_integration
date: 2026-11-03
tests_executed: true
---

# AB01 Action source budget 修复

合同依据：`07-04-PREFLIGHT.md` 方案A、主会话明确指令、`target/phase7-claude-action-budget-review.json` 的AGREE。RF01把Action接到带Query limits的ScopedEngine，错误继承HTTP deadline与独立读row/byte限制；正式Action应保持原领域source bounds，HTTP deadline/断连只结束响应等待。

## 方案

保留同一个ScopedEngine与固定drift view，增加显式 `EnginePurpose::{Query,Action}`。Action execute原样透传QueryContext；Query保留现有预算收紧与trace。target gate不依赖purpose，REST正式/validate-only与MCP Action均继续gate。无新client、schema或公开配置。

## 测试范围

独立 `crates/ontoforge-server/tests/action_source_budget.rs`，复用已有fixture数据但使用真正遵守ctx.timeout的source Engine。由barrier证明source已进入，再触发HTTP deadline或TCP断连，释放后通过原command取得receipt；重开数据库对账edit/log/outbox与receipt恰好一次。对照Action领域timeout/max_rows/max_bytes与Query收紧后的值；validate-only继续零写。

## 当前状态

独立测试草案已落盘并通过rustfmt解析，5例：REST/MCP各deadline与TCP断连、validate-only双transport与Query预算对照。Engine把barrier等待和真实SQLite执行一起纳入ctx.timeout。MCP deadline覆盖client HTTP timeout终止SSE等待，不假设一定有JSON/504；与原transport合同一致。

transport worker完成原源码恢复后主会话明确交接，本executor使用Cargo（jobs2）取得下列证据，现已将Cargo正式交回transport执行DC01与整合验证。`target/phase7-action-budget-red.log` exit101：0 passed/5 failed，编译成功；4个正式Action在deadline/断连后GET receipt为404，validate-only为SOURCE_UNAVAILABLE/503。

已最小修改context/rest/mcp：EnginePurpose enum、Action execute原样透传、8处ScopedEngine字面量显式Query/Action。共同check_source_target保持不变。`target/phase7-action-budget-green.log` exit0：5 passed/0 failed，四种正式Action情形均GET+replay原receipt，新DB连接证明receipt/edit/log/outbox各1；validate-only保持原领域ctx且零command/outbox，Query仍受rows/bytes/deadline限制。未commit/push。

## FT01 补测

主会话与Claude Code同意增加test-only合同验收：旧v1多页列表第1次source调用barrier暂停，reload并唤醒新版本degraded报告，新请求应SOURCE_SCHEMA_DRIFT/503；旧请求释放后必须实际发生第2次gate/SQL且成功，durable audit仍为v1。不以Action证明此合同（Action还有commit fence），不增加生产API。

已接入已有 `source_drift.rs`，使用受控256行首批和真实SQLite空后续页，确保生产list循环确实执行两次source；metadata仅在新报告阶段注入missing-table。`target/phase7-fixed-view-green.log` exit0：20 passed/0 failed，旧请求第2次gate/SQL成功、256行完整返回、audit def_version=v1；新请求SOURCE_SCHEMA_DRIFT、audit def_version=v2。测试fixture扩批逻辑补充空页检查，不修改生产行为。

## 冻结与验证交接

- 生产文件仅 `server/src/context.rs`、`rest.rs`、`mcp.rs`；自AB01 GREEN后仅补enum文档行，业务逻辑未再变。
- `target/phase7-action-budget-clippy.log`：五包all-target `-D warnings` exit0，包含transport的新DC01测试编译；初次测试helper101行告警已通过合并transport path/body分支消除。
- `target/phase7-action-budget-fmt.log`：本lane五个Rust文件rustfmt --check exit0；git diff --check无错误。
- 测试最后增强：validate-only除已验证command/outbox为零外，新增edit/log为零断言，并机械合并path/body分支。增强后的测试已编译/lint，尚未动态重跑；主会话与transport已收到交接，安排与DC01一起运行 `action_source_budget`。不得把增强后的断言算作已执行。
- 不改共享planning、其他worker文件，不commit/push；已释放Cargo，不留运行任务。

| 冻结文件（crates/ontoforge-server/） | Git blob |
|---|---|
| src/context.rs | 5f55345d24301b8b43d8ff669d51e32ae556f3ec |
| src/rest.rs | e39b93f6131187488233bd728adc2118b4b6420c |
| src/mcp.rs | c0cd841d97aeb63f6805bfb66de15ff1e49e8465 |
| tests/action_source_budget.rs | 84634688a5feca5d3b29477b7035c16f697cc18f |
| tests/source_drift.rs | 45083f2ee5caeb8bbeaac7bc0f2eddb6acc3521b |
