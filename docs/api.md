# HTTP API

`ontoforge serve --server-config FILE --bind 127.0.0.1:8080` 启动认证 HTTP 宿主。
当前端点是 `GET /api/objects/{object_type}/{pk}`，access token 通过
`Authorization: Bearer TOKEN` 发送。只有显式指定 `--dev`，且配置了
`[auth.dev] key_file`，宿主才接受开发 token。

## 持久查询签名密钥

宿主要求服务配置提供可信文件引用：

```toml
[query]
signing_key_file = "keys/query-signing.key"
```

路径相对配置文件解析。文件保存 32 bytes 至 64 KiB 的原始字节，不解码为文本，
不移除换行。该 key 独立于 OIDC 凭据和开发认证 key。文件缺失、不可读、过短或
过大时，宿主不会启动监听。其他 CLI 命令仍接受不含 `[query]` 的配置。

在版本控制之外的私有目录中一次生成 key。使用仓库中的开发配置时，从仓库根目录
运行以下 PowerShell 命令：

```powershell
$directory = [System.IO.Path]::GetFullPath('target/ontoforge-dev')
[System.IO.Directory]::CreateDirectory($directory) | Out-Null
$path = [System.IO.Path]::Combine($directory, 'query-signing.key')
$file = [System.IO.File]::Open($path, [System.IO.FileMode]::CreateNew)
try {
    $key = [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32)
    $file.Write($key, 0, $key.Length)
} finally {
    $file.Dispose()
}
```

`CreateNew` 拒绝覆盖已有 key。将目录 ACL 限制为服务账户可访问；Unix 上使用
私有目录，并将文件权限设为 `0600`。命令不打印 key 字节。不要把 key 写入 TOML、
命令行参数、HTTP 请求或日志。

正常重启复用同一文件，已有 `_version` token 才能继续验证。替换文件会使旧 key
签发的 token 失效。宿主不创建或轮换该 key。开发环境自动初始化由计划 `07-12` 交付。

## 读取与关闭

响应包含逻辑对象行及其 `_version`。查询预留审计项后，`x-request-id` 与持久读取
审计 ID 一致。认证与早期校验错误使用服务端生成的 transport ID。错误包含
`code`、`apiName`、`request_id`。客户端提供的 principal、roles、scope、limits 会被拒绝。

Ctrl+C 停止接收新连接，等待活跃请求结束，并显式 drain 读取审计。监听和 audit drain
失败返回非零退出码。启动前使用既有 CLI 命令准备 definition、scope 和持久 edit store。
完整开发数据 seed 与 reset 属于计划 `07-12`。

## MCP HTTP 与 stdio

`serve` 同时提供 `POST /mcp` 的 Streamable HTTP MCP endpoint。stdio 客户端必须先
启动同一份 `serve`，然后运行：

```powershell
$env:ONTOFORGE_TOKEN = '<token>'
ontoforge mcp --server-url http://127.0.0.1:8080/mcp
```

也可以用 `--token-file FILE` 读取 token。token 不应放入命令行参数；stdio bridge
只把 JSON-RPC frame 写入 stdout，诊断写入脱敏 stderr。`server-url` 必须是没有
userinfo、query 或 fragment 的 `http(s)` URL，并以 `/mcp` 结尾。

stdio bridge 是现有 HTTP host 的认证转发层，不会创建独立 edit store、engine 或
数据库。HTTP transport 禁止 redirect，并关闭 expired session 的透明初始化；连接、
认证或 session 错误直接返回调用者。请求和通知保留 JSON-RPC `id`，取消通知沿同一
session 转发，stdin EOF 会停止 bridge 并等待服务端清理。

REST 与 MCP 共享同一主体认证、scope、Action receipt、command result、history 和
read audit。写请求的显式 `command_id` 是唯一重试边界；stdio 或 HTTP 断连不会自动
重放 Action。MCP 的错误可能以 HTTP 200 携带 JSON-RPC `error` 返回，调用者必须同时
检查 HTTP 状态和响应 envelope。
