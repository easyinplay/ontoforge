# rmcp shell spike

日期: 2026-10-28。研究专用独立 Cargo workspace，不修改业务 crate 或根 Cargo.lock。

## 步骤与验收

1. 从官方 docs/registry 核查 rmcp、axum 与 transport features。
2. 先编写失败测试，再实现最小动态工具 handler；两个 principal 的 list_tools 不同，Action JSON Schema 来自运行时定义；直接调用未授权工具必须拒绝。
3. 在真实 listener 验证 HTTP request extensions 传递 principal、每请求认证与跨主体 session 拒绝。
4. 编译 stdio 到既有 HTTP 服务的桥接方向及取消/关闭 API；能实测的生命周期用有界超时断言。
5. 运行独立 cargo test/check，保存真实日志；明确区分已运行、仅编译、仅文档核查。

ralph-loop 与 Warp 未安装，本 spike 使用 PowerShell 的有界研究、验证、修正、复验。COMPLETE 仅在证据落盘后报告。

## 结果

最终 `cargo test --locked`：4 unit + 1 真实 stdio 子进程测试通过，0 失败。见 `final.log`。

- `red.log`：空动态工具列表导致预期失败，0 passed / 1 failed。
- `green.log`：初始动态列表、授权/session、真实 stdio 桥接及 EOF 关闭，共 3 passed。
- `transport.log`：新协议请求缺少 `Mcp-Method` 被真实返回 400；HTTP 取消测试通过。
- `transport-fixed.log`、`final.log`：修正 header 后 5 passed，stdio 测试包含取消到 HTTP handler 及 EOF 有界退出。
- `numbers.log`：Node 24 原生 lossless JSON 编解码可行性探针，不等于完整 TS client。

复现：

```powershell
cargo test --manifest-path .planning/phases/07-rest-mcp-shell/spikes/rmcp-shell/Cargo.toml --target-dir .planning/phases/07-rest-mcp-shell/spikes/rmcp-shell/target --locked
node .planning/phases/07-rest-mcp-shell/spikes/rmcp-shell/numbers.cjs
```

限制：这里的 token、Action metadata 和 scope 是专用 fixture，未接业务 JWT/DB/ActionService；未证明 PG 快照、业务写入 drain、真实 Trino health、完整 OpenAPI/TS 生成和三平台 release。session owner map 只用于可行性测试，正式宿主必须有删除/过期回收与容量限制。raw transport pump 的 64 个发送上限也是 spike 边界，正式桥接须区分普通请求与取消控制通道。
