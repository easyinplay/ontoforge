# Phase 07 Plan 02 Execution Ledger

## Scope

## 2026-10-29 验收修复计划

- 本轮以 `837c288` 为修复起点，原实施记录不作验收依据。沿用批准合同，跳过重复战略/Phase/brainstorming；新灰区回传主会话。
- Task 02-1：双 store 真实 SQLite metadata 事务，以 Notify 固定 reader/writer 次序，覆盖 reload、scope apply、revoke；复用同一完整合同到 ignored PG。保留旧 snapshot 与新代际强断言、锁释放验证。已有正确行为仅记 characterization，不制造 RED。
- Task 02-2：先用 admission snapshot 捕获后外部删除 Action 的真实 replay 失败形成 RED，再仅删除 replay 中的二次当前定义读取；旧 wrapper 拒绝、当前 grant/跨主体与原 receipt 继续断言。
- validate-only 分别证明成功、CAS、guard、stale，并对每个分支比较 edit/log/action_commands/outbox 基线。history 组合旧定义、外部 reload、当前 scope 遮蔽和撤权。
- 所有 Cargo 子进程固定 `CARGO_BUILD_JOBS=2`，保留增量 target。执行定向用例、fmt、all-features Clippy、embedded-pg、TLS 和 workspace 单测试线程回归，原始日志保存在 `07-02-evidence/`。
- 共享 STATE/ROADMAP/REQUIREMENTS/task_plan/progress/findings/WINDOWS 由主会话维护；本执行者仅更新本台账、授权代码/测试和最终 SUMMARY。Warp/ralph-loop 不可用，不声称调用。

- Task 1: metadata snapshot 双后端共同锁合同
- Task 2: frozen Action、receipt 与 history 入口
- TDD: 每个 task 先运行真实行为 RED，再最小 GREEN，最后 REFACTOR；已有 GREEN 行为记录 characterization，不制造假失败。
- Allowed production/test files: `crates/ontoforge-storage/src/lib.rs`, `crates/ontoforge-storage/src/request_snapshot.rs`, `crates/ontoforge-storage/tests/request_snapshot.rs`, `crates/ontoforge-action/src/lib.rs`, `crates/ontoforge-storage/src/history.rs`, `crates/ontoforge-action/tests/frozen_definition.rs`, `crates/ontoforge-action/tests/idempotency_pg.rs`, `crates/ontoforge-storage/tests/frozen_history.rs`.

## Acceptance

- Task 1: SQLite race and short-lock tests pass; PG case exists and is ignored only when `ONTOFORGE_PG_URL` is absent.
- Task 2: snapshot-aware submit/validate/replay/result/history APIs pass fence, CAS, guard, current-grant and receipt-time contracts while wrappers remain compatible.
- Full verification: specified cargo tests, fmt, clippy/TLS gates where applicable, then workspace single-threaded regression.

## Command Log

### 验收修复进度

- `replay-red.json` / `replay-red.junit.xml`：1 executed、1 assertion failed，真实外部移除 Action 后 admission snapshot replay 返回 `Err(ScopeDenied)`；GSD `tdd-red-evidence` 判定 `RED_EVIDENCE_OK`。RED commit `930ee87`，生产 GREEN 授权仅修复该二次读取。
- `snapshot-characterization.log`：测试导入路径编译错误，非 RED；改用现有 `request_snapshot::RequestSnapshot` 后重跑。无生产更改。
- snapshot/history/validate-only 新增断言为既有正确行为的 characterization；不把编译问题或通过结果当 RED。
- `action-green`：frozen 3/3、idempotency 9/9；`storage-final`：frozen history 1/1、history masking 12 passed/1 ignored、snapshot 2 passed/1 ignored。已提交 Task 1 `df8b748`、Task 2 GREEN `3c0fb76`。
- all-features Clippy 初轮新增 history 测试过长，拆辅助函数保留断言；次轮仅余已批准三处 `idempotency_pg.rs` large_futures，沿现有 `Box::pin(race(...))` 最小分配调整。该纯测试分配调整跳过新增 TDD，理由已在共享 findings 由主会话记录；PG 仍未执行。
- frozen clock characterization 补固定时钟 2026-10-01/02、外部 reload 改变 default 与新 admission definition；submit replay/result 均断言原 receipt/time、四类业务表基线不变。
- `fbf5b9f` 完成 lint 整理，history 保留独立 ontology clone 比较，全部 masking/revoke/NewGate 断言保留。`action-final` 12 passed；`storage-verified` 15 passed、2 ignored；`workspace-clippy-passed` all-targets/all-features exit 0。中间 Clippy 错误原始日志全部保留，没有作为业务 RED。
- 当前执行 `workspace-tests`（jobs2、test-threads=1），未提前声称最终通过；共享指针与独立审查由主会话维护。
- 独立复签 `16adf6b` 锚定 `fbf5b9f`，原五项审查问题 resolved；主会话 `0709e01` 关闭 WINDOWS#16。真实 PG 仍待 07-13。
- 后续顺序门禁由 `07-02-evidence/finish-gates.ps1` 执行，额外保存 `cargo build --workspace --locked` 的 wave build 证据；不重启现有构建、不并发 Cargo。
- 主会话批准最后一行 history 测试强化：NewGate 的旧 snapshot 与当前 wrapper 均在持新定义的同一 external store 执行，排除误用 store 缓存。现有 workspace 回归对应生产/测试 anchor `fbf5b9f`；结束后 `history-same-store` 实际执行新断言，再跑 fmt/all-features Clippy 等必要门禁。纯测试一行差异由定向闭环，不重复整轮 workspace。
- `workspace-tests`：UTC 04:47:44.884 至 05:00:28.037，exit 0，121 组总计 1279 passed / 0 failed / 107 ignored / 0 filtered；构建 8m31s，总计 12m43s。`workspace-tests-totals.json` 由本轮原始 suite summary 汇总，未挪用 07-03 计数。
- `history-same-store`：1 passed / 0 ignored，最终 test-only commit `366bb4c`；生产仍为 `fbf5b9f`。`workspace-clippy-terminal` 重新覆盖最终测试差异，all-targets/all-features exit 0；`workspace-clippy-default` exit 0。
- 最终顺序门禁：`fmt`、`embedded-pg-clippy` 均 exit 0；`tls-native`、`tls-openssl` 含 embedded-pg 的依赖图查询均以 exit 101 返回 `did not match any packages`，证明对应 TLS 依赖不存在，不作为测试失败。
- `workspace-build`：UTC 05:02:36.617 至 05:03:55.856，`cargo build --workspace --locked` exit 0，耗时 1m18s。所有 Cargo 子进程已结束。
- 独立最终补签 `87f9367` 确认同 store history 对照、全量证据锚点和 test-only 差异边界；Task 02-1、Task 02-2 本地验收均完成。真实 PG/Trino 等服务未运行，107 ignored 不计通过，交 07-13/14。
- Stub/threat 扫描：代码/测试无阻塞 TODO/FIXME/占位实现；`ActionError::ScopeDenied` 的既有英文错误包含 `not available`，不是 stub。生产仅删除 replay 的重复定义查询，未引入新 endpoint、文件访问、auth 或 schema 边界。

### 历史实施记录（保留，不作本轮验收证据）

| Stage | Command | Result |
|---|---|---|
| Init | root/branch/HEAD assertion | `D:/DataSpellProjects/ontoforge`, `gsd/phase-07-rest-mcp-shell`, `46f0058c0ceafcb8ae7ab0e076b555e2e15f2474` |
| Task 1 RED | `cargo test -p ontoforge-storage --test request_snapshot --locked -- --nocapture` | Initial run: 1 passed, 1 failed from missing `homeAirports` test auth attribute, 1 ignored PG; characterization implementation compiled. |
| Task 1 GREEN | `cargo test -p ontoforge-storage --test request_snapshot --locked -- --nocapture` | 2 passed, 1 ignored PG, 0 failed. |
| Task 2 RED | new frozen tests, then specified regressions | Initial compile fixture syntax failure; after correction behavior tests exposed history visibility ordering regression. |
| Task 2 GREEN | `cargo test -p ontoforge-action --test frozen_definition --test idempotency --locked -- --nocapture` | frozen 3 passed; idempotency 9 passed; 0 ignored. |
| Task 2 GREEN | `cargo test -p ontoforge-storage --test frozen_history --test history_masking --locked -- --nocapture` | frozen 1 passed; history masking 12 passed, 1 PG ignored by existing contract. |
| Final | `cargo fmt --all -- --check` | passed |
| Final | `cargo clippy --workspace --all-targets --locked -- -D warnings` | passed |
| Final | `cargo clippy -p ontoforge-storage --features embedded-pg --all-targets --locked -- -D warnings` | passed |
| Final | `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i native-tls` | exit 101 because `native-tls` package ID was absent; TLS gate passed (no native-tls dependency). |
| Final | `cargo test --workspace --locked -- --test-threads=1` | exit 0; all executed tests passed; environment-gated PG/Trino/private tests remained ignored. |

## Notes

- Warp and `ralph-loop` are unavailable in this environment; no completion promise is claimed.
- PG tests must never be reported as passed when ignored due to missing `ONTOFORGE_PG_URL`.

## Deviations

- Rule 1 bug fix: preserved `history` visibility short-circuit before active-definition lookup after the new wrapper refactor; this restored `invisible_check_precedes_store_access`.
