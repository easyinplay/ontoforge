---
status: complete
phase: 07-rest-mcp-shell
source: [07-01-SUMMARY.md, 07-02-SUMMARY.md, 07-03-SUMMARY.md, 07-04-SUMMARY.md, 07-05-SUMMARY.md, 07-06-SUMMARY.md, 07-07-SUMMARY.md, 07-08-SUMMARY.md, 07-09-SUMMARY.md, 07-10-SUMMARY.md, 07-11-SUMMARY.md, 07-12-SUMMARY.md, 07-13-SUMMARY.md, 07-14-SUMMARY.md, 07-15-SUMMARY.md, 07-16-SUMMARY.md]
started: 2026-11-04T00:28:00+08:00
updated: 2026-11-03T16:32:58.949Z
verified_source_commit: adc02278b5cc55111def9a30a1c3b778485e8279
---

## Current Test

[testing complete]

## 验收方式与证据边界

按用户授权执行可自动化的验收，以下 `source: automated` 表示实际测试或 CI 证据，不表示用户手工确认。每项引用最终或相同生产源码的动态结果；详细测试矩阵见 `07-VALIDATION.md`，整体门禁与三平台 checksum 见 `07-14-SUMMARY.md`。没有网页 UI 验收项。

历史 SUMMARY 的 PG snapshot（07-02 D4）和 stdio 正向等价性（07-09 D3）曾为 unknown，现由下述第 3、9 项的后继自动化证据闭合，原始 SUMMARY 保留时点事实。该归并方式经 Claude Code 第二意见 AGREE，见 `target/phase7-final-uat-review.json`。

## Tests

### 1. 冷启动、演示、重启与显式 reset
expected: 解包后的单二进制在独立目录启动 serve --dev，查询、Action、历史可用；重启保留数据，显式 reset 重建，退出能清理进程。
result: pass
source: automated
evidence: release CI 41669982362 的三个 native install smoke；07-12-SUMMARY.md；07-14-SUMMARY.md。

### 2. REST 授权读取与结构化错误
expected: 元数据、pk、query、aggregate、history 遵循主体 scope，拒绝越权；错误含稳定 code、apiName、request_id，L0 不泄漏物理名。
result: pass
source: automated
evidence: 07-03-SUMMARY.md、reviews/07-SERVER-SECURITY-EVIDENCE.md；fresh Rust9/9、TS17/17 与 PG4/4。

### 3. 固定请求版本与 PG 并发隔离
expected: reload/revoke 并发不会混用 definition、roles、bindings；definition fence 拒绝后无业务写入；REST/MCP 同 command 仅一条持久 receipt。
result: pass
source: automated
evidence: target/phase7-services/evidence/request_snapshot.log 1/1、postgres_http_it.log 4/4，commit.txt=adc0227；关闭历史07-02 D4。

### 4. OpenAPI 与 wire 契约
expected: 无服务依赖可导出确定性 OpenAPI 3.1；快照无漂移，SDK 生成/typecheck/build 通过，整数与 decimal 保持无损。
result: pass
source: automated
evidence: 07-05-SUMMARY.md；target/phase7-final-adc0227-verify.log 中 check:generated/typecheck/build、Rust9/9 与 TS17/17。

### 5. Action、validate-only、receipt 和历史
expected: Action 真实写入与 edit log 一致；validate-only 零写；越权、过期版本及故障不产生额外持久副作用，显式 command replay 可取原 receipt。
result: pass
source: automated
evidence: reviews/07-SERVER-SECURITY-EVIDENCE.md 的HTTP3、MCP Action10、contract16；最终PG4/4、SDK合同与stdio成功链。

### 6. MCP toolset 与动态 schema
expected: core/full/sql:raw 按主体过滤；动态 Action 参数和 validate-only 可调用；L3 与定义/权限管理工具不可见且直调拒绝。
result: pass
source: automated
evidence: 07-07-SUMMARY.md、07-08-SUMMARY.md、reviews/07-SERVER-SECURITY-EVIDENCE.md 与 07-BRIDGE-SECURITY-EVIDENCE.md。

### 7. L2 六工具、默认值与 typed EXPLAIN
expected: 六工具名称/参数符合合同；catalog/schema defaults 和 allow-list 本地检查先于上游；四种 EXPLAIN 格式的取消、超时、限流与单次 audit 生效。
result: pass
source: automated
evidence: 07-15/07-16-SUMMARY.md；query15/15、E6六工具拒绝矩阵；target/phase7-mcp-admission-green-r2.log 50/50；fresh Trino2/2和ACL6/6。

### 8. MCP session 容量与执行资源
expected: session 容量、TTL、owner、撤销和关闭受控；REST/MCP 共用实际业务并发限制，SSE 响应头不会提前释放，独立控制槽可在满载时取消。
result: pass
source: automated
evidence: reviews/07-T24-IMPLEMENTATION.md、07-T24-SECURITY-RECHECK.md、07-MCP-EXECUTION-ADMISSION-IMPLEMENTATION.md、07-T08-SECURITY-RECHECK.md；session12/12与最终server50/50。

### 9. 真实 stdio 桥接与故障
expected: stdio describe/query/Action/dryrun/history 与 HTTP 同权限、同持久数据；EOF 有界退出，stdout 仅协议帧；在线 bad token、302、session 失效和响应丢失不会泄密或隐式重发。
result: pass
source: automated
evidence: target/phase7-bridge-security-cli-r2.log 11/11、Mia扩展1/1；reviews/07-BRIDGE-SECURITY-EVIDENCE.md；最终native stdio smoke。关闭历史07-09 D3。

### 10. Rust/TypeScript SDK 主体与取消隔离
expected: 两种 SDK 覆盖全部 REST 端点组；主体撤权、并发取消与实例配置彼此隔离；传输失败不自动重试，显式 replay 仅一个持久提交。
result: pass
source: automated
evidence: target/phase7-services/evidence/rust-client.log 9/9、typescript-contract.log 17/17，最终adc0227 binary；reviews/07-SDK-SECURITY-EVIDENCE.md。

### 11. 健康告警、drift、恢复与关闭
expected: L2 attestation 失败/过期时 fail-closed，health 脱敏告警且 core 仍可用；恢复后重新开放；source drift 隔离目标，关闭排空并完成审计与取消。
result: pass
source: automated
evidence: fresh health2/2、drift2/2、ACL6/6；target/phase7-services/evidence/final-adc0227-verify.log，exit0含scan/cleanup，Windows服务exited/exit0。

### 12. 三平台发行包与发布边界
expected: Linux x64、macOS arm64、Windows x64 实际 archive、manifest、SHA256 一致并通过安装 smoke；PR 构建不上传 Release，不发布 npm，不合并 PR。
result: pass
source: automated
evidence: release41669982362 native3/3与aggregate success、publish skipped；target/phase7-native-artifacts-41669982362-verification.json；实际打包b75d2949与source adc0227 tree相同；PR7/8状态OPEN/Draft。Release published事件上传仅静态审查，按用户约束未触发。

### 13. Windows/Ubuntu 完整回归
expected: adc0227 的 workspace、fmt、Clippy、embedded-pg、TLS、平台附加测试、integration及真实服务四个必需job全部通过。
result: pass
source: automated
evidence: CI41669982375 completed/success，Windows/Ubuntu/integration/phase7-real-services四个必需job全部success。元数据target/phase7-ci-41669982375.json，完整日志及workspace计数保存于同前缀windows/ubuntu文件；isolated-query-bench为本次非必需条件作业，skipped。

## Summary

total: 13
passed: 13
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

无未解决验收缺口。13项均由实际自动化证据通过。
