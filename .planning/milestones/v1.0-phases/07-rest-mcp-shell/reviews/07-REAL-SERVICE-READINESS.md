# Phase 7 真实服务与发行环境预查

2026-10-29，只读查询，未启动/修改服务、未运行测试、未发布。环境状态会变化，执行07-13/14时须再次核对。

| 项目 | 已核查 | 执行前置 |
|------|--------|----------|
| Docker | daemon29.7.2、Linux containers可用；PG17.11/Trino483指定镜像已缓存 | 隔离容器、新凭据 |
| 旧环境 | ontoforge-pg-05-06、ontoforge-trino-05-06均Exited(255)；5432/8080/58081/58082无listener | 不依赖旧服务状态 |
| 连接配置 | 当前进程四个ONTOFORGE_*_URL未设置 | setup后安全传入，不回显凭据 |
| WSL | Ubuntu-26.04已安装但Stopped | 尚未验证内部工具 |
| Windows | Rust1.98.0、MSVC、pwsh、Node/npm可用；debug binary存在 | release打包/checksum/安装smoke未跑 |
| GitHub | gh可访问仓库；ci/nightly active | 三平台native jobs，禁止创建Release |

## 脚本前置

- `ci/query-services.sh:19` 依赖RUNNER_TEMP/GITHUB_ENV；Git Bash无psql，不能直接照搬CI启动命令。可用WSL/Linux CI或先安全补齐工具。
- 同脚本`:16` 输出GitHub add-mask指令，本机运行须捕获并脱敏；不得将凭据显示在命令输出。
- 同脚本`:51` 依赖Docker host networking，Desktop能力尚未验证。
- 同脚本`:145` cleanup忽略删除错误；07-13 wrapper须核实自己创建的资源已清理，cleanup失败返回非零。
- 07-13 server-services/真实suite与07-14 release workflow/脚本是待创建产物。现有`.github/workflows/ci.yml:65`可作为Ubuntu真实服务入口模式。

执行07-13必须实跑超过60秒的L2证明过期场景；07-14须同一commit的Linux/macOS/Windows实包与native smoke证据，未跑平台保持pending。
