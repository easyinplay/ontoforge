# Phase 7: REST & MCP Shell - Discussion Log

仅供人工审计；下游研究、规划和实施读取 `07-CONTEXT.md`。

**Date:** 2026-10-28
**Areas discussed:** MCP stdio 运行方式、serve --dev 重启的数据保留

## MCP stdio 运行方式

| Option | Description | Selected |
|--------|-------------|----------|
| 1 | 连接已启动的 serve，共享状态、权限与审计 | 是 |
| 2 | 独立启动 runtime，无需预先运行 serve，但需额外处理进程与状态协调 | 否 |

**User's choice:** `1`
**Notes:** 客户端使用前先启动 `serve`。

## serve --dev 重启的数据保留

| Option | Description | Selected |
|--------|-------------|----------|
| 1 | 默认保留 Action 修改和历史，需要时显式重置 | 是 |
| 2 | 每次重启恢复示例初始数据 | 否 |

**User's choice:** `1`
**Notes:** 本次用户答复完成该问题，不再重复询问。

## 工程审查与第二意见

Grok 咨询有界超时并返回部分正文；其健康、关闭和重试建议留在 `findings.md` 供核对。未将建议写成用户决策。

## Deferred Ideas

无。
