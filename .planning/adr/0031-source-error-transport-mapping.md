---
status: accepted
date: 2026-11-03
clarifies: 0030-source-drift-host-health
---

# Source 错误的 REST 与 MCP 映射

ADR-0030中的source拒绝HTTP503针对REST：返回HTTP503及安全ErrorEnvelope。

MCP `tools/call` 沿用既有rmcp传输合同，以JSON-RPC error及data中的稳定`code`、`apiName`、`request_id`表示失败；底层HTTP仍为200。HTTP200不能作为工具执行成功的验收依据。认证middleware的已有401/403/503拒绝不在本修复范围内。

两条路径必须保留相同领域错误类别和已有审计ID；被drift门禁拒绝时不发送对应上游业务SQL。MCP测试检查JSON-RPC error/code/ID及零发送，不强制改变HTTP transport。

这是既有协议的范围澄清，不新增错误机制。主会话核查现有MCP实现并取得Claude Code第二意见AGREE，按用户2026-11-03同方向自动推进授权接受。证据：`target/phase7-claude-mcp-drift-status.json`。
