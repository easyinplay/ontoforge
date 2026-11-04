# L2 edit catalog 合同修复

## 范围与计划

- 2026-11-04，worker 负责 `mcp_l2.rs`、最小 MCP 接线、TrinoEngine 只读配置 accessor、server 回归测试。本文件为 `planning-with-files` 分配的独立 ledger，主会话拥有共享计划与状态。
- 读取 task_plan / findings / progress 与 Claude Code AGREE 后，按既定方案实施。跳过重复 brainstorming：主会话已锁定方案；此项为既有合同修复。
- [x] RED：真实 HTTP MCP，自定义 `private_edits` 被误授 allowlist，SQL/metadata 拒绝或过滤；分别计算所有上游请求与业务请求。
- [x] GREEN：从 TrinoEngine 的可信 ACL 配置读取 edit catalog，注入统一 L2 policy；无配置保持 fail-closed。
- [x] 回归：默认 editstore、正常 demo、无 Trino；相关 server suite、fmt、Clippy。
- 不修改历史 VERIFICATION，不执行 Docker / 完整 workspace；不提交。
- TDD 必跑；Warp 与 ralph-loop 未提供，不声称调用。

## 当前证据

- `McpL2Tools::gate` 和 `allowed_catalog` 写死 editstore，破坏自定义 deployment catalog 的硬排除合同。未证明数据泄露。
- TrinoEngine 持有不可变 L2Health.config，可提供只读 `l2_edit_catalog()`，避免另建配置副本。
- 现有 HTTP fixture 已区分全部 statement 与带 mcp tag 的业务 statement，且不会安装后台 ACL monitor。
- CodeGraph 不存在，直接读取源码。

## 命令日志

- 前置源码/计划/skill 读取、`git status --short`、`git diff --stat` 与 plan resolver 成功，退出码 0。两次 `rg` 使用 Windows 不支持的路径通配产生 `os error 123`，改为显式文件路径后成功；未改生产代码。
- RED：`CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test mcp_l2_defaults mcp_l2_custom_edit_catalog -- --nocapture`，退出码 101，0 passed / 2 failed；日志 `target/l2-catalog-red.log`。编译 1m26s，测试 0.26s。
- 失败是预期合同破坏：9 个 SQL/metadata 敏感请求全部成功返回，每个 `upstream=2, business=1`；catalog 列表包含 `PRIVATE_EDITS` 和 `private_edits`。上游 fixture 仅 ACL attestation 探针执行 deny，业务路径返回合成数据，从而隔离应用层 fence；此证据不等于真实环境数据泄露。
- RED 时生产代码无修改，server/engine diff 仅 `tests/mcp_l2_defaults.rs`。HEAD 81788e4 的变化只含其他 worker 的 Phase1 测试与文档。即将进行最小生产修改。
- `CARGO_BUILD_JOBS=2 cargo fmt -p ontoforge-engine -p ontoforge-server`，退出码 0，日志 `target/l2-catalog-fmt.log`。
- GREEN：与 RED 完全相同测试命令，退出码 0，2 passed / 0 failed，日志 `target/l2-catalog-green.log`。编译 1m34s，测试 0.28s。9 个敏感请求现在均在任何上游请求前拒绝，list_catalogs 只返回 demo；正常 read catalog 的六工具都成功。
- 回归：`CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --lib --test mcp_actions --test mcp_contract --test mcp_execution_admission --test mcp_l2 --test mcp_l2_defaults --test mcp_transport`，退出码 0，83 passed / 0 failed / 0 ignored，日志 `target/l2-catalog-server-regression.log`。目标计数依次 25 / 10 / 16 / 4 / 2 / 15 / 11；编译 7m28s，defaults suite 的 62.67s 包含真实 60 秒 attestation 过期测试。
- `CARGO_BUILD_JOBS=2 cargo fmt -p ontoforge-engine -p ontoforge-server -- --check`，退出码 0，日志 `target/l2-catalog-fmt-check.log`。
- `CARGO_BUILD_JOBS=2 cargo clippy -p ontoforge-engine -p ontoforge-server --all-targets -- -D warnings`，退出码 0，43.47s，日志 `target/l2-catalog-clippy.log`。
- `git diff --check -- crates/ontoforge-engine/src/trino/mod.rs crates/ontoforge-server/src/mcp.rs crates/ontoforge-server/src/mcp_l2.rs crates/ontoforge-server/tests/mcp_l2_defaults.rs`，退出码 0。

## 实现

- `TrinoEngine::l2_edit_catalog()` 直接读取私有不可变 ACL 配置，普通 engine / 无 ACL 返回 None。
- MCP 在已有 Enabled 检查后取 deployment catalog，缺失即 SOURCE_UNAVAILABLE，无 fallback。
- `McpL2Tools::gate()` 用可信 edit catalog 构造 policy。SQL validate 和 metadata filter / resolve 使用同一份 policy。
- 主会话已对当前稳定生产 diff 取得独立 Claude Code AGREE；全部相关 MCP suite 与 lint 门通过。

## 可信 catalog 校验核对

- `config.rs:572` 将 `trino.l2_acl.edit.catalog` 与配置的 `store.trino.catalog` 比较，不同则拒绝配置；`config.rs:827` / `is_catalog_name` 要求 store catalog 匹配 `^[a-z][a-z0-9_]{0,62}$`。
- 只有 ACL 配置、未声明 store.trino 的场景，配置解析本身不直接验证 probe catalog，但 attestation 仍保证 fail-closed：`trino/l2_health.rs:274` 调用 `acl_table_probe(&config.edit)`，失败返回 `InvalidProbe`；`printer/l2.rs:97` 的 `probe_identifier` 拒绝空值、长度 >128、任何非 `[a-z0-9_]` 字符，因此 catalog 必须是非空合法单段 identifier。
- MCP `call_l2` 先要求 `L2HealthStatus::Enabled` 才读取 accessor；新 accessor 不可改变健康状态，非法或缺失配置不能绕过该检查。未发现需要扩大修复范围的校验缺口。
- 主会话 Claude Code 对实际 diff 的第二次审查为 AGREE，原始结果 `target/milestone-gap-validation-review.json`；上述核对落实其可信 catalog 前提。

## 验收与交还

- 自定义与默认 catalog 均覆盖 SQL execute / explain、大小写、非 probe 表、metadata 显式目标、省略参数及 qualified override；每项拒绝均断言全部上游和业务上游发送数为 0，且不降级到 source engine、不预留 read audit。
- `list_catalogs` 在一次合法 catalog 枚举后过滤 edit catalog（含大小写变体）；正常 demo 六工具均成功，精确断言 6 个业务请求 + 6 个 status probe，metadata 分支仅发送 demo 对应 SQL。
- 无 Trino 时六工具均 SOURCE_UNAVAILABLE、source engine 调用不增加、read audit 为空。
- 新增 5 个真实 HTTP 测试；修改 3 个生产文件 + 1 个测试文件，无 state/config 副本，无新增产品配置，无提交。
- 完成受分配范围，归还 Cargo 执行权。完整 workspace / Docker / CI feature 补测由主会话最终门禁执行；本报告不代替最终 milestone 验收。
