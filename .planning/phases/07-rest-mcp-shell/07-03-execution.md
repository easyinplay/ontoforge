# Phase 07 Plan 03 执行台账

## 2026-10-29 验收修复

- 基线：`0a84fee3d9a69bd1777f35f2f580a5ddc1efb1b1`，`gsd/phase-07-rest-mcp-shell`，根目录已核验。
- 既有 complete 记录不构成验收：路由字符串测试、helper 测试、编译错误 RED 均无效。下方历史记录保留用于追溯。
- 当前：六个指定真实验收测试、三个tracer、13个Query audit回归通过；workspace test最终1279 passed/0 failed/107 ignored。业务修复已提交，SUMMARY与自检收口中；all-features Clippy仍有三项既有Action错误。
- 已读取 GSD 执行/TDD、planning-with-files 与 superpowers TDD 规则；本执行者只维护本台账和证据，不写主会话状态文件。
- `AGENTS.md`/`RTK.md` 不存在于项目根，遵循派发中提供的规则。无 `.codegraph/`，无需 CodeGraph。
- Warp 与 `ralph-loop` 未暴露，不声称使用。未触发新功能战略讨论；原 PLAN 已锁定范围，新增 API 灰区向主会话回流。
- 待主会话决策：list/aggregate audit ID 不返回调用方，HTTP request_id 与 audit ID 对齐需最小 Query API 扩展。
- 主会话已批准 A 型兼容 audited wrappers（`9279dae`）；新增 query 文件所有权按最新 PLAN。
- 03-1：三个真实 REST 测试初轮通过，证明既有操作/排序/snapshot 行为；不为这些 GREEN 行为制造 RED。
- 03-1 修复 RED：`rest-audit-red.json` 原生 nextest JUnit，实际 HTTP query ID 未匹配 durable query.list audit ID。测试执行至预期断言，非编译/fixture 失败。
- 03-1 追加边界：实际 9 MiB 对象产生短页、`truncated=true`、`hasMore=true`，后续通过 cursor 延续读取。
- 03-1 GREEN：`rest-green.json`，指定三个真实用例全部通过，0 ignored。
- 03-2 RED：`http-framework-red.json`，非法 UTF-8 path 返回纯文本，统一 envelope 断言失败；非 fixture 或 JSON parser 错误。
- 主会话批准 request-scoped Engine adapter 仅按 EngineError 类型记录 Timeout/Cancelled，普通 Source 映射保留；HTTP timer 明确 deadline，Drop 审计保持原语义。
- 03-2 追加 RED：`http-concurrency-red.json`（第33个阻塞请求未及时拒绝），`http-fault-red.json`（typed Timeout 被折叠为503而非504）。两条真实HTTP断言获 RED_EVIDENCE_OK。
- 主会话批准 observed wrappers：reserve 成功后同步回调 ID 一次，失败不回调，保持原 guard；REST 每请求 ID 槽用于 deadline 与 cancelled audit 关联。
- 03-2 GREEN（分段证据）：`http-admission-fault-green`（真实framework/32并发/typed fault），`http-budget-boundaries`（rows=3/2、精确decoded bytes、UTF-8 bytes与body等值/越界），`http-deadline-cancel`（pk/list/aggregate deadline ID匹配cancelled audit；真实TCP断开drop）。完整gate仍待运行。
- metadata脱敏状态另获 `rest-metadata-status-red`，字段只允许 available/unavailable；不包含后端reason。
- 取消边界：当前middleware用于独立REST读取；后续Action接线必须保留Action拥有的依赖与生命周期，不能把drop独立查询策略机械用于已接受Action。
- 独立审查追加405缺口：`http-method-red-junit` 获 `RED_EVIDENCE_OK`，补 `method_not_allowed_fallback` 后真实 GET query / POST ontology 返回统一405，`Allow: POST` / `Allow: GET,HEAD` 保留。
- 独立审查追加PK预算缺口：`http-pk-budget-red` 获 `RED_EVIDENCE_OK`，editOnly Airport 的ObjectRow字节等值成功、少1byte原先404。REST仅将truncated映射既有400 COST_EXCEEDED，显式断言同ID及Query原审计success/0，主会话已批准不扩展finish/status合同。
- 最终定向gate：`rest-final` 3 passed、`http-final` 3 passed、`tracer-final` 3 passed、`query-audit-final` 13 passed，均0 ignored；`fmt-final`、`server-clippy-final` exit0。
- Stub扫描：本次修改的生产/测试文件未发现 TODO/FIXME/placeholder/coming soon 等占位；新增fixture空容器用于真实查询输入/错误注入，不是交付stub。威胁边界沿原REST输入、Engine与audit，未引入未登记新网络端点或schema。
- 资源中止：`workspace-tests-final` 在编译/链接阶段因15个并发link造成严重换页，未运行测试。主会话批准后通过CIM核验本次PowerShell PID64880 -> Cargo PID38284/39320命令，再只结束其进程树；保留exit1原日志，标记resource-aborted，非测试失败。后续重跑仅子进程设置CARGO_BUILD_JOBS=2，不改全局配置、编译flags或target产物。
- `workspace-clippy-fixed`：exit101，仅既有 `crates/ontoforge-action/tests/idempotency_pg.rs:242/257/398` 三处 `large_futures`（16464 bytes），按范围边界不修改Action。首次workspace Clippy检出本次observer测试102行，移出局部imports后已无本次lint。
- TLS负向依赖检查：`tls-native-final` / `tls-openssl-final` 各exit101且报 package ID did not match any packages，含义是lock依赖图不存在native-tls/openssl-sys，不把非零码写成命令通过。
- 证据脚本只增加空日志初始化，以保留静默成功命令的原始零字节log；该可逆工具记录修正不属业务逻辑，跳过单独TDD，已通过实际fmt gate验证。
- `workspace-tests-jobs2`：实际PowerShell子进程 `CARGO_BUILD_JOBS=2`，原命令 `cargo test --workspace --locked -- --test-threads=1`；UTC04:06:49.6138073至04:22:52.7004087，exit0。121组结果累计1279 passed/0 failed/107 ignored/0 filtered，包含doctests；忽略项与default-feature下0 tests的PG套件均不计通过，原始log可逐条追溯。
- 最终业务提交：`93fad96`。独立审查 `reviews/07-03-ACCEPTANCE-AUDIT.md` 已由主会话确认metadata/405/PK预算三项resolved；共享STATE/ROADMAP/REQUIREMENTS和跨phase缺陷登记归主会话维护。

## 范围

- 2026-10-29 接续：当前状态为验收修复进行中，旧任务表和实际执行结果属于历史记录，不作为本次完成证据。
- 独立审查发现405缺少统一envelope，新增真实HTTP断言并先运行RED；保留Allow header。预算测试拆分helper处理Clippy，不更改断言含义。

- Task 03-1：扩展真实 REST 元数据和完整 L0 查询
- Task 03-2：固定拒绝响应、预算与最终模块职责
- 允许修改文件严格遵循 `07-03-PLAN.md` 的 `files_modified` 白名单；另允许创建本台账、`07-03-evidence/` 与 `07-03-SUMMARY.md`。

## TDD 适用性

- 两个 task 均为核心 API、授权、HTTP contract 与错误映射，强制 RED -> GREEN -> REFACTOR。
- RED 必须由真实行为测试触发，编译失败、0 tests、假断言不计入。
- `ralph-loop` 与 Warp 当前不可用，执行中不声称使用 completion promise。

## 任务记录（历史失效记录）

| Task | 状态 | RED 命令/结果 | GREEN/REFACTOR 命令/结果 | Commit |
|---|---|---|---|---|
| 03-1 | complete | `cargo test -p ontoforge-server --lib --locked rest_ -- --nocapture` 首次 RED：`REST_ROUTE_PATHS` 未接入测试作用域 | GREEN/REFACTOR 通过；tracer 三项真实 listener 用例通过 | `66c6a08` |
| 03-2 | complete | 独立 `http_contract` 合同测试已建立并先于实现运行 | 3 个结构化错误/UTF-8/request_id/budget 用例通过；workspace 回归通过 | `989535d`, `0ef3ba1` |

## 验证矩阵

- `cargo test -p ontoforge-server --lib --locked rest_ -- --nocapture`
- `cargo test -p ontoforge-server --test http_contract --locked -- --nocapture`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked -- --test-threads=1`
- TLS gate 与 PG/Trino/private ignored 项真实执行状态逐项记录，未运行不得标记 passed。

## 实际执行结果（历史失效记录）

- `cargo fmt --all -- --check`：passed。
- `cargo clippy -p ontoforge-server --all-targets --all-features --locked -- -D warnings`：passed。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：既有 `ontoforge-action/tests/idempotency_pg.rs` 三个 `large_futures` 失败，非本 plan 文件。
- `cargo test --workspace --locked -- --test-threads=1`：passed；外部 PG/Trino/private fixtures 按环境 ignored。
- `cargo tree -i native-tls --workspace --locked`：无 `native-tls` package。
- Warp/`ralph-loop`：当前环境不可用，未声称运行 completion promise。
