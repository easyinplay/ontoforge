# 07-02 Acceptance Audit

日期：2026-10-29。独立 `gsd-code-reviewer` 只读审查，未运行重复 workspace 测试。初审结论：验收未通过，原完成结论撤销，保留实现记录。原发现保留如下；修复复验见后续章节。

| 级别 | 问题 | 最小修复与验收 |
|------|------|----------------|
| BLOCKER | `action/src/lib.rs:314` replay 命中后重读 current_action_definition，绕过传入 snapshot | replay 使用传入 active；snapshot 捕获后外部移除 Action，再 submit replay 必须返回原 receipt。旧 wrapper 当前定义拒绝仍成立 |
| WARNING | `storage/tests/request_snapshot.rs:36` 仅顺序 seed 后单读，无竞态和代际切换 | 双 store 共同锁/barrier，覆盖 reload/scope_apply/revoke 交错、旧 snapshot 不变、新 snapshot 见完整提交代际 |
| WARNING | PG case 吞掉 request_snapshot 错误，没有断言 | 复用完整 metadata race 合同，成功值/代际断言；保留 ignored，07-13 真实执行 |
| WARNING | frozen fence/CAS/guard 测试只触发 stale，expected_versions 为空，零写只数 log | 分别触发 validate success、CAS/guard/stale 拒绝，每次比较 edit/log/command/outbox 基线 |
| WARNING | frozen history 没有定义切换或遮蔽/撤权，仅看对象 version | 旧 snapshot + 外部 reload，当前 scope 隐藏值与不可见拒绝；不要把对象 version 当 definition version |

补充：现有原 idempotency 测试确实覆盖变 clock/定义后 receipt、授权撤销/Action 移除/跨主体与最小 receipt；原 history_masking 覆盖 keyset/整记录页。缺口是这些行为与 admission snapshot/外部变更的组合。返回后锁释放用例有有效断言，未报告为缺陷。已有正确行为应记录 characterization，不制造假 RED。

## 修复复验（2026-10-29）

生产修复基线：`3c0fb76b385bc2db1dee48e0e425749109ab0540`；前置测试提交为 `930ee87`、`df8b748`。生产/全量回归 anchor：`fbf5b9fe3e0f4211ed378c6f5b68b2f4daabe02e`；最终代码/测试复签 anchor：`366bb4c5162762c697bbf7c61d649020e4448475`。本轮只读取源码、git diff 和既有原始日志，没有运行 Cargo，没有修改生产代码或测试；仅更新并按授权提交本报告。五项原发现对最终 anchor 均为 resolved；下表行号保留生产修复基线的证据定位。真实 PostgreSQL 执行和其他 finish gates 仍须各自证据，不能由本报告替代。

| 原级别 | 状态 | 独立复验事实与源码指针 |
|--------|------|--------------------------|
| BLOCKER | resolved | `ontoforge-action/src/lib.rs:299` 先检查传入 active 的 Action；`:314` replay 仅检查原 action/hash 并返回原 receipt，不再读取当前定义。`tests/frozen_definition.rs:314` 捕获真实 request snapshot，经第二 store 移除 Action，`:321` submit replay 和 `:334` result lookup 均断言原 receipt；`:373` 撤权拒绝，`:384`、`:395` 旧 execute/result wrapper 仍按当前定义拒绝。生产修复位于 `3c0fb76`。 |
| WARNING | resolved | `ontoforge-storage/tests/request_snapshot.rs:38` 使用同一真实文件的双 store；`:60` 用第三连接持有 SQLite BEGIN IMMEDIATE 或同 schema 的 PG advisory lock；`:101` 同时 poll 真 snapshot/writer，持锁期间任一完成即失败，解锁后两个 Result 均 expect 成功。`:163` reload、`:192` scope_apply、`:213` revoke 各自交错，期间只接受完整旧/新代际，提交后断言新完整值；`:224` 保留旧 active/scope，`:230` 保留撤权前 scope。修复位于 `df8b748`。 |
| WARNING | resolved（PG 未运行） | `request_snapshot.rs:261` 的 PG 用例明确要求 ONTOFORGE_PG_URL，建立同 schema 第二 store，并调用与 SQLite 相同的完整 metadata_contract；`:123`、`:124` 不吞 writer/snapshot 错误。仍保留 ignored，真实 PG 执行属于 07-13，不能将本次编译或 ignored 计为通过。 |
| WARNING | resolved | `frozen_definition.rs:135` 读取实际对象 code/name/of_version，并统计 edit log、log entry、command、outbox。`:155` 签名真实版本 token，`:171` validate 成功；`:184` 使 Original 可见而 New 不可见，触发 post-state guard；`:211` 第二 store 修改对象后用原 token 触发 CAS；`:257` 外部 reload 后分别触发 submit/validate stale。每个分支均立即比较自身业务基线，没有用 stale 替代 CAS/guard。guard 路径对应 `criteria.rs:277`，版本 token 路径对应 `version.rs:49`。 |
| WARNING | resolved | `3c0fb76:ontoforge-storage/tests/frozen_history.rs:57` 捕获旧 snapshot 后由第二 store reload，并断言 definition 版本确实变化。`:128` 当前 scope 隐藏值、保留 masked_changes 且序列化无值；`:133` 无 grant、`:142` 行不可见、`:148` 无当前对象均为 NotFound。`:155` 新增类型在旧 active 下拒绝，而当前 wrapper 返回其实际历史，能识别误用当前定义；未再把对象 version 当 definition version。 |

### 原始运行证据

目录：`../07-02-evidence/`。

- `replay-red.log` / `replay-red.json` / `replay-red.junit.xml`：一个指定 replay 用例真实执行失败，原文为 `admission snapshot replay must survive external Action removal: Err(ScopeDenied)`，exitCode 100。属于行为 RED；不是编译失败。
- `action-green.log` / `action-green.json`：2026-10-29T04:41:18Z 结束，exitCode 0；frozen_definition 3 passed、idempotency 9 passed，均 0 ignored。包括同一 replay 用例及旧 wrapper 拒绝回归。
- `snapshot-characterization.log`：错误 import 导致编译失败，只是修正过程，不计行为 RED。`snapshot-characterization-fixed.log` / JSON 为 exitCode 0，SQLite 两用例通过、PG 一个 ignored。
- `storage-final.log` / `storage-final.json`：2026-10-29T04:42:01Z 结束，exitCode 0；frozen_history 1 passed、history_masking 12 passed/1 ignored、request_snapshot 2 passed/1 ignored。PG 的两个 ignored 均没有获得真实后端验收。

### GREEN 后的变更与待验边界

复验期间 HEAD 已出现文档提交 `47f299b`，不改变上述生产行为。工作树另有 executor 的四个测试文件整理，已逐个比较 `git diff 3c0fb76 -- crates`，没有生产业务改动：request_snapshot/idempotency_pg 的 `Box::pin(...).await` 仅改变 future 分配位置，保留参数、等待顺序、返回值和所有断言；frozen_history 拆分 seed/publish helpers 后保留执行顺序和关键 NewGate 区分断言。中途发现 publish 将 ontology 借用后同值自比，已通知主会话，executor 已改回独立 `active.ontology.clone()`，本轮重新读取 diff 确认修复。

frozen_definition 另将跨主体断言移入 helper，并加强 clock/definition 用例：固定 2026-10-01 原时间、2026-10-02 重放时间，外部 store 实际修改 Action default、断言 definition 版本改变，同时 submit replay/result 保持完整原 receipt、业务基线不变。属于测试强化，没有改生产行为。

最新 `action-final.log` / JSON 于 2026-10-29T04:46:42Z 结束，exitCode 0；frozen_definition 3 passed、idempotency 9 passed，均 0 ignored，覆盖上述强化后的 Action 测试。`workspace-clippy-passed.log` / JSON 于 2026-10-29T04:46:13Z 结束，命令为 `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`，exitCode 0。此前 initial/second/final/verified 的失败文件保留为过程证据，不当作最终通过。

### fbf5b9f 指纹补签

最终提交 `fbf5b9fe3e0f4211ed378c6f5b68b2f4daabe02e` 的四个测试文件与本轮已审查的 helper/Box::pin/clock 整理一致。补签时 `git diff fbf5b9fe3e0f4211ed378c6f5b68b2f4daabe02e -- crates/ontoforge-action/src/lib.rs crates/ontoforge-action/tests/frozen_definition.rs crates/ontoforge-action/tests/idempotency_pg.rs crates/ontoforge-storage/tests/frozen_history.rs crates/ontoforge-storage/tests/request_snapshot.rs` 为空。history 独立 ontology clone、NewGate 对照、四类 validate 分支和旧 wrapper 拒绝断言均保留；helper/Box::pin 无断言退化，clock/definition 用例进一步加强。

整理后 `storage-verified.log` / `storage-verified.json` 于 2026-10-29T04:47:28Z 结束，exitCode 0；frozen_history 1 passed/0 ignored、history_masking 12 passed/1 ignored、request_snapshot 2 passed/1 ignored，总计 15 passed/2 ignored。与 action-final 的 12 passed/0 ignored、workspace-clippy-passed 的 all-features exitCode 0 一并构成最终 anchor 的定向证据。

本次补签时 workspace 回归仍由 executor 独占运行，当时标为 pending；后续完整结果见下一节。真实 PG 仍未运行。若最终 anchor 后出现生产业务改动，需对新基线复签；本报告不预签尚未出现的 diff 或日志。

### 366bb4c 最终补签与覆盖边界

最终代码/测试 anchor：`366bb4c5162762c697bbf7c61d649020e4448475`；生产/全量回归 anchor：`fbf5b9fe3e0f4211ed378c6f5b68b2f4daabe02e`。已读取完整 commit patch，并确认两个 anchor 的 crates diff 仅为 `ontoforge-storage/tests/frozen_history.rs:180` 一行 `db.store.history_with_snapshot` 改为 `external.history_with_snapshot`。该行现在与后续 `external.history` 共用同一 store：同一当前缓存下旧 snapshot 拒绝 NewGate、当前 wrapper 成功，排除了旧 store 缓存导致假通过；所有断言和生产代码均未改变。补签时最终 anchor 与工作树 crates diff 为空。五项原发现继续全部 resolved，helper/Box::pin 检查结论保持。

新增 `history-same-store.log` / `history-same-store.json` 于 2026-10-29T05:01:43Z 结束，命令为 `cargo test -p ontoforge-storage --test frozen_history --locked -- --nocapture`，指定 history 用例 1 passed/0 failed/0 ignored，exitCode 0，覆盖最终一行测试强化。

此前 `workspace-tests.log` / `workspace-tests.json` 对生产/全量回归 anchor 执行 `cargo test --workspace --locked -- --test-threads=1`，于 2026-10-29T05:00:28Z 结束，exitCode 0。本轮直接读取原始日志并汇总 121 条 `test result`，得到 1279 passed/0 failed/107 ignored，与 `workspace-tests-totals.json` 一致。该全量结果发生在最终测试强化之前；不声称对 366bb4c 再跑了全量。生产代码没有变化，最终 test-only 差异由上述新定向运行覆盖。

107 ignored 不计通过，真实 PG 等外部后端验收仍留给对应阶段。其他 finish gates 仍由 executor 独占进行中，本报告不宣称全部收尾门禁完成。本次补签未运行 Cargo，仅提交本审查报告。
