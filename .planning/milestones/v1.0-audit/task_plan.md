# v1 milestone 审计与已知合同缺口收口

## Goal

独立核对79项v1需求的三源证据、跨阶段接线与当前验证状态；修复已确认的既有合同缺口，不新增产品范围。v1.0仅审计标识，不是版本发布授权。

## Tasks

- [x] 读取GSD audit/validate与历史phase状态，识别stale/Nyquist draft。
- [x] 撤回fa81ca0过早通过结论，恢复独立integration与三源审计。
- [x] Phase1 Nyquist：颜色环境行为补测，451定向GREEN；保留用户UAT观察边界。
- [ ] L2-03/MCP-07自定义edit catalog硬排除：实际RED、可信配置接线、六工具拒绝/过滤GREEN、独立review。
- [ ] TLS依赖门：修复openssl-only与Cargo查询失败的fail-open，hermetic反例与真实依赖图/双平台CI通过。
- [ ] 补执行现有feature测试（SQLite action_atomic 5例、PG source_guard 1例）和实际Iceberg 483/484证据，保留非零发现。
- [ ] 汇总INTEGRATION.md、REQUIREMENTS-AUDIT.md与逐项矩阵，替换不充分的初稿评分。
- [ ] 按实际缺口推进Phase2/3/5 Nyquist与Phase1–6历史验证重新核对；只运行缺乏证据的门。
- [ ] 对最终源码完成必需CI/受影响服务验证、恢复Phase7报告新鲜性，再生成最终milestone结论与状态。

## Current Phase

**Status:** in_progress
主会话负责决策/共享文档，worker实施；Cargo独占且jobs2。生产变更前必须先有RED。实施灰区回流主会话并取Claude Code第二意见。

## Next Step

执行自定义edit catalog硬排除的真实HTTP MCP RED→GREEN。

## Constraints

- PR7/8保持Draft OPEN，不合并、创建Release、发布npm或自动归档。
- 不改旧报告指纹来冒充重新验证，不将unknown/未运行改列manual-only取得绿色。
- 自动推进依据用户授权：Claude Code AGREE/同向REFINE无需再问，实质分歧才请用户裁决。
- Warp/ralph-loop未提供；使用明确验收闭环但不声称调用它们。

## Decisions

- L2修复为既有合同内的bounded修复；可信deployment config注入McpL2Tools并统一SQL与metadata排除。全面迁移到另一service会扩大改动，暂不采用。
- 用户授权覆盖brainstorming的重复确认要求；主会话已明确方案且CC AGREE，继续TDD。office-hours/CEO跳过：这是已定义合同的bug修复。
