# 本地开发服务

`serve --dev` 是不依赖 checkout、Docker 或 Trino 的单二进制演示入口。首次启动会在当前目录的 `.ontoforge/dev` 创建持久数据目录；使用 `--data-dir DIR` 可覆盖目录。

```powershell
ontoforge serve --dev --bind 127.0.0.1:8080
ontoforge dev-token --server-config .ontoforge/dev/server.toml --sub alice --groups dispatchers
```

`dev-token` 的唯一输出是 token。将它作为 `Authorization: Bearer <token>` 发送到 `/api/query` 或 `/api/actions/{action}`；健康检查路径为 `/health`。服务会把 token key、独立 query signing key、SQLite edit store、ontology、scope 和 CSV fixtures 写入 data dir；路径只写到 stderr，不写 secret。

正常重启不会重新 seed 已有 definition，也不会删除 Action 修改、history 或 receipt。相同 `commandId` 的 Action 请求沿用 edit store 的幂等合同。

服务在整个 dev 生命周期持有 `.ontoforge-dev.lock` 的独占 OS 文件锁；竞争启动和活跃实例上的 reset 会被拒绝。进程退出或崩溃后由内核释放锁，锁文件保留，不通过 PID 文本判断所有权或删除锁 inode。

Unix 上新建 dev 目录使用 `0700`，`dev-token.key` 与 `query-signing.key` 使用 `0600`。已有 dev 根目录或 key 若具有任意 group/other 权限，会拒绝启动（包括 `--reset`）并分别提示 `chmod 700`、`chmod 600`；不会自动修改已有目录权限或 key 的字节与权限，也不检查父目录全树。Windows ACL 保持不变。

```powershell
ontoforge serve --dev --data-dir .ontoforge/dev --reset --bind 127.0.0.1:8080
```

`--reset` 必须和 `--dev` 一起使用，且 data dir 必须带有 Ontoforge marker。活跃实例、缺 marker 的非空目录、symlink 路径和 data dir 之外的路径都会被拒绝。reset 只删除明确的 dev assets，重新生成 auth key 和 query signing key，因此旧 token 会失效。

## 服务关闭

收到关闭信号后，服务先停止接入新请求，等待已接受的 Action 及其 source 依赖，再取消独立读和 MCP SSE，持久化 read audit，停止 L2 monitor，最后等待 core/L2 engine 的远端查询取消请求完成。客户端取消或断连不会重发 Action，也不会中止已接受的 Action。

默认关闭总预算为 10 秒，从收到关闭信号计时：Action、独立读和 HTTP 连接等待截止于第 5 秒，audit drain 截止于第 6 秒，monitor cleanup 截止于第 8 秒，core/L2 和 monitor 取消任务的并发 drain 截止于第 10 秒。各步骤完成后立即进入下一步，不额外等待；任一步失败或超时仍在剩余预算内尝试后续清理，最终返回失败退出码。

## Windows 服务验收工具

`bash ci/server-services.sh verify` 在 Windows 上额外使用 Python 3 标准库工具 `ci/windows-server.py`，无需安装 Python 包。Git Bash 的 POSIX `kill -INT` 无法可靠触发原生 Windows CLI 的 Ctrl+C；工具在独立隐藏 console 启动服务，由后台 owner 保留进程和 Job handle，在唯一运行目录原子记录 PID、创建时间及真实退出码。

脚本通过 `uname -s` 识别 MINGW、MSYS、CYGWIN，不依赖可能缺失的 `OS` 环境变量。owner 在创建服务前清除自身继承的 Ctrl+C 忽略标志，避免 Bash 后台启动把该标志继续传给服务；回归测试会清除 `OS` 并执行脚本实际的启动及清理函数。

清理时工具持有目标 process handle 并核验创建时间，仅在 console 成员恰为目标服务与停止助手时发送 `CTRL_C_EVENT`。不会向共享 console 广播。停止最多等待 30 秒，超时强制清理仍返回失败，退出码 `130`、`137` 或其他非零结果不会视作成功；owner 异常结束时 Job 会回收已纳管的进程。启动与 Job assignment 之间不宣称具备原子保证，assignment 失败会立即回收刚启动的子进程并报错。Linux 继续使用现有信号路径。

状态文件和 owner 保留完整 Windows 退出码；Bash 调用 stop 的 `--shell-status` 模式，将所有非零退出转换为 shell 失败并输出原始退出码，防止 `256` 等值被 8 位 shell 状态截断成成功。

已有 `target/debug/ontoforge.exe` 时，可运行 `python -B -m unittest discover -s ci -p test_windows_server.py -v` 单独验证真实进程关闭。Windows CI 在 workspace 测试产物就绪后运行同一组用例，不需要 Docker。
