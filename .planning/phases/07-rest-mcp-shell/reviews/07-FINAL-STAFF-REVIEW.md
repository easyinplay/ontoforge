# Phase 7 本轮增量 Staff / Security / Performance / API 审查

首次审查日期：2026-11-03。最终 T08 复签：2026-11-04。审查方式：只读静态审查，独立追调用链；没有运行 Cargo、服务或 commit。

最终状态：T08 / N3 实际业务并发 finding 与修复期 REST permit 回归均 resolved；E6 finding resolved。已独立复签冻结生产 blob，并亲读执行 worker 的 RED / GREEN 日志；原 T24 / CI / helper 范围结论保留。当前指定范围无未解决 finding，本报告不替代主会话最终真实服务与 CI 门禁。

## T08 / N3 扩展范围：MCP 执行生命周期并发

- 状态：RESOLVED（原分类 FIXABLE，confidence 100）；合同 `07-MCP-EXECUTION-ADMISSION-FIX.md`。
- 主会话真实 RED：`target/phase7-explain-server-tests-r2.log`，cap=1 时首 typed EXPLAIN 已实际进入上游并被 barrier 阻塞，第二请求收到 200 而非 429。
- 原实现根因：`context::admit` 在 SSE response headers 返回后强制 take business permit，rmcp 分派的业务 future 仍执行。旧 `call_tool` 仅 Action 分支尝试 take，存在与 middleware 抢夺窗口。
- 接受合同：业务 permit 由执行 future 的 RAII guard 持有；Action 把同一 guard 转交 host-owned task。MCP POST 先以 business 或唯一 control slot 限定缓冲/auth 资源，auth/owner 后精确分类取消通知；控制通道不授予普通业务执行。GET/SSE 不长期占 business，DELETE 控制通道仍受 auth/owner。
- 已完成 worker 冻结 context / mcp / lib / state 增量及负向测试复签；T24 session manager 未改，保留 E6 与既有已关闭 finding。

### T08 草稿发现：REST permit 被 Body extractor 提前释放

- 状态：RESOLVED（原分类 FIXABLE，confidence 100）；发现时即时通知 orchestrator 请求 Claude Code 第二意见，本 lane 未修改实现。
- 第一轮草稿 `context::admit` 移除外层 `permit_slot`，REST 分支仅把 `AdmissionPermit` 放 request extensions。`rest.rs:201` query、`:299` aggregate 不提取该 extension，最后参数为 `Body`。
- 精确调用链：锁定 axum 0.8.9 `handler/mod.rs:248` 构造 request 并交给最后一个 extractor；axum-core 0.5.6 `extract/request_parts.rs:170` 用 `req.into_body()` 提取 Body，消耗并丢弃 request parts / extensions。此时最后的 permit Arc 已 drop，尚未开始实际业务 handler。结果 REST→REST / REST→MCP 仍可越过 cap。
- 最小修复：REST outer 保留同一个 AdmissionPermit Arc 直至 `next.run` 完成，仅 drop 该 Arc、不 take 内部 OwnedSemaphorePermit；MCP 仍由 Parts / handler 保活，REST Action 现有 guard 转交路径保留。冻结修复与真实交叉竞争 GREEN 已核验。
- orchestrator 已收到 Claude Code AGREE（`target/phase7-rest-permit-lifetime-claude-review.json`），worker 的真实 REST→MCP cap 测试也得到 200 != 429 的 RED，确认并非仅静态推测。最终 `phase7-mcp-admission-green-r2.log` 中同一测试通过，finding resolved。

### T08 冻结增量静态复签

状态：静态接受，无新增 finding；实际 GREEN 日志已独立读取，两个 T08 finding 均 resolved。

| 生产文件 | 冻结 blob |
|---|---|
| `crates/ontoforge-server/src/context.rs` | `12de800b01837b3b8ec8e26b819c82d382e0ac82` |
| `crates/ontoforge-server/src/mcp.rs` | `7b154b0100f8e9433ab95061369d075cd8d1539e` |
| `crates/ontoforge-server/src/lib.rs` | `d14d825c8e60199fe983bd472fa4c0e66837c5e1` |
| `crates/ontoforge-server/src/state.rs` | `c6a2287eb87f6b06759a8c9a493e7cd8ca4faad0` |

- Staff critical：`context.rs:296` 只为 REST 保留 `_http_permit` Arc 至 response / timeout，不再强制 take。MCP request Parts 自身保 reservation，`mcp.rs:542` 首次 await 前唯一 take；`:624` Action 将 guard move 到宿主 task，其他分支局部 RAII 覆盖 handler 收尾。修复了原 SSE 头提前释放与实现中 REST Body extractor 提前释放两条路径。
- Security：`context.rs:52` POST 先占 business 或唯一 control reservation，auth/owner 在 `:484` 完成后才进入 `:509` body 分类。`:101` 从原 bytes 用 typed envelope、typed RequestId、rmcp params 与完整 message 交叉校验；重复 method/params/requestId、batch、id:null、伪造 header 不作为 cancel 特权。普通业务若只拿 control fallback 则 429。
- Security / concurrency：合法 cancel 若先持 business，会先取得 control 再替换 reservation；MCP outer 不持额外旧 Arc，旧 business 槽及时释放。精确 endpoint GET 不持长业务/control，带 session 的 DELETE 持 control，二者仍经认证/owner。子路径没有 control 豁免。`state.rs:39` control 槽固定 1，未增加产品配置或无界队列。
- API / Performance：使用实际 `ServerLimits` 的 max_body_bytes 与 `context.rs:321` 统一 deadline；body 缓冲前已有 reservation，最大同时缓冲受 business cap + 1 限制。原始 bytes 重建给 rmcp，未重写通知语义。解析/认证/超限/超时的早退通过 RAII 释放 reservation，业务超限维持 HTTP 429 / CONCURRENCY_LIMIT。
- 已读新增 integration blob `10090eb81b28be113519b5cfdbfd1393ce2fe64c`：REST→MCP、MCP→REST、满槽真实 cancel、SSE/DELETE、Action cancel/断连后继续持槽、畸形 JSON/过大 body/初始化错误后槽回收。unit blob `d4204f34d83518505c639ae4b0421a20327dddda` 使用真实慢 TCP body 验证唯一 control reservation 与 deadline 回收。T24 fixture callsite blob `f98955789069fed1e2a27f3d101f45033c666692` 仅随 admission state 参数更新。
- 已亲读 `target/phase7-mcp-admission-red.log`（MCP→REST RED）与 `target/phase7-mcp-admission-green.log`（第一轮 3 passed / REST→MCP 1 failed），不将文件名中的 green 误记为通过。`target/phase7-mcp-admission-clippy.log` 显示 all-targets 检查完成，worker 报 exit 0。本 lane 未运行 Cargo。
- 最终亲读 `target/phase7-mcp-admission-green-r2.log`：lib 25 passed（含 T24 与 slow-body）、准入 integration 4 passed、L2 defaults 10 passed（含原 typed EXPLAIN RED 与真实 61.34s expiry）、transport 11 passed；合计 50 passed，0 failed。transport 中主动制造 Action task panic 的测试最终为 ok，未将预期注入日志误记为测试失败。最终读取日志后再次计算四个生产 blob，均与冻结表一致；`git diff --check` exit 0。

## E6 扩展范围：发送前拒绝缺口

- 状态：RESOLVED（原分类 FIXABLE，confidence 100）；主会话真实 RED 证据 `target/phase7-server-security-retest.log`，合同 `07-E6-PREFLIGHT-FIX.md`。
- 根因已独立读取确认：原 `crates/ontoforge-server/src/mcp_l2.rs:93` 的异步 gate 在 allowlist / SQL / typed target 校验前于 `:103` 调用 `engine.status()`（修复前 blob `8aeab63cd468d6955203455c280dfb701e712b0c`），而 `TrinoEngine::status()` 发起 `SELECT 1`。因此同 session 撤销 allowlist 后的拒绝仍产生 1 次上游请求，违背发送前拒绝合同。
- 该缺口在基线 `e6ae310` 已存在，原 T24 增量没有修改此文件；当前动态验收确认后纳入最终修复审查，不能以超出原 diff 为由忽略。
- 接受的修复顺序：纯本地 policy / typed 参数 / SQL / format / defaults / allowlist preflight → 原 availability probe → 原 audit reserve → execute / describe。保留发送侧 fresh attestation 与无 target 的 list_catalogs 既有过滤语义。
- 前置独立核查：`crates/ontoforge-engine/src/printer/l2.rs:155` 的 typed resolver 只有本地 AST parse、defaults 合并、identifier quoting；`:123` 的 SQL validator 只有本地 parser thread，没有网络 I/O。`trino/mod.rs:456` 的发送侧 admission gate 与 `trino/l2_health.rs:95` 的 Mutex / proof 时间检查均为本地；`trino/mod.rs:516` 的 availability status 才会经 execute 发起 SELECT 1。`describe_tables` 仍汇入同一 execute fresh gate，不能用移动 availability 来代替发送侧检查。
- worker 冻结后已独立复签 `mcp_l2.rs` blob `667102e12decd2fae4381579202df92c71d0c3e2`；最终负向测试 `mcp_l2_defaults.rs` blob `186e4199883eaa1c93fadb766f2c74631308196b`。首轮测试 blob `8835d49b3c3c5c23b0fd0fed726ebc8a9f28c02d` 后仅扩充同一个测试的缺权限矩阵，生产 blob 未变。静态修复接受、对应动态日志 GREEN，此 finding resolved。

### E6 冻结增量独立复签

- Staff / Security：纯 `gate` 在 `mcp_l2.rs:93` 只查 scope/toolset。execute_query `:165`、explain_query `:172`、list_schemas `:195`、list_tables `:205` 先完成 SQL / format / target / allowlist 校验，再进入 `run_sql:126` 的 availability。get_table_schema `:226` 先 resolve，`:227` 才 probe。list_catalogs `:188` 沿用无 target、结果过滤语义。未发现本轮新增的可达提前发送路径。
- API / audit：availability probe 保留且仍在 reserve 前；合法调用继续一次 probe、一次现有 audit reserve 与原 execute/describe。错误映射、request_id、limits、结果过滤未改。本地拒绝优先于 availability 的错误顺序变化正是接受合同，未引入新的外部字段或 endpoint。
- Performance：只把原有 probe 后移，五个 SQL 入口共用 `run_sql`，metadata 入口调用同一 `ensure_available`；没有重复 probe、新增 SQL parse 或新增网络往返。
- 新负测 `mcp_l2_defaults.rs:203` 覆盖非法 SQL、非法 EXPLAIN format、缺失 defaults、越界显式与默认 target；`:249` 另以缺 SqlRaw、缺 Raw 分别直调六个 adapter。RecordingEngine 的 `status` 在 `:173` 累加，再于 `:278` 断言 22 次拒绝的累计 probe 为 0，同时检查 execute、describe 和持久 audit 均为 0。因为计数只增不减，该累计断言不能用另一个 case 抵消提前 probe。
- 真实同 session 测试继续以 endpoint `all_statements` 计数，`mcp_l2_defaults.rs:54` 在识别 user/tag 前累加，`:418` 对拒绝前后总数精确比较，没有排除 status 的 SELECT 1；expiry 测试 `:528` 使用同一口径。
- 已亲读 `target/phase7-e6-preflight-red.log`：新增负测以 `left: 10` / `right: 0` 的真实行为失败完成 RED，非编译错误。无新增 confidence >= 80 finding；本 lane 未运行 Cargo。
- 已亲读 `target/phase7-e6-preflight-green.log`：`mcp_l2` 2 passed、`mcp_l2_defaults` 9 passed、0 failed；包含同 session 撤权零发送、真实 proof expiry/recovery（61.38s）、相邻合法请求、取消与 audit。最终测试 blob 的扩大矩阵另见 `target/phase7-e6-six-tools-green.log`，1 passed、0 failed。日志由执行 worker 产生，本 lane 仅独立读取并核对生产/test blob。

## 范围与版本

- 基线 `e6ae310`，生产 HEAD `552556e7bf9324c996f347b0960a52a096cfc90f`；检查本轮 T24 生产增量、CI 缓存与 discovery 增量，以及当前未提交 server/CLI evidence helper。
- 原范围合同：`07-T24-DESIGN.md`、`07-E3-BRIDGE-DESIGN.md`；扩展范围合同：`07-E6-PREFLIGHT-FIX.md`。不重审 Phase 7 全量、既有 release 实现；testing 与 maintainability 由另外的独立 lane 负责。
- 已读 gstack `review/checklist.md`、`specialists/security.md`、`specialists/performance.md`、`specialists/api-contract.md`。只接收 confidence >= 80 的可执行 finding；如有则标记 FIXABLE / INVESTIGATE。原范围无满足该门槛的 finding；扩展 E6、T08 及修复期 REST 回归共 3 项已确认 finding 均 resolved，当前指定范围无未解决 finding。
- 已按 `planning-with-files` 恢复指定 phase 的 task_plan/progress/findings；resolver 返回 `D:\DataSpellProjects\ontoforge\.planning\phases\07-rest-mcp-shell`。本报告为本 worker 唯一写入，主计划由 orchestrator 单写。

下表是首次 T24 审查时与 HEAD 一致的历史 blob；E6 / T08 后续生效 blob 单独记录于上节，历史行号按对应历史 blob 解释：

| 文件 | 精确 blob |
|---|---|
| `crates/ontoforge-server/src/mcp_sessions.rs` | `892360952af2e3f1459ec8bd366811f525456c38` |
| `crates/ontoforge-server/src/context.rs` | `f4d09de9c9e42e824eda8fb34a62f1d8be6bbff8` |
| `crates/ontoforge-server/src/state.rs` | `3fa5c9a2d1a5924db6d466ca7ae9473ea1c7b476` |
| `crates/ontoforge-server/src/lib.rs` | `ca4cb7a8c51f60181ff223000cfe1fd2577af0ba` |
| `crates/ontoforge-server/src/lifecycle.rs` | `00b95093ead7d8eda33028f168658fdf9add9dc1` |
| `crates/ontoforge-server/src/mcp.rs` | `9f067c1eb7a82cb9b69136c2900a2d9e51ba31b4` |
| `.github/workflows/ci.yml` | `840df5c8a41238e4d1cb6a3c4c13c258ae7a9936` |

未提交输入按审查时工作树内容记录：

| 文件 | 工作树 blob |
|---|---|
| `ci/server-services.sh` | `ef44d35076303a05484dc2a0544ee58946aa4b46` |
| `crates/ontoforge-server/tests/support/security_evidence.rs` | `6d1d7a5d944cf8e8f2f1d3241a242bd574e45728` |
| `crates/ontoforge/tests/stdio_security_support/mod.rs` | `ac9c29d01bd494f990afc3311a36f723b960e1c7` |

## Staff critical pass

Pre-Landing Review: No issues found.

核验的攻击与故障路径：

- 容量与创建取消：`mcp_sessions.rs:240` 的 mutation gate 覆盖过期回收、最老项驱逐与 create。锁定依赖 rmcp 3.8.0 `session/local.rs:74` 在 map 写锁取得后同步 insert、spawn、return；wrapper 随后同步登记 pending entry。此段未发现成功插入真实 map 后独立的 pending await 窗口。
- 请求取消与初始化登记：rmcp `tower.rs:2164` 的 create → spawn_session_worker → initialize_session 顺序中，create 返回后到 wrapper `mcp_sessions.rs:280` 装入 slot 前没有异步等待点；slot 在 inner initialize 的首次 await 前安装。`InitializationGuard::drop` 在 `mcp_sessions.rs:49` 同步取消 token 并唤醒现有 reaper。
- close 中断与重入：`mcp_sessions.rs:171` 先取消 token，等待真实 map 锁时保留容量；取得锁后两张 map 同步删除。rmcp `session/local.rs:46` 先 clone handle 再释放读锁，之后才等待 worker；未发现 inner map 锁持有者反等 mutation gate。自然退出由 rmcp `tower.rs:1636` 调用已装配 wrapper 的 close，重复删除幂等。
- 驱逐后 bind：`mcp_sessions.rs:104` 使用 `get_mut`，不会重新插入；取消或过期 entry 被拒绝。`context.rs:370` bind 与 `initialization.disarm()` 间没有 await，因此成功绑定不留下独立可取消的 guard 窗口。
- shutdown 与 Action：`lib.rs:268`、`lifecycle.rs:53` 先按既定总预算等待宿主 Action，再取消读取并于 `lifecycle.rs:69` 关闭 session。`mcp.rs:616` 的 Action 仍由宿主 spawn_action 接收；本轮 session token 未接入 Action abort。`mcp_sessions.rs:212` 设置 stopped、停止并等待 reaper，再在 mutation gate 内清理 registry。

## Staff informational pass

NO FINDINGS

- `.github/workflows/ci.yml:24` 的 debug/incremental 配置仅作用于 test job；`:41` 使用独立 cache prefix，`:42` 禁止缓存 targets。增量未改变发布路径、认证 secrets 或工具链版本。
- `ci/server-services.sh:169` 新增的两个 PG 精确名称，对应 `postgres_http_it.rs:37` 与 `:75` 的 ignored tests。`:177` 新增的两个 Rust client 精确名称，对应 `contract.rs:154` 与 `:239`。既有 `run_ignored` 的 discovery 精确匹配与运行整套 ignored target 语义保留；此结论是源码接线检查，不代替 CI 执行结果。

## Security specialist

NO FINDINGS

- `context.rs:340` 在所有 MCP HTTP 方法进入 transport 前重新认证；`:352` 对 session owner 验证。`mcp.rs:109` 的 tool 授权 context 仍重取，`:124` 使用同一宿主 manager 验证 session。
- `mcp_sessions.rs:125` 在刷新 touched 前验证取消状态、TTL、owner 存在与匹配，未知、过期、跨主体访问无法延长 session 寿命。
- 已移除进程级 owner 单例，transport、middleware 与 handler 引用 `ServerState` 所持有的同一 manager；未发现本轮引入的旧全局 owner 旁路。
- 未提交 helper 的动态 SQL 仅用于受控 fixture 状态快照：server helper 对 identifier 与字符串进行对应转义；CLI helper 对表名 allowlist、identifier 字符集检查，并绑定 JSON key 值。未把 fixture identifier 组合误报为生产 SQL injection。
- CLI proxy 绑定 `127.0.0.1:0`，上游 URL 由 fixture 指定，使用参数数组启动 CLI，并禁用 redirect；本轮未新增生产网络或 shell 信任边界。

## Performance specialist

NO FINDINGS

- owner / expiry / oldest 扫描受默认 256 session 上限约束。mutation gate 只串行资源变更，不跨 handler initialize 或业务请求等待。
- reaper 使用弱引用、单一 task、合并 Notify；没有每次取消新增 task、无界 cleanup queue 或 tombstone 集合。
- `close_locked` 不等待可能满的 Close event queue；真实 map 锁内只做同步 map 删除，未发现新增阻塞 I/O 或等待 worker 的循环锁依赖。

## API Contract specialist

NO FINDINGS

- 本轮不改变 REST DTO / endpoint、MCP tool schema 或默认 modern 无 session 路径。legacy missing / mismatched owner 仍通过 `context.rs:355` 返回既有 404 / 403，初始化 bind 不可用仍返回 503。
- `07-E3-BRIDGE-DESIGN.md` 与 CLI helper 一致：legacy 模式仅由测试 gateway 的 correlated discover method-not-found 触发；默认 modern 分支透传 discover。关闭真实 session 后失败来自 host，未把模拟 gateway 声称为生产默认协议。
- 新的 session 关闭错误仅汇入宿主 shutdown failure；没有增加 HTTP 客户端必填字段或自动业务重试。

## 独立 adversarial 结论与证据边界

尝试反证容量逃逸、取消后的 pending 复活、错误 owner 延寿、close 反向锁死、自然退出绕过 wrapper、session cancel 中断已接收 Action，均未在本轮默认生产调用链建立可执行 finding。

rmcp 的握手 handler 等待不承诺任意第三方永久挂起 handler 的 join；生产 `OntologyMcpHandler` 沿用立即返回的 default initialize。此限制已在接受设计中说明，未将 registry 清零等同于任意 handler 已退出，也未把该边界重新包装为已证明的生产漏洞。

本 lane 未执行动态测试；主会话报告的 12 session + 11 transport GREEN 未作为静态结论前提。运行了只读 `git diff --check`，exit 0。TDD 不适用：本 lane 没有实现修改或新增测试，仅生成审查报告。

Adversarial recommendation：原 T24 / CI / helper 结论保留，E6、T08 及修复期 REST 回归在对应冻结 blob 与 GREEN 证据上 resolved；可以继续主会话最终真实服务与 CI 门禁，不将本范围复签替代整体验收。
