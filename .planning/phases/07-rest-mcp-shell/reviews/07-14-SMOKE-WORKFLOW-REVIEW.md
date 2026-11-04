---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T13:48:40Z
depth: deep
files_reviewed: 3
files_reviewed_list:
  - ci/release-smoke.ps1
  - .github/workflows/release.yml
  - docs/installation.md
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
validation: static_review_and_windows_smoke_log
head_at_review: e0f534d135f2017f7de811903ec13460a30f270e
source_blobs:
  ci/release-smoke.ps1: bb49d0d49e61017ad0cb33428fcdbf4c3e22d1c4
  .github/workflows/release.yml: 4bb987019fb6f92a264102fa1c343d194c25c45c
  docs/installation.md: cf17144a4e802017a3592b074f62943eeef705ab
---

# 07-14 Smoke 与 Workflow 静态审查

## Narrative Findings (AI reviewer)

本次未确认新增 BLOCKER 或 WARNING。`clean` 仅表示上述 blob 的静态审查未发现可证实缺陷，不代表三平台 artifact smoke 已实际运行或 Phase 已通过验收。

## 审查证据与排除项

- `ci/release-smoke.ps1:31-45,146-154`：解包目录独立创建，所有产品子进程使用该目录作为 cwd，binary 使用解包后的绝对路径。version/help 由该 binary 执行。Windows helper 使用 checkout 中的绝对路径，但未加入发行包；产品运行不从 checkout 读取 fixture。
- `ci/release-smoke.ps1:155-183`：追踪 query、Action、history、command receipt、重启与 reset 断言，交叉核对 `crates/ontoforge-server/src/rest.rs:721-755` 的 receipt 序列化和 `context.rs:258-277` 的未认证 JSON 错误。smoke 核对 F-1001 的 LH100、scheduled/0 与 DELAYED/15、eventId/committedAt，以及旧 token 的重启保留和 reset 后 401。
- `ci/release-smoke.ps1:65-108,194-218`：正常关闭要求 helper/owner 成功退出、Windows durable state 成功、端口关闭。Unix 发送 SIGINT，与 `crates/ontoforge/src/serve.rs:32-38` 的 ctrl_c 监听一致。超时先抛错，finally 强杀未把异常转换为成功。失败清理的无参数 WaitForExit 仍依赖 OS 完成强制终止；本次未证明其在当前子进程路径导致实际无限等待，不将一般性假设列为 finding。
- `ci/windows-server.py`（关联读取，blob `034e2e1dead1c024222c8db701ee21dc66ff24a8`）：核对隐藏 private console、Job Object、PID/creation_time 校验及 durable completion record；未运行 helper 或中断正在构建的 executor。
- `ci/release-smoke.ps1:125-143,184-200`：MCP 响应按 id 匹配，读响应与 EOF 后退出均有超时，structuredContent 复用业务断言。`crates/ontoforge-server/src/mcp.rs:684` 通过 CallToolResult::structured 返回 query 结果，`crates/ontoforge/src/mcp.rs:178-185` 转发远端结果。
- 已排除 `isError` 缺失导致当前成功路径失败的疑点。Cargo.lock:2874-2877 锁定 rmcp 3.8.0；本机相应依赖 `src/model.rs:4318-4324` 的 structured 构造器写入 `is_error: Some(false)`。PowerShell 7.7.0-rc.1 的内存表达式验证确认 StrictMode 对缺失键的属性访问会抛错，但当前响应构造器不会省略该字段。执行 lane 后续已将 smoke:192 改为 ContainsKey guard；本报告绑定更新后的 blob，不把该防御性修改记为已发现业务缺陷。
- `.github/workflows/release.yml:20-88`：三个 native target 分开构建与 smoke，上传名称包含 target、run_id、run_attempt；aggregate 依赖整个 native matrix 成功，逐一下载三个确切名称并通过 VerifyOnly 校验同一个 GITHUB_SHA 及 checkout workspace version。
- `.github/workflows/release.yml:90-128`：publish 仅接受 release/published 且前置成功，写权限仅该 job；重验全部资产、逐个核 tag/version、九个资产数量、非 draft release 与同名冲突。上传没有 clobber 参数，未发现 PR/workflow_dispatch 进入发布路径或覆盖资产的路径。
- `docs/installation.md:7-15,19,53,69-71`：平台表与 workflow target 一致，明确动态 GNU、arm64、MSVC、无源码运行与未运行平台不能视为已验收。文档的 CI helper 描述未将 Python 列为产品运行依赖。

## 范围与置信度

读取了 07-14-PLAN、07-14-PREFLIGHT、完整三个目标文件及上述关联调用链。仓库没有 AGENTS.md、.codex/skills、.agents/skills 或 .codegraph；遵循会话提供的项目规则。GSD agent-skills 查询成功且未返回需额外加载的技能。已读取 gsd-code-review skill。

未重复报告 package lane 已发现的 PK01（ExpectedCommit 与 checkout HEAD 绑定），该问题由独立 lane 与主会话处理。本 reviewer 未运行 Cargo、package、smoke、native matrix 或发布命令。静态结论置信度高；动态结论严格限于下列 executor 日志。

## 最终 blob 与 Windows 日志复核

2026-11-03T13:48:40Z 重新执行 git hash-object：三个目标文件与 frontmatter 中已审查 blob 完全一致，未发现需新增审查的源码变化。

所读日志 SHA256：`848D7EC179DFD78798496445C6647FF2E0B0DFDFB5B7BEDDDA48C2AD3F8CCC10`。

亲读 executor 生成的 `target/phase7-release-smoke.log`，日志最后明确为 `EXIT: 0`。实际安装包身份为 target `x86_64-pc-windows-msvc`、version `0.1.0`、commit `e0f534d135f2017f7de811903ec13460a30f270e`；archive SHA256 为 `ec09bec6292b9d456dbba2b745849c34b0c5cb14f092f9f8e6e1bdab25950f9c`。日志记录以下路径全部通过：

- 无 checkout 临时目录中的 version/help。
- F-1001 初始 LH100 scheduled/0，Action 后 DELAYED/15，history 与 command receipt 对账。
- 重启后旧 token 继续有效，query/history/receipt 保留。
- reset 后旧 token 拒绝、新数据 scheduled/0、history 为空、旧 command 404。
- MCP initialize/query structuredContent 通过，stdin EOF 后 bridge exit=0。
- 三次正常关闭均记录 stop/owner exit=0、Windows durable state 和端口关闭。

该日志支持本机 Windows artifact smoke 成功；它不证明 GitHub Windows runner 或完整 native CI 已通过。Linux、macOS 与 GitHub 三平台 native CI 仍为 pending；Unix pwsh、tar executable bit、SIGINT 和相关实际业务路径尚无本 lane 读取的运行证据。保持新增 BLOCKER 0、WARNING 0，不将本报告提升为 Phase 验收通过。

专用 Read/Write 工具未暴露，按 PLAN 中既定环境适配以 PowerShell 读取、apply_patch 仅写入本报告。未修改实现文件，未 commit。
