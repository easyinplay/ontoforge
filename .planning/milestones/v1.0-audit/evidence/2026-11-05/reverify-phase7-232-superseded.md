---
phase: 07-rest-mcp-shell
verified: 2026-11-05T09:02:32Z
status: passed
score: 80/80 must-haves verified
covered_files: [".github/workflows/ci.yml",".github/workflows/release.yml",".planning/phases/07-rest-mcp-shell/07-01-PLAN.md",".planning/phases/07-rest-mcp-shell/07-01-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-02-PLAN.md",".planning/phases/07-rest-mcp-shell/07-02-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-03-PLAN.md",".planning/phases/07-rest-mcp-shell/07-03-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-04-PLAN.md",".planning/phases/07-rest-mcp-shell/07-04-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-05-PLAN.md",".planning/phases/07-rest-mcp-shell/07-05-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-06-PLAN.md",".planning/phases/07-rest-mcp-shell/07-06-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-07-PLAN.md",".planning/phases/07-rest-mcp-shell/07-07-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-08-PLAN.md",".planning/phases/07-rest-mcp-shell/07-08-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-09-PLAN.md",".planning/phases/07-rest-mcp-shell/07-09-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-10-PLAN.md",".planning/phases/07-rest-mcp-shell/07-10-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-11-PLAN.md",".planning/phases/07-rest-mcp-shell/07-11-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-12-PLAN.md",".planning/phases/07-rest-mcp-shell/07-12-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-13-PLAN.md",".planning/phases/07-rest-mcp-shell/07-13-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-14-PLAN.md",".planning/phases/07-rest-mcp-shell/07-14-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-15-PLAN.md",".planning/phases/07-rest-mcp-shell/07-15-SUMMARY.md",".planning/phases/07-rest-mcp-shell/07-16-PLAN.md",".planning/phases/07-rest-mcp-shell/07-16-SUMMARY.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/pg/seed.sql","ci/query-services.sh","ci/release-package.ps1","ci/release-smoke.ps1","ci/server-services.sh","ci/test_tls_deps.py","ci/test_windows_server.py","ci/windows-server.py","clients/typescript/package-lock.json","clients/typescript/package.json","clients/typescript/src/codec.ts","clients/typescript/src/generated.ts","clients/typescript/src/index.ts","clients/typescript/tests/contract.test.mjs","clients/typescript/tests/error-types.ts","clients/typescript/tests/health-types.ts","clients/typescript/tests/numeric-security.test.mjs","clients/typescript/tests/support-security.mjs","crates/ontoforge-action/src/lib.rs","crates/ontoforge-action/src/source.rs","crates/ontoforge-action/tests/frozen_definition.rs","crates/ontoforge-action/tests/idempotency_pg.rs","crates/ontoforge-action/tests/trino_action_it.rs","crates/ontoforge-api/Cargo.toml","crates/ontoforge-api/src/dto.rs","crates/ontoforge-api/src/lib.rs","crates/ontoforge-api/src/openapi.rs","crates/ontoforge-api/tests/openapi_snapshot.rs","crates/ontoforge-client/Cargo.toml","crates/ontoforge-client/src/lib.rs","crates/ontoforge-client/tests/contract.rs","crates/ontoforge-client/tests/error_diagnostics.rs","crates/ontoforge-client/tests/support/mod.rs","crates/ontoforge-client/tests/support/security.rs","crates/ontoforge-engine/Cargo.toml","crates/ontoforge-engine/src/bind_check.rs","crates/ontoforge-engine/src/cardinality_check.rs","crates/ontoforge-engine/src/config.rs","crates/ontoforge-engine/src/lib.rs","crates/ontoforge-engine/src/printer/ast.rs","crates/ontoforge-engine/src/printer/dialect.rs","crates/ontoforge-engine/src/printer/l2.rs","crates/ontoforge-engine/src/printer/tests.rs","crates/ontoforge-engine/src/trino/cancel.rs","crates/ontoforge-engine/src/trino/l2_monitor.rs","crates/ontoforge-engine/src/trino/mod.rs","crates/ontoforge-engine/tests/cardinality_check.rs","crates/ontoforge-engine/tests/drift_budget.rs","crates/ontoforge-engine/tests/l2_explain.rs","crates/ontoforge-engine/tests/l2_monitor.rs","crates/ontoforge-engine/tests/server_config.rs","crates/ontoforge-engine/tests/trino_resilience.rs","crates/ontoforge-query/src/aggregate.rs","crates/ontoforge-query/src/execute.rs","crates/ontoforge-query/src/l2.rs","crates/ontoforge-query/src/lib.rs","crates/ontoforge-query/src/link.rs","crates/ontoforge-query/tests/audit_pk.rs","crates/ontoforge-query/tests/l2_execute.rs","crates/ontoforge-schema/src/diag.rs","crates/ontoforge-server/Cargo.toml","crates/ontoforge-server/src/context.rs","crates/ontoforge-server/src/drift.rs","crates/ontoforge-server/src/error.rs","crates/ontoforge-server/src/health.rs","crates/ontoforge-server/src/history_page.rs","crates/ontoforge-server/src/lib.rs","crates/ontoforge-server/src/lifecycle.rs","crates/ontoforge-server/src/mcp.rs","crates/ontoforge-server/src/mcp_actions.rs","crates/ontoforge-server/src/mcp_catalog.rs","crates/ontoforge-server/src/mcp_l2.rs","crates/ontoforge-server/src/mcp_sessions.rs","crates/ontoforge-server/src/rest.rs","crates/ontoforge-server/src/state.rs","crates/ontoforge-server/tests/action_source_budget.rs","crates/ontoforge-server/tests/health_lifecycle_it.rs","crates/ontoforge-server/tests/health_trino_it.rs","crates/ontoforge-server/tests/history_errors.rs","crates/ontoforge-server/tests/history_pagination.rs","crates/ontoforge-server/tests/http_action_diagnostics.rs","crates/ontoforge-server/tests/http_action_safety.rs","crates/ontoforge-server/tests/http_actions.rs","crates/ontoforge-server/tests/http_contract.rs","crates/ontoforge-server/tests/mcp_actions.rs","crates/ontoforge-server/tests/mcp_contract.rs","crates/ontoforge-server/tests/mcp_execution_admission.rs","crates/ontoforge-server/tests/mcp_l2.rs","crates/ontoforge-server/tests/mcp_l2_defaults.rs","crates/ontoforge-server/tests/mcp_transport.rs","crates/ontoforge-server/tests/postgres_http_it.rs","crates/ontoforge-server/tests/source_drift.rs","crates/ontoforge-server/tests/source_drift_cleanup.rs","crates/ontoforge-server/tests/source_drift_trino_it.rs","crates/ontoforge-server/tests/support/mod.rs","crates/ontoforge-server/tests/support/pg_real.rs","crates/ontoforge-server/tests/support/real.rs","crates/ontoforge-server/tests/support/security_evidence.rs","crates/ontoforge-server/tests/trino_http_it.rs","crates/ontoforge-server/tests/unit/drift.test.rs","crates/ontoforge-server/tests/unit/http_contract.test.rs","crates/ontoforge-server/tests/unit/mcp_admission.test.rs","crates/ontoforge-server/tests/unit/mcp_sessions.test.rs","crates/ontoforge-server/tests/unit/rest.test.rs","crates/ontoforge-server/tests/unit/tracer.test.rs","crates/ontoforge-storage/src/history.rs","crates/ontoforge-storage/src/lib.rs","crates/ontoforge-storage/src/request_snapshot.rs","crates/ontoforge-storage/tests/frozen_history.rs","crates/ontoforge-storage/tests/history_masking.rs","crates/ontoforge-storage/tests/request_snapshot.rs","crates/ontoforge/Cargo.toml","crates/ontoforge/src/cli.rs","crates/ontoforge/src/dev.rs","crates/ontoforge/src/dev_assets.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/mcp.rs","crates/ontoforge/src/online.rs","crates/ontoforge/src/serve.rs","crates/ontoforge/tests/cli_check_cardinality.rs","crates/ontoforge/tests/cli_color.rs","crates/ontoforge/tests/cli_l2_identity.rs","crates/ontoforge/tests/cli_shutdown.rs","crates/ontoforge/tests/cli_stdio.rs","crates/ontoforge/tests/cli_trino_it.rs","crates/ontoforge/tests/dev_lifecycle.rs","crates/ontoforge/tests/stdio_security_support/mod.rs","crates/ontoforge/tests/unit/dev.test.rs","crates/ontoforge/tests/unit/serve.test.rs","docs/api.md","docs/dev.md","docs/installation.md","docs/openapi.json","examples/server/ontoforge-server.dev.toml","target/l2-catalog-clippy.log","target/l2-catalog-green.log","target/l2-catalog-red.log","target/l2-catalog-server-regression.log","target/milestone-audit-evidence-20261104/cleanup.log","target/milestone-audit-evidence-20261104/commit.txt","target/milestone-audit-evidence-20261104/final-check.log","target/milestone-audit-evidence-20261104/health_trino_it.discovery.log","target/milestone-audit-evidence-20261104/health_trino_it.log","target/milestone-audit-evidence-20261104/postgres_http_it.discovery.log","target/milestone-audit-evidence-20261104/postgres_http_it.log","target/milestone-audit-evidence-20261104/request_snapshot.discovery.log","target/milestone-audit-evidence-20261104/request_snapshot.log","target/milestone-audit-evidence-20261104/runner.log","target/milestone-audit-evidence-20261104/rust-client.log","target/milestone-audit-evidence-20261104/server-cleanup.log","target/milestone-audit-evidence-20261104/server.log","target/milestone-audit-evidence-20261104/source-equivalence.diff","target/milestone-audit-evidence-20261104/source-guard.log","target/milestone-audit-evidence-20261104/source_drift_trino_it.discovery.log","target/milestone-audit-evidence-20261104/source_drift_trino_it.log","target/milestone-audit-evidence-20261104/sqlite-action-atomic.log","target/milestone-audit-evidence-20261104/start-commit.txt","target/milestone-audit-evidence-20261104/trino-483-ctas.log","target/milestone-audit-evidence-20261104/trino-483-fixture.log","target/milestone-audit-evidence-20261104/trino-483-iceberg.log","target/milestone-audit-evidence-20261104/trino-484-ctas.log","target/milestone-audit-evidence-20261104/trino-484-fixture.log","target/milestone-audit-evidence-20261104/trino-484-iceberg.log","target/milestone-audit-evidence-20261104/trino_http_it.discovery.log","target/milestone-audit-evidence-20261104/trino_http_it.log","target/milestone-audit-evidence-20261104/trino_l2_acl_it.discovery.log","target/milestone-audit-evidence-20261104/trino_l2_acl_it.log","target/milestone-audit-evidence-20261104/typescript-contract.log","target/milestone-ci-41759804733-integration.log","target/milestone-ci-41759804733-ubuntu.log","target/milestone-ci-41759804733-windows.log","target/milestone-ci-41759804733.json","target/milestone-release-41759804768/packaged-commit.json","target/milestone-release-41759804768/run.json","target/milestone-release-41759804768/run.log","target/milestone-release-41759804768/source-commit.json","target/milestone-release-41759804768/verification.json","target/milestone-tls-real-tree.log","target/phase7-bridge-security-cli-r2.log","target/phase7-bridge-security-mia.log","target/phase7-ci-41669982375-ubuntu-counts.json","target/phase7-ci-41669982375-windows-counts.json","target/phase7-ci-41669982375.json","target/phase7-explain-query-tests.log","target/phase7-mcp-admission-green-r2.log","target/phase7-native-artifacts-41669982362-verification.json","target/phase7-pr7-final-status.json","target/phase7-pr8-final-status.json","target/phase7-release-ci-41669982362.json","target/phase7-services/evidence/commit.txt","target/phase7-services/evidence/final-adc0227-verify.log","target/phase7-verify-pre-api-coverage.json","tools/openapi-gen/generate.mjs","tools/openapi-gen/package-lock.json","tools/openapi-gen/package.json"]
covered_digest: "v4:sha256:bb31129c390b1163601613324c4da4023e9834f5aa71e50941cea0218d6e67d7"
behavior_unverified: 0
overrides_applied: 0
human_verification: []
decision_coverage:
  honored: 3
  total: 3
  not_honored: []
source_commit: 060019fe52005c6b85ae5c5f59da9868f464e579
verified_source_commit: 060019fe52005c6b85ae5c5f59da9868f464e579
historical_verified_at: 2026-11-04T03:07:13Z
historical_source_commit: f8851ca073fed34e80e605f0ac91880259df7e12
current_ci_run: 41994479295
package_checkout_commit: 22913c8e79ae98b6604109d6e05a15f27cd50292
verified_implementation_tree: c5423a76803201bd49e5584833ece58bde32ebf5
historical_implementation_tree: 0ecc59382aaf12e782c508ba64858e2689299fd7
uat_source_commit: f8851ca073fed34e80e605f0ac91880259df7e12
requirements_verified: 22
re_verification:
  previous_status: passed
  previous_score: 80/80
  gaps_closed: []
  gaps_remaining: []
  regressions: []
incremental_scope: "2026-11-05 final-source affected-path regression, local workspace/SQLite/TLS and fresh five-job CI; original 80 truths retained, historical UAT/release evidence not rerun"
unresolved_prohibitions: 0
---

# Phase 7: REST & MCP Shell Verification Report

## 当前最终源码增量复验（2026-11-05）

**Status:** passed。保留原合同 80/80 truths，behavior_unverified=0、overrides_applied=0；80 是既有合同分母，不是本轮新增测试数。本轮是目标反推的增量回归复签：以 `f8851ca` 为上一 verified source，确认受影响实现、依赖传播与实际行为，再以当前 HEAD `060019fe52005c6b85ae5c5f59da9868f464e579` 的真实执行恢复新鲜性。未将 SUMMARY、旧测试、空 harness、ignored-only 或新 fingerprint 当作本轮行为证据。

### 受影响合同与实证

| 真值或接线 | 当前实现、实际行为与新原始证据 | 判定 |
| --- | --- | --- |
| CLI/engine 增量接线不破坏下游 API/MCP | `cli.rs` → `online.rs` → `cardinality_targets/check_cardinality` → typed CountDistinct printer → `Engine.execute` → ONT-W18。完整实际 diff/扫描见 `reverify-source.diff`、`reverify-source-checks.log`；本地 cardinality CLI 13、engine cardinality 11、monitor 19 passed，逐 target discovery/execution 均正数 | VERIFIED |
| HTTP/MCP、schema 与 L2 transport 既有合同回归 | 本地完整 workspace 152 个 identity 一一对应，1647 discovered，1528 passed/0 failed/119 ignored；其中 HTTP/MCP、source drift、mcp_l2_defaults 15、Rust client contract 9 实际执行，逐项见 `reverify-workspace-counts.json` | VERIFIED |
| 真实 PG/Trino 服务链路 | 新 run `41994479295` 的 `phase7-real-services`：postgres_http_it 4、source_drift_trino_it 2、PG request_snapshot 1、trino_http_it 2、health_trino_it 2、trino_l2_acl_it 6，全为 nonignored passed。原始结果行 606/618/633/645/657/706 | VERIFIED |
| SDK 合同不被底层增量破坏 | 同 job Rust contract 9 passed（结果行 731），TypeScript 17 passed、fail/skipped/cancelled=0（793–799）。Rust 是真实 HTTP + SQLite fixture；TS 是真实 binary `serve --dev` + 临时 SQLite，部分异常案例受控 HTTP peer，不冒充 PG/Trino SDK | VERIFIED |
| 持久化与 fault rollback 依赖链回归 | 本地 SQLite `action_atomic_` test-support 5 discovered/5 passed，CI 两平台各再跑 5；新 integration 的 PG atomic 5、same-pk 4、idempotency 6、webhook 2、source guard 1 与 Trino action 4 真实执行 | VERIFIED |
| TLS enforcement 与 Windows cleanup 行为 | 本地 TLS hermetic 6/6、真实依赖检查 exit=0；两平台 CI 同样执行。Windows private-console 实际 11/11，原始行 3834–3849，覆盖错误 PID、stale creation time、额外 console member 拒绝、owner crash reap 与 timeout owned-target cleanup | VERIFIED |

### 终态 CI、计数与证据边界

- 已独立逐行读取全部五份新原始 job 日志，保留 BOM/ANSI；HEAD 与 final metadata 一致，5/5 jobs completed/success。计数仅在各 workspace step 时间窗口内：Ubuntu `08:30:42Z–08:41:17Z` 152 groups、1535 passed/0 failed/119 ignored；Windows `08:34:24Z–08:52:48Z` 152 groups、1528 passed/0 failed/119 ignored。每个 runtime `running N tests` 与结果相符；不把后续 target 重跑或跨平台相加作唯一测试数。
- CI 真实服务、integration、isolated bench 的 job-scoped credentials 扫描、artifact upload、cleanup 均 success。本机没有启动或修改 Docker；受保护容器保持原状。isolated bench 1 passed，四路径 p95 均低于各自阈值；只有原始日志与 upload 证据，未把未下载的 JSON 扩展为完整产物审计。
- 证据计数器经已确认 seam（日志输入 → JSON `complete`）真实 TDD 修复：zero-required 和 duplicate-identity 反例先 RED，后最小修复 GREEN；11/11 Node 回归通过。12 个显式命名空 harness、12 个具理由/精确数量的 ignored-only 排除项均不计成功，128 个 nonignored target 才计成功；119 ignored 不声称全部已在 CI 执行。原始 RED 保留，它们是计数器反例，不是产品失败。
- 本地 D 盘首次 discovery 因剩余空间主动中止，未执行测试，也不计通过；改用 C 盘 owner 专属 target 后完整执行。gh 下载 escape protection 和 CI API 一次 TLS 握手超时重试是取证环境边界，不是产品失败。
- 本轮不重跑历史 native release、14 项人工 UAT、独立安全审查与 07.2 MinIO483/484；下方 2026-11-04 及更早内容均为保留的历史证据，不虚改其日期、来源或绿色结论。`package_checkout_commit`、`uat_source_commit` 仍只标识历史 package/UAT，不是本轮 checkout。
- 指纹在本地测试完成、五份原日志亲读与计数器 GREEN 之后，由 GSD `verification.fingerprint` 生成，200 个显式实现/测试/CI 输入加工具自动收集本 phase PLAN/SUMMARY，共 232 covered files。输出完整保存在 `reverify-fingerprint.json`，覆盖旧实现及新 cardinality/online/printer/diag/fixture 接线；指纹只是内容绑定，不是测试。
- 完整执行命令、原日志、SHA256、逐 target 和影响矩阵归档于 `.planning/milestones/v1.0-audit/FINAL-REVERIFY.md` 与其 `evidence/2026-11-05/reverify-*`。本轮不扩签 P1–6：P01 human_needed、P02 gaps_found、ENGINE-05 SQLServer Partial 保留；P03–06 测试已重跑但其原报告尚未重新签署，不宣称 milestone 完成。

### 增量判定与人工项

受影响代码无未引用 TBD/FIXME/XXX，无新 stub、断链或动态复验失败；现有负约束未见回归。未新增 override、BLOCKER、WARNING 或需人工裁决项；原 80 条合同与历史判定不缩减。本轮仅签受影响当前源码的行为回归与 Phase 07 新鲜性，不为未重跑的历史交付另造行为证据。owner `verification.status` 的写后复核另存 `reverify-status-after.json`。

## 历史增量复验（2026-11-04，保留原始来源与范围）

**Status:** passed。**Score:** 80/80原合同truth保留并复验受影响项，behavior_unverified=0，overrides_applied=0。原6条roadmap SC与74条PLAN truth未缩减；INT-01是R3/P08/P16既有硬排除合同的缺口，本轮关闭，不通过另加重复truth稀释分母。TLS为新增CI enforcement核验，不改变80条分母。原21项需求另加入L2-03的P07 transport集成义务，共22项。

**源码:** `f8851ca073fed34e80e605f0ac91880259df7e12`；原动态/终审基线 `adc0227`。本轮独立读取实际diff、调用链与测试断言，读取原始CI/运行日志，实跑TLS hermetic6项及只读archive/hash/tree重算。未执行Cargo、Docker或服务，未重跑全部旧80条测试、未重新审计全部48项威胁。SUMMARY只用于定位及dated归属，不作为行为证据。

## User Flow Coverage（当前增量）

| 用户步骤 | 预期 | 当前证据 | 状态 |
|---|---|---|---|
| 安装并启动演示 | 单二进制在独立目录可查询、Action、历史、restart/reset与stdio | release41759804768三native实际archive安装smoke；source与packaged完整tree一致 | VERIFIED |
| 应用与agent按同一身份读取/修改 | REST/SDK/MCP权限一致，写入可审计 | 最终CI四必需job；fresh43项（PG/Trino17、Rust9、TS17）；旧逐truth行为证据保留 | VERIFIED |
| 使用六个L2工具 | 只访问允许范围，自定义edit catalog误授仍排除 | 新5个真实HTTP测试在两OS最终CI均ok；原RED2→GREEN2；最终真实Trino/ACL服务矩阵 | VERIFIED |
| 获得用户故事outcome | 应用与agent按相同授权使用并验证平台能力 | 上述完整流程及14/14 automated UAT；发布仅按既定release.published接线，本轮publish skipped | VERIFIED |

### 增量 Observable Truths 与接线

| 合同/检查 | 状态 | 源码、数据流和行为证据 |
|---|---|---|
| INT-01 / R3 / P08 / P16：自定义edit catalog硬排除 | VERIFIED | `trino/mod.rs:144` accessor取不可变ACL配置；`mcp.rs:157`在Enabled后注入；`mcp_l2.rs:94`构造同一L2Policy，SQL validate与metadata最终target检查共用。五个新增HTTP测试断言九次敏感拒绝在任何上游前发生、source不fallback、audit为空；default/custom正向六工具精确返回值及6条成功audit。 |
| catalog可信来源不能被请求或角色替换 | VERIFIED | config中store/edit一致性、合法单段名；未配置store时attestation的`acl_table_probe`拒绝非法名，不能取得Enabled。allow-list误授不能覆盖deployment exclusion，qualified/default最后解析目标仍检查。 |
| TLS依赖门失败关闭 | VERIFIED | `ci/check-tls-deps.sh`单次完整图、精确首字段、Cargo异常中止；workflow双OS调用真实helper及hermetic测试。verifier亲跑6/6 exit0，最终双OS两个步骤success。 |
| 最终证据与源码一致 | VERIFIED | CI41759804733 headSha=f885；868643c→f885唯一cli_color测试类型别名，动态服务输入相同。release source f885与packaged22913c8完整tree均0ecc59382aaf12e782c508ba64858e2689299fd7，archive/binary/manifest只读重算通过。 |

应用数据流没有新静态业务返回：ACL config→可信edit catalog→L2Policy→validated SQL/final metadata target→真实Trino engine→过滤结果→MCP响应；真实source输出与read audit链保留。测试fixture合成响应专门隔离应用fence，不能解释为真实环境泄漏。

### 增量 Behavioral Evidence

| 检查 | 原始证据 / 实际结果 | 证据边界 |
|---|---|---|
| INT-01 named HTTP | `target/l2-catalog-red.log` 0/2；同名GREEN2/2；server相关83/83，Clippy完成；最终Windows日志3245–3257、Ubuntu3265–3277新5项全部ok | verifier亲读原始日志与断言；没有另起Cargo |
| TLS hermetic | `python -B -m unittest discover -s ci -p test_tls_deps.py -v`，6 tests in 0.399s，OK，exit0 | verifier实际执行；Cargo为边界fixture；最终两OS另有真实helper成功 |
| 完整CI | run41759804733 completed/success，四必需job成功；Windows1496/0/117、Ubuntu1503/0/117，各150组 | verifier按workspace步骤时间窗口独立重计；平台数不相加为唯一测试总数 |
| 本地动态 | SQLite5 + PG1 + fresh43 + Iceberg2 = 51 passed；verify commit.txt=f885 | 原runner仍exit1（HEAD changed during validation）；独立final-check provenance/scan/cleanup exit0。服务输入等价不改写原退出码 |
| 三native发行 | run41759804768三native/aggregate success，publish skipped；三个archive/binary/hash/manifest与完整tree只读复核exit0 | 本机不运行包内程序；安装流程由该run三平台实际smoke证明 |

### Requirements 与测试质量

API-02由07-05、MCP-02/03/04/05由07-07补dated累计完成metadata；L2-03由真实六工具adapter的07-08补归属，明确Phase06基础与本次54a47dd后继贡献。原07-08 PLAN未列L2-03，不倒写原计划。其余原21项逐项映射保留于下方历史表，最终回归没有观察到退化。当前22项含P07 transport L2-03，中央勾选由主会话结合P06报告决定。

新增HTTP5与TLS6测试均活动，无ignore或循环oracle；断言包含数据值、全部上游计数、audit/fallback，不只检查status。TLS测试直接执行workflow实际command。债务扫描无未引用TBD/FIXME/XXX。schema-only grant与每个metadata direct大写请求未逐一枚举，代码共享policy与eq_ignore_ascii_case已核查，无可复现新问题。旧48项安全记录继承；INT-01/TLS局部独立2/2 CLOSED与本verifier增量一致，07-SECURITY为SECURED、threats_open=0。

### Decision Coverage

本轮实际 `query check.decision-coverage-verify` 为3/3 honored，not_honored=[]。

### Advisory (New Scope, Unevidenced)

无新增未有证据的阻断性判断。未把边界偏好或未枚举组合改写为旧must-have退化。

### Human Verification / Deferred / Gaps

无本Phase待人工行为项；14项UAT均为用户授权的automated证据，不冒称用户见证。旧禁止项的用户决议与测试enforcement边界继续保留；无新增豁免或静默通过。没有把INT-01延期到后继phase。**STORE-05 source缺口及ENGINE-05历史MinIO差异仍属milestone未关闭范围；本报告不宣布milestone全绿，也不授权发布。**

详细证据与限制：[PHASE-07-REVERIFY.md](../../milestones/v1.0-audit/PHASE-07-REVERIFY.md)、[GAP-SECURITY-REVIEW.md](../../milestones/v1.0-audit/GAP-SECURITY-REVIEW.md)、[DYNAMIC-VALIDATION.md](../../milestones/v1.0-audit/DYNAMIC-VALIDATION.md)、[RELEASE-ARTIFACT-AUDIT.md](../../milestones/v1.0-audit/RELEASE-ARTIFACT-AUDIT.md)。

## 历史完整验证（adc0227，2026-11-03T16:34:44Z）

以下保留原80条truth、artifact、key-link、测试与需求逐项证据，其日期/source/run/旧计数均为当时事实。早先milestone审计发现自定义catalog缺口时曾暂停为gaps_found；当前状态由上方增量及frontmatter决定，不将历史结论冒充全部新执行。

**Phase Goal:** As a 应用开发者与 agent 使用者, I want to 经 REST（OpenAPI 3.1、Rust / TS client）和 MCP（按主体过滤的 toolset）使用全部能力，以 `ontoforge serve --dev` 单二进制在本机跑通端到端演示，并通过 release workflow 发布 Linux / macOS / Windows 二进制, so that 应用与 agent 能按相同授权使用并验证完整平台能力.
**Verified:** 2026-11-03T16:34:44Z（本地2026-11-04）
**Status:** passed
**Re-verification:** No，首次正式验证
**结论:** 六条roadmap成功标准、74条PLAN细化truth与21项需求义务均有源码及行为证据。完整CI、fresh服务/SDK、三平台实际artifact和13项自动UAT全部通过。


## 核验边界与证据身份

这是首次正式 goal-backward verification；开始时没有既有 `07-VERIFICATION.md`，无 overrides。核验从 ROADMAP 六条成功标准出发，保留 16 份 PLAN 的 74 条细化 truth，共 80 条；PLAN 不缩减 roadmap 合同。MVP Goal 已通过 `query user-story.validate --pick valid`，下表先核对用户流程。SUMMARY 仅用于定位交付范围，结论来自实际源码、调用链、测试断言、原始运行日志和下载产物。

源码冻结于 `adc02278b5cc55111def9a30a1c3b778485e8279`。CI/native 的实际 PR merge checkout 为 `b75d2949f9db1276ce07f4864a6206f7932bb11b`；verifier 亲自运行 `git rev-parse <commit>^{tree}`，两者均为 `711d68e9956316d34f27aca95be87f13ab953007`。源 head 与 archive manifest commit 分列，未把它们伪称为同一 commit。实现 fingerprint 由 GSD 生成，含本阶段 PLAN/SUMMARY；本报告本身不计入。

Cargo、服务及完整 matrix 由主会话统一持有执行槽；本 verifier 没有另起 Cargo 或服务，独立读取新运行输出并核对其测试源码断言。自身执行的无状态 spot-check 为 OpenAPI 八路径/3.1.0，以及 u64 max/decimal lossless codec 与 unsafe number 拒绝。未修改实现、测试、STATE、ROADMAP、REQUIREMENTS 或其他报告，未 commit。

## User Flow Coverage

| Step | Expected | Evidence | Status |
|---|---|---|---|
| 安装并启动 | 下载对应平台 archive，解包后以单二进制运行 `serve --dev`，无需 checkout/Trino/Docker | 三 native runner 用实际 archive 执行 `ci/release-smoke.ps1`；Windows/Linux/macOS 全链 smoke；dev assets 使用 include_str | VERIFIED |
| 应用读取 ontology 与对象 | Rust/TS client 可取得当前主体可见的 metadata、对象、查询及 aggregate | 八 REST 路由、共享 DTO、Rust 9 与 TS 17 的真实 HTTP contract，含双有效主体与撤权 | VERIFIED |
| agent 连接并发现工具 | stdio 连接既有 serve，或直接连接 /mcp；只见其 toolset/Action | 真正 stdio 子进程/HTTP session、ToolCatalog 当前 scope、真实坏 bearer/跨主体拒绝 | VERIFIED |
| 预演并提交修改 | validateOnly 不写业务状态；Action 成功后可见新值与同一 receipt | SQLite/PG stale/CAS/guard、跨 transport 同 command race、stdio Alice 成功链与 Mia 八次拒绝 | VERIFIED |
| 查看历史和重启 | 查询到同一 edit event/time，正常重启保留历史/receipt，只有显式 reset 重建 | REST/MCP history 共用 snapshot-aware helper；native smoke 重启保留、reset 旧 token401/command404 | VERIFIED |
| 在授权内使用 L2 | 六个批准工具、四 EXPLAIN 格式与 allow-list；proof 不可用或过期时拒绝并显示脱敏 health | 真实 Trino ACL/health/61s expiry/recovery，E6 上游总请求零增量，T08 四格式数量上限 | VERIFIED |
| 获得最终 outcome | 应用与 agent 按相同授权使用并验证完整平台能力 | 上述实际流程、完整 matrix、三 native artifacts 与同一实现树；上传仅接线至未来授权 release.published | VERIFIED |

## Goal Achievement

### Observable Truths

| # | Roadmap truth（合同原文） | Status | Evidence |
|---|---|---|---|
| R1 | REST 覆盖元数据、对象读取、查询、Action 提交与历史；错误统一为结构化错误码；OpenAPI 3.1 纳入快照测试；Rust 与 TS client 覆盖每个端点组 | VERIFIED | `server/src/lib.rs:216`八路由；rest/context/error→共享服务；OpenAPI snapshot、Rust9/TS17真实HTTP与错误合同。 |
| R2 | agent 经 stdio 或 `/mcp` 连接；`list_tools` 只返回主体被授予的 toolset（core / full / sql:raw），L3 与任何变更工具不出现 | VERIFIED | `ontoforge/src/mcp.rs:42`认证桥；`mcp_catalog.rs:39,66`白名单；`mcp.rs:514,534`每调用重取权限与同catalog拒绝隐藏name；CLI/transport真实测试。 |
| R3 | 每个 action type 自动暴露为 MCP 工具（含 validate-only）；6 个 L2 工具与 mcp-trino 名称参数一致，结果按 allow-list 过滤 | VERIFIED | `mcp_actions.rs`按ParameterDef生成schema，六工具静态schema与typed resolver；Actions10、真实Trino六工具/四格式与L2defaults10。 |
| R4 | 同一身份经 MCP 与 REST 得到相同授权结果，写入同样记入 edit log | VERIFIED | 两入口共用context→snapshot→ActionService/history；SQLite及PG真实REST/MCP race核对五表各一次及完整相同receipt；stdio双主体逐次完整持久行比较。 |
| R5 | `ontoforge serve --dev`（SQLite 替身 + dev token）在 Windows 本机零外部依赖跑通示例 ontology 的查询 → Action → 历史演示；release workflow 发布 Linux / macOS / Windows 二进制 | VERIFIED | dev持久化与嵌入assets；release41669982362三native实际archive完整安装smoke、hash/manifest一致；按已批准07-14合同只接线release.published上传，本轮publish skipped。 |
| R6 | 服务启动并维持 Phase 6 的 L2 ACL attestation；无法证明或证明过期时 L2 禁用，`/health` 告警并暴露脱敏原因，覆盖失败和恢复的端到端测试（ADR-0028） | VERIFIED | `serve.rs:177,209`持有monitor；发送侧fresh gate与health同proof；真实health expiry/recovery、宽松ACL对照、shutdown/dependency/audit与source drift。 |

### PLAN 细化 truth

以下每行保留 PLAN 原文；代码引用省略的前缀按该 PLAN 的 artifact 表解释。行为项以已实际执行的断言为依据，未用 symbol presence 或单独测试名称取得 VERIFIED。

| # | Truth | Status | 源码及行为证据 |
|---|---|---|---|
| P01.1 | 真实认证 HTTP 请求通过 request_snapshot、QueryService 读取对象，正常关闭后同 request_id 的 read_audit 已持久化。 | VERIFIED | `context.rs:451` 认证后 snapshot；`rest.rs:177` query；tracer test 核对 shutdown 后持久 audit/request_id。 |
| P01.2 | 缺少、失效或过期 bearer 的读取被拒绝；客户端传入的 principal、roles、scope、limits 不成为可信上下文。 | VERIFIED | `context.rs:449` 单 bearer 与 Authenticator、`:487` forged headers 拒绝；tracer/transport 真实坏及过期 token。 |
| P01.3 | 请求固定开始时的 definition、roles、bindings，短事务结束后才 compile_scope 或执行 source 网络。 | VERIFIED | `request_snapshot.rs:24-44` 锁内读定义/scope并commit后返回；snapshot共同锁和source barrier测试。 |
| P01.4 | serve 使用同一个真实宿主，监听失败与 audit drain 失败返回非零。 | VERIFIED | `serve.rs:144,222` listener及同一host；serve/cli_shutdown断言错误exit和audit故障。 |
| P02.1 | SQLite/PG 的新请求看到已提交的 reload 或 scope apply，进行中的请求继续使用开始时的固定版本，definition/roles/bindings 不混代。 | VERIFIED | `request_snapshot.rs:24` 共同事务；SQLite/PG metadata_contract及PG4场景验证新旧版本、角色/bindings。 |
| P02.2 | Action submit、validate-only、replay/result lookup 和 history 均有 snapshot-aware 入口，旧 wrapper 继续可用。 | VERIFIED | `action/src/lib.rs:199,242`、`history.rs:120`；wrapper转发到snapshot入口，frozen tests实跑。 |
| P02.3 | validate-only 保留 fence、CAS、guard；旧定义写返回 DefinitionStale 并且零业务写，服务不自动重试。 | VERIFIED | `commit.rs:617-678` fence/CAS/guard在validate rollback前；frozen_definition及HTTP/PG stale完整持久行零变。 |
| P02.4 | replay/result 使用本次授权，保留 winner receipt/time；不绑定原定义，不增加当前对象可见性重验或历史 schema。 | VERIFIED | `action/src/lib.rs:199,308` 当前scope控制结果与replay；frozen_replay/clock测试逐字段比winner receipt/time。 |
| P03.1 | REST 提供 metadata、pk、filter/search/searchAround、集合运算、aggregate；逻辑 DTO 不让 client 引入 query/engine/storage。 | VERIFIED | `rest.rs:22,120,201,299`八路径路由到QueryService；Rust/TS只依赖wire DTO；真实L0端点合同。 |
| P03.2 | API-01 adjacency/ordering/empty：相等排序值由现有 pk/keyset 稳定破同，空/单项/nullable 和重复对象按原 Query 语义，has_more 决定结束。 | VERIFIED | rest_equal_sort_keys_empty_nullable_and_pagination核对稳定PK/空/nullable和hasMore；QueryService输出直接映DTO。 |
| P03.3 | API-01 concurrency：在途请求固定快照，新请求反映已提交撤权；慢查询不持 metadata 锁。 | VERIFIED | `context.rs:451`每请求快照；rest_reload_and_revoke_freeze_inflight_only的source barrier+第二store证明锁释放。 |
| P03.4 | API-03 empty/encoding/concurrency：空错误描述也有稳定 code/apiName/request_id，UTF-8 正确保留且按字节限额，并发请求 ID 隔离。 | VERIFIED | `error.rs:14`固定code/apiName/request_id；http_contract验证Unicode字节限制与16并发ID。 |
| P03.5 | 非法 JSON、未知字段、413、404、超时、取消、上游/audit 故障均返回同 envelope，L0 不含物理名；结果在受控读取时执行 rows/bytes 限制。 | VERIFIED | `error.rs`框架fallback与安全domain映射；http_contract、tracer、MCP budget/audit故障实测。 |
| P04.1 | REST 调用单项/批量 Action、validate-only、按 command_id 取结果和对象历史，经现有 ActionService/history snapshot-aware 入口。 | VERIFIED | `rest.rs:400,510,548`分别调用execute_with_snapshot/result/history_page；http_actions完整receipt/history值断言。 |
| P04.2 | TTL 内同 command_id 相同参数返回原 receipt/time，异参/跨主体/撤权拒绝；过期 GET 结果不可见，POST 沿既有 TTL 可重新执行；丢响应后可显式 replay 或查询结果。 | VERIFIED | `action/src/lib.rs:199,308`与commit幂等存储；HTTP/SDK/stdio故障proxy核对单发送、显式replay原receipt。 |
| P04.3 | validate-only、DefinitionStale、VersionConflict 的失败响应零业务写，服务不自动切换定义重试。 | VERIFIED | `commit.rs:625,672,778`；frozen_definition+http_action_safety+PG stale分别核对零业务行变化。 |
| P04.4 | 历史响应保留当前 scope 遮蔽、keyset 和同记录完整页，超限显式 truncated/继续方式。 | VERIFIED | `history_page.rs:8`按seq完整组截断并二次探测older；history_pagination及MCP/REST逐页对等。 |
| P05.1 | OpenAPI 文档版本严格为3.1.0，覆盖所有REST端点、request/response/errors和安全要求，snapshot漂移会失败。 | VERIFIED | `openapi.rs`document/render、docs/openapi.json八路径；openapi_snapshot严格3.1/security/schema断言和字节snapshot。 |
| P05.2 | API-02 empty：空metadata/数组、单项、required/nullable、任意JSON与未知字段的schema和serde正反例一致。 | VERIFIED | openapi_snapshot serde正反例核对required/nullable/unknown/任意JSON；dto.rs serde派生。 |
| P05.3 | API-02 encoding：tagged union真实字段名、递归ObjectSet/WhereClause、additionalProperties、Unicode描述、decimal字符串与Int number一致。 | VERIFIED | 递归ObjectSet/WhereClause schema与integer/decimal wire；openapi_snapshot及numeric SDK合同。 |
| P05.4 | API-02 concurrency：OpenAPI导出由静态平台DTO生成，多个并发导出字节稳定，不依赖可变租户/定义或开启服务。 | VERIFIED | `serve.rs:55`导出在config/store/listener前返回；openapi_并发导出字节稳定和坏目标非零。 |
| P05.5 | 完整轻量 DTO 与 OpenAPI 生成供 Rust/TS clients 共用，导出文件可在全新检出再生成而无漂移。 | VERIFIED | `tools/openapi-gen/generate.mjs`消费docs/openapi.json；check:generated、strict typecheck与客户端真实HTTP合同。 |
| P06.1 | 按D-03，已配置L2故障且core健康时/health为HTTP200 degraded并给脱敏原因；core故障503，未配置L2不算故障，L2始终fail-closed。 | VERIFIED | `health.rs:17-74`以core/drift/L2状态映200 degraded或503；真实health61s expiry/recovery测试。 |
| P06.2 | L2-04 idempotency：重复启动/关闭不留monitor lease或活跃probe，失败/过期/恢复转换不产生重复审计，monitor不写storage read_audit。 | VERIFIED | `state.rs`单monitor所有权、`lifecycle.rs:84`take；health_lifecycle与L2 monitor测试核对lease/probe清理。 |
| P06.3 | L2-04 concurrency：证明过期与发送侧同时拒绝L2；在途probe/recovery/停机交错仍关闭准入且清理失败可见。 | VERIFIED | Trino发送侧fresh gate与health读取同一proof；expiry/recovery、取消及shutdown测试核对拒绝与清理。 |
| P06.4 | 宿主启动并持有drift检查与L2AclMonitor；端口或初始化失败显式清理，网络不占metadata锁。 | VERIFIED | `serve.rs:177,209`安装monitor，`lib.rs:166`start_drift；source_drift/health启动失败及metadata barrier。 |
| P06.5 | 关闭先停止接入，等待已接受Action及其依赖，才取消独立读/SSE，排空audit，再shutdown monitor/DB；超时或清理失败非零。 | VERIFIED | `lib.rs:269`等Action后cancel_reads；`lifecycle.rs:50-133`有界drain/monitor清理；transport/real_shutdown持久提交一次。 |
| P07.1 | agent可经/mcp使用describe、pk、object-set/filter/search/searchAround、aggregate、history，toolset按主体core/full过滤；REST与MCP同权限/edit log。 | VERIFIED | `mcp.rs:96,514,534`重取context，`call_read`共用QueryService/history；真实MCP contract+stdio/REST对照。 |
| P07.2 | MCP-02/03 adjacency/empty/ordering：先授权再分页，空/单项结果与同apiName/排序值稳定；跨页没有重复/遗漏，has_more决定结束。 | VERIFIED | `mcp_catalog.rs:39,95`先授权排序再分页；catalog keyset、L0排序和history完整页合同。 |
| P07.3 | MCP-02/03 concurrency：reload/revoke后新工具请求采用新snapshot，在途请求固定旧definition；权限不按session缓存。 | VERIFIED | mcp_inflight_snapshot_and_revoke_on_next_call用真实session/source barrier，后续拒绝零source，旧audit def_version不变。 |
| P07.4 | 每个Action按真实参数定义生成工具schema，list/call共用名称/授权检查，直接猜隐藏名被拒绝。 | VERIFIED | `mcp_catalog.rs:66`作用于scope.actions；`mcp_actions.rs:20`真实ParameterDef/ontology生成；猜隐藏name被catalog拒绝。 |
| P07.5 | MCP-04 empty/encoding/concurrency：无参/required/nullable/Unicode描述的schema与真实validator一致；definition切换不混用旧schema和新执行。 | VERIFIED | ActionToolSchema及真实validator同源；mcp_actions覆盖numeric refs/default/nullable/Unicode及旧schema fence。 |
| P07.6 | MCP-05/06 idempotency/concurrency：dryrun重复调用零业务写且走fence/CAS/guard；提交重放同command_id保留receipt，跨transport并发恰好一份提交。 | VERIFIED | MCP dryrun与Action入口同execute_with_snapshot；SQLite及PG REST/MCP race完整receipt相同、五表各一次。 |
| P07.7 | MCP-08 concurrency：无论reload/toolset/session交错，L3和定义/授权变更工具从不列出或接受调用。 | VERIFIED | catalog仅白名单读/L2/action，不注册L3或管理；call_tool先catalog拒绝，跨reload/revoke真实测试。 |
| P08.1 | sql:raw主体可调用六个原名L2工具，参数与mcp-trino一致，所有发现结果按allow-list过滤，执行/解释走既有gate/limits/audit。 | VERIFIED | `mcp_catalog.rs:16-21,108`六工具与参数；McpL2Tools直接复用typed printer、Engine gate与ReadAudit，真实Trino六工具验证。 |
| P08.2 | MCP-07 adjacency/empty/ordering：授权过滤后的空/单项catalog/schema/table正确，稳定keyset页无重复遗漏，相等边界不跨allow-list。 | VERIFIED | `mcp_l2.rs:386`过滤后sort/dedup，catalog keyset先scope过滤；L2发现空/单项及allowlist边界测试。 |
| P08.3 | MCP-07 concurrency：撤销sql:raw或L2证明失败/过期立即阻断下一请求，不能使用旧session授权。 | VERIFIED | `mcp.rs:109,149`每调用scope/fresh L2；同session撤权及61s proof expiry断言所有上游请求零增量，恢复再放行。 |
| P08.4 | MCP HTTP每POST/GET/DELETE都认证；旧协议session绑定初始主体且有容量/TTL/删除回收，新2026-07-28协议无session并校验metadata headers。 | VERIFIED | `context.rs:489`所有HTTP方法auth+owner；真实session manager容量/TTL/DELETE与modern metadata测试。 |
| P08.5 | MCP-01 concurrency：取消、expiry、跨主体、disconnect、SSE和重连不改变身份或自动重发写入；管理工具不可达。 | VERIFIED | T24真实inner.sessions与T08实际permit；transport/CLI ID取消、过期token、跨host、SSE、断连Action单提交合同。 |
| P09.1 | 按D-01，stdio只连接已运行serve的/mcp，共用store、snapshot、权限和审计；不创建第二runtime/DB。 | VERIFIED | `ontoforge/src/mcp.rs:42-51`只建HTTP/stdio桥；不打开EditStore/ServerHost；双主体真实stdio与REST同receipt/audit。 |
| P09.2 | bridge关闭redirect和reinit_on_expired_session，任何HTTP/session故障不自动重发Action；token不进入argv/stdout/错误正文。 | VERIFIED | `mcp.rs:44,91,210`redirect none/reinit false/redact；302 trap0、bad token、closed session与丢响应proxy行为。 |
| P09.3 | MCP-01 concurrency：有界并发转发保留JSON-RPC id，取消控制留容量，慢请求不堵取消，EOF/断连有界退出。 | VERIFIED | Bridge bounded config与forward可取消request；真实两个ID并发、定向cancel/EOF退出，独立请求不受影响。 |
| P09.4 | REST、MCP HTTP和真实stdio子进程同身份读写授权及edit log一致，未启动服务/坏token/跨origin均明确失败。 | VERIFIED | cli_stdio成功链+Mia八次跨transport拒绝，完整持久行不变和Alice原receipt；bad token/absent host/redirect实跑。 |
| P10.1 | ontoforge-client手写轻量client并复用ontoforge-api DTO，生产依赖不带query/storage/engine。 | VERIFIED | `ontoforge-client/Cargo.toml`生产仅api/reqwest/serde/error；lib.rs方法复用dto，无后端生产依赖。 |
| P10.2 | Rust client覆盖metadata/pk/query/aggregate/Action/validate/history/result/health；每端点组有真实HTTP contract。 | VERIFIED | client lib真实request方法覆盖八paths；contract9逐值验证metadata/PK/query/aggregate/Action/history/result/health。 |
| P10.3 | API-04 idempotency：同command_id显式重试返回原receipt，client不自动重试写入；异参/过期/丢响应语义明确。 | VERIFIED | contract transport fault proxy先发送/提交后断连，计数1；显式replay与完整持久行、原receipt/time一致。 |
| P10.4 | API-04 concurrency：每实例bearer隔离、并发请求request_id/取消独立，跨principal、撤权、超时和结构化错误不被吞掉。 | VERIFIED | 两个有效主体scope隔离+真实scope_apply；两个在途barrier取消一个，独立请求成功，后续撤权拒绝。 |
| P11.1 | TS平台client来自OpenAPI3.1，openapi-typescript+openapi-fetch提供可调用wrapper，npm lock/strict/noUncheckedIndexedAccess和生成漂移门齐备。 | VERIFIED | package.json generator/openapi-fetch/lock；strict和noUncheckedIndexedAccess；实际check:generated/typecheck/build通过。 |
| P11.2 | API-05 boundary/precision：±(2^53−1)、2^53±1、i64 min/max、u64 seq/version上界、decimal长小数原样往返；wire仍用既有整数JSON token与decimal字符串。 | VERIFIED | codec.ts原始整数token↔bigint、decimal字符串；独立Node u64 smoke及TS boundary/public query合同。 |
| P11.3 | API-05 idempotency：相同command_id显式重放原receipt/time，写client不自动重试或换session重发。 | VERIFIED | TS proxy单发送/持久行差值+完整行、显式command replay原receipt/time；index.ts无retry循环。 |
| P11.4 | API-05 concurrency：每实例bearer、AbortSignal与并发响应隔离；缺lossless运行时能力明确失败，不能静默舍入。 | VERIFIED | index.ts每实例bearer/每调用signal；TS双主体/revoke/在途abort+runtime capability拒绝测试。 |
| P11.5 | 每REST端点组与auth/scope/revoke/stale/invalid/schema/error/limits均有真实HTTP contract；只有生成types不算完成。 | VERIFIED | TS真实fresh serve覆盖每端点组及错误diagnostics；17合同测试，非仅generated types。 |
| P12.1 | 按D-02，serve --dev使用持久edit store，重启保留Action修改/history/receipt；仅空状态seed，显式reset才重建。 | VERIFIED | `dev.rs:122`仅空active seed，`:83`锁定数据目录；真实dev重启原receipt/history与native smoke核对。 |
| P12.2 | DX-01/02 idempotency：重复启动/seed不覆盖数据；相同command显式重放不重复修改，重复reset行为明确且仅在dev目录执行。 | VERIFIED | dev_lifecycle显式重复seed/reset；native smoke重启保留、reset旧token401/旧command404。 |
| P12.3 | DX-01/02 concurrency：活跃实例持独占dev锁，第二实例与并发reset拒绝；异常退出后可恢复且不会删任意路径。 | VERIFIED | `dev.rs:45,83,322`目录/marker+OS try_lock；并发reset和危险路径/异常退出恢复测试。 |
| P12.4 | 单二进制嵌入示例ontology/source fixtures/scope，脱离checkout在Windows无Docker/Trino可查询→Action→history。 | VERIFIED | dev_assets.rs include_str嵌入aviation；三native实际archive无checkoutcwd全链smoke，Windows不依赖Trino/Docker。 |
| P12.5 | dev token仍经过真实认证/授权，密钥/持久目录有稳定默认与显式覆盖，stdout不输出密钥。 | VERIFIED | `serve.rs:113-128`dev签名Verifier；dev目录/key权限测试；stdio/native日志无token。 |
| P13.1 | 真实PG双store+真实HTTP/MCP请求证明definition/roles/bindings原子快照、外部reload/revoke、Action/history/result合同，非SQLite替身冒充。 | VERIFIED | postgres_http_it真实PG4+request_snapshot PG1：双store共同锁、外部reload/revoke、两新增完整持久行/receipt竞争断言。 |
| P13.2 | 真实Trino证明L2启动/失败/过期/恢复与D-03 health一致，六工具包括四EXPLAIN格式与allow-list；core故障503而单L2故障200 degraded。 | VERIFIED | trino_http_it六工具四format；health_trino_it真实61s proof lifecycle、ACL宽松对照拒绝与core503。 |
| P13.3 | SCOPE-07 idempotency/concurrency：重复管理操作与并发reload/revoke/请求/重放不产生混代或绕过当前授权，MCP无管理写入口。 | VERIFIED | 共享request_snapshot与scope当前权限；PG mixed snapshot和跨transport race，catalog无管理入口。 |
| P13.4 | 长source查询时metadata共同锁已释放，停止服务等待Action依赖，取消独立读并持久审计，真实服务失败退出可见。 | VERIFIED | trino长读barrier后第二store可reload；real_shutdown等待Action依赖、持久edit一次、audit已停止；cleanup明确exit。 |
| P13.5 | Windows/Linux快速suite、PG/Trino ignored suite、Rust/TS完整contract在CI明确执行非零指定用例，环境缺失不能skip为绿。 | VERIFIED | ci.yml双OSworkspace、integration与phase7-real-services；server-services.sh精确发现ignored名字并运行，缺环境fail closed。 |
| P14.1 | Linux x86_64、macOS arm64、Windows x86_64分别在native runner构建release单二进制，实际tar.gz/zip和SHA256存在且校验通过。 | VERIFIED | release41669982362三native实际archive已下载，manifest0.1.0及SHA256重算一致；同source tree绑定。 |
| P14.2 | 从每个平台实际打包artifact解包后在无checkout目录执行--version、serve --dev查询→Action→history→重启，不能只验证workflow文本。 | VERIFIED | release-smoke.ps1实际解包binary在隔离cwd查询/Action/history/restart/reset/stdio；三个native job该step成功。 |
| P14.3 | DX-04 concurrency：并发build使用唯一平台artifact名和隔离目录，聚合要求三平台齐全且校验匹配；失败/取消不发布。 | VERIFIED | release.yml平台/run/attempt唯一artifact、隔离目录；aggregate要求3targets同commit/hash，publish依赖native+aggregate成功。 |
| P14.4 | 只有明确授权的release published事件触发上传GitHub Release assets；PR/workflow_dispatch仅构建验收，不发布npm、不创建Release、不合并PR#7。 | VERIFIED | release.yml:93仅release.published且依赖success；本轮event pull_request/publish skipped；PR7/8均Draft OPEN，用户既定禁止。 |
| P14.5 | 全套正式验证与code-review/gstack review/security/简化复核完成后才能生成passed VERIFICATION；PR#7仍Draft OPEN。 | VERIFIED | 最终workspace/真实服务/SDK/native门及Spec/Staff/security/尾部简化报告；无开放high；PR7仍Draft OPEN。 |
| P15.1 | explain_query的format完整支持LOGICAL/DISTRIBUTED/VALIDATE/IO，省略时保留现有DISTRIBUTED；旧explain_query(sql) wrapper行为不变。 | VERIFIED | `printer/l2.rs`ExplainType四枚举；`query/l2.rs`旧wrapper默认DISTRIBUTED；engine与query测试断言各TYPE SQL。 |
| P15.2 | 四格式在同一validate_l2→allow-list→fresh ACL→limits→audit链执行，任何format都不接受ANALYZE/多语句/写SQL。 | VERIFIED | query typed调用与McpL2Tools纯预检→availability→audit→fresh Engine；四格式timeout/取消单audit、非法SQL零发送。 |
| P15.3 | get_table_schema的s.t/c.s.t在结构化identifier resolver中解析，qualified table覆盖可选catalog/schema；歧义/非法输入明确拒绝，不插入未经验证SQL。 | VERIFIED | `printer/l2.rs:155`AST identifier resolver，结构化TableRef；qualified/default precedence及非法/注入用例。 |
| P15.4 | 本计划仅补ADR-0019/ref-mcp-trino已批准的参数能力，MCP transport在07-08消费typed入口，真实Trino四格式在07-13验收。 | VERIFIED | mcp_l2已消费typed printer/identifier resolver，真实Trino四格式合同；只保留批准的六工具。 |
| P16.1 | MCP L2 optional catalog/schema由--mcp-catalog/--mcp-schema可信部署默认值补齐；s.t/c.s.t覆盖默认，缺必要上下文明确报错，四EXPLAIN格式完整可用。 | VERIFIED | cli Serve flag→deployment_mcp_defaults→state.store；real listener验证省略参数/qualified覆盖及四format。 |
| P16.2 | qualified table对应段优先于显式同名参数，显式参数优先于部署默认；list_catalogs不要求默认，任何默认都不能从principal的首个allow-list项猜出。 | VERIFIED | mcp_l2.rs resolve_table显式参数合并trusted defaults再AST解析；qualified优先，list_catalogs独立；two-principal隔离。 |
| P16.3 | 缺上下文、非法identifier、越allow-list均在发送前明确拒绝，两个principal不借用上下文，所有发现结果继续按当前scope过滤。 | VERIFIED | `mcp_l2.rs:93,165,226`纯本地policy/SQL/default/allowlist先probe；22次拒绝status/execute/describe/audit全0及真实session零发送。 |

**Score:** 80/80 truths verified；behavior_unverified=0；overrides_applied=0。18 个本阶段 requirement 加 3 个继承服务义务均覆盖。

### Required Artifacts

`query verify.artifacts` 对全部 16 PLAN 返回 59/59。此计数只证明存在/基本内容；verifier 另读实际函数、消费者与输出源完成后三级检查，下表按 PLAN 合并重复声明，不将文件存在等同功能完成。

| PLAN | Artifact declarations | Status | 核验 |
|---|---|---|---|
| 07-01 | `crates/ontoforge-storage/src/lib.rs`<br>`crates/ontoforge-server/src/lib.rs`<br>`crates/ontoforge/src/serve.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P01 及下表接线。 |
| 07-02 | `crates/ontoforge-storage/src/request_snapshot.rs`<br>`crates/ontoforge-action/src/lib.rs`<br>`crates/ontoforge-storage/src/history.rs`<br>`crates/ontoforge-action/tests/frozen_definition.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P02 及下表接线。 |
| 07-03 | `crates/ontoforge-api/src/dto.rs`<br>`crates/ontoforge-server/src/rest.rs`<br>`crates/ontoforge-server/src/context.rs`<br>`crates/ontoforge-server/tests/http_contract.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P03 及下表接线。 |
| 07-04 | `crates/ontoforge-api/src/dto.rs`<br>`crates/ontoforge-server/src/rest.rs`<br>`crates/ontoforge-server/tests/http_actions.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P04 及下表接线。 |
| 07-05 | `crates/ontoforge-api/src/openapi.rs`<br>`crates/ontoforge-api/tests/openapi_snapshot.rs`<br>`docs/openapi.json` | VERIFIED | 存在、非stub；实际调用与行为证据见 P05 及下表接线。 |
| 07-06 | `crates/ontoforge-server/src/state.rs`<br>`crates/ontoforge-server/src/health.rs`<br>`crates/ontoforge-server/src/lifecycle.rs`<br>`crates/ontoforge/tests/cli_shutdown.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P06 及下表接线。 |
| 07-07 | `crates/ontoforge-server/src/mcp.rs`<br>`crates/ontoforge-server/src/mcp_catalog.rs`<br>`crates/ontoforge-server/src/mcp_actions.rs`<br>`crates/ontoforge-server/tests/mcp_contract.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P07 及下表接线。 |
| 07-08 | `crates/ontoforge-server/src/mcp_l2.rs`<br>`crates/ontoforge-server/src/mcp_sessions.rs`<br>`crates/ontoforge-server/tests/mcp_transport.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P08 及下表接线。 |
| 07-09 | `crates/ontoforge/src/cli.rs`<br>`crates/ontoforge/src/mcp.rs`<br>`crates/ontoforge/tests/cli_stdio.rs`<br>`docs/api.md` | VERIFIED | 存在、非stub；实际调用与行为证据见 P09 及下表接线。 |
| 07-10 | `crates/ontoforge-client/src/lib.rs`<br>`crates/ontoforge-client/tests/contract.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P10 及下表接线。 |
| 07-11 | `clients/typescript/package.json`<br>`clients/typescript/src/generated.ts`<br>`clients/typescript/package-lock.json`<br>`clients/typescript/src/index.ts`<br>`clients/typescript/src/codec.ts`<br>`clients/typescript/tests/contract.test.mjs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P11 及下表接线。 |
| 07-12 | `crates/ontoforge/src/dev.rs`<br>`crates/ontoforge/src/dev_assets.rs`<br>`docs/dev.md` | VERIFIED | 存在、非stub；实际调用与行为证据见 P12 及下表接线。 |
| 07-13 | `crates/ontoforge-server/tests/postgres_http_it.rs`<br>`ci/server-services.sh`<br>`.github/workflows/ci.yml` | VERIFIED | 存在、非stub；实际调用与行为证据见 P13 及下表接线。 |
| 07-14 | `.github/workflows/release.yml`<br>`ci/release-package.ps1`<br>`ci/release-smoke.ps1`<br>`docs/installation.md` | VERIFIED | 存在、非stub；实际调用与行为证据见 P14 及下表接线。 |
| 07-15 | `crates/ontoforge-engine/src/printer/l2.rs`<br>`crates/ontoforge-query/src/l2.rs`<br>`crates/ontoforge-engine/tests/l2_explain.rs`<br>`crates/ontoforge-query/tests/l2_execute.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P15 及下表接线。 |
| 07-16 | `crates/ontoforge/src/cli.rs`<br>`crates/ontoforge/src/serve.rs`<br>`crates/ontoforge-server/src/state.rs`<br>`crates/ontoforge-server/src/mcp_l2.rs`<br>`crates/ontoforge-server/tests/mcp_l2_defaults.rs` | VERIFIED | 存在、非stub；实际调用与行为证据见 P16 及下表接线。 |

### Key Link Verification

自动 `verify.key-links` 的路径子串启发式只识别 3/48；其余返回 `Target not referenced in source`。Rust module/API、CLI 进程与codegen的连接不包含完整文件路径，verifier 用实际调用补查全部 48 条，未把启发式误报当成代码未接线。

| PLAN | 三条关键连接的实际路径 | Status |
|---|---|---|
| 01 | lib Router→context.build→EditStore.request_snapshot；rest→QueryService.execute；CLI Serve→serve.run→ServerHost.serve→lifecycle audit drain | WIRED |
| 02 | request_snapshot→reload.begin_definition_tx/sql_exit/scope_store；execute_with_snapshot→commit_action_guarded fence/CAS；history_with_snapshot→frozen_history测试消费者 | WIRED |
| 03 | rest→api.dto；context→request_snapshot→compile_scope；framework/domain错误→error.wire/response | WIRED |
| 04 | rest action/result→ActionService snapshot入口；history→history_page→history_with_snapshot；HTTP race→实际SQLite五表/overlay断言 | WIRED |
| 05 | openapi.document注册DTO/operations；serve export→render且在初始化前返回；openapi-gen读取docs/openapi.json写generated.ts | WIRED |
| 06 | serve.start_l2_acl_monitor→state.install_monitor；health→l2_status/reason_code；lifecycle按Action/read/audit/monitor顺序await | WIRED |
| 07 | MCP Parts→context_from_headers；ToolCatalog授权后call_read→QueryService；ActionToolSchema→call_tool→execute_with_snapshot | WIRED |
| 08 | 六tool dispatch→McpL2Tools→typed printer/Engine fresh gate/ReadAudit；authenticate_mcp→同host BoundedSessionManager；MCP/health读取同L2 engine proof | WIRED |
| 09 | stdio→StreamableHttpClientTransport→现有/mcp；CLI Mcp分支→mcp.run读取env/file token；EOF/cancel→remote取消和宿主audit | WIRED |
| 10 | Rust client方法→ontoforge-api DTO；真实contract→HTTP routes；ClientError保留error envelope/request_id | WIRED |
| 11 | generator→generated.ts；index.ts bodySerializer/parseAs text→codec；contract启动实际serve并调用全部端点 | WIRED |
| 12 | serve --dev→dev.prepare/lock/seed；dev_assets include_str→aviation canonical assets；restart/完整native smoke→持久history/receipt | WIRED |
| 13 | server-services.sh→query-services.sh setup/cleanup；PG HTTP/MCP→同snapshot/commit；real health/Trino tests→实际HTTP+ACL engines | WIRED |
| 14 | release native→package脚本真实build/archive；smoke→解包binary实际dev流程；native+aggregate成功→仅release.published上传，installation文档对应真实平台 | WIRED |
| 15 | L2Service typed API→ExplainType printer；MCP generic adapter消费同typed printer/验证能力；qualified table→AST resolver→TableRef | WIRED |
| 16 | CLI flags→deployment_mcp_defaults；serve→ServerState per-store defaults；McpL2Tools→resolver→当前allow-list→fresh engine gate | WIRED |

07-08/15 计划中的 `L2Service` 路径措辞在实际实现中由 generic `McpL2Tools<E>` 适配器承接，非直接导入 query::L2Service。已核验其连接到生产 `validate_l2`/typed printer、allow-list、`Engine` fresh attestation 和持久 ReadAudit，并有六工具/四格式/拒绝/取消/limit 行为证据；这里记录实际调用，未伪造不存在的 import。

### Data-Flow Trace (Level 4)

| 输出 | 实际源与传递链 | Produces real data | Status |
|---|---|---|---|
| metadata / describe | EditStore request_snapshot.active.ontology + 当前compiled scope→REST metadata/MCP describe | 是，reload/revoke新请求变化；非静态JSON | FLOWING |
| PK / query / aggregate | wire ObjectSet/WhereClause→QueryService→ScopedEngine→SQLite fixture或Trino + edit overlay→DTO/structuredContent | 是，固定示例source只是dev真实引擎输入；Action后可见变化 | FLOWING |
| Action receipt/result | ActionService→fence/CAS/guard→数据库事务→edit_log/entries/history/action_command/outbox→receipt/result | 是，五表与overlay完整持久行、event/time断言 | FLOWING |
| history | 当前scope可见对象→history_with_snapshot真实历史查询→按seq分组分页/byte budget→REST/MCP | 是，授权遮蔽和完整页跨transport一致 | FLOWING |
| L2 discovery/query/explain | 当前policy/defaults/AST→availability→audit reserve→TrinoEngine发送侧proof→真实SQL结果过滤 | 是，非法/越界输入上游总请求零增量 | FLOWING |
| health | 核心Engine.status + 当前drift view + 同L2 engine proof状态→稳定脱敏reason | 是，失败/61s过期/恢复分别改变状态 | FLOWING |
| Rust/TS client | per-instance HTTP bearer→真实REST→DTO或lossless decoder | 是，fixture主体只供应正常caller输入，生产context自行认证/取scope | FLOWING |


### Behavioral Spot-Checks 与完整动态门

| 行为 / gate | 实际执行入口 | 结果与证据 | Status |
|---|---|---|---|
| verifier独立OpenAPI spot-check | `node -e` 读取docs/openapi.json，assert openapi===3.1.0且paths精确八项 | exit0，`PASS: OpenAPI 3.1.0, eight paths` | PASS |
| verifier独立lossless spot-check | `node --input-type=module -e` 导入已构建codec，assert u64 max原始token/decimal、decode相同、unsafe Number抛LosslessCodecError | exit0，`PASS: u64 token, decimal, unsafe-number rejection`；仅此纯codec范围，不冒充整套HTTP测试 | PASS |
| T24/T08/L2相邻 | `cargo test -p ontoforge-server --lib --test mcp_execution_admission --test mcp_l2_defaults --test mcp_transport --locked -- --nocapture` 对应主会话运行组合 | `target/phase7-mcp-admission-green-r2.log`：lib25+admission4+L2defaults10+transport11=50 passed，0failed/ignored；verifier已逐项读原日志 | PASS |
| typed EXPLAIN timeout/cancel | `cargo test -p ontoforge-query --test l2_execute --locked -- --nocapture` | `target/phase7-explain-query-tests.log`：15/15；四format各有timeout、定向DELETE及独立并发审计断言 | PASS |
| stdio真实进程全链 | `cargo test -p ontoforge --test cli_stdio --locked -- --nocapture`；后续exact Mia成功链 | `phase7-bridge-security-cli-r2.log`11/11；`phase7-bridge-security-mia.log`1/1覆盖8次真实拒绝。后者不与11相加冒充不同测试数 | PASS |
| fresh真实PG/Trino/ACL/SDK | `bash ci/server-services.sh verify`（主会话Windows环境） | `target/phase7-services/evidence/final-adc0227-verify.log`：PG4、drift2、snapshot PG1、Trino2、health2、ACL6、Rust9、TS17，共43 passed；`fresh_verify_exit=0`，TS0skipped；commit.txt=`adc02278b5cc55111def9a30a1c3b778485e8279` | PASS |
| TS生成/strict/build | 同fresh脚本执行 `npm run check:generated`、`typecheck`、`build`、`test:contract` | 使用该次fresh编译binary，非早期debug副本；17合同与精度/故障场景全部实际执行 | PASS |
| 完整CI / 两OSworkspace | CI `41669982375`，同source tree；fmt/Clippy/all-targets/embedded-pg/TLS、Windows+Ubuntu workspace、integration、phase7-real-services | `target/phase7-ci-41669982375.json` completed/success：test (windows-latest)、test (ubuntu-latest)、integration、phase7-real-services全部success；workspace分别1490/0/117与1497/0/117（passed/failed/ignored，各149个suite result）；可选isolated-query-bench skipped不计必需门 | PASS |
| 发行包与安装 | release `41669982362` 的3native与aggregate；package/smoke在各native runner执行 | 3native+aggregate success；PR event publish skipped；实际下载archive SHA256 verifier独立重算一致 | PASS |
| 审查闭环 | FINAL-SPEC/STAFF/STANDARDS/SIMPLIFICATION、07-SECURITY及T24/E6/T08独立复签 | 无开放finding；SECURED、48/48 CLOSED；简化建议0，未为简化而扰动冻结代码 | PASS |

主会话最终服务清理证据：Windows服务状态 exited/exit_code=0，fresh verify返回0；凭据扫描/cleanup门由服务CI/主会话执行，未读或输出env凭据。初始preflight `jq is required` 返回1已由可执行环境更正后的整套fresh43/43替代；该初始失败未被计为通过。

### Native Artifacts

三份manifest版本均0.1.0、commit均`b75d2949f9db1276ce07f4864a6206f7932bb11b`。元数据 `target/phase7-release-ci-41669982362.json`；实际产物及复核记录 `target/phase7-native-artifacts-41669982362/`、`target/phase7-native-artifacts-41669982362-verification.json`。

| Target | Native环境 | Archive bytes | 独立复算SHA256 |
|---|---|---:|---|
| aarch64-apple-darwin | macOS15.8.2 Arm64 | 18386682 | 085ea1fad049c7289edba50471039cfc5d300da1877c2febb1a4e85b7ed9fdc1 |
| x86_64-pc-windows-msvc | Windows10.0.26100 X64 | 18814840 | ed48f0b8aa6e1cbef4ba87d240a50f95beacb0669386862387aa7d2ef2918a88 |
| x86_64-unknown-linux-gnu | Ubuntu24.04.5 X64 / glibc2.39 | 20506039 | 944e90de1e6238ec08627dee06aaecf83fff6ef8bb115c5cdc811133b33c4d05 |

这些产物仅证明所列native平台，未宣称universal/static或未测试Linux libc兼容性。安装smoke实际验证解包binary的version/help、query→Action→history→command receipt→restart保留→reset→stdio→正常关闭。

### Probe Execution

没有PLAN声明的 `scripts/*/tests/probe-*.sh` 或其他待运行probe文件；本phase为服务壳与SDK，原始动态门入口是上述Cargo/Node contract与native PowerShell package/smoke。未以SUMMARY里的PASS文字替代这些命令结果。

### Requirements Coverage

| Requirement | Source PLAN | 描述 | Status / 证据 |
|---|---|---|---|
| API-01 | 01–07,13 | REST metadata、对象、object set、Action、history | SATISFIED，R1/R4、八路由与真实HTTP/PG/SDK |
| API-02 | 05 | OpenAPI3.1与snapshot | SATISFIED，P05、静态render/独立schema断言/CLI导出 |
| API-03 | 01,03–06,13 | code/apiName/request_id与L0脱敏 | SATISFIED，error mapping、http_contract、MCP/SDK错误及audit关联 |
| API-04 | 10,14 | Rust全部REST能力 | SATISFIED，P10、真实contract9 |
| API-05 | 11,14 | OpenAPI生成TS callable client | SATISFIED，P11、fresh17/17、严格生成/类型门 |
| MCP-01 | 07–09,13 | stdio及streamable HTTP | SATISFIED，T24/T08真实HTTP和CLI进程 |
| MCP-02 | 07 | describe ontology结构/属性/links/Action描述 | SATISFIED，mcp_describe与scoped REST真实内容一致 |
| MCP-03 | 07 | filter/search/searchAround/aggregate | SATISFIED，call_read→QueryService及声明参数形状实测 |
| MCP-04 | 07 | 每Action真实参数派生工具schema | SATISFIED，ActionToolSchema、numeric refs/default/nullable/动态切换 |
| MCP-05 | 07 | validate-only dryrun | SATISFIED，SQLite/PG/stdio成功与失败路径完整持久行不变 |
| MCP-06 | 02,04,07–09,13 | REST/MCP同授权/edit log | SATISFIED，R4、跨transport竞争与双主体反例 |
| MCP-07 | 07–09,13,15,16 | 主体toolset与六L2名称参数 | SATISFIED，catalog白名单、真实六工具/四formats、defaults/allow-list |
| MCP-08 | 07,08,13 | agent无L3或定义/授权管理工具 | SATISFIED，list/call同catalog、猜名拒绝、reload/revoke不缓存授权 |
| DX-01 | 12,14 | Windows单binary dev SQLite/token零外部依赖 | SATISFIED，嵌入assets、真实Windows native安装smoke |
| DX-02 | 12,14 | 示例ontology query→Action→history | SATISFIED，aviation真实业务值/receipt/history/重启 |
| DX-04 | 14 | 三平台单binary发行workflow | SATISFIED，3实际native artifacts+smoke，release.published授权上传接线；本轮按禁止项不触发发布 |
| L2-04 | 06,08,13 | 启动与持续ACL证明、fail-closed/health | SATISFIED，monitor持有、真实health61s过期/恢复、宽松ACL对照 |
| SCOPE-07 | 01,02,04,07–09,13,16 | 管理通道限制、每请求固定定义 | SATISFIED，request_snapshot共同锁+PG混代测试、MCP无管理入口 |
| BIND-03（继承服务部分） | 13 | 启动/周期drift执行 | SATISFIED，source_drift真实目标隔离/恢复/外部reload2/2 |
| ENGINE-02（继承服务部分） | 06 | /health与describe暴露引擎状态 | SATISFIED，metadata/describe真实状态、core503/L2degraded合同 |
| ENGINE-04（继承服务部分） | 12 | serve --dev SQLite宿主 | SATISFIED，dev lifecycle及三native安装 |

最终PLAN声明集合与REQUIREMENTS Phase7职责一致，21/21有对应artifact与行为证据；原18项roadmap contract全部覆盖。没有ORPHANED requirement。REQUIREMENTS checkbox由主会话依正式报告派生，未作为本verifier通过依据。

### Test Quality Audit

| 测试族 | 核心断言强度 | Disabled / circular 核对 | 结论 |
|---|---|---|---|
| snapshot / frozen Action/history | 共同metadata锁、pending barrier、旧新定义、完整业务状态零写、receipt/time相同 | PG用ignore隔离依赖，真实服务门明确执行；SQLite主动运行 | VERIFIED |
| REST/MCP contract / Actions | scope先过滤、真实reload/revoke、完整history/receipt、错误code/apiName/request_id与脱敏 | 无skip/todo；同transport比较外还有独立持久行/值断言 | VERIFIED |
| T24 sessions | inner.sessions实际容量、pending初始化、取消/TTL/DELETE/宿主关闭、wrong owner不延寿 | 不是仅owner HashMap计数；12个session场景经实际wrapper/HTTP | VERIFIED |
| T08 admission | source进入Notify后保持阻塞，REST↔MCP双向429/零发送、满槽真实cancel/SSE/DELETE、Action断连仍持槽、慢body限额 | 无纯sleep决定因果；readonly逻辑与Action owned permit分别验证 | VERIFIED |
| typed EXPLAIN | 四format分别timeout/nextUri GET后取消，精确DELETE目标、单条audit/独立ID，HTTP limiter四格式 | 原空名覆盖已补实；query15与serverdefaults10均实跑 | VERIFIED |
| SDK / stdio | 真实双主体、scope_apply撤权、proxy发送前断连/提交后丢响应、五表及overlay完整行、显式replay原receipt | 无自动生成“期望结果”冒充外部oracle；数值边界由独立字面量规定 | VERIFIED |
| native package/smoke | archive实际解包binary、query变值、history event/time、restart/reset、stdio和cleanup | 三native真实执行；非只检查workflow文本 | VERIFIED |

10 个 server integration `#[ignore]`（PG4、drift2、Trino2、health2）已由 `server-services.sh` 精确发现并在真实服务门执行；storage PG snapshot、engine ACL likewise 有非零独立运行。默认workspace ignored不被计为通过，也不被错误视为遗失测试。相关 TS 测试0 skipped。未发现需求仅由disabled测试支撑。

反证检查：`dev_distributed_binary_query_action_history_without_checkout` 实际只测复制binary启动及config存在，其名称不足以证明全链；完整用户故事由实际三平台 archive smoke 的业务值/receipt/history/restart/reset断言补足。OpenAPI render equality只证明漂移，不能独自证明API正确；另有独立schema常量/serde正反例及真实客户端合同。此前T24/E6/T08确有RED，只有对应修复后的行为GREEN被用于本报告。

### Anti-Patterns Found

| 文件 / 检查 | 匹配 | 分类 | 处理 |
|---|---|---|---|
| Phase6结束基线 `c9f823c` 至冻结head的128个实现/测试/CI/文档文件 | 无未引用的债务注释TBD/FIXME/XXX；无todo!/unimplemented!/业务空返回stub | 无BLOCKER | `ci/*services.sh` 的mktemp `XXXXXX` 是模板占位语法，不是债务注释 |
| production dynamic outputs | 无hardcoded空数据替代真实查询 | 无BLOCKER | 初始空Vec、分页无结果与fixture常量有明确语义，不按regex误报 |
| runtime/tests | 原始失败日志与后继GREEN并存 | INFO | 保留RED归因；不因文件名含green就当成功，不重复累计重叠测试 |
| plans的跨phase requirement tags | BIND-03/ENGINE-02/ENGINE-04初始未声明 | 已修复 | 主会话补07-13/06/12 metadata；最终21 IDs均映射，无ORPHANED |

### Decision Coverage

执行 `query check.decision-coverage-verify`：3/3 honored，not_honored=[]。D-01 stdio桥接既有host、D-02 dev持久化/显式reset、D-03 core健康时L2故障200 degraded均有上述生产接线和行为证据。API coverage实际 `verify:pre`：passed，25 surface / 21 INTEGRATE / 4 OPT-OUT；证据 `target/phase7-verify-pre-api-coverage.json`。

### Prohibitions

| 来源 | Must-NOT | Enforcement / disposition |
|---|---|---|
| 07-06 + 07-13，test | core健康时不得隐藏配置L2故障/过期 | health/engine fresh gate + 真实61s expiry/recovery、negative ACL及零发送拒绝，VERIFIED |
| 07-09，test | stdio不得新开独立数据host/edit store | bridge仅HTTP/stdio transports；真实existing host与相同durable receipt/audit，VERIFIED |
| 07-11，test | TS不得静默舍入整数或改变wire合同 | native rawJSON/context.source、unsafe integer主动拒绝；u64/i64/decimal原始token实测，VERIFIED |
| 07-12，test | 正常重启不得清空修改/history/receipt | 空seed+OS锁，dev restart与三native完整smoke，VERIFIED |
| 07-14，judgment | 本轮不得合并PR7、创建Release或发布npm；PR7须Draft OPEN | 用户既有明确决定，resolved-by-user；PR7/8 fresh元数据OPEN/isDraft=true/mergedAt=null，release pull_request事件publish skipped，TS package private=true。自动过程审查为 **NON-AUTHORITATIVE LLM-judge**，不是新的人工批准；没有借此授权任何发布动作。 |

四项唯一test-tier禁止项均有已接线且已通过的enforcement evidence。唯一judgment项已有明确用户决议，按该决议核验过程遵守；无未决方案，不将其改造成重复人工确认。没有未接线的test-tier禁止项，也没有静默丢弃禁止项。

### Human Verification Required

无。此phase没有产品UI，用户故事通过真实CLI/SDK/HTTP/stdio与三native安装流程自动化验证。所有状态转换、取消、清理和顺序不变量都有行为断言与运行证据；不要求用户重复运行已自动完成的测试。没有验收豁免、未验证行为或backstop abstention。

### Deferred Items

无。ROADMAP中没有可用于延后本phase六成功标准的后续phase；没有将真实缺口推至后续milestone。

### Gaps Summary

最终未保留BLOCKER、WARNING或behavior-unverified truth。原T24真实session资源、E6发送前预检、T08 SSE执行permit三项真实缺陷已以各自RED→修复→GREEN及独立security/Staff复签关闭；N1/N2 stdio与N3 typed EXPLAIN原合同缺失证据也已补齐。Spec/Staff/Standards与最终简化复核无开放finding，`07-SECURITY.md`为SECURED、48/48 CLOSED、threats_open=0。

Release上传能力按PLAN14已批准条件接线；本次仅验证实际native产物与安装，没有创建Release、发布npm或合并PR。下一步是否交付/发布由主会话执行既有授权边界。


---

_Verified: 2026-11-03T16:34:44Z_
_Verifier: gsd-verifier（goal-backward verification；未提交）_
