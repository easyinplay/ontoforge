---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T13:49:12Z
depth: deep
files_reviewed: 1
files_reviewed_list:
  - ci/release-package.ps1
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
source_sha256: 215A9BBBF2DC07F28195C188E14C3A020D822A34324556716813D5074C8E05AD
source_git_blob: 213d2ac1520bb675f4da751ac327687bcf742c1f
head_at_review: e0f534d135f2017f7de811903ec13460a30f270e
validation: static_review_with_windows_artifact_and_owner_execution_logs
---

# 07-14 Package Review

## Narrative Findings (AI reviewer)

当前绑定源码无未关闭 finding。已复核 Windows 实物 hash 与 owner 的 package/smoke 成功日志；`clean` 不表示 Linux/macOS 或三平台聚合验收通过。

## 已关闭 finding

### CR-01: BLOCKER（已修复并关闭）：构建参数可以把实际二进制错误标记为另一 commit

**File:** `D:/DataSpellProjects/ontoforge/ci/release-package.ps1:32-33`, `:54`

**原问题:** 构建模式直接把任意格式合法的 `ExpectedCommit` 写进 manifest，未与实际 checkout HEAD 对比，后续又与同一个输入值自比。

**修复复核:** 构建模式第 32 行无条件独立读取 `actualCommit`，第 33 行在创建 OutDir（第 40 行）和任何 Cargo 调用（第 43-44 行）前拒绝不匹配的 `ExpectedCommit`；manifest 第 54 行写入 `actualCommit`。该断言被 `!$VerifyOnly` 分支保护，不会把显式预期 commit 的跨平台下载验证绑定到验证机 HEAD。新增第 81-82 行也将 manifest architecture 与目标架构匹配。

**拒绝证据:** 已读取 owner 补全的 `target/phase7-release-commit-rejection.log`，日志显示 `release-package.ps1:33` 抛出原文 `Expected commit differs from checked-out source`，并包含全 0 SHA 的完整 `COMMAND` 与 `OBSERVED: exit=1; output directory absent; Cargo not started`。静态执行顺序与错误日志共同支持关闭原缺陷。未独立重跑此 probe。

**Confidence:** 高。此次仅复核增量及已有拒绝日志，未运行正常 package 或 Cargo。

## Review evidence and boundaries

- 已逐行读取完整 `ci/release-package.ps1`，并追踪 `ci/release-smoke.ps1:12-17,146-154`、workspace/package version 来源、07-14 PLAN 和 PREFLIGHT，以及 gstack `review/checklist.md` 的 Distribution/Data Safety 项。
- 最终 package 源码 SHA256 与 Git blob 为 frontmatter 所列值；初审基线 SHA256 为 `C6A01DEFC1AB17D79B57AE5FA4F52EEF2BCB3644053E503B425B065140A93566`。初审关联 smoke 源码 SHA256：`3002794B300B6C73F72C03CE12DF1FBC70C783E76ED838E64D931B0A632B6DE2`。此次只增量复核 package，不替其他 lane 对 smoke 最终版本签字。
- 静态追踪覆盖 archive 文件集合、ZIP/tar 成员名与普通文件限制、内外 manifest hash、binary hash、target/native OS/architecture/rust host、输出目录不覆盖、临时目录 GUID 与精确清理路径。
- package 解包后的步骤只比较 hash，不执行解包二进制。实际解包 `--version`、`--help` 位于 `release-smoke.ps1:149-153`，通过 `ProcessStartInfo` 在新临时目录运行；该调用链验证启动失败、退出码和版本。跨平台 `VerifyOnly` 无法运行其他平台二进制，不将此设计误报为缺陷。
- 最终增量第 83-84 行要求非空 `os`、`rustc`，Linux target 额外要求非空 `glibc`；结合 StrictMode 的缺字段失败和内外 manifest hash 一致性，未发现新增缺陷。
- 已读取 `target/phase7-release-package-final.log` 和 `target/phase7-release-smoke.log`，两者均有 `EXIT: 0`，对应同一 archive SHA256 `ec09bec6292b9d456dbba2b745849c34b0c5cb14f092f9f8e6e1bdab25950f9c`。smoke 日志包含实际临时安装 cwd 的 version/help、query、Action/history/receipt、正常停止、重启保留、reset、stdio 及最后 `SMOKE PASSED`。这些是 owner 真实执行的证据，本 lane 负责复核。
- 已直接读取 `target/release-dist-final` 的三文件集合、外部 manifest/checksum，并独立使用 `Get-FileHash` 计算实际 ZIP，得到上述同一 SHA256。manifest 版本 `0.1.0`、commit `e0f534d135f2017f7de811903ec13460a30f270e`、target `x86_64-pc-windows-msvc`、architecture `X64`，binary SHA256 `62cdb3e781854d000f035ea75ae2084812fe7e8f3dfb9b0b42f9d93a09912a1c`。
- 已读取 `target/phase7-release-native-rejection.log`（错误 macOS target，exit 1、OutDir absent、Cargo 未启动），以及 `target/phase7-release-reject-{checksum,commit,target,version,extra-file}.log` 全部五份日志。各日志含匹配的失败原因与 `CASE`/`EXIT: 1`：checksum mismatch、manifest identity mismatch、或额外文件集合拒绝。
- 本 lane 未运行 Cargo、package、smoke 或实际 archive 解包。Windows 动态证据来自上述 owner 日志；Linux/macOS 实际运行、Unix executable bit 与远程 native matrix 仍 pending，不能从 Windows 结果推定通过。
- 未修改实现，未 commit。专用 Read/Write 工具未暴露，按 PLAN 中批准的环境适配使用 PowerShell 读取、`apply_patch` 写入本报告。

_Reviewer: gsd-code-reviewer; static package lane_
