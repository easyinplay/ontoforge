---
phase: 07-rest-mcp-shell
review_date: 2026-11-03
review_base: 12fcee8
review_target: frozen-working-tree-blobs
contract: RF-05
decision: .planning/adr/0029-mcp-action-parameter-envelope.md
status: clean
findings: 0
independent_review: true
tests_rerun: false
phase_acceptance: pending
source_blobs:
  crates/ontoforge-server/src/mcp_actions.rs: 3d7d770a16e58a6bfc56ecd02812cfa2f65a485d
  crates/ontoforge-server/src/mcp.rs: eefc5584a05b84baa6b611bcc0da94d8f78d4fe8
  crates/ontoforge-server/tests/mcp_actions.rs: dbec960fc4d395e2d373aa9fa92e8fa74d709b0c
  crates/ontoforge-server/tests/mcp_contract.rs: 7f4b14c6cde4963e38c99dd100e755a193681f63
  crates/ontoforge-server/tests/mcp_transport.rs: a94725fcd3de4f0a1f9bca47a99c424c8297ab04
  docs/api.md: 72951978a5a1988b4707b1a1578751fec647da5e
---

# RF-05 MCP Action 参数信封独立复签

结论为 clean，未发现需要修复的实现或验收问题。冻结实现满足 ADR-0029 的嵌套参数、控制字段隔离与拒绝平铺合同。本结论只覆盖 RF-05，不代表 Phase 7 整体验收通过。

## 审查边界

独立 reviewer 未参与该实现。读取 `git diff 12fcee8 -- <指定六文件>`、ADR-0029、`07-MCP-ENVELOPE-FIX.md`、相关完整函数和测试断言。读取源码前后均执行 `git hash-object`，六文件均与上述 owner 冻结值一致。blob 尚未写入 git object store，因此以哈希匹配的工作树内容读取，不把无法读取的 object 当作证据。没有修改实现、共享 planning、owner 报告，也未 commit。

## 合同与证据

| 验收项 | 核验结果 |
| --- | --- |
| 三个同名业务字段完整送达 | `mcp_actions.rs:30-65` 将业务 validators/schema 独立保留，外层控制字段不再写入业务 map。`mcp_contract.rs:556` 的真实 MCP 测试创建对象，再经 REST 分别验证 `commandValue=business-command`、`validateValue=business-validation`、`payload=business-payload`。 |
| 控制字段保留、无平铺双读 | `mcp.rs:589-601` 只从顶层提取控制字段，只从 `params` object 克隆业务字段。`:587` 的校验先于提取和 spawn。真实测试证明外层 validateOnly 生效、缺外层 commandId 返回 MISSING_COMMAND_ID、回执使用 `control-receipt`，平铺 delayFlight 参数返回 INVALID_PARAMETERS。 |
| required、optional 与 default | `mcp_actions.rs:66-88` 仅在存在必填业务参数时要求外层 params；`:119-139` 将缺失视为空 map，再按内层 required 校验。`mcp_actions.rs:175` 校验 schema 默认值和省略/空 object；`mcp_contract.rs:606` 真实调用验证二者可 validate，省略 params 正式执行后对象保存 `fallback`。 |
| 错误类型与未知字段拒绝 | `mcp_actions.rs:103-160` 分别校验外层键、控制值、params object、内层键和原领域类型。`:147` 单测与 `mcp_contract.rs:651` HTTP 测试覆盖缺参、业务值类型错误、内外层未知键、错误控制类型；optional HTTP 测试另覆盖 null、数组、boolean、string params。 |
| validateOnly、拒绝和 replay 无额外业务 commit | 同名字段 HTTP 测试先断言 validated=true，再查对象为 404；正式提交与 replay 回执相等。`tests/support/mod.rs:267-289` 的 finish_actions 实际查询 of_edit_log 并要求总数为 2（既有 seed + 一次正式提交）；纯拒绝路径调用 finish 要求总数为 1。这是实际计数断言，不以测试名称替代行为证据。helper 对 12fcee8 无 diff。 |
| scope/auth 与 Action 生命周期 | `mcp.rs:539,554-557` 仍先构造认证 context、检查当前 scope 的 catalog，再进行 Action 校验；`:628-641` 继续把固定 principal/scope/active 传入 ActionService。该文件唯一 diff 是业务 params 提取。`mcp_transport.rs` 只迁移 accepted Action fixture 的请求形状；对应取消、关闭、session owner、认证回归均出现在通过日志。 |
| 文档 | `docs/api.md` 明确新嵌套形状、同名业务字段、optional/default 行为、两层 unknown 拒绝与不兼容平铺。未更改 REST 请求合同。 |

## 亲读的动态日志

本 reviewer 没有运行 Cargo 或复跑测试，以下均为读取 owner 留存日志后的结论。

- `target/phase7-mcp-envelope-red.log`：3 个新增 schema/validator 测试真实断言失败，0 passed、3 failed，`EXIT=101`。
- `target/phase7-mcp-envelope-http-red.log`：3 个新增真实 MCP 测试失败，`EXIT=101`。其中旧平铺请求实际返回 validated=true，证明负向测试能够检出原缺陷。
- `target/phase7-mcp-envelope-green.log`：mcp_actions 9/9、mcp_contract 16/16、mcp_transport 11/11，共 36 passed、0 failed、0 ignored，`EXIT=0`。日志包含上述新增三项 HTTP 测试及已有认证、scope 撤权、取消和 shutdown 测试。
- `target/phase7-mcp-envelope-clippy.log`：server Checking/Finished 输出及 `EXIT=0`；具体 target 参数来自 owner 报告，日志本身不包含完整命令。
- 实际格式日志为 `target/phase7-mcp-envelope-format.log`，内容为 `EXIT=0`；任务中提到的 `phase7-mcp-envelope-fmt.log` 不存在。由 owner 报告描述为定向 rustfmt/diff check，极简日志本身不能还原完整命令；不将其称为独立复跑证据。

## 排除范围与记录

RF-01 source drift、API worker 后续改动、release 与完整 workspace 不在本次复签范围。按 planning-with-files 的 worker 所有权规则只新增本报告；主会话负责共享进度与最终状态。无新增业务代码，因此不新增测试、不重复执行 TDD。

Recommendation: Accept RF-05 because the frozen implementation and recorded 36-test tranche demonstrate independent control fields, complete colliding business values, and rejection of flat requests without extra business commits.
