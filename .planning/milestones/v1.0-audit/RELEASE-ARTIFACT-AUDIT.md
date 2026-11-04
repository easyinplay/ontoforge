---
status: passed
source_commit: f8851ca073fed34e80e605f0ac91880259df7e12
packaged_commit: 22913c8e79ae98b6604109d6e05a15f27cd50292
release_run: 41759804768
updated: 2026-11-04
---

# 最终源码 release artifact 独立审计

三个指定 native artifact 的 archive SHA256、checksum、包内 binary SHA256、内外 manifest 均通过独立核验。实际打包 PR merge commit 与 source commit 不同，但完整 Git tree 相同。三平台 CI 安装 smoke 与 aggregate 均通过，publish skipped。

## 验收范围与状态

- [x] 使用 `gh run download` 下载 run `41759804768` 的三个指定 artifact，保留 API 元数据与完整日志。
- [x] 重算每个发行 archive 的 SHA256、字节数，核对同包 `.sha256` 的 hash 与文件名。
- [x] 直接读取 archive 中的 binary 并重算 SHA256，核对外部 manifest 的 `binary_sha256`；包内 `manifest.json` 与外部 manifest 对象完全一致。
- [x] 核对 target 集合、version `0.1.0`、实际打包 commit 与 source 完整 Git tree。
- [x] 核对本 run 三平台安装日志及 aggregate / publish 状态，记录证据和限制。

执行计划、发现及结果集中记录于本文件，按 `planning-with-files` 单一 owner 规则未修改共享 task_plan / progress / findings / STATE / ROADMAP。审计采用 `07-14-SUMMARY.md` 的 source / checkout 分离证据模式；历史 run 不作为本轮结果。

## 源码身份

| 身份 | SHA |
|---|---|
| release run source / headSha | `f8851ca073fed34e80e605f0ac91880259df7e12` |
| 三个 manifest 的实际 packaged PR merge commit | `22913c8e79ae98b6604109d6e05a15f27cd50292` |
| source 的完整根 tree | `0ecc59382aaf12e782c508ba64858e2689299fd7` |
| packaged commit 的完整根 tree | `0ecc59382aaf12e782c508ba64858e2689299fd7` |
| packaged commit 第一 parent | `2c9eee6461a412ead8f998c7fa038acaa458b2f9` |
| packaged commit 第二 parent | `f8851ca073fed34e80e605f0ac91880259df7e12` |

GitHub Git commit API 的两个根 tree SHA 相同，且 source tree 通过本地 `git rev-parse 'f8851ca073fed34e80e605f0ac91880259df7e12^{tree}'` 复核。该比较覆盖完整已跟踪 Git tree，并非仅生产代码过滤 diff。source 是 packaged commit 的直接 parent。CI checkout 日志明确使用 `refs/remotes/pull/8/merge`，不能把 PR merge SHA 写成 source SHA，也不代表 PR 已合并。

本地缺少 packaged commit 对象，因此首次 `git rev-parse` / `git show` / `git diff` 报 unknown revision / bad object；随后使用只读 API 取得并保存 commit 对象完成核验，未 fetch 或改动共享 `.git`。没有声称本地 `git diff` 成功。

## 独立 archive 校验

以下为发行 archive 本身的 SHA256，不是 GitHub Actions artifact 外层 ZIP digest。

| Target | Archive | Bytes | SHA256 |
|---|---|---:|---|
| aarch64-apple-darwin | `ontoforge-0.1.0-aarch64-apple-darwin.tar.gz` | 18383929 | `a285ab0c33436076298b694b3da24f1e0dd1e85f504aae972333c44b77e831f6` |
| x86_64-pc-windows-msvc | `ontoforge-0.1.0-x86_64-pc-windows-msvc.zip` | 18807106 | `8618e7fd604b49edebdc2e5793f0db9714d15fce9de10e9ad3cb045029daf93c` |
| x86_64-unknown-linux-gnu | `ontoforge-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | 20513628 | `a9b5a22feb40580ccf273ef55e52da8b85bb662284fd5f0b3a16db007a77f59e` |

三个 archive 各包含 binary 与 `manifest.json` 两个文件；Windows binary 为 `ontoforge.exe`，其余为 `ontoforge`。逐文件 binary hash 及完整 manifest 保存在 `verification.json`，三项 `checksum_matches`、`binary_manifest_matches`、`embedded_manifest_matches` 均为 true。

## 本 run CI 动态证据

重新读取的 run 为 `pull_request`、`completed/success`，最后更新 `2026-11-04T02:26:39Z`。三个 native job 的 build/package、安装 smoke、upload 均 success；`verify all release packages` success；`upload assets to the published release` skipped。

| Target / manifest 环境 | 完整 run.log 行号 | 最终记录 |
|---|---|---|
| aarch64-apple-darwin / macOS 15.8.2 Arm64 | 486–495 | `SMOKE PASSED`，`2026-11-04T02:23:45.0017560Z` |
| x86_64-pc-windows-msvc / Microsoft Windows 10.0.26100 X64 | 1037–1046 | `SMOKE PASSED`，`2026-11-04T02:26:09.1940820Z` |
| x86_64-unknown-linux-gnu / Ubuntu 24.04.5 LTS X64 | 1599–1608 | `SMOKE PASSED`，`2026-11-04T02:22:14.9958483Z` |

三份日志均逐项记录隔离 cwd 的 installed version/help、query、Action/query/history/command receipt、graceful shutdown 和端口关闭、restart 保留、reset 旧 token 拒绝及旧 command 404、stdio initialize/query 与 EOF exit 0。aggregate 日志 1839–1841 再次确认三 archive 的同一实际 checkout commit 和对应 checksum。Linux manifest 的 glibc 为 `ldd (Ubuntu GLIBC 2.39-0ubuntu8.9) 2.39`；不推断 universal 或全静态支持。

## 证据文件

所有原始下载与机器证据位于 `target/milestone-release-41759804768/`：

- `release-{target}-41759804768-1/`：三个发行 archive、manifest、checksum，API artifact ID 分别为 Linux `13290545413`、macOS `13290252475`、Windows `13289054601`。
- `artifacts-api.json`：精确三个 artifact 的身份、run source、外层 ZIP digest、有效期，查询时均 `expired=false`。
- `run.json`、`run.log`：本次重新取得的 run metadata 与完整日志。
- `source-commit.json`、`packaged-commit.json`：GitHub Git commit API 原始响应。
- `verify-artifacts.py`、`verification.json`、`verification.log`：可复执行的只读 archive 核验程序、结构化结果与实际 exit 0 输出。

本地执行命令：`python target/milestone-release-41759804768/verify-artifacts.py`，实际退出码 0，`status=passed`，三 archive 全部通过。

## 限制与执行边界

本次本地仅下载并读取 archive 字节，未运行包内程序、Cargo 或 Docker。动态行为结论来自上述精确 run 的三平台原始 CI 日志；不冒称本机重跑。`gh run download` 解开了 Actions 外层 ZIP，本次未独立重算外层 ZIP digest；已核验的是实际发行 archive 与包内 binary。

TDD 跳过：本任务是既有产物审计，没有生产逻辑修改，验收使用真实下载、SHA256 和 Git tree 对账。`ralph-loop` 与 Warp 未提供，采用明确验收闭环，不声称调用。指定 `.Codex/rules/web-search.md` 缺失，已定位并读取 `.claude/rules/web-search.md`，GitHub 查询遵循只读 `gh` 路由。

未发布 Release、上传 Release assets、发布 npm、merge PR 或发送外部消息；没有 commit。本结论仅完成本次 release artifact 审计，不替代 milestone 需求或服务验收。
