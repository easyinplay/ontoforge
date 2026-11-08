---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T06:56:57Z
depth: standard
review_head: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
initial_review_head: 02a9c4ea0caee1ad8abe0fa1031470ed2938b32b
review_protocol: gstack-review-spawned-session
files_reviewed: 5
files_reviewed_list:
  - ci/windows-server.py
  - ci/test_windows_server.py
  - ci/server-services.sh
  - .github/workflows/ci.yml
  - docs/dev.md
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
phase_acceptance: pending
---

# Phase 7 Windows Helper Review

对 `02a9c4e` 五文件及 `fd24beb` 的四文件增量完成独立定向审查，最终未发现剩余 BLOCKER 或 WARNING。初次审查没有识别真实 Bash 入口的两处集成问题；其后完整服务执行暴露问题，并由 `fd24beb` 修复。本结论不代表完整服务验收或远程 CI 已通过。

## Narrative Findings (AI reviewer)

无未解决发现。

## fd24beb 增量复签

### WH-01: [BLOCKER, resolved] OS 缺失使实际脚本跳过 Windows 分支

原 caller 只检查 `OS=Windows_NT`，初始回归又主动设置该变量，因此 9/9 不能证明实际 Git Bash 入口。完整执行发现该环境没有 OS。最终 `ci/server-services.sh:17` 使用 `uname -s` 的 MINGW/MSYS/CYGWIN 结果建立唯一 `IS_WINDOWS`，binary 后缀、Python 前置检查、runtime binary 后缀、实际 start 和 cleanup 五处共用该判断；不再有 OS 环境判断残留。

`ci/test_windows_server.py` 新增实际入口测试：从 subprocess 环境移除 OS，并在 Bash 内再次 `unset OS`；source 真实脚本后调用生产同一个 `start_server`，检查默认 binary 的 `.exe` 后缀、Windows owner state、HTTP health，执行真实 cleanup，检查 PID/state 指针删除、state 零退出及端口关闭。Python 在此只启动 Bash，没有直接 Popen helper 替代生产后台启动。另一个测试明确检查缺 OS 时退出 137 仍使 cleanup 失败，原已有 Bash 137/256 测试也不再强设 OS。

### WH-02: [BLOCKER, resolved] Bash 后台 owner 继承 Ctrl+C 忽略状态

最终 `ci/windows-server.py:143` 在 Popen 前调用当前进程的 `SetConsoleCtrlHandler(None, False)`，清除 owner 自身的忽略 flag，使新建 private-console child 不继承错误状态。这不是向任何 console 发送事件，也没有修改其他进程的 handler；真正发信号仍经过原有 process identity 与 private-console 成员检查。新增真实 Bash 后台启动测试覆盖该继承路径，正常退出不会再只由 Python 直接启动用例证明。

**本轮证据：** 独立读取四文件完整增量与新增测试函数，核读 `target/phase7-windows-platform-tests.log`：11 个明确测试均 `ok`，`Ran 11 tests in 17.867s`，`OK`。动态执行归属 owner，本 lane 未重跑。五个审查文件与 `fd24beb` 没有未提交差异。完整 PostgreSQL/Trino 脚本仍待主会话在新冻结版本重跑，不能把此次 source/start_server seam 回归称为完整 Docker pipeline。

以下初次核查证据继续适用；初始 9 项结果保留为历史证据，最终以本节的 11 项和新 head 为准。

## 核查证据

| 边界 | 源码及断言 |
| --- | --- |
| 私有 console | `ci/windows-server.py:125` 使用 `CREATE_NEW_CONSOLE` 与隐藏窗口启动目标；`:97` AttachConsole 后要求成员恰好为目标 PID 和本次 stop helper，之后才发送 CTRL_C_EVENT。额外成员的真实测试要求 helper 失败且两个原进程仍存活，没有向共享 console 广播的 fallback。 |
| 进程身份 | `windows-server.py:163` 打开 process handle，创建时间与 state 一致才进入信号路径；同一 handle 用于退出检查、等待与强制清理。错 PID 和错 creation time 的测试均要求拒绝操作且目标仍活着。state 使用独占 run lock、原子 replace，重复 run 不覆盖已有身份。 |
| owner 异常 | `windows-server.py:86` 配置 KILL_ON_JOB_CLOSE，run 保留 Job handle；assignment 失败的 finally 回收已创建 child。owner crash 测试真实终止 owner，等待目标 process handle 变成 signalled，并要求之后 stop 不能误报成功。 |
| 退出状态 | `windows-server.py:171` 对 handle 返回零还要求 owner 的 durable completion record，避免把 owner crash 导致的 Job 回收当正常退出。真实 state 和 owner 保留 Windows 退出码。`:225` 在 shell-status 模式把所有非零变成 1；`server-services.sh:206` 确实启用该模式。测试覆盖 17、130、137、256，以及实际 Git Bash cleanup 对 137/256 返回 1。 |
| 超时与拒绝 | stop 的 timeout 验证有限、正数且最多 30 秒；超时使用已核验 handle 强杀并返回 1，不接受强杀为正常关闭。Bash 在身份拒绝后不无界等待仍存活 owner；跨 shell 的 wait 127 只有在 stop 独立成功时才可忽略。超时测试检查非零、目标已退出及时间上限。 |
| Linux 路径 | 本提交只在 Windows 分支选择 Python owner 和 stop；Linux 仍保留原 native launch、SIGINT、有限等待及 SIGKILL 失败逻辑。没有把本次 Windows 测试当作 Linux 运行证据。 |
| CI 与真实服务 | `.github/workflows/ci.yml:53` 在 workspace 测试产物就绪后，仅 Windows 执行 Python unittest discover。`ci/test_windows_server.py:94` 复制真实 `target/debug/ontoforge.exe`，启动 dev server、等待 HTTP health、正常停止并检查零退出和端口关闭。其余测试为本套件自行创建的真实进程，未使用 Docker。 |

## 动态证据归属

- 本 lane 读取 `target/phase7-windows-helper-tests.log` 原始输出：9 个明确命名的测试均为 `ok`，`Ran 9 tests in 12.545s`，`OK`。这是 owner 执行的日志，本 lane 没有重复运行这些测试。
- 独立读取全部测试实现，核对真实 CLI、private console、creation time、owner crash、强杀、重复 state、Git Bash 状态映射的具体断言；并检查五文件与冻结提交没有未提交差异。
- 原 SIGINT 根因 RED 及 256 截断 RED 来自主会话/owner 提供的上下文，本 lane 没有运行或重新验证其 RED 过程。
- 没有执行 Cargo、完整 Docker 服务、额外 console probe 或远程 CI。未声明此日志能替代完整 `server-services.sh verify` 的 Windows 成功运行。

## 明确限制

启动 child 到 AssignProcessToJobObject 之间仍存在窗口，`docs/dev.md` 已明确说明；本报告不宣称 owner 在该窗口被强制终止也有原子回收保证。正常 assignment 失败会走本地 finally，已纳管 child 才由 Job 保证 owner 异常回收。这是本次批准方案的已知边界，没有用正常路径测试掩盖它。

state 和 run 目录是本地 CI 控制文件，不作为抵御同权限本机恶意篡改者的安全边界。成功 owner record、creation time 与 console 检查解决的是服务归属、陈旧状态及误发送信号问题。

本报告只新增当前审查文件；不修改实现、测试或共享 planning。整体门禁及 Phase 验收仍由主会话推进，提交由 orchestrator 负责。
