---
status: accepted
date: 2026-11-03
---

# MCP Action 参数命名空间

## 决策

所有动态 MCP Action 使用 `{commandId?, validateOnly?, params?: {业务参数}}`。顶层只允许这三个字段，业务参数只从 `params` 提取，不兼容平铺双读。业务字段可以命名为 `commandId`、`validateOnly`、`params`。

`commandId` 仍是可选 string，正式执行缺失时沿用 ActionService 的既有错误；`validateOnly` 仍是可选 boolean，默认 false。有必填业务参数时 schema 必须要求 `params`；否则省略 `params` 等价于空 object。显式 null/数组/标量均拒绝。嵌套业务 required、unknown、类型、default 规则沿用已有领域合同，外层和内层均拒绝未知键。

## 理由与授权

原 schema 将同名业务字段覆盖为控制字段，dispatch 又删除这些字段，存在实际参数丢失。项目尚未发布，统一嵌套避免永久禁止合理业务名称及维护双读歧义。用户于2026-11-03授权主会话和Claude Code第二意见同方向时自动推进；只读第二意见为AGREE，主会话核实源码后一致接受。

## 验收与证据

以真实MCP工具调用证明同名业务值完整送达、控制行为保持、平铺被拒绝；单测覆盖嵌套schema及缺失/错误类型/未知字段。实施与动态证据归Phase7 SUMMARY和reviews，不以本决策代表实现完成。

第二意见原始证据：`target/phase7-claude-contract-review.json`（exit0；Claude Code配置模型MiniMax-M3.1-Flash[1m]）。
