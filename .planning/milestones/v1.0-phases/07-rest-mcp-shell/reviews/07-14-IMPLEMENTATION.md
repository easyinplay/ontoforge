# 07-14 实施记录

状态：2026-11-04，最终源码 adc0227 的三平台 native 与 aggregate 已通过，publish skipped，三个 archive 独立 hash 校验通过。最终版本、tree、产物及动态门真相源为 [07-14-SUMMARY.md](../07-14-SUMMARY.md)；本文件保留实施历史，不用旧产物替代最终验收。最终CI `41669982375` 四必需job全部success；UAT已complete、13/13（975f78e），Phase目标判定交独立VERIFICATION归并。

## 范围与验证选择

- 仅修改 `.github/workflows/release.yml`、`ci/release-package.ps1`、`ci/release-smoke.ps1`、`docs/installation.md`。
- 已读取 GSD 与 planning-with-files 技能及现有 phase 工作记录。主会话维护共享 STATE、ROADMAP、findings 与最终 SUMMARY；本文件记录该执行 lane 的证据。
- 14-1、14-2 按 PLAN 跳过 TDD：属于配置、打包及文档接线，不改业务逻辑；以实际 release 构建、archive 自验、无 checkout cwd 的安装 smoke 验收。
- brainstorming 跳过：已有批准 PLAN、PREFLIGHT 和既有 helper/API 模式，无新架构选择。
- Warp、ralph-loop 工具未暴露；实际命令使用 PowerShell，编辑使用 apply_patch，不声称执行不可用工具。
- 运行时包仅包含 binary、manifest；archive 的 SHA256 和相同 manifest 作为旁置资产。CI helper 不进入包，Python 仅用于 Windows CI 验收。
- 不创建 Release、不上传 Release assets、不发布 npm、不合并 PR。

## 最终实现指纹

| 文件 | Git blob |
| --- | --- |
| `.github/workflows/release.yml` | `4bb987019fb6f92a264102fa1c343d194c25c45c` |
| `ci/release-package.ps1` | `213d2ac1520bb675f4da751ac327687bcf742c1f` |
| `ci/release-smoke.ps1` | `bb49d0d49e61017ad0cb33428fcdbf4c3e22d1c4` |
| `docs/installation.md` | `cf17144a4e802017a3592b074f62943eeef705ab` |

四文件之外未修改业务生产源码或既有 helper。验证时 `git diff --name-only -- crates Cargo.toml Cargo.lock` 为空。共享 planning 与另外两个 review 报告由主会话及审查 lane 维护。

## 14-1：真实构建与包验证

本机环境：Windows `Microsoft Windows 10.0.26300`，`X64`；`rustc 1.98.0 (88d9e12ae 2026-08-18)`；源码 commit `e0f534d135f2017f7de811903ec13460a30f270e`。

```powershell
$env:CARGO_BUILD_JOBS='2'
pwsh -NoProfile -File ci/release-package.ps1 -Target x86_64-pc-windows-msvc -OutDir target/release-dist
pwsh -NoProfile -File ci/release-package.ps1 -Target x86_64-pc-windows-msvc -OutDir target/release-dist-final
```

两次均 exit 0。首次 release 编译 15m 28s，最终脚本复跑使用缓存，Cargo 0.53s。最终目录是新的空目录，未覆盖第一次产物。日志：`target/phase7-release-package.log`、`target/phase7-release-package-final.log`。

最终资产：

| 资产 | 大小 / SHA256 |
| --- | --- |
| `target/release-dist-final/ontoforge-0.1.0-x86_64-pc-windows-msvc.zip` | 18,543,434 bytes；`ec09bec6292b9d456dbba2b745849c34b0c5cb14f092f9f8e6e1bdab25950f9c` |
| `target/release-dist-final/ontoforge-0.1.0-x86_64-pc-windows-msvc.manifest.json` | 444 bytes |
| `target/release-dist-final/ontoforge-0.1.0-x86_64-pc-windows-msvc.zip.sha256` | 110 bytes |
| 包内 `ontoforge.exe` | 51,854,336 bytes；`62cdb3e781854d000f035ea75ae2084812fe7e8f3dfb9b0b42f9d93a09912a1c` |

manifest：`schema_version=1`、`version=0.1.0`、`target=x86_64-pc-windows-msvc`、`commit=e0f534d135f2017f7de811903ec13460a30f270e`。archive 重新打开后严格检查仅 binary 与 manifest 两个成员、binary hash 与内外 manifest 一致。发行目录严格检查 archive、manifest、checksum 三个文件。

## 14-2：真实安装 smoke

```powershell
pwsh -NoProfile -File ci/release-smoke.ps1 -ArtifactDir target/release-dist-final -ExpectedTarget x86_64-pc-windows-msvc
```

exit 0，完整日志：`target/phase7-release-smoke.log`。

实际解包 cwd：`C:\Users\easyi\AppData\Local\Temp\ontoforge-install-f8283baabc4340babb52ab9c91e31efa`。以下均由该目录中的 binary 执行，没有 Cargo.toml 或 .git：

1. `--version` 精确匹配 `ontoforge 0.1.0`，`--help` 包含 serve。
2. `serve --dev` 启动，查询 F-1001 得到 LH100、scheduled/0。
3. `delayFlight` Action 后查询得到 DELAYED/15，receipt 的 commandId、edits、Flight 类型存在。
4. history 的 action、eventId、committedAt 与 receipt 一致；command endpoint 返回完整相同 receipt。
5. helper stop 与 owner exit 0、durable state `exited/0`、端口关闭；重启后旧 token 仍可用，query/history/receipt 全部保留。
6. 第二次正常关闭后显式 reset，旧 token HTTP 401；新 token 查询 scheduled/0，history 空，旧 command HTTP 404。
7. stdio 完成协议 `2025-11-25` initialize、initialized、query，按 id 读取并核对 structuredContent 业务值，之后才关闭 stdin，bridge exit 0，stderr 不含 token。
8. 第三次正常关闭再次核验 helper、owner、durable state 与端口。

最终清理日志：`target/phase7-release-cleanup.log`，安装临时目录已删除，来自该目录的 binary 进程数为 0，3 次 graceful shutdown 记录齐全。

## 拒绝分支与审查修正

- 构建传入全 0 `ExpectedCommit`：exit 1，Cargo 前拒绝，输出目录未创建。日志 `target/phase7-release-commit-rejection.log`。
- Windows 请求 macOS arm64 target：exit 1，输出目录未创建。日志 `target/phase7-release-native-rejection.log`。
- 空 artifact 目录：exit 1。日志 `target/phase7-release-empty-rejection.log`。
- 从实际成功发行目录复制出的 5 份样本，分别改 checksum、commit、version、target 或添加额外文件：全部 exit 1；日志 `target/phase7-release-reject-{checksum,commit,version,target,extra-file}.log`。
- 自动审批两次拒绝清理 probe 样本的命令，返回仅 `blocked by policy`。第二次为经 Resolve-Path 确认后的固定绝对路径删除，未执行成功；停止重试并保留 `target/phase7-release-negative-20261103/` 供复核。它们是无秘密的发行包变异样本，不是 smoke 服务、token 或 dev data；不影响已通过的安装环境清理。
- 修复独立 package review 的 CR-01：构建无条件取得 actual HEAD，ExpectedCommit 只作断言，manifest 记录 actual HEAD；VerifyOnly 保留显式预期 commit 的跨 checkout 验证。见 `07-14-PACKAGE-REVIEW.md`。
- MCP isError 是可选字段，使用 ContainsKey 后再读取；现有 rmcp successful structured response 实际也写 false。见 `07-14-SMOKE-WORKFLOW-REVIEW.md`。
- 两个 PowerShell 文件最终 parser 检查通过，`git diff --check` 通过。没有新增依赖、stub、skip test 或业务实现修改。

## 三平台 native CI 历史证据（2026-11-03）

2026-11-03，run `41648281446` completed/success，三个 native job及aggregate均success，publish skipped。保存元数据 `target/phase7-release-ci-41648281446.json`，下载目录 `target/phase7-native-artifacts-41648281446/`。CI source head为 `e6ae31055c17d6d4005226b21c2d5991ac5a34ef`；PR实际打包checkout及三个manifest commit均为 `f315d4650114c449d9f857fccbe0b6db8d80b686`，version均为0.1.0。

| Target | 实际环境 | Archive bytes | SHA256 |
|---|---|---:|---|
| aarch64-apple-darwin | macOS 15.8.2 Arm64 | 18137046 | b181417965138557ac033b1fea160d01ee6eef752d7b3cf029404ce5c80efeb9 |
| x86_64-pc-windows-msvc | Windows 10.0.26100 X64 | 18548325 | b60329a52f0e1713a93518fad9082ee2f3d4760d7606811a82bc124f6b06a5f8 |
| x86_64-unknown-linux-gnu | Ubuntu 24.04.5 X64, glibc 2.39 | 20231605 | 3b62032e1995e3dab5259e3f30bd6f0f59b3ce8cb27f69c5a173f4729941e841 |

三个实际archive已下载并重算hash，与各自checksum一致。三个native runner均完成实际release build、package验证、无checkout安装smoke及artifact上传，aggregate核同commit/version/target/checksum。该结果替代先前Linux/macOS pending记录。后续T24/E6/T08修复已由2026-11-04 run `41669982362` 复验，最终产物见 `../07-14-SUMMARY.md`，本节保留早期产物历史。

## 交接与未跑项

- Linux x64、macOS arm64、Windows Server 2025 x64 native CI已通过，环境与产物见上节。
- tar executable bit、Linux glibc及macOS实际安装已由native job实测。
- workflow 的 Release upload 路径仅静态审查，未实际触发；本轮授权范围禁止创建/上传 Release。PR 和 dispatch 仅构建验收。
- 发行实现无剩余finding；T24/E1–E7/T08/N1–N3均已动态闭合及独立复签。最终CI四必需job均success，与native共同形成最终自动验收证据。
- 原实现lane未commit/push；后续由主会话提交 `cba62af`，本次文档lane仅更新授权收口文档，不运行Cargo/服务或提交。T1由主会话维护。
