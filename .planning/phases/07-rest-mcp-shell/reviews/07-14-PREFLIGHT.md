# 07-14 只读实施预检

2026-11-03：在07-13整合验收期间只读检查现有代码，不代表进入07-14执行；未修改发行脚本、未构建发行包。正式范围仍以07-14-PLAN.md为准，无新增产品选择。

## 现有模式

- release.yml、ci/release-package.ps1、ci/release-smoke.ps1、docs/installation.md均尚不存在。
- 复用ci.yml的checkout/rust-toolchain/cache/upload-artifact完整SHA；没有已核验download-artifact pin，可考虑runner已有gh下载本次run的明确artifact，或正式核验新action后使用。
- 版本来自Cargo.toml workspace.package.version，CLI继承并通过clap输出；Rust版本取rust-toolchain.toml。不能另造版本常量。
- 已批准native矩阵见PLAN；在runner实际核对架构与target，尤其macOS arm64，不能仅凭label宣称通过。
- 2026-11-03另核GitHub官方runner表：macos-15是arm64，Intel标签为macos-15-intel；因此原PLAN标签无需更换，仍须在job中动态验证。来源：`https://docs.github.com/en/actions/reference/runners/github-hosted-runners`。

## 实际smoke入口

- 在无checkout的新cwd运行解包binary：`serve --dev --data-dir <temp>/data --bind 127.0.0.1:<port>`。让binary创建data dir，避免Unix提前创建成0755被安全检查拒绝。
- `dev-token --server-config <temp>/data/server.toml --sub alice --groups dispatchers`取得token，stdout仅内部读取，不输出凭据。
- 以`/api/query`查询Flight F-1001，接口示例见tests/unit/rest.test.rs:45；初始flightNumber=LH100、status=scheduled、delayMinutes=0。
- `/api/actions/delayFlight`：`{"commandId":"release-smoke-unique","items":[{"params":{"flight":"F-1001","delayMinutes":15}}]}`。精确核receipt.commandId与items.edits，重查应DELAYED/15。
- history接口为`/api/objects/Flight/F-1001/history?limit=10`，对账action/eventId/committedAt；`/api/commands/<id>`返回同一完整receipt。参考http_actions.rs:80、:91。
- 正常关闭后复用data dir与旧token重启，验证query/history/receipt持久保留；再正常关闭后`--reset`，旧token失效，重新签token后恢复scheduled/0、history空、旧command404。
- stdio执行`mcp --server-url http://127.0.0.1:<port>/mcp`，以child env传ONTOFORGE_TOKEN。参考cli_stdio.rs:34/54/94，用2025-11-25 initialize→initialized→query tool，必须先读到匹配id结果并核structuredContent业务值，再关闭stdin，bridge有界exit0。

## 正常关闭与现有测试限度

- Windows复用ci/windows-server.py已验证private-console run/stop helper，helper与binary用绝对路径，进程cwd设为解包temp；每次重启使用新的state路径。检查stop和owner都exit0、durable state exited/exit_code0及端口关闭。该helper只服务CI验收，不加入发布包或产品运行依赖。
- Unix发送SIGINT，src/serve.rs只监听ctrl_c；SIGTERM/.Kill()不能冒称正常关闭。模式见ci/server-services.sh:230。
- dev_lifecycle.rs的stop使用child.kill()，其distributed_binary命名测试也没有完整query/Action/history断言；不得借旧测试名称宣称满足本次artifact验收。
- macOS/native发行构建、tar executable bit、Linux实际动态链接/glibc基线、三平台archive完整smoke均尚无本轮证据，留07-14实际执行。

未创建Release、未发布npm、未合并PR。07-13通过前不实施本文件所述工作。
