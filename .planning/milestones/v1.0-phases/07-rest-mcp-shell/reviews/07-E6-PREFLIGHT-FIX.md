# E6 发送前拒绝修复合同

2026-11-03，状态：真实RED确认，设计接受待GREEN。属于07-08/16既有零上游发送合同，不是新功能。

## 证据

target/phase7-server-security-retest.log：mcp_l2_same_session_revoke_defaults_and_qualified_zero_send在allowlist撤销后endpoint总请求16!=15。worker及主会话亲读mcp_l2.rs gate先engine.status、后resolve_table；TrinoEngine::status执行SELECT1。此请求是被测调用同步产生，不能作为后台monitor排除。HTTP3/3、MCP actions10/10、contract16/16、L2其余7项（含真实61秒proof expiry/recovery）通过。此项为真实实现RED，非测试fixture修正。

## 决策

Claude Code REFINE支持A，原文target/phase7-l2-preflight-claude-review.json。保持同方向自动推进：纯scope/policy授权 → 本地typed参数/SQL/format/defaults/allowlist检查 → availability探测 → 既有audit reserve → 实际execute/describe。把异步gate拆成纯policy与availability，后者可在run_sql共同入口执行；get_table_schema在resolve_table之后执行同一availability函数。六工具本地拒绝均不得发status请求。

不接受排除SELECT1的计数弱化；不删除status或换缓存值，避免改变availability行为。typed resolver是纯本地函数，可以保留；CC笼统的“不得调用resolver”仅对有上游副作用者适用。无需新增PreparedOperation枚举/通用编排框架，现有run_sql与唯一metadata入口足以集中availability。list_catalogs无target，保持原空结果过滤合同，不顺便改空allowlist语义。

保留现有拒绝是否留audit的合同（本地预检拒绝不新增audit）、成功/执行错误最多一条既有audit、request_id、安全错误映射/limits、发送侧fresh attestation。Action不变。

## Ownership和验收

server_security_evidence增加拥有src/mcp_l2.rs及原tests/mcp_l2_defaults.rs/必要既有mcp_l2.rs测试；其他文件边界不变。已读TDD，保留已实际失败日志作为RED，最小GREEN覆盖当前case及非法SQL、非法format、越界defaults/qualified的status+execute计数0，相邻合法/expiry/audit/cancel仍绿。完成server相关Clippy/fmt后归还Cargo，主会话请求独立增量复签及最终真实服务。
