# 独立集成审计验收记录

## 授权与方法

- 日期：2026-11-05；目标 HEAD：060019fe52005c6b85ae5c5f59da9868f464e579。
- 仅允许写 INTEGRATION.md 与本目录 integration-final-* 新证据；保留其他 worker 的工作区变化。
- 已读 agent-skills-bootstrap、planning-with-files SKILL.md、task_plan.md、findings.md、STATE.md、REQUIREMENTS.md；gsd-tools query agent-skills gsd-integration-checker --skills-only 成功，未返回配置技能。项目不存在 .codegraph/、.codex/skills/、.agents/skills/。
- 主会话已持久化计划，本 worker 不改共享计划或状态。纯审计，不修改业务实现，不执行 TDD；逐项源码追踪替代不可用的 ralph-loop/Warp，不声称调用它们。
- fa81ca0 的通过结论已撤回；旧 INTEGRATION.md 只用于确认既有 B01-B12/F01-F09 编号，不作为当前接线证据。
- git diff 8d9d8c6 HEAD -- crates clients ci .github Cargo.toml Cargo.lock schemas examples tools 无输出；限这些目录/文件的源码一致性已查证，不代表动态复验。
- 当前只运行只读 PowerShell、git、rg、node 查询与 apply_patch 文档写入。未运行 Cargo、Docker、commit、push、网络或服务。

## 发现登记

- 已亲读生产 mcp.rs::call_l2：从 Enabled dedicated engine 读取 l2_edit_catalog，缺失即拒绝，再注入 McpL2Tools。
- mcp_l2.rs::gate 将 edit_catalog 写入 L2Policy，execute/explain 经 validate_l2，metadata 经 allowed_catalog/resolve_*；allowed_catalog 对 edit catalog 与 system 大小写不敏感排除。
- mcp_l2_defaults.rs 已存在自定义/default catalog 上游零发送与 metadata 过滤断言。本轮未执行，不能记为新 GREEN。
- L2Service 在 query 测试中有构造，生产 MCP 使用独立 McpL2Tools；这是已确认的替代实现，不将未消费的 service 误写成生产调用链。
- 已追到 validate --check-cardinality → compare_bindings → check_cardinalities → engine cardinality_check，与原 PK 检查为不同分支。
- 初次目录探测不存在；两次 apply_patch 经 bat 管道/参数调用失败且未写文件，现改为直接调用 bat 指向的 codex.exe --codex-run-as-apply-patch，仍使用 apply_patch 引擎。

## 逐项闭环

- [x] B01-B12：12/12 静态 WIRED，0 BROKEN，0 新 BLOCKER。
- [x] F01-F09：9/9 静态 WIRED，动态重跑 0；不签动态 E2E passed。
- [x] 八类实际 REST 路径均有 Rust/TS 消费者；7 类敏感 REST 操作与 MCP 挂载接鉴权，health 有意公开。
- [x] REQUIREMENTS 与报告逐项 ID 集合相同：79/79，缺失/多余/重复均 0；74 项有接线路径，5 项单阶段验收标 N/A。
- [x] INT-01 关闭依据为当前生产 accessor → adapter → SQL/metadata policy；07.1/07.2 有真实消费者和历史补证，不沿用撤回报告结论。
- [x] 保留 Phase01 human_needed、Phase02 gaps_found、ENGINE-05 SQL Server Partial；7 个 WARNING 的限制与归属写入报告。
- [x] git diff --check 针对报告返回 0；正文无占位符、em-dash 或 surrogate emoji。
- [ ] 动态 owner 另行复验当前源码；主 auditor 汇总最终 milestone 判断。

### 操作问题与处置

- Windows bat 管道/参数传递 apply_patch 各失败一次，均未写文件；直接调用 bat 指向的 codex.exe --codex-run-as-apply-patch 成功，仍由 apply_patch 引擎编辑。
- 一次完整替换命令超 Windows 命令行长度，进程未启动、原报告未变；随后分为正文、流/API、需求表三次最小 patch 成功，没有改用其他写入工具。
- 初始目录探测不存在；查找阶段误用过简写的 auth/runner/prepare 路径，rg 报不存在，已通过文件枚举和实际模块路径纠正，未据此判缺实现。
- 一次 emoji 字符区间正则无效，改为 em-dash 字符检查和 surrogate 分类检查；两项均 0，不把失败正则计为通过。
- 限制内新证据为本文件与 integration-final-provenance.json。归档索引只读，未恢复/改写旧证据；没有触碰他人 FINAL-REVERIFY 或共享状态。

最终报告是验收/接线判断的唯一正文，本文件仅记录方法、检查结果与交付边界。
