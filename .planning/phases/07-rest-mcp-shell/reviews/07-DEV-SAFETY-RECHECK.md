---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T08:17:48Z
depth: standard
review_base: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
review_target: frozen-dev-commit
review_head: 0a3e3596ab9bed093b2815cbc019128f0323a048
linux_validation: passed_in_ci_merge_of_review_head
source_blobs:
  crates/ontoforge/src/dev.rs: c5586785c4fcc813a7ec4b8fe52583acfb655ffa
  crates/ontoforge/tests/dev_lifecycle.rs: 42a9d82423cec87b6976222bf14424ffb0f685b2
  crates/ontoforge/tests/unit/dev.test.rs: c40d9104d9258a210b2ab612f19ee16d0ced173e
  docs/dev.md: fc9927a57352aad6bc2331679b3ea942baa8edc5
files_reviewed: 6
files_reviewed_list:
  - crates/ontoforge/src/dev.rs
  - crates/ontoforge/tests/dev_lifecycle.rs
  - crates/ontoforge/tests/unit/dev.test.rs
  - docs/dev.md
  - crates/ontoforge/src/serve.rs
  - crates/ontoforge/src/dev_assets.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
original_findings:
  CR-01: resolved
  CR-02: resolved
  CR-03: resolved
phase_acceptance: pending
---

# Phase 7 Dev Safety Recheck

当前固定 blob 的定向源码复签为 clean：前轮 3 项 BLOCKER 的实现缺口已关闭，无新增未解决发现。本次补充核读 Linux CI，两条锁内复核测试及 12 项 dev_lifecycle 均通过，其中含此前待执行的 4 项 Unix 测试。RF-08/RF-11 的定向平台证据已补齐；不表示整个 Phase 7 完成。

读取修复合同、完整目标 diff、相关测试全文；serve.rs 仅检查 DevState 启动/生命周期持有路径，dev_assets.rs 仅检查 write_to。未扩大为整个 Phase 审查。只更新本报告，不修改实现、测试或共享 planning，不提交、不运行 Cargo。

## Narrative Findings (AI reviewer)

无新增未解决 BLOCKER 或 WARNING。以下为原发现的关闭证据，severity 仅标记历史问题。

### CR-01: [BLOCKER, resolved] 锁内重新读取目录状态

**源码：** `crates/ontoforge/src/dev.rs:41-42` 保留只读前检；`:83-90` 在 DevLock::acquire 成功后重新调用 validate_directory，重新读取根权限、marker、初始化状态和 entries；校验失败先于 reset 或资产写入。没有继续使用锁外快照。

**测试：** `tests/unit/dev.test.rs:13` 在前检后插入用户文件，真实 prepare_locked 必须拒绝且保留文件、无 marker/key；`:28` 在 reset 前检后替换 marker，要求拒绝且保留 key 和被替换 marker。使用生产 continuation 确定性复现检查到使用间的状态变化，不将其称为多进程调度测试。

### CR-02: [BLOCKER, resolved] reset 前拒绝不安全已有 key

**源码：** `dev.rs:86-88` 在任何 marker/资产改写前检查两把 key；`:218-233` 遇 group/other 任意权限位非零即报错，没有 chmod 或 key 写入。reset 与普通启动共用检查，reset 无豁免。新 root/key 分别采用 `:117` 的 0700、`:208` 的 0600。

**测试：** `dev_lifecycle.rs:363` 逐一将两把 key 设为 0640，分别执行 reset=false/true，检查非零退出、chmod 600 提示、key 原字节/权限与数据库原字节。代码顺序还证明拒绝先于 marker 写入；测试没有直接断言 marker 字节，不夸大覆盖。此 Unix 用例已在下述 Linux CI 日志中列名通过。

### CR-03: [BLOCKER, resolved] 拒绝 dangling lock symlink

**源码：** `dev.rs:307-314` 使用 symlink_metadata，拒绝所有 symlink，仅真正 NotFound 可以继续；悬空链接不会进入 create(true)。`:300-326` 仍持有 OS 锁文件，不通过 PID 判定、不 truncate、不 unlink；DevState 保留至宿主退出。

**测试：** `dev_lifecycle.rs:408` 创建指向根外不存在文件的 lock symlink，要求启动失败、外部目标未创建、marker/auth key 未创建、原 symlink 保留。此 Unix 用例已在下述 Linux CI 日志中列名通过。

## Dynamic Evidence And Limits

以下均独立读取 owner 日志，执行归属 owner；本 reviewer 未复跑：

| 证据文件 | 实际核读结果 |
| --- | --- |
| `target/phase7-dev-review-lock-recheck.log` | 2 passed，0 failed；两个锁内复核测试均列名通过，0.03s |
| `target/phase7-dev-review-fixes-tests.log` | Windows dev_lifecycle 8 passed，0 failed；锁生命周期/崩溃释放/空锁拒绝/无 owner 空锁恢复及原功能回归，4.82s |
| `target/phase7-dev-review-clippy.log` | 最新日志包含 Finished dev profile，25.36s，无诊断错误；进程退出状态由 owner 核验 |
| `target/phase7-dev-review-evidence.md` | owner 保存的 RED 摘录，包含原 OS 锁失败、空锁第二 owner 与新增两个锁内复核 0/2；不是 reviewer 亲自运行的原始 RED |

3 项 Unix 权限测试和 1 项 dangling symlink 测试现已有 Linux 实际执行证据，见下节。没有关于同权限恶意进程持续替换路径或父目录全树安全的新增保证。

## Review History

初轮快照 `dev=73fc7429`、`dev_lifecycle=b836b820` 于 2026-11-03T07:40:57Z 留下上述 3 项 BLOCKER。本次使用 frontmatter 的新 blob 重新检查实际源码与断言后，将剩余发现数更新为 0；当时保留 Unix 验收 pending；下列后续 Linux 证据补齐该门禁，未用静态关闭替代动态执行。

## Linux Evidence Update

独立核读 `target/phase7-dev-linux-ci-41600147824.log`。日志标识 Ubuntu 26.04.1 与 `test (ubuntu-latest)`，实际 checkout 为 synthetic merge `dcaf6dd`（将 `0a3e3596ab9bed093b2815cbc019128f0323a048` 合入 `2c9eee6461a412ead8f998c7fa038acaa458b2f9`），不是直接 checkout PR head。run `41600147824` / job `125796698650` 的 success 状态由主会话报告，本 reviewer 独立验证的是保存的作业日志。

- 日志 :512-513 两条 `dev::tests::lock_acquisition_*` 均 `ok`，所属 lib suite 为 21 passed、0 failed。
- :777-793 的 `dev_lifecycle` 为 **12 passed、0 failed、0 ignored**，8.24s。新目录/两把 key 私有权限、已有 exposed key（含 reset）保留字节/权限、已有 public root 拒绝、dangling lock 不创建外部目标四项 Unix 测试均列名 `ok`。
- 已读取 `git show 0a3e359 -- crates/ontoforge/tests/unit/dev.test.rs`：仅新增 5 行 `cfg(unix)` fixture 0700 设置，使锁内复核单元使用满足其前提的 private root，不绕过实际权限检查。生产 dev.rs、dev_lifecycle.rs、docs/dev.md 的提交 blob 与既有复签一致；unit blob 更新为 frontmatter 所列值。

本 reviewer 未重新运行测试、拉取或修改实现。Linux 证据只用于关闭本报告明确列出的平台待验收项，不用该作业的其他绿色输出扩张本轮审查范围。
