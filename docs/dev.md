# 本地开发服务

`serve --dev` 是不依赖 checkout、Docker 或 Trino 的单二进制演示入口。首次启动会在当前目录的 `.ontoforge/dev` 创建持久数据目录；使用 `--data-dir DIR` 可覆盖目录。

```powershell
ontoforge serve --dev --bind 127.0.0.1:8080
ontoforge dev-token --server-config .ontoforge/dev/server.toml --sub alice --groups dispatchers
```

`dev-token` 的唯一输出是 token。将它作为 `Authorization: Bearer <token>` 发送到 `/api/health`、`/api/query` 或 `/api/actions/{action}`。服务会把 token key、独立 query signing key、SQLite edit store、ontology、scope 和 CSV fixtures 写入 data dir；路径只写到 stderr，不写 secret。

正常重启不会重新 seed 已有 definition，也不会删除 Action 修改、history 或 receipt。相同 `commandId` 的 Action 请求沿用 edit store 的幂等合同。

```powershell
ontoforge serve --dev --data-dir .ontoforge/dev --reset --bind 127.0.0.1:8080
```

`--reset` 必须和 `--dev` 一起使用，且 data dir 必须带有 Ontoforge marker。活跃实例、缺 marker 的非空目录、symlink 路径和 data dir 之外的路径都会被拒绝。reset 只删除明确的 dev assets，重新生成 auth key 和 query signing key，因此旧 token 会失效。
