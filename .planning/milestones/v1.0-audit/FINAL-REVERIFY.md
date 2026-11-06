# 最终源码真实复验

日期：2026-11-05。状态：最终源码动态复验与 Phase 07 增量复签完成；不是 milestone 完成声明。

## 执行边界与计划

- 目标 HEAD：`060019fe52005c6b85ae5c5f59da9868f464e579`。先按 P1-7 canonical 中实际 verified source/delivery/CI head 与 `covered_files` 逐文件 diff；完整机器证据：`evidence/2026-11-05/reverify-impact.json`。
- 唯一 Cargo/Docker owner。本地先查 Cargo/rustc 占用，再以 `CARGO_BUILD_JOBS=2`、独立 `target/reverify-060019f-20261105` 串行 discovery、workspace、SQLite `action_atomic_` test-support、TLS hermetic 与真实依赖树检查。不启动或更改 Docker、既有容器。
- 真实 PG/Trino 服务与 TypeScript SDK 使用主会话新 dispatch 的 CI `41994479295` 原始日志，Rust SDK 另在本地 workspace 实跑，均绑定当前 HEAD；不重复 dispatch，不以旧测试或 fingerprint 冒充重跑。独立读取新日志后才更新 `07-VERIFICATION.md`。
- 每次命令记录完整 stdout/stderr、command、head、起止时间、exit。逐 target 对照 discovery/execution；zero discovery 不得通过，ignored 不计 PASS。
- 只写本报告、`evidence/2026-11-05/reverify-*`、测试完成后的 Phase 07 canonical 及自己的临时 target。不修改共享 progress/STATE/REQUIREMENTS、业务源码或用户五项 untracked；不 commit/push，不修复失败。
- 主会话已调用 planning-with-files 落盘。本 worker 已读取 `gsd-verify-work` 并执行 GSD `init.verify-work 7`，只将审计状态写入本报告。审计复验无实现改动，跳过 RED；Warp、ralph-loop 不可用，以串行执行、原始日志、exit 与发现/执行计数构成替代闭环，不声称调用。
- 保留 Phase 01 `human_needed`、Phase 02 `gaps_found`、ENGINE-05 SQLServer Partial。最终不宣称 milestone 完成。

## 源码影响矩阵

七个 canonical 的最近 verified delivery/source/CI 基线均为 `f8851ca073fed34e80e605f0ac91880259df7e12`。P1/P3 无显式 source frontmatter，使用正文明确的核验 HEAD，未把 implementation 起点当作最新 verified head。完整路径与新增证据边界见 `reverify-impact.json`。当前 HEAD 相对 `8d9d8c6` 的 crates/ci/clients/Cargo/workflow diff 为空，但不替代真实复验。

| Phase | covered 非 planning 变更数 | 受影响实现或输入 | 复验套件 |
| --- | --- | --- | --- |
| 01 | 5 | schema diag、CLI manifest/cli/lib、CONTRIBUTING | schema + CLI workspace |
| 02 | 16 | engine lib/printer、doc_type_table/l2_monitor、CLI online、fixture、nightly | engine/CLI workspace + 当前 CI integration；MinIO 07.2 仅保留已归档独立证据 |
| 03 | 10 | engine、schema diag、CLI、PG seed | workspace storage/auth/query/actions + SQLite test-support + CI PG |
| 04 | 8 | engine printer/lib、doc_type_table/l2_monitor、PG/Iceberg fixture | workspace query/engine + CI integration + isolated bench |
| 05 | 4 | action Trino integration test、CLI、nightly | workspace action + SQLite atomic + CI action PG/Trino |
| 06 | 1 | l2_monitor test | workspace L2/corpus/monitor + CI ACL |
| 07 | 5 | engine lib/l2_monitor、CLI manifest/cli/lib | workspace API/server/client/CLI + CI phase7-real-services Rust/TypeScript SDK |

计数为精确 `covered_files` 与 git diff 交集，含文档、配置、测试，不等同于生产源码文件数。依赖传播由完整 workspace 和真实服务 CI 补足，不以交集之外无变化推定无下游影响。

### 测试重跑与 canonical 复签是两件事

初次独立 GSD 查询保存在 `reverify-phase-statuses-before.json`：Phase 01 canonical=`human_needed`、owner routing=`stale`；Phase 02 canonical/owner=`gaps_found`；Phase 03–06 canonical=`passed`、owner routing=`stale`；Phase 07 canonical=`passed`、owner routing=`stale`。本 worker 不改 Phase 01–06 报告，也不以当前完整 workspace 已重跑推导其报告已新鲜。Phase 03–06 的旧通过结论与当前 stale 必须并列呈现，直至所属 verifier 另行复签。

受影响新增代码不能只看旧 covered 交集：实际 diff 还新增 `cardinality_check.rs`、`tests/cardinality_check.rs`、`cli_check_cardinality.rs`，并更改 `online.rs`、printer 与 diag 接线。已亲读 CLI → online → cardinality_targets/check_cardinality → typed CountDistinct printer → Engine.execute → ONT-W18 diagnostics 的数据流，完整 diff 及债务/disabled 扫描保存在 `reverify-source.diff`、`reverify-source-checks.log`。扫描无未引用 TBD/FIXME/XXX，新增 cardinality 与 monitor 测试无 disabled 标记。

## 证据质量与执行边界

### 计数器证据质量修复（TDD）

Standards reviewer 发现 owner 自建计数器的两个确定性反例：零发现 target 可被其他 target 的通过掩盖；重复执行 identity 可掩盖缺失 target。原始 Cargo 日志不受影响，但旧计数器 `complete` 结论不得用于复签。用户已确认测试 seam 为脚本 CLI 输出的 `complete`/逐 target 分类，使用 Node 原生 `assert`/`node:test` 经真实子进程与专属临时日志验证，不 mock 内部聚合实现。已读取安装的 `tdd` skill（及 tests/mocking 指南），本次证据核心逻辑修改单独执行 RED→最小修复→GREEN；此前“审计跳过 RED”仅指生产业务无实现改动，不豁免本修复。合法空 harness 必须显式 allowlist，不能计成功；ignored-only 只能明确豁免待外部套件覆盖，不能冒充 nonignored PASS。

初次独立 target 设在 D 盘，发现剩余空间不足 4 GiB 后，owner 主动中止 discovery 编译，未进入测试。原始 `reverify-workspace-discovery.log` 保留，不作为通过。后续改用 C 盘专属 `C:/Users/easyi/AppData/Local/Temp/ontoforge-reverify-060019f-20261105`，沿用 CI 的 debug=0/incremental=0 构建配置；不清理任何共享 target，不把中止当作源码失败或测试成功。

本地执行与五份新 CI 原始证据均已完成，终态计数与复签见下文；先前的中止不计入成功。

## 当前 CI 独立读取

已亲读主会话下载的 run `41994479295` integration 与 isolated-query-bench 原始日志全部行，确认 checkout 为 `060019fe52005c6b85ae5c5f59da9868f464e579`。原 UTF-8 BOM/ANSI 不改写，解析时仅内存去除；逐步骤、逐 target、原始行号与 SHA256 写入 `reverify-ci-counts.json`。主会话首次 gh 因 terminal escape sequences 拒绝下载，重试使用 `--allow-escape-sequences` 成功，这是下载边界，不是产品失败。

- integration：PG conformance 六 target 分别 45/1/2/4/11/3 passed；audit 重跑 9；atomicity 5、same-pk 4、idempotency 6、webhook 2、source guard 1；Trino ACL 6、engine 9、CLI 4、row predicate 3、reload 3、query_read_pg 2、trino_query_it 5、bench correctness 1、action 4。所有结果 failed=0。audit 与 conformance 可能重合，以上是命令执行数，不汇总成唯一测试数。
- discovery：audit 9、atomicity 5、same-pk 4、idempotency 6、webhook 2、ACL 6、Trino action 4，均为正数并与相应执行数一致；source guard 采用同名 exact-list grep gate，执行 1。
- isolated bench：`phase4_real_backend_p95` 1 passed/0 failed，四路径 p95 为 source_pk 107.648791 ms <500、filtered_list_page 472.588841 ms <1000、aggregate 407.732135 ms <1500、edited_pg_direct_pk 1.127974 ms <50。原日志记录 artifact `13400156425` 上传及凭证扫描/cleanup 步骤。尚未下载 JSON，不能将日志中的 p95 叙述扩展为完整产物审计。
- 上述为先到日志；后续已独立读取另外三 job 与完整 workspace，全部终态 success。Phase 07 复签前 owner query=`stale`，原结果保留 `reverify-status-before.json`；复签后结果另存，不覆盖历史证据。

### Phase 07 服务与客户端分层（新日志已亲读）

| target | runtime discovery | passed / failed / ignored | 原始日志行 |
| --- | --- | --- | --- |
| postgres_http_it | 4 | 4 / 0 / 0 | 606 |
| source_drift_trino_it | 2 | 2 / 0 / 0 | 618 |
| request_snapshot (PG ignored subset) | 1 | 1 / 0 / 0 | 633 |
| trino_http_it | 2 | 2 / 0 / 0 | 645 |
| health_trino_it | 2 | 2 / 0 / 0 | 657 |
| trino_l2_acl_it | 6 | 6 / 0 / 0 | 706 |
| Rust client contract | 9 | 9 / 0 / 0 | 731 |
| TypeScript contract + numeric security | 17 | 17 / 0 / 0 (skipped=0) | 793–799 |

服务 job 合计执行 43 次，但不与其他 job 或本地 workspace 相加作唯一测试数。前六 target 是实际 PG/Trino 17 项；Rust SDK 是真实 HTTP + SQLite fixture（`tests/support/mod.rs:15,56,223`），不是 PG/Trino SDK 端到端证明；TypeScript 用真实 `ontoforge serve --dev` 临时 SQLite 数据目录（`tests/contract.test.mjs:56,64,80`），部分 malformed/transport 案例另用受控 HTTP peer。Rust SDK 在本地与两平台 workspace 也会重跑，重复执行保留但不重复累计覆盖。CI `ci/server-services.sh:58` exact named discovery 检查与实际 `running N tests`/结果 N 一致，无 zero discovery。

该 job 原始 SHA256 为 `5cad89dc1e75935cc5bdaa3ca82e5e1f4df06c81c56e7dc64425b0996820c727`；主会话 progress/final metadata 记录 completed/success，凭证扫描、artifact upload、cleanup 均实际运行成功。没有将缓存或历史 SDK 结果冒充本轮重跑。

## 本地真实执行终态

完整命令、UTC 时间、exit 与日志路径保存在 `reverify-local-results.json`；所有命令串行，`CARGO_BUILD_JOBS=2`，无并行 Cargo。C 盘专属 target 的 profile 与 CI 一致（debug=0、incremental=0），不动共享 target。本地没有因缺失工具继续重建其他全新 target。

| 命令范围 | UTC 起止 | discovery / execution | exit | 原始日志（本目录 evidence/2026-11-05/ 下） |
| --- | --- | --- | --- | --- |
| 完整 workspace discovery | 08:31:17–08:43:50 | 152 harness / 1647 tests | 0 | reverify-workspace-discovery-c.log |
| 完整 workspace execution | 08:43:50–08:51:34 | 152 harness；1528 passed、0 failed、119 ignored | 0 | reverify-workspace.log |
| SQLite test-support action_atomic_ discovery | 08:51:34–08:53:01 | 5 tests，非零 | 0 | reverify-sqlite-atomic-discovery.log |
| SQLite test-support action_atomic_ execution | 08:53:01–08:53:02 | 5 passed、0 failed、0 ignored、43 filtered | 0 | reverify-sqlite-atomic.log |
| TLS hermetic | 08:53:02–08:53:02 | 6/6 passed | 0 | reverify-tls-hermetic.log |
| TLS 真实依赖树 | 08:53:02–08:53:03 | check-tls-deps.sh 成功，非 Rust test harness | 0 | reverify-tls-real.log |

逐 target 计数源 `reverify-workspace-counts.json`：**152 harness 不等于 152 successful targets**。128 个 mandatory nonignored successful targets，12 个显式命名空 harness，12 个显式 reviewed ignored-only 排除项；所有 discovery/execution identity 集合一一对应、无重复。必验 target 的 discovery 与 nonignored execution 均非零。12 个空 harness 的政策逐项命名 CLI main、两个 feature-off PG harness 与九个空 doctest；12 个 ignored-only 政策逐项给出精确期望数量与外部 CI 覆盖/明确忽略文档示例理由。未知 zero harness、未知 ignored-only、重复或缺失 identity 一律拒绝 `complete`。119 ignored 原样报告，不冒称 119 个全部在另一个 job 执行；本地 complete 只表示已声明范围的执行与明确排除政策完整。

### 计数器 TDD 与独立复检

已用安装的 `tdd` skill，遵守已确认公共 seam：既有 discovery/execution 日志输入 → 原 JSON `complete` 输出，不改变 CLI。Node 原生 `assert`/`node:test` 通过真实子进程测试，不 mock 计数器内部。真实 RED 与 GREEN 原始记录如下，均保留 exit：

| 切片 | RED（exit=1） | GREEN（exit=0） |
| --- | --- | --- |
| A nonzero、B zero 必验 target 拒绝 complete | reverify-counts-zero-red.log | reverify-counts-zero-green.log |
| discovery A/B、execution A/A 拒绝 complete | reverify-counts-identity-red.log | reverify-counts-identity-green.log |
| 显式命名合法空 harness 不计成功 | reverify-counts-empty-policy-red.log | reverify-counts-empty-policy-green.log |
| 显式 ignored 文档例排除不计 nonignored 成功 | reverify-counts-ignored-policy-red.log | reverify-counts-ignored-policy-green.log |

最终 `reverify-counts-regression-green.log`：11 tests/11 pass/0 fail/0 skipped，exit=0，另涵盖未知空 doctest、未知 ignored-only、重复 discovery、等量不同 identity 集合、合法重排序、缺失豁免 target、仅空 harness、ignored 数量漂移。最小修复只在 owner 的 `reverify-counts.cjs` 及其测试中；生产业务零修改。重新读取同一真实本地日志生成 `reverify-workspace-counts.json`，`complete=true`，152 harness/128 successful/1528 passed/119 ignored/0 failed；原 RED 不是产品测试失败。

主会话于本轮转交 Standards reviewer 独立终态复检：原 zero discovery、identity 两项 finding 全部 CLOSED，新增高置信 finding=0，真实 CLI seam 11/11 passed；独立真实日志复算同为 152 harness、128 successful、1528 passed、119 ignored、0 failed、complete=true。此处准确标为主会话转交的独立复检结论，不伪造 reviewer 原始日志；本 worker 自有 RED/GREEN 原始证据独立留存。

## 双平台与五 job 终态复核

`ci-41994479295-final.json`：run completed/success、5/5 job completed/success、headSha=`060019fe52005c6b85ae5c5f59da9868f464e579`。本 worker 已亲读全部原始 job 日志，不只采信 metadata。`reverify-ci-counts.json` 保存逐步骤/target/行号/SHA256；`reverify-ci-window-audit.json` 将 workspace 原始结果行再与 final metadata 的独立 step 时间窗口核对，无遗漏、越界或 runtime N 不匹配。

| 范围 | workspace UTC 窗口 | harness | passed / failed / ignored | 原始行范围 |
| --- | --- | --- | --- | --- |
| Ubuntu workspace | 08:30:42–08:41:17 | 152 | 1535 / 0 / 119 | ci-41994479295-ubuntu.log:1041；结果 1302–3838 |
| Windows workspace | 08:34:24–08:52:48 | 152 | 1528 / 0 / 119 | ci-41994479295-windows.log:1029；结果 1291–3820 |

两平台独立执行数不同，不能跨平台求和成唯一测试总量。每个 runtime `running N tests` 与相应 passed+failed+ignored+measured 一致，五份日志无 `##[error]` 或 FAILED summary。后续步骤单独归类：每平台 L2 corpus 6 passed（覆盖 104 IDs，不是 104 tests）；Action 重跑 64 passed/6 ignored；SQLite fault injection 5 passed/43 filtered。Windows private-console shutdown 11/11，raw:3834–3849；不把这 11 或后续重跑加回 workspace 窗口。

integration、phase7-real-services、isolated-query-bench 的 job-scoped credentials 扫描、artifact upload 和 always cleanup 均实际 success，失败诊断日志步骤为 skipped 不算失败。Windows/Ubuntu runner cleanup 与 post job completed；本机 Docker 不参与。CI API 一次 TLS 握手超时后主会话重试成功，是取证网络事件，不是产品失败；首次下载 escape sequences 拒绝也是取证事件。

### 原始 CI 日志完整性

| 原始日志 | SHA256 |
| --- | --- |
| ci-41994479295-integration.log | bdc51412697da080151c6b7f68e42f88c0e74bb21ecbfe67acc2fdaba23b449e |
| ci-41994479295-isolated-query-bench.log | dbfefa449e95c858c9e9c11f3ef7690ae47b80dfbfa4a73c9a88d56a4828aa99 |
| ci-41994479295-phase7-real-services.log | 5cad89dc1e75935cc5bdaa3ca82e5e1f4df06c81c56e7dc64425b0996820c727 |
| ci-41994479295-ubuntu.log | ae3d8ed246f84a5ba0ea323ee3a84311f76a8119e8deffeb7679da6161e723c8 |
| ci-41994479295-windows.log | 243cf6436c9e9c7ca1b081cd0ce1df3ca486a25d7b678d10303e5ad9adda1753 |

## P1–6 受影响套件本轮实际覆盖

下表是影响矩阵的动态执行对应，不是重签旧报告。workspace 证据均为本地 `reverify-workspace.log` + 逐 target JSON，并由上述双平台新 workspace 交叉复验；不同命令/平台重复测试不汇总唯一数。

| Phase | 本地实际执行的受影响及下游代表 target | 本轮真实外部/专项覆盖 | 原 canonical 边界 |
| --- | --- | --- | --- |
| P1 | schema unit 139、loader 34、cli_schema 4、invalid_examples 1；Ubuntu schema unit 140 另记录 | CLI/schema 全 workspace；TLS hermetic 6 与真实依赖检查 | human_needed 保留，未重新签署 |
| P2 | engine unit/printer 56、cardinality_check 11、cli_check_cardinality 13、bindings 32 | integration Trino engine 9、CLI 4；真实 source drift 2；07.2 MinIO483/484 本轮未重跑 | gaps_found 保留；SQLServer Partial 不提升 |
| P3 | storage unit 49、SQLite conformance 43、reload 32、query_read 11、auth unit 3 | SQLite action_atomic 5；PG conformance 45、migrate 4、reload 11、query_read 2、audit 9（重叠不加总） | 测试已重跑；原报告尚未重新签署，仍 stale |
| P4 | query unit 5、overlay_prop 11、engine printer 56、下游 server/query 合同 | Trino query 5、PG read 2、isolated p95 1；阈值见上文 | 测试已重跑；原报告尚未重新签署，仍 stale |
| P5 | action 6、idempotency 9、entrypoints 4、action unit 4 | SQLite atomic 5、PG atomic 5/same-pk 4/idempotency 6/webhook 2/source guard 1、Trino action 4 | 测试已重跑；原报告尚未重新签署，仍 stale |
| P6 | l2_monitor 19、其状态/并发/停止行为测试实际执行，mcp_l2_defaults 15 | 每平台 L2 corpus 6（104 IDs）、实际 Trino ACL 6 | 测试已重跑；原报告尚未重新签署，仍 stale |

P7 的真实服务与 SDK 分层见上表；API/MCP、本地 SQLite 和 transport 通过 workspace 重跑，不以 PG/Trino ignored-only 默认结果当作服务通过。

## Phase 07 复签与最终边界

所有本地执行与新 CI 证据完成、原始日志独立读完、计数器 GREEN 后，才更新 `.planning/phases/07-rest-mcp-shell/07-VERIFICATION.md`：原 80/80 合同分母不变，当前增量 passed、behavior_unverified=0，无新增 override/BLOCKER/WARNING。历史 native release、14 项 UAT、安全审查、07.2 证据明确保留历史日期和范围，没有声称本轮重新运行。

当前 GSD `verification.fingerprint` 输出归档 `reverify-fingerprint.json`：历史 237 项与前次 232 项完整并集为 248 covered files，历史 gate 全保留、精确新增 11 个实现/fixture；digest=`v4:sha256:18dcf94e6e282b85930df7c8c10a183abcd0f66009c82f985a3388ab8aa67396`。报告自身不计入。前次 232 项和 digest=`v4:sha256:bb31129c390b1163601613324c4da4023e9834f5aa71e50941cea0218d6e67d7` 已另存 superseded 派生快照，不作为当前范围结论。写后 owner `verification.status` exit=0、status=`passed`，当前结果见 `reverify-status-after.json`。fingerprint 仅绑定内容，不替代测试。

复签只限 Phase 07；P01 human_needed、P02 gaps_found、ENGINE-05 SQLServer Partial 继续存在，P03–06 原报告仍 stale，不宣称 milestone 完成。发现的失败原样保留：计数器 TDD 反例 RED 与 D 盘主动中止 discovery；两者不是产品失败。本轮实际产品执行 failed=0，未为修绿修改生产源码、共享 STATE/REQUIREMENTS/progress、用户五项 untracked 或受保护 Docker 容器；未 commit/push。Warp/ralph-loop 不可用，使用已记录的串行执行→原日志→TDD/独立复检→GSD owner 查询替代闭环，不声称使用不可用工具。

最终收口复核 `reverify-final-validation.log`（09:04:35–09:04:39Z）：Node 11/11 再次通过，HEAD/生产目录与 `8d9d8c6` 等价、生产目录无未提交修改、报告与 GSD fingerprint 输出逐值一致，断言 exit=0，Cargo/rustc 进程=0。`reverify-phase-statuses-after.json` 再查证 P01 stale/human_needed、P02 gaps_found、P03–06 stale、P07 passed；各原 canonical 均未扩签。证据字节数与 SHA256 索引保存 `reverify-final-evidence-index.json`（不对自身求 hash）。

### 限定修正：保留完整历史 gate

最终抽查发现前次 237→232 是新增 11 项、漏掉 16 项治理证据，不是生产覆盖删减。根因是本 worker 构造 fingerprint 参数时使用 `old.filter(f => !f.startsWith('.planning/'))`，错误地把所有 planning 下证据排除；GSD 只自动补回本 phase PLAN/SUMMARY，未补回 SECURITY/UAT/VALIDATION/COVERAGE 和 12 份 reviews。GSD 参数解析与 digest 实现没有 200 个显式输入限制；共享 planning 根直接子文档 inert 规则不适用于这 16 项。仅输入 200 项是 owner 筛选结果，不是工具限制。

这些文档仍用于原 80/80 历史合同，删除其覆盖会让其内容变化不再参与该 fingerprint 的 stale 判定，没有充分依据。本次采用原 237∪前次 232=248：old⊆new、11 个新增实现/fixture 与全部 16 份治理证据均在，gate 不再缩小。历史人工 UAT/安全/release 来源继续保留原日期，不冒称本轮重跑。

修正前保留五份不可变派生快照：`reverify-fingerprint-232-superseded.json`、`reverify-status-after-232-superseded.json`、`reverify-phase-statuses-after-232-superseded.json`、`reverify-final-evidence-index-232-superseded.json`、`reverify-phase7-232-superseded.md`。原始产品、CI、TDD 和原 `reverify-final-validation.log` 不改写；`reverify-scope-preservation-before.json` 固化修正前 200 个非 planning 输入和 25 份原始日志的 SHA256/字节数。新 `reverify-scope-preservation.log` 独立核对集合、治理项及这些字节不变，另新增收口复核记录，不以覆盖旧日志伪装旧 gate 未缩小。本次无源码修改，不重跑未变化的产品测试/CI，不改根目录或共享 summary/review/state/taskplan，不 commit/push。

限定修正收口 `reverify-scope-final-validation.log`（09:37:08–09:37:12Z）exit=0：旧索引 47 项中非本次可变派生数据的 42 项 SHA256 全部一致；superseded fingerprint/status/phase-status/report 与旧索引原哈希逐项一致。当前 GSD owner query 为 passed，P01/P02 与 P03–06 路由边界未变。当前 `reverify-final-evidence-index.json` 为 **56 条 entries**（旧 47 加 9 个限定修正快照/取证文件，索引自身不计入），并记录 entry_count=56；所有条目重新逐文件验证字节数与 SHA256，不改原产品执行结论。
