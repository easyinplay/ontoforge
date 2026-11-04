---
phase: 07-rest-mcp-shell
plan: "14"
subsystem: release
status: complete
tags: [native, release, artifact, smoke, ci]
provides:
  - Linux x64、macOS arm64、Windows x64 实际发行包与安装 smoke
  - archive、manifest、checksum 独立校验及发布事件隔离
key-files:
  created: [.github/workflows/release.yml, ci/release-package.ps1, ci/release-smoke.ps1, docs/installation.md]
  modified: []
key-decisions:
  - "PR 与 workflow_dispatch 仅构建验收，publish skipped；不创建 Release、不发布 npm、不合并 PR"
  - "source commit 与实际 PR checkout commit 分别记录，以同 tree 证明最终源码一致"
requirements-completed: [DX-04, DX-01, DX-02, API-04, API-05]
actuals:
  tasks: 2
  commits: 1
updated: 2026-11-04
completed: 2026-11-04
---

# 07-14 三平台发行包与实际安装验收

## 2026-11-04 最终增量证据

当前source为 `f8851ca073fed34e80e605f0ac91880259df7e12`。CI `41759804733` completed/success，Windows、Ubuntu、integration、phase7-real-services四必需job成功；两个workspace原始日志独立计数分别1496/0/117、1503/0/117，各150组，不相加成唯一测试数。INT-01新5测试及TLS两步在双平台实际通过。当前GSD UAT14/14，`source: automated`；VALIDATION规范状态为 `validated`。

release `41759804768` native3/3与aggregate成功、publish skipped。三个实际包manifest的checkout为 `22913c8e79ae98b6604109d6e05a15f27cd50292`，与source不同；Git commit API与本地source tree复核二者完整tree均为 `0ecc59382aaf12e782c508ba64858e2689299fd7`。独立审计下载实际archive并重算archive SHA、binary SHA、内外manifest，verifier又执行只读重算通过；详细表及源码身份见 [RELEASE-ARTIFACT-AUDIT.md](../../milestones/v1.0-audit/RELEASE-ARTIFACT-AUDIT.md)，原始数据 `target/milestone-release-41759804768/`。没有本机重新运行包内程序；三平台动态行为由本run安装smoke日志证明。

本地动态实际51 passed：SQLite5、PG source guard1、fresh服务/SDK43、Iceberg2。原runner exit1（`HEAD changed during validation`）保留；亲核868643c→f885只改变cli_color测试类型别名，服务输入相同，verify commit.txt=f885。独立final-check的provenance/scan/cleanup exit0是单独证据，不能改写runner退出码。Windows owner正常exited/0与清理结果见 [DYNAMIC-VALIDATION.md](../../milestones/v1.0-audit/DYNAMIC-VALIDATION.md)。旧48项安全证据保留，INT-01/TLS另有局部独立CLOSED复签。

本次不创建Release、不发布npm、不merge PR、不触发nightly或外部消息。Phase07通过不代表milestone全绿：STORE-05 source与ENGINE-05历史MinIO差异仍由主会话保留。以下为原adc0227时点记录，日期、计数、hash和当时结论均保留。

## 历史 adc0227 收口记录

最终源码的三个 native 包与完整安装 smoke 已通过，archive 下载后独立校验一致。最终CI四个必需job全部成功，32个task动态门已对账通过；GSD UAT已complete，13/13通过（commit `975f78e`）；最终goal VERIFICATION由独立verifier归并。

## 交付与提交

| Task | 实际交付 | 结果 |
|---|---|---|
| 14-1 | native release build，平台唯一 archive、manifest、checksum，重开包验证与三平台聚合 | GREEN |
| 14-2 | 从实际包解压至无checkout目录执行 dev完整链、restart/reset、stdio及cleanup；发布事件隔离 | GREEN：smoke与最终四必需CI job全部通过 |

实现提交 `cba62af`（`feat(07-14): 构建三平台发行包并验证实际安装流程`）。四个交付文件见frontmatter；本次文档整理不新增实现commit。配置/打包/文档接线按PLAN跳过业务TDD，以真实构建、包自验、故障拒绝与安装进程验证，理由已记录 `reviews/07-14-IMPLEMENTATION.md`。未引入依赖或新产品配置。

## 最终源码和打包来源

- CI/release source：`adc02278b5cc55111def9a30a1c3b778485e8279`。
- native PR checkout与三个manifest commit：`b75d2949f9db1276ce07f4864a6206f7932bb11b`。
- 两个commit的tree均为 `711d68e9956316d34f27aca95be87f13ab953007`，本次已通过`git rev-parse '<commit>^{tree}'`逐项核对。manifest忠实记录实际checkout，不将source SHA改写成实际打包SHA。
- release run `41669982362`：三个native job及aggregate completed/success，publish skipped。元数据：`target/phase7-release-ci-41669982362.json`；独立下载/hash校验：`target/phase7-native-artifacts-41669982362-verification.json`。
- 三个manifest version均为`0.1.0`。独立hash校验结果如下；旧run `41648281446`与本机早期`e0f534d`仅作历史，不冒充最终源码证据。

| Target / 实测环境 | Archive | Bytes | SHA256 |
|---|---|---:|---|
| aarch64-apple-darwin / macOS 15.8.2 Arm64 | `ontoforge-0.1.0-aarch64-apple-darwin.tar.gz` | 18386682 | `085ea1fad049c7289edba50471039cfc5d300da1877c2febb1a4e85b7ed9fdc1` |
| x86_64-pc-windows-msvc / Microsoft Windows 10.0.26100 X64 | `ontoforge-0.1.0-x86_64-pc-windows-msvc.zip` | 18814840 | `ed48f0b8aa6e1cbef4ba87d240a50f95beacb0669386862387aa7d2ef2918a88` |
| x86_64-unknown-linux-gnu / Ubuntu 24.04.5 LTS X64; ldd (Ubuntu GLIBC 2.39-0ubuntu8.9) 2.39 | `ontoforge-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | 20506039 | `944e90de1e6238ec08627dee06aaecf83fff6ef8bb115c5cdc811133b33c4d05` |

三个native job均实际运行`ci/release-package.ps1`和`ci/release-smoke.ps1`，完成version/help、隔离cwd启动、query→Action→history、正常关闭与restart保留、显式reset与token轮换、stdio真实query、进程/端口清理。aggregate核对三target齐全、version/commit一致及checksum。安装scope与限制见 `docs/installation.md`；不宣称universal或全静态二进制。

## Phase 最终动态证据对账

| 门 | 源码/日志 | 实测结果 |
|---|---|---|
| 最终fresh服务与SDK | `target/phase7-final-adc0227-verify.log`；副本`target/phase7-services/evidence/final-adc0227-verify.log`，commit.txt=`adc0227` | 服务17（PG4、drift2、snapshot1、Trino2、health2、ACL6）+ Rust9 + TS17 = 43，零失败/required skip；`fresh_verify_exit=0`，主会话session31544 exit0 |
| fresh配套门 | 同一fresh脚本 | OpenAPI生成无diff、strict typecheck、build、credential scan、cleanup通过；finish强制scan/cleanup，正常exit0为完成证据 |
| 本机清理 | 主会话检查runtime/server-windows.N5DO2s/state.json及Docker | owner phase=exited、exit_code=0；Docker只剩用户既有vennuap-casdoor，无本任务服务遗留 |
| T08最终回归 | `target/phase7-mcp-admission-green-r2.log` | lib25 + admission4 + L2defaults10 + transport11 = 50/50，0 failed/ignored；Clippy/fmt通过 |
| N1/N2与typed query | `target/phase7-bridge-security-cli-r2.log`、`phase7-bridge-security-mia.log`、`phase7-explain-query-tests.log` | stdio11/11、Mia定向1/1含8次拒绝、query15/15；均独立复签 |
| required CI | `target/phase7-ci-41669982375.json`，Ubuntu/Windows完整日志`target/phase7-ci-41669982375-{ubuntu,windows}.log` | 四个必需job（Windows test、Ubuntu test、integration、phase7-real-services）均completed/success；Ubuntu workspace1497 passed/0 failed/117 ignored、Windows1490/0/117（各149个suite result） |
| native | `target/phase7-release-ci-41669982362.json` | 三native+aggregate success，publish skipped |

两个平台独立汇总为`target/phase7-ci-41669982375-{ubuntu,windows}-counts.json`，117 ignored不视为已执行，由真实服务门另证。上述集合有重叠，不加总为唯一测试总数。各target耗时由原日志记录：最终fresh PG5.25s、drift92.08s、snapshot1.21s、Trino5.20s、health65.15s、ACL4.56s、Rust0.43s、TS22.26s；不把局部测试时间之和冒充完整编译/运行墙钟时长。早期Windows首次release编译15m28s仅属于历史本机基线。

## 历史 RED 与审查闭合

- T24资源生命周期两轮RED由真实manager session12/transport11及最终相邻回归闭合；详 `reviews/07-T24-IMPLEMENTATION.md`、`reviews/07-T24-SECURITY-RECHECK.md`。
- E6越权请求提前触发availability probe：实际RED后把授权/参数预检置于探测前，六工具零发送及同session撤权/TTL恢复GREEN；详 `reviews/07-E6-PREFLIGHT-FIX.md`、`reviews/07-E6-SECURITY-RECHECK.md`。
- N3/T08 typed cap1第二请求200≠429：真实执行permit生命周期修复；草稿REST extractor提前丢Arc的回归也有独立RED，outer只drop与handler唯一take后50/50 GREEN。详 `reviews/07-MCP-EXECUTION-ADMISSION-IMPLEMENTATION.md`、`reviews/07-T08-SECURITY-RECHECK.md`。
- Bridge首轮5/3是Initialize/session协议假设与receipt fixture错误，按实际合同修正后11/11；N1/N2补齐真实成功链、不同主体拒绝、在线认证与302 trap。没有把fixture失败改称生产漏洞。
- Spec、Staff/security、Standards复签无未解决finding；安全48/48 CLOSED；末尾简化建议0，未为风格改动冻结代码。证据见 `reviews/07-FINAL-SPEC-REVIEW.md`、`reviews/07-FINAL-STAFF-REVIEW.md`、`reviews/07-FINAL-STANDARDS-REVIEW.md`、`reviews/07-FINAL-SIMPLIFICATION.md`。
- 07-02 D4、07-09 D3历史unknown/human仅表示当时未实跑，现由最终PG与真实stdio自动证据覆盖。历史SUMMARY保持原文，最终UAT记录后继技术覆盖，不伪称用户亲测或替代人工产品判断；CC AGREE见`target/phase7-final-uat-review.json`。

## 发布边界与完成标准

本轮没有创建Release、上传Release assets、发布npm或合并PR；PR #7与#8均为OPEN、isDraft=true、mergedAt=null；最终状态文件为`target/phase7-pr7-final-status.json`与`target/phase7-pr8-final-status.json`。publish skipped是授权边界的预期结果，不是required验证失败。

最终CI `41669982375` completed/success；可选isolated-query-bench skipped不属于本Phase必需门。G1–G6已全部通过，`07-VALIDATION.md`的32个task全部GREEN，`status: passed`、`nyquist_compliant: true`。本报告完成Plan14交付记录，GSD `07-UAT.md`已complete、13/13，uat-only检查passed=true（commit `975f78e`）；Phase goal最终判定由独立VERIFICATION归并；不自行推进下一phase。此前Windows pending仅是等待期间状态，已被本次CI终局替代。
