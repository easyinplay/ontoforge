# TLS dependency guard 修复

## 目标与边界

- 修复 CI 原有两行 `! cargo tree ...` 的 fail-open；当前依赖树未发现禁包，不将门禁缺陷描述为依赖违规。
- 按主会话已定方案：单次完整依赖图、精确 package 匹配、Cargo 错误 fail-closed。
- 独占 `ci/check-tls-deps.sh`、`ci/test_tls_deps.py` 和 CI TLS step。保留其他 worker 的修改，不运行真实 Cargo。
- 复用 audit 的 task_plan/findings/progress；本文件是 worker 的唯一状态与证据记录。

## 验收计划

1. 先用 fake Cargo 执行真实 workflow TLS shell，复现 openssl-only 与 Cargo failure 漏报。
2. 添加最小 helper 与两 OS 测试入口。
3. 验证 neither、openssl-only、native-tls-only、both、Cargo failure、相似包名；核对单次调用及完整图参数。
4. 执行 Bash syntax / whitespace 检查，交还文件与 blob 给主会话执行真实依赖图和 CI。

## 状态

已读取 `superpowers:test-driven-development`、测试规范与 `planning-with-files`，恢复 audit 工作计划并通过 resolver 选中 `v1.0-audit`。已读取 CC AGREE 和原始 negative probe。根因、方案与范围明确，跳过重复 brainstorming。Warp/ralph-loop 不可用，使用明确验收闭环且不声称调用。

## RED 证据

2026-11-04，先创建 `ci/test_tls_deps.py`，执行原始 workflow TLS step（未改生产脚本），用导出的 Bash function 在 Cargo 边界提供固定图/退出码；未执行真实 Cargo。测试选中 `C:/Program Files/Git/bin/bash.exe`，避开 System32 WSL launcher。

命令：`python -B -m unittest discover -s ci -p test_tls_deps.py -v`

结果：`Ran 6 tests in 0.234s`、`FAILED (failures=6)`。其中 openssl-only 与 Cargo failure 明确出现 `AssertionError: True != False`，即实际门禁 exit 0，预期非零。其余四项退出码符合旧逻辑，但不满足单次完整图调用契约。与已有 `target/tls-guard-negative-probe.log` 的 `guard_exit=0` 一致。

## GREEN 与交付

2026-11-04，新增 12 行 `ci/check-tls-deps.sh`。`set -euo pipefail` 保证完整图查询错误中止；逐行取第一个 package 字段，精确拒绝 `openssl` 和 `native-tls`。CI 的 test job 两 OS matrix 均运行 hermetic regression 和真实 helper。

- `python -B -m unittest discover -s ci -p test_tls_deps.py -v`：`Ran 6 tests in 0.390s`、`OK`。覆盖 six scenarios，执行 workflow 中的实际 command；断言一次 Cargo 调用、workspace/locked/embedded-pg/normal,build,dev/无 prefix 的完整图参数。
- `C:/Program Files/Git/bin/bash.exe -n ci/check-tls-deps.sh`：exit 0。
- `git diff --check -- .github/workflows/ci.yml ci/check-tls-deps.sh ci/test_tls_deps.py .planning/milestones/v1.0-audit/TLS-GUARD-FIX.md`：exit 0。
- 本机未发现 `shfmt` 或 `fmtbash` executable；未声称运行格式化工具。无 Rust 变更且 Cargo 仍由另一 worker 独占，未运行 `cargo fmt`。
- 核对 CI diff，保留主会话新增的 `SQLite Action fault injection` 和 `PostgreSQL absent-target source guard`。

实现与 hermetic 验收完成，未提交。未运行真实 Cargo、Ubuntu 或远端 CI；这些验证由主会话负责。本地 PowerShell profile 输出的 `[ERROR] - (starship::print): Under a 'dumb' terminal (TERM=dumb).` 来自 shell 初始化，测试本身通过。

交付 blob：

| 文件 | git hash-object |
|---|---|
| `ci/check-tls-deps.sh` | `dd70463dd72b164e2a66a520f7ffd673cff56362` |
| `ci/test_tls_deps.py` | `6019ab3bf77e8a90b3a166e9b34624ed284e3e46` |
| `.github/workflows/ci.yml` | `5a0ff8327a0305b0972d9652cce5672330b96656` |

下一步：主会话收回文件 ownership，在 Cargo owner 释放后执行真实 helper 和最终 CI，并把本文件证据蒸馏到 audit 的共享状态。
