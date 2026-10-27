---
phase: "06"
slug: "l2-raw-dql-toolsets-audit"
status: verified
threats_open: 0
asvs_level: 1
created: "2026-10-28"
---

# Phase 6 Security

本报告按九个 PLAN 的 STRIDE register 核对实际代码与 SUMMARY。`threats_open` 只计 high 及以上未关闭项；Phase 6 总验收仍由 verifier 判断。

## Trust Boundaries

| Boundary | Data Crossing | Check |
| --- | --- | --- |
| 主体权限到 L2 SQL/Trino 身份 | principal、scope、SQL、catalog/schema | AST/allow-list、独立受限 Trino 身份与 ACL 探针 |
| 公开读取到异步审计 sink | 元数据、SHA-256、request_id | 容量预留、健康门禁、双后端事务与关闭 drain |
| 管理变更到在途请求 | definition/scope 版本 | validate/lint、锁内 hash、事务管理审计与请求快照 |
| CI 到外部服务 | Trino 483、PostgreSQL | 非零发现、真实受限 ACL 与独立 catalog |

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

## Resolved Threat

**T-06-01 (high)：已关闭。** `0d3d0bf` 的 RED 在共用 conformance 用例中证明正常 shutdown 会丢失已准入 guard 的记录；`5fd876d` 改为 shutdown 只停止新准入，guard 继续按 sink 健康状态入队，worker 等 sender 释放后 drain。`audit_shutdown_drains_inflight_guard` 对 SQLite 和 embedded PostgreSQL 各实测 1/1，要求持久行恰有一条，且新 reserve 被拒。故障关闭保持 fail-closed，不把正常关闭误报为 sink 故障。

审查报告的 WR-01 指出生产启动和周期复检尚未接线；Phase 6 提供可复用的 fail-closed 库状态，外壳接线在 Phase 7，未在本注册表将它当作权限绕过。WR-02 在 `bc43cbe` 后已有 32 条 allow 样例的真实 Trino 执行证据；104-ID fixture 是 constructed-equivalent，历史原文并未恢复。远端 CI 仍待执行。

## Accepted Risks Log

No accepted risks.

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
| --- | ---: | ---: | ---: | --- |
| 2026-10-28 | 18 | 17 | 1 | Phase 6 security audit |
| 2026-10-28 | 18 | 18 | 0 | T-06-01 fix verification |
| 2026-10-28 | 18 | 18 | 0 | `bc43cbe` Trino ACL fixture recheck |

## Sign-Off

- [x] 18 项均有 disposition 和证据位置。
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
