---
phase: "06"
slug: "l2-raw-dql-toolsets-audit"
status: verified
threats_open: 0
asvs_level: 1
created: "2026-10-28"
---

# Phase 6 Security

本报告保留前九个 PLAN 的 STRIDE register，并补充 Phase 06-10 ACL monitor 的实际代码和回归证据。`threats_open` 只计 high 及以上未关闭项；Phase 6 总验收仍由 verifier 判断。

## Trust Boundaries

| Boundary | Data Crossing | Check |
| --- | --- | --- |
| 主体权限到 L2 SQL/Trino 身份 | principal、scope、SQL、catalog/schema | AST/allow-list、独立受限 Trino 身份与 ACL 探针 |
| 公开读取到异步审计 sink | 元数据、SHA-256、request_id | 容量预留、健康门禁、双后端事务与关闭 drain |
| 管理变更到在途请求 | definition/scope 版本 | validate/lint、锁内 hash、事务管理审计与请求快照 |
| CI 到外部服务 | Trino 483、PostgreSQL | 非零发现、真实受限 ACL 与独立 catalog |
| manual/monitor 到 L2 health admission | owner ticket、generation、epoch、状态 | monitor 租约期间仅 owner 可启用；停止及旧结果均 fail-closed |
| monitor generation 到 probe DELETE | query ID、origin、pending token | engine 级清理义务、串行取消、匹配 token 清除、旧 owner 隔离 |

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation and Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- |
| T-06-01 | Repudiation | `ReadAudit` admission/drain | high | mitigate | `0d3d0bf` RED、`5fd876d` GREEN：`read_audit.rs:98-123,227-233,271-276` 分离 `accepting` 与 sink 健康；`conformance/mod.rs:2630-2660` 验证已发 guard 在并发 shutdown 时持久落库、新 reserve 被拒；SQLite/embedded PG 目标测试各 1/1，sink 失败时 guard 拒绝仍由 `:2662` 覆盖。 | closed |
| T-06-02 | Information disclosure | `of_read_audit` | high | mitigate | `read_audit.rs:244-245` 只计算 SQL SHA-256；双后端 conformance 检查无 `raw_sql`/`result_data` 列，`conformance/mod.rs:2525`。 | closed |
| T-06-03 | Repudiation | PostgreSQL sink | high | mitigate | `read_audit.rs` 事务批量写；PG conformance 现有持久化、故障与保留期测试，加并发 shutdown 目标用例经 embedded PG 真执行。 | closed |
| T-06-04 | Information disclosure | PostgreSQL system schema | high | mitigate | 双后端 conformance 检查 PG 审计表只在 system schema，`conformance/mod.rs:2510-2523`；受限 Trino 对 edit catalog 返回 `PERMISSION_DENIED`，`trino_l2_acl_it.rs:123`。 | closed |
| T-06-05 | Elevation of privilege | L2 SQL gate | critical | mitigate | `printer/l2.rs` 对 AST、relation、scope 限制；`l2_gate.rs` 拒绝路径零 POST、104-ID constructed-equivalent corpus 6/6；`bc43cbe` 使 32 条 allow 样例通过重序列化 SQL 在受限 Trino 483 真执行，ACL 集成测试 3/3。 | closed |
| T-06-06 | Tampering | `PrintedSql` | high | mitigate | `printer/l2.rs` 验证后 Display 成唯一可信 SQL 入口，`l2_gate.rs:21,139,205` 检查拒绝零 POST 与重序列化。 | closed |
| T-06-07 | Elevation of privilege | Trino ACL | critical | mitigate | `trino/l2_health.rs` 要求已有对象负探针精确 `PERMISSION_DENIED`；`bc43cbe` 的 `rules.json` 只给 L2 的 `hive.sales` 指定表 SELECT，末尾仍默认拒绝；受限 Trino 483 3/3、outside/editstore 拒绝、松规则对照禁用。额外实测未列入规则的 `hive.sales.private` 读和 `hive.sales.new` 建表均返回 `Access Denied`。 | closed |
| T-06-08 | Information disclosure | `system.runtime.queries` | critical | mitigate | `trino/l2_health.rs` 先由 ontology 身份确认另一主体活跃 query ID，再验证 L2 身份不可见；真实 Trino 测试 `trino_l2_acl_it.rs:160`。 | closed |
| T-06-09 | Repudiation | L0 list/set/aggregate | high | mitigate | `audit_l0.rs:187-752` 覆盖公开入口成功、拒绝、源故障、截断、取消和 sink 故障；共同关闭漏洞归 T-06-01。 | closed |
| T-06-10 | Elevation of privilege | L2 service | critical | mitigate | `query/l2.rs:414-451` 先核 `sql:raw`、scope 与 ACL 健康；`l2_execute.rs:163,208,438,473` 验证未授权零 POST。 | closed |
| T-06-11 | Denial of service | L2 results | high | mitigate | engine 查询设置有界；`l2_execute.rs:251,380,406,513` 覆盖截断、字节上限、超时及不释放半行。 | closed |
| T-06-12 | Information disclosure | L2 discovery | high | mitigate | `query/l2.rs:133-246` 对 catalog/schema 交叉过滤并排除 `system`/edit；`l2_discovery.rs` 拒绝与零 POST 测试。 | closed |
| T-06-13 | Repudiation | L2 discovery | high | mitigate | `query/l2.rs:99-129` 每请求保留并提交审计；`l2_discovery.rs` 四入口审计、取消及故障测试。共同关闭漏洞归 T-06-01。 | closed |
| T-06-14 | Spoofing | CI Trino ACL fixture | high | mitigate | `ci/query-services.sh` 独立受限/松规则容器并以 ontology 身份种下 memory `hive.sales`；`trino_l2_acl_it.rs` 确认 483、真实身份及已知活跃 query ID，本地 3/3、32 条 allow 样例均实际执行；CI 非零发现门禁。远端 CI 尚未触发。 | closed |
| T-06-15 | Repudiation | PostgreSQL ignored tests | medium | mitigate | `06-08-SUMMARY.md` 记录 embedded PG 5/5 真执行及 SHA，CI 非零发现/无服务失败门禁；远端 GitHub Actions 尚未运行。 | closed |
| T-06-16 | Tampering | reload/scope apply | high | mitigate | `cli_reload.rs`、`cli_scope.rs`、`scope_store.rs` 检查 lint/hash/锁/事务管理审计，`06-09-SUMMARY.md` 记录目标回归通过。 | closed |
| T-06-17 | Elevation of privilege | request snapshot | high | mitigate | `definition_snapshot.rs:107,328` 的受控并发测试验证 L0/L2 在途读、授权与审计版本固定；2/2 通过。 | closed |
| T-06-SC | Tampering | cargo installs | high | mitigate | Phase 6 使用 `--locked` 构建与测试；`Cargo.lock` 有锁定依赖，未执行新包安装。 | closed |
| T-06-19 | Elevation of privilege | manual/monitor health ownership | critical | mitigate | `06-10-REVIEW.md` CR-01：`97d3eef` RED 证明 manual 可绕过 CleanupFailed；`0cedb99` 在 `trino/l2_health.rs:114-149` 锁内校验 owner，`:171-180` 保持 stopped；`tests/l2_monitor.rs:609-638` 覆盖持有 monitor 与失败 shutdown 后的 manual 绕过，执行者最终 monitor 16/16 GREEN。 | closed |
| T-06-20 | Tampering | cross-generation pending cleanup | high | mitigate | `06-10-REVIEW.md` CR-02/CR-03：`97d3eef`、`8006851` RED；`0cedb99` 保留 engine 级 pending，`5f32e7f` 按 generation/token 串行清理并检查继承义务，`9738166` 加固旧响应在途断言。`trino/l2_monitor.rs:40-52,174-246,256-307,329-335`；`tests/l2_monitor.rs:641-722` 覆盖重启、延迟 DELETE、继承 shutdown 错误，执行者 monitor 16/16 和加固 unit 1/1 GREEN。 | closed |

## Resolved Threat

**T-06-01 (high)：已关闭。** `0d3d0bf` 的 RED 在共用 conformance 用例中证明正常 shutdown 会丢失已准入 guard 的记录；`5fd876d` 改为 shutdown 只停止新准入，guard 继续按 sink 健康状态入队，worker 等 sender 释放后 drain。`audit_shutdown_drains_inflight_guard` 对 SQLite 和 embedded PostgreSQL 各实测 1/1，要求持久行恰有一条，且新 reserve 被拒。故障关闭保持 fail-closed，不把正常关闭误报为 sink 故障。

审查报告的 WR-01 指出生产启动和周期复检尚未接线；Phase 6 提供可复用的 fail-closed 库状态，外壳接线在 Phase 7，未在本注册表将它当作权限绕过。WR-02 在 `bc43cbe` 后已有 32 条 allow 样例的真实 Trino 执行证据；104-ID fixture 是 constructed-equivalent，历史原文并未恢复。远端 CI 仍待执行。

## Phase 06-10 Security Evidence

本轮按 ADR-0028 和 `06-10-PLAN.md` 审查库级 monitor；完整修复复审基线为 `9738166f88d92e720b32f4de65a5a803e6e6dbc8`，最新测试结构增量签署快照为 `e81127fa41099365c44c2c42750024efad6653cb`。部署提供 bounded readonly probe SQL，同一 coordinator 的 ontology 与 L2 使用不同身份。每轮提交新的 ontology query，并在权限检查前后核实真实 RUNNING；L2 必须看不到该 ID；物理存在的负探针必须精确返回 `PERMISSION_DENIED`。GET/DELETE 前校验 origin，只有 DELETE 2xx/404 才认可清理成功并允许 owner 启用。

默认周期为 30s，整轮 20s deadline 包含两把锁等待；独立 expiry 为 60s，服务端 run/execution 均为 25s，清理锁等待与 HTTP 共用 2s 限制。generation/epoch 拒绝旧检查结果，engine 级 pending 保留跨代清理义务，token 匹配防止旧响应清除新 probe。shutdown 立即停止 admission 并报告继承清理失败，Drop 只取消自身 generation；超时或清理失败不会重新启用。

测试 fixture 以 stateful registry 追踪 ID、owner、state，成功 DELETE 移除条目，未知或已取消 ID 不能得到假 RUNNING 证明。真实 Trino 清理断言使用独占 `monitor_startup_probe` SQL alias，避免并行用例互相污染。执行者报告 lib 55/55、health 10/10、monitor 16/16、最后加固 unit 1/1 GREEN，以及默认并行真实 Trino 6/6 GREEN (8.60s)、32 条 allow SQL 全部 FINISHED。reviewer 读取三个 BLOCKER 的 RED JUnit，独立核对修复与测试断言，没有自行运行 Cargo；workspace/Clippy 由主会话负责。

当前 20 项 threat 全部 closed，没有新增 accepted risk。服务启动持有 monitor handle 和 HTTP `/health` 属于 Phase 7；本轮没有正式 PR 的 remote/base/评论核验、外部 Codex CLI gate、远端 CI 或发布审批，不能将本地证据表述为这些门禁已完成。

2026-10-28T01:39:03Z 增量复核 `80d053b`：仅 `trino/l2_monitor.rs:258,263` 在 `#[cfg(test)]` 内把 wiremock `method("DELETE")` 改为 `method(reqwest::Method::DELETE.as_str())`，HTTP 方法匹配语义相同，没有 production 行为变化或新增 threat。当前该文件 SHA256 为 `4A1F4AA69440BC153B180E97B6FE5ED2333F725253A4B2B7407BE401154E61AF`。主会话报告原 SQL 源码边界守卫保持不变、定向守卫 1/1 通过；此前测试数字属于修正前基线。本 reviewer 未运行 Cargo；本次安全签署只覆盖这两处测试表达修正，最新 workspace/Clippy 由主会话验证，未提前认定通过。详细范围见 `06-10-REVIEW.md` 的 HTTP Matcher Incremental Re-review。

2026-10-28T02:15:09Z 增量复核 `5988da8b21147b87363606677087e8ee158d3779`：唯一 diff 是 `tests/trino_l2_acl_it.rs:353` 把 `is_none_or(|rows| rows.is_empty())` 改为 `is_none_or(Vec::is_empty)`，相同的 None/空数组判定保持不变，没有 production 行为变化或新增 threat。当前该文件 SHA256 为 `0CE4DAB88AA3B61AF8BE2B2DDC4957CC579167D36E8554E0E4973A54CC0BAF0A`。主会话报告前一快照 `80d053b` 的 workspace 为 1251 passed、103 ignored、0 failed；最终快照的定向编译、Clippy 和增量 workspace 由主会话执行，本 reviewer 未运行 Cargo，也未认定这些最终门禁通过。本次签署只覆盖该单行测试表达修正，20 项 threat 均保持 closed。

2026-10-28T02:22:04Z 增量复核 `e81127fa41099365c44c2c42750024efad6653cb`：相对 `5988da8` 只把 `src/trino/l2_monitor.rs` unit module 原样移到末尾，并在 `tests/l2_monitor.rs` 提取 DELETE `cancel_response`、进行 4 处等价 `clone_into` 赋值。独立比较确认迁移 module 与其外 production 文本一致，helper 的 registry/status/延迟处理顺序不变，无新增 lint suppression、production 行为变化或 threat；当前 20 项仍全 closed。两个文件 SHA256 分别为 `5D7E63F134F67E9231E0EB82ADA268FEB0F9415D32F367794A300F974F716A8B`、`33C9976F5B165EF98348F729AF08163F2CE7382B6BC0D24182D19499F5B3AACD`。执行器报告 unit 55/55、monitor 16/16、完整 workspace Clippy `-D warnings` exit 0 (25.79s)、fmt/diff exit 0；reviewer 未运行 Cargo。最终 workspace 回归由主会话执行，当前 pending，本次安全签署仅覆盖上述测试整理。最终位置映射及范围见 `06-10-REVIEW.md` 的 Test Structure Incremental Re-review。

## Accepted Risks Log

No accepted risks.

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
| --- | ---: | ---: | ---: | --- |
| 2026-10-28 | 18 | 17 | 1 | Phase 6 security audit |
| 2026-10-28 | 18 | 18 | 0 | T-06-01 fix verification |
| 2026-10-28 | 18 | 18 | 0 | `bc43cbe` Trino ACL fixture recheck |
| 2026-10-28 | 20 | 20 | 0 | Phase 06-10 monitor independent re-review at `9738166` |
| 2026-10-28 | 20 | 20 | 0 | HTTP matcher expression incremental re-review at `80d053b` |
| 2026-10-28 | 20 | 20 | 0 | Closure expression incremental re-review at `5988da8` |
| 2026-10-28 | 20 | 20 | 0 | Test structure incremental re-review at `e81127f` |

## Sign-Off

- [x] 20 项均有 disposition 和证据位置，原 18 项保留。
- [x] 没有 accepted risk 代替修复。
- [x] `threats_open: 0`。
- [x] `status: verified`。

**Approval:** verified 2026-10-28

## Security Audit 2026-10-27

| Metric | Count |
|---|---|
| Threats found | 18 |
| Closed | 18 |
| Open | 0 |

## Security Audit 2026-10-28

| Metric | Count |
|---|---|
| Threats found | 20 |
| Closed | 20 |
| Open | 0 |

## Security Audit HTTP Matcher Re-review 2026-10-28

| Metric | Count |
|---|---|
| Threats found | 20 |
| Closed | 20 |
| Open | 0 |

## Security Audit Closure Re-review 2026-10-28

| Metric | Count |
|---|---|
| Threats found | 20 |
| Closed | 20 |
| Open | 0 |

## Security Audit Test Structure Re-review 2026-10-28

| Metric | Count |
|---|---|
| Threats found | 20 |
| Closed | 20 |
| Open | 0 |
