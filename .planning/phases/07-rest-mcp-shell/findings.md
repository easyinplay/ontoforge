# Phase 7 findings

- 2026-11-03 正式helper回归新增真实RED：Windows child exit256经Git Bash低8位截断成为0，导致cleanup误成功。沿既定“任何非零不能吞”合同增加stop的shell-status模式，state和原生接口保留完整退出码，Bash模式将所有非零归一为1并输出真实code；无需新产品/API决策。

- 2026-11-03 用户明确批准方案A。正式边界：Python标准库Windows helper仅供CI启动/停止原生server；隐藏私有console、验证进程身份和console成员后发送CTRL_C_EVENT，真实exit0才成功，强杀/超时/身份错误非零。复用已证实probe机制，禁止新增Rust unsafe/公开shutdown API。本任务涉及进程隔离和错误传播，使用TDD；配置/文档仅随真实回归验收，不另写镜像测试。既有scope已批准，不重复战略/Phase讨论。

- 2026-11-03 Windows收尾方案第二意见：Grok首轮180s无输出已停止；精简verbatim第二轮正常exit0，记录`target/phase7-grok-windows-cleanup-second-20261103.log`。建议A：隔离隐藏console传递真实CTRL_C_EVENT，避免CREATE_NEW_PROCESS_GROUP使Ctrl+C disabled；不使用CTRL_BREAK替代ctrl_c，不将强杀137当成功。仅采纳与本地probe一致的机制，Grok对MSYS内部实现的泛化不当作独立证据。可丢弃`target/phase7-private-console-probe.py`实测2/2：console成员严格为server+helper，退出码0、端口关闭、无强杀，耗时1.363s/1.362s。正式接入需保留进程身份（PID+创建时间/句柄）与失败传播；B为补齐WSL运行Linux但不覆盖Windows原生缺口。已按用户第二意见约定提出A/B，等待路径选择，尚未改tracked实现。

- 2026-11-03 本地完整服务脚本收尾失败根因已最小复现2/2：Git Bash启动Windows原生`serve --dev`，`kill -INT`返回0但30秒后health仍200，最终`kill -KILL`后wait137，按现有合同传播exit1。原credential scan单跑exit0，四本轮Docker容器已清理。复现入口`target/phase7-signal-probe.sh`及`target/signal-probe.EQ9175/`、`target/signal-probe.LvFeAT/`；没有向共享console广播。候选为隔离隐藏console helper或迁移本地WSL验收，尚未选定；WSL Ubuntu-26.04当前缺Linux Cargo/Node/jq，未发现Docker socket。完整CI均通过，不将此脚本失败误写为业务测试失败，也不吞掉137。

- 2026-11-03 最终澄清：generation边界仅约束PendingProbe归属确认；Engine::drain_cancellations仍等待该实例全部已登记取消任务，不新增分组tracker或第二种drain。旧handle不会仅因新generation pending存在而判失败。PG测试Clippy收尾限于局部长场景注解、task类型别名与折叠条件，保留断言和运行语义；无新业务行为，跳过新增TDD，由Clippy及真实PG suite验收。

- 2026-11-03 Grok第二意见正常返回并确认monitor recovery方案：用当前状态谓词替代一次性sticky failure；普通QueryGuard取消失败继续sticky，monitor PendingProbe按generation恢复。drain只等待engine任务并确认`pending.generation <= handle.generation`为空；新generation不阻塞旧handle。失败必须在tracked task内先写入PendingProbe再完成，enable检查与pending写入需同锁/原子化，deadline耗尽保持monitor disabled并报告失败。该方案不构成持久化跨进程保证，当前合同由Trino自身TTL与下次启动probe承担，不能宣称进程崩溃后债务持久化。
- transport旧DELETE合同锁定：无token 401，owner不符403，owner缺失/过期404，禁止静默重新认领；不把新增全transport认证误报为原404回归。
- 2026-11-03 stdio真实测试额外确认旧版本local initialize被重复转发已初始化远端，2025-03-26与远端2025-11-25冲突。纳入CR-03最小本地握手处理，独立建立旧新版本行为RED/GREEN；既有ID取消成功不冒充跨版本握手证明。monitor子deadline取消shutdown future时需保留其cleanup handle，最后总deadline排空覆盖该实例与Drop重试；独立复签已提醒且两lane协作处理。
- 2026-11-03 tracked cleanup窄扩展至trino/l2_monitor.rs：monitor使用独立monitor_ontology实例，必须由L2AclMonitor持有并在后台任务join后drain其取消任务，避免只排空host core/L2而漏掉probe实例。复用既有monitor shutdown预算与错误传播，不改全局Engine选择结构。
- 2026-11-03 原子Action准入里程碑：context registry以稳定store Arc key识别，spawn与stop在同一mutex检查/登记，REST与MCP采用同一接受边界。四目标26项回归通过后记录小提交；该结果不代表之后新增10秒分段预算/engine drain的最终验证。
- 2026-11-03 shutdown内部预算锁定为总10秒：Action及read/HTTP截止5秒、audit截止6秒、monitor截止8秒、core/L2并行DELETE drain截止10秒，保留原Action的5秒窗口。无额外sleep；前段失败仍尝试后段有界清理并保留首个错误。此为可信实现默认值而非新SLA，transport同步docs/dev.md。
- 2026-11-03 关闭顺序以07-06-PLAN第32/51/160行优先：stop admission→accepted Action及其依赖→独立read/SSE取消→audit→monitor。修正Staff/Grok先取消独立读的建议；stop与Action注册在同一锁下原子执行，禁止迟到Action绕过drain。Trino QueryGuard::Drop现为detached DELETE，新增per-engine tracked cancellation及Engine默认drain接口，使shutdown可在总budget内等取消任务结束；保持原query Timeout/Cancelled语义与实例隔离，不能把spawn视为已完成cleanup。L2 lane增补engine/lib+trino/cancel/mod及行为测试，transport接收尾调用。
- 2026-11-03 Grok第二意见正常exit0（`target/phase7-grok-review-opinion-20261103.log`）：支持完整edits、独立L2能力、全transport认证、remote handle.id取消、有界关闭。采纳多edit/zero-edit/缺失能力/跨主体/取消隔离/真实SSE回归；不采纳把history强行等同receipt（违反当前scope遮蔽），也不采纳把health与执行的L2 engine割裂（二者应共享同一受限实例）。L2具体实现采用现有sidecar持`Arc<TrinoEngine>`，setter拒绝普通ontology engine，避免无必要的类型擦除适配器。
- 2026-11-03 workspace编译回归最小修复：history_masking::Ask::run参数补`RowValues + Sync`以匹配既有HistoryRequest合同；只改测试类型，不新增业务行为，跳过新TDD，运行该target原有测试验证。
- 2026-11-03 修复合同澄清：07-04明确要求原receipt/time与完整调用，07-10要求客户端保留receipt；因此CR-05采用完整items[].edits[]，统一camelCase DTO与显式序列化，拒绝静默取first或空值掩盖缺字段。此为恢复既定合同，不新加兼容摘要。五项已知根因不重复战略/Phase讨论；主会话已锁定边界，执行者必须真实RED/GREEN，灰区回流。Grok第二意见重新以120秒上限发起，未返回前不宣称获得意见。
- 并行所有权：L2 lane负责state/L2 adapter/serve及mcp.call_l2；transport lane负责lib/lifecycle/session/认证与mcp.call_tool和CLI bridge；wire lane负责DTO/REST/Rust/TS/schema。mcp.rs按函数边界协调，任何冲突由主会话调度。主会话维护共享planning，修复history_masking单行类型注解。
- 2026-10-31 Clippy gate恢复：本次修复openapi.rs两个`doc_markdown`注释，仅给OpenAPI补反引号，不改变行为；TDD跳过，使用全workspace Clippy/fmt验证。

- 2026-10-30 07-13 恢复：缺 env 的失败只证明前置检查，不能当 snapshot/ACL/shutdown 的行为 RED。Grok 第二意见支持必须真实六工具/四 EXPLAIN/PG 双 store/审计 flush；其把 ACL proof TTL 误解为 token TTL、把 alice 称作管理主体的推测不采纳，真实合同以代码和绑定为准。
- 本地启动已绕过 WSL integration：Git Bash 调用 native Windows Docker；`ci/query-services.sh trino-l2` 完整 seed/readiness exit 0，58081/58082 可用。base Trino 的旧 host network 无法映射到 Windows8080，已仅在本次 runtime 配置改为端口映射及 `host.docker.internal`；真实 `SELECT count(*) FROM demo.aviation.flights` 返回23。runtime密码仅在进程内读取，不进入证据或代码。
- 已开始的本地真实验收：L2 health 61秒证明过期/recovery通过（65.32s），MCP六工具/四格式通过；PG双store在途快照测试已通过，Action测试暴露撤权后的隐藏404与测试期望403不一致，交给PG lane修正断言，未放宽生产授权。

- 2026-10-29 07-05 execution started：TDD applies to both tasks because OpenAPI and export are externally observable wire behavior. `utoipa = 6.0.0` and `utoipa-axum = 0.3.0` are approved in `07-RESEARCH.md`; no new dependency legitimacy gate is needed. Warp and `ralph-loop` are unavailable, so execution uses PowerShell and Cargo directly.
- 2026-10-29 07-05 RED/GREEN：Task 05-1 RED failed semantically on missing `docs/openapi.json` with 3 tests discovered; GREEN added recursive `ToSchema` with `no_recursion`, static 3.1.0 document, all REST paths/errors/security, and checked-in render equality. Task 05-2 RED failed semantically because `serve --export-openapi` was not accepted; GREEN added an early static export branch and verified 3 concurrent files plus explicit write failure. The first export test run spent 13m18s downloading/compiling the workspace; subsequent run was 18.75s.

- 2026-10-29：07-02本地收口依据为 `07-02-SUMMARY.md`、最终独立复签与原始证据；此前pending描述是历史记录。共享进度现为3/16，完整requirements仍Pending，未将PLAN slice冒充Phase完成。本次仅同步文档，跳过新增TDD；最后一行同store history强化属于test-only characterization，以定向history通过和最终all-features Clippy验证。07-04的HTTP deadline行为仍proposed，不能把推荐A当作已批准。
- 2026-10-29：用户确认07-04方案A。该决策仅解除行为阻塞，不代表实现或PLAN完成；executor必须验证宿主任务所有权、owned permit、504/显式receipt replay及正常退出等待，不能用Query取消语义替代Action领域结果。
- 2026-10-29：07-04已按方案A完成并通过真实HTTP验收。新增最小越界修复 `history.rs + Sync` 与 `query::scope_scalar pub` 属Rule 3编译/可达性修复，已在SUMMARY Deviations记录；permit在响应完成后显式释放，避免既有concurrency回归。真实PG/Trino/private仍按07-13/14 deferred，不计ignored为passed。

- 2026-10-29 `fbf5b9f` 的07-02最终代码经独立复签，原5项resolved，报告commit `16adf6b`。all-features Clippy `workspace-clippy-passed.json` exit0，WINDOWS#16已fixed；workspace回归仍pending，不据此新增完成计数。

- 2026-10-29 07-04只读预查发现HTTP deadline/drop与已接受Action所有权未明确，已运行主会话brainstorming并形成可审阅选项，详 `reviews/07-04-PREFLIGHT.md`（proposed，待用户确认HTTP响应deadline语义）。Grok session25404有正文但55秒后exit1，只作部分第二意见；未修改07-04实现，07-02继续独立修复。

- 2026-10-29 恢复收口：07-03 的验收与 wave 证据见 SUMMARY、wave-gates；有效完成2/16。本次只同步状态文档，跳过新增TDD；07-02沿既有批准合同修复，跳过重复战略/Phase/brainstorming，核心replay行为必须先真实RED。所有新灰区先返回主会话并取Grok第二意见。

## 真实服务前置预查

- 2026-10-29：只读核查Docker/PG/Trino/WSL与native runner前置，详 `reviews/07-REAL-SERVICE-READINESS.md`。资源可用不等于真实服务验收通过；07-13/14执行时重新核对。
- 07-03重跑all-features clippy仍受Action idempotency_pg三个16464-byte race future阻塞。核查action_pg已有Box::pin(race(...))模式；纳入07-02 Action修复的最小门禁收尾，先看生产修复是否已消除lint，仍有才按既有模式调整测试分配。该纯测试future分配调整跳过新增TDD，因为不新增业务行为且保留全部断言；验证为all-features clippy，真实PG合同继续由07-13执行。

## 07-03 超时与取消补充决定

- 2026-10-29：Grok session 68211 正常 exit 0 返回第二意见，支持请求内 Engine adapter；已核查 Engine QueryContext 无 CancellationToken、Query 将 EngineError 映为 Source。采用类型分支记录 Timeout/Cancelled，REST 仅覆写对应 Source 响应，合同见 `07-03-PLAN.md`。
- 不采纳第二意见中“无记录默认 cancelled”的泛化：正常 Source 仍须保留原映射；HTTP timer 显式区分 deadline，真实 drop 与持久审计验证取消，保留原 guard finish/Drop。
- ID 暴露时机缺口：现有 audited 方法仅完成后返回 ID，deadline/drop 无法取得它。核查 `ReadAudit::reserve` 同步生成 ID，批准三种兼容 observed 入口在成功 reserve 后同步通知请求局部 ID 槽；不改 storage 信任边界或 QueryService struct。Grok session 46521 在55秒超时，返回实质意见支持 observer 与 exactly-once，但仅记为部分第二意见；其“增加字段必然跨请求共享”的推断不成立，未作为决策依据。
- 03-2只读抽查另发现405 envelope与editOnly PK byte截断误报404；补充处理边界已写PLAN，证据与状态真相源为 `reviews/07-03-ACCEPTANCE-AUDIT.md`。PK底层Query审计继续表示bounded执行结果，保持原success/0，不重写为HTTP预算拒绝状态。

- 2026-10-29 独立审查确认07-02 submit replay内部重读当前定义，违背snapshot；现有frozen_replay只测result lookup。SQLite race只是顺序读，PG用例吞错误无断言，fence/CAS/guard及history当前scope组合覆盖不足。修复清单唯一家为 `reviews/07-02-ACCEPTANCE-AUDIT.md`，保留原提交并撤销完成，不把默认workspace绿当作验收充分。

- 2026-10-29 query audit 接口核查：list/aggregate 各自 reserve guard却只返回page，REST transport ID无法与 durable audit关联。Grok 本轮exit0完整意见支持同型 audited wrappers，拒绝扩大storage上下文。保留原finish/status/Drop语义，reserve失败None、单次reserve、旧wrapper兼容；具体合同见07-03-PLAN补充。

- 2026-10-29 07-04 合同核查：`lib.rs` 是唯一 router 装配点，补白名单仅作注册；当前 principal 的 command lookup 后复用 domain result grant/expiry 检查；POST 过期 key 沿既有七天 TTL 可新执行，GET 过期不可见。Grok 第一次55秒超时无正文；第二次55秒超时但返回上述三项与撤销假完成的实质建议，仅作为部分第二意见，不标完整审查。结论以本地源码与既有 idempotency 测试交叉核对为准。

- 2026-10-29 验收纠偏：不能只消费 executor 的 PLAN COMPLETE 或套名测试。07-03 实际 rest unit 仅常量断言，http_contract 直接构造 helper response/空 page，缺乏计划要求的真实 listener/source/storage 证据。已撤销完成标记，保留生产提交，要求补真实 HTTP 行为测试与可归因 RED。后续每份 SUMMARY 必须逐项对照验收测试的实际含义，不能只数 passed。

- 2026-10-29 wave 3 收尾：四条 REST 路由与结构化错误 envelope 已接入，使用请求 snapshot；workspace clippy all-features 的唯一失败来自既有 `crates/ontoforge-action/tests/idempotency_pg.rs`，不属于本 plan 文件，不能标记全量 clippy passed。真实 PG/Trino/private 仍归环境验收计划。

- 2026-10-29 wave 2 收尾：RequestSnapshot 已模块化，Action/history 提供 snapshot-aware 入口并保留旧 wrappers；history visibility short-circuit 回归已修复。`cargo tree ... -i native-tls` 因无 package ID 返回 101，但 TLS gate 结论为无 native-tls 依赖；不能把该探针写成通过。核心授权与 race 逻辑继续要求 TDD，真实 PG race 归 07-13。

- 2026-10-29 wave 1 收尾：build/workspace 回归 exit 0；schema drift 不阻塞，codebase drift 无 STRUCTURE.md 跳过；UI gate 重试通过但 scope degraded（no-task-commit-rows），不能声称完整 UI scope 核验。当前只做文档同步，跳过新增 TDD；07-02 核心 snapshot/授权逻辑继续强制 TDD。沿用已批准合同，跳过重复战略审查与 brainstorming，新增灰区须返回主会话并先取 Grok 第二意见。

## 07-01 执行前接口核查与第二意见

- 首轮 workspace 在 action/tests/entrypoints.rs 的 production_mutation_callers_are_action_import_and_read_audit 失败：guard递归src并只识别同文件cfg(test)块，将独立src测试文件fixture误判为生产写入。最小修复为机械移动到 tests/unit，再由cfg(test)/path加载；保持--lib、私有super访问与既有安全guard，不扩大production allow-list。两条旧src测试路径仅此明确允许删除，实际diff须确认rename与内容保留。

- 01-2 signing key 核查：engine/config.rs 的 ServerConfig 无查询签名配置，只有 auth.dev.key_file；进程随机 key 会使已有版本 token 重启失效。Grok 本轮 exit0，推荐立即补可信持久文件配置。采用最小可选 query.signing_key_file 与 serve 必须加载；不采纳预加 key id 前缀，避免无需求的 wire 变更。具体合同落 07-01-PLAN，dev owner 接线指针落 07-12-PLAN。
- 主会话初次定位尝试 crates/ontoforge-config/src 不存在，随后 rg --files 定位至 crates/ontoforge-engine/src/config.rs；dev.rs 尚为 07-12 待创建文件。没有依据缺失文件推断已实现。

- `QueryService::execute` 自行 reserve guard，`QueryPage` 不含 ID，宿主原接口无法取得 durable audit 的 request_id。executor 在 RED 前返回 checkpoint，未写业务实现或生产提交。锁定的最小兼容合同见 07-01-PLAN 执行前修订；这是既有追踪要求的 bounded 技术修正，已按 brainstorming 在主会话核对方案，无新增产品决策。
- Grok 第一轮 55 秒 ETIMEDOUT/SIGTERM，无正文；第二轮 exit 0，支持返回 guard ID、旧 wrapper 兼容、reserve 失败 None、保留取消 Drop。建议 Option<Ulid> 未采用：现有公共 guard/storage 使用 String，额外类型转换没有本次需求依据。审计 ID 与 transport ID 语义须区分，不能凭 None 假造持久审计。
- Grok 提醒自研 libtest 文本解析受输出变化/测试打印影响，优先成熟 reporter。实测本机 stable libtest 的 --format=junit 拒绝，cargo-nextest 原未安装。已从官方 get.nexte.st/0.9/windows 下载独立工具到系统 TEMP 的 ontoforge-phase07-nextest，不修改项目依赖或全局 PATH。cargo-nextest 0.9.150 (d225b1daf)，Authenticode Status=Valid，Signer=SignPath Foundation。恢复 executor 后使用原生 JUnit，不新增解析器、不修改全局 GSD。
- Rust RED commit gate 通过实际的 tracer.test.rs / serve.test.rs 测试模块满足现有 *.test.* 识别规则，cargo --lib 验收不变；不扩大 gate 到普通源码。后续 list/aggregate 的审计关联须在执行前复核。

- 2026-10-29 执行入口：沿用批准的 Phase、CONTEXT 与工程审查，跳过战略审查和重复 brainstorming。07-01 核心认证/snapshot/audit 强制 superpowers TDD；Rust inline test 的 GSD pathspec 存在已知识别缺口，必须保留真实 Cargo 行为 RED、机器证据与 test/feat 提交，不能用编译失败或伪造报告绕过。Warp/ralph-loop 仍不可用，使用 PowerShell，不声称其 completion-promise。旧 resolver model ID 不在当前平台支持列表，executor 继承当前模型并采用 resolver 的 high effort。

- 2026-10-29：研究已完成，rmcp 3.8.0 / axum 0.8.9 独立 spike 5 passed、0 failed，主会话已读取最终日志。现代无 session 协议、旧 session 主体绑定、取消与 EOF 均有 spike 证据；正式业务验收边界见 `07-RESEARCH.md`，不把 spike 代替阶段完成。

## 07-08 执行记录

- 2026-10-29：初始化复核确认计划列出的 L2/transport 测试和两个生产模块不存在，不能把缺失文件当作既有实现；先以真实 Cargo 测试建立 RED，再补 `#[path]` 模块接线。
- 2026-10-29：本执行环境未提供 Warp 或 `ralph-loop`，实际命令只使用 PowerShell、`cargo test`、`cargo fmt` 与 `apply_patch`。
- 2026-10-30：`cargo test -p ontoforge-server --tests --locked -- --nocapture` 通过全部 server unit/integration tests；`cargo fmt --all -- --check` 通过。`cargo clippy -p ontoforge-server --all-targets --locked -- -D warnings` 只在计划外 `crates/ontoforge-api/src/openapi.rs:202,208` 的既有 `clippy::doc_markdown` 失败，按 scope boundary 未修复。
- 2026-10-30：rmcp 旧 session 的认证失败通过 HTTP 200 + JSON-RPC error 传输，测试按 MCP 协议检查 body 中 `error`，不能把该路径误判为 HTTP 401；新 2026-07-28 请求缺 metadata 返回 client error。
- 2026-10-29：plan:pre UI gate 返回 frontend=false、block=false；本阶段是服务外壳与平台 clients，不含产品 UI 或 LLM 推理，跳过 UI/AI 设计生成。仅规划文档同步，本次跳过新增业务 TDD；执行 PLAN 继续要求适用任务 Red-Green-Refactor。ralph-loop 未安装，不能声称运行；实际终端为 PowerShell。

- 2026-10-28：两项用户决策已蒸馏到 `07-CONTEXT.md`，完整选项见 `07-DISCUSSION-LOG.md`，不重复询问。ADR-0007/0009/0019/0028 已固定框架、DTO、toolset 和 ACL 生命周期边界。
- Grok 上轮有界咨询超时，但返回了建议正文，仅作为部分第二意见：建议 stdio 认证桥接同一服务、dev 数据默认持久化、部分 L2 降级在健康正文表达。来源为 Phase 6 findings 的 Phase 7 预查条目。health HTTP 状态现已由用户接受，见 CONTEXT D-03；不采纳“所有非 200 都会触发重启”的泛化推断。
- 本轮仅记录讨论和状态，不修改业务实现，按项目判据跳过新 TDD；以 JSON 解析、git diff --check 和 Phase 6 verification.status 验证。ralph-loop 本机此前确认不可用，本轮未运行或声称运行该工具。
- 跳过 office-hours/plan-ceo-review：本轮延续已获批 milestone 的既定 Phase 7 范围，不涉及新产品方向。Phase 层 discuss 正在进行，复杂架构在 P2 进入 plan-eng-review；实现前按任务判据判断 brainstorming。

## 07-13 收尾复验

- 首轮完整服务的所有行为用例通过，但末尾Bash报`syntax error near unexpected token ;;`并exit1；当前脚本`bash -n`通过，执行期间曾插入代码，文件读取偏移变化是待证实解释。已显式credential scan及清理，四个本轮容器/server均不存在；冻结脚本第二轮重跑验证整个流程。Grok有界55秒第二意见exit124，无正文，不构成外部审查证据。
- `7fa2401`通过真实HTTP锁定history分页：旧实现两页均返回[3,2]，新实现limit=1返回[3]、beforeSeq=3返回[2]；4 unit+6 contract、client Clippy/fmt绿。`49deb40`/`4290bed`保留独立复签与执行边界。
- CLI新增test-only manual_assert诊断属于同批lint收尾，保留已有dev/stdio/shutdown/serve行为测试，跳过新增TDD。
- 全workspace Clippy越过server/client后暴露CLI六项lint（cli doc、dev map_or、stdio unit/ready future、serve长度与条件折叠），交原server lane最小收尾。纯lint不新增业务合同，跳过新增TDD，以CLI全target Clippy及dev/stdio/serve已有行为测试验证；未通过前不声明全workspace绿。
- 交叉审查定位Rust client `history`的`limit=${limit}`/`beforeSeq=${before_seq}`生成literal`$`；server数值解析失败会使用默认limit/None。按既有API合同修复两处格式串，跳过重复brainstorming，真实HTTP分页测试必须先RED再GREEN，不以原只检查非空的history合同作为充分证据。
- `4b0b18d`修复SDK capability检查与公开raw绕过问题，8/8真实HTTP contract及typecheck/build/生成漂移通过；最终复签仍待全部修复落定。
- Rust client Clippy修复属于既有合同收尾，跳过重复brainstorming；数值checked conversion属于数据逻辑，要求真实边界RED/GREEN，其余lint按既有pattern最小修改。
- 完整workspace test遇到`target/debug/ontoforge.exe`被运行中的服务占用，Windows返回`os error 5`。CI lane将运行时binary复制至隔离目录；这次退出不证明业务断言失败，也不证明通过。
- 本轮继续既定Phase，跳过office-hours/plan-ceo-review与重复Phase讨论。Warp/ralph-loop工具仍未提供，使用实际PowerShell执行与明确验收循环，不声称调用了不可用工具。

## Open Decisions

- 用户已确认两项运行方式，产品讨论无剩余问题。剩余项均为工程审查事项。
- 工程审查 D1 已接受。继续核对服务 shutdown、桥接认证与断连、定义快照；均须服从既有 fail-closed、审计与权限合同。

## 只读勘查与第二意见

- startup scout 已完成，只读、未跑 Cargo。API/server/client 尚为占位；复用入口与合同指针见 CONTEXT。
- `ActionService::execute_inner` 在 `crates/ontoforge-action/src/lib.rs:258` 重新读数据库定义；`EditStore::history` 在 `crates/ontoforge-storage/src/history.rs:112` 重新读缓存。跨进程 reload 及单请求一致快照是本阶段接线风险，尚未复现为具体漏洞。
- Grok session 91385 返回 `ETIMEDOUT` / `SIGTERM`，不是成功完成。部分建议：进程存活与能力健康分开，L2 禁用时维持核心服务；先停止接入、排空请求和审计，再关闭 monitor/连接；Action 重试需幂等键。其建议用 MCP request id 作为幂等键不能直接接受，协议 ID 与持久 command_id 的生命周期不同；需沿用现有 Action 幂等合同。seed/reset 部分输出被截断，无完整结论。
- gstack preamble 已执行：显式 checkpoint、无 push、telemetry off。检测到工具更新，本阶段不引入无关全局工具升级。项目规则优先：已有范围已批准，继续完整范围，不重复要求批准 SDK/release。
- 已通过 planning-with-files resolver 显式选择 `.planning/phases/07-rest-mcp-shell`，不会恢复其他 phase 的同名计划。
- Context7 primary-source 查询确认 axum graceful shutdown / extensions 与 rmcp 动态列表入口；详情见 `07-ENG-REVIEW.md`。尚未执行 rmcp 编译 spike，也未选定依赖版本。
- 用户于 2026-10-28 选择 D1 的 `1`；四节工程设计审查现已完成，详 ENG-REVIEW。研究 spike、PLAN 和实现验收仍未完成。

## Grok 第二轮有界咨询

- 2026-10-28 session 41414：exit 0，无 signal/error，单轮、禁用工具/subagent/web。与上轮超时区分，本轮收到完整正文。
- 有效提醒：两个独立读语句不能保证 definition/scope 一致；保留 Action 的提交 fence，不自动重试写；在等待 Action 完成前不得取消其依赖的 source read。
- 不采纳的建议：把 replay 绑定原定义会改变 ADR-0018 的当前授权合同；新增 history 版本列前需注意现有 edit log header 已写入 def_version（`commit.rs` 的 `NewRecord`）。history 的问题是请求内读取一致性，并非已证实缺少持久版本。
- 不采纳的简化：仅增加宿主 joined SELECT，仍不能消除 Action/history 内部后续读取定义的入口；是否需要新增最小 snapshot-aware 入口须由源码核查确定。
- 需要核验而非直接接受：Grok 担心 L2 monitor 在 audit flush 后继续产生日志，现有 monitor 位于 engine crate，未查到它依赖 storage read_audit。不能据此改变关闭顺序。

## 本轮检查边界

- 仅更新决策/审查和计划文档，尚不新增业务测试；核心实现继续要求 TDD。
- 探查时尝试的 `query/src/budget.rs`、`query/src/limits.rs`、`action/src/types.rs`、`action/tests/versioned_submission.rs` 不存在；已改用 `rg --files` 获取实际布局，未据此断言功能缺失。
- scout 追加核查已返回：`begin_definition_tx` 与 scope_apply 共享 advisory lock，PG 普通 begin 不保证多次 SELECT 快照；最小方案是短事务复用该锁读取定义和完整 ScopeSet。Action/history 需最小 snapshot-aware 入口，旧 wrapper 保留。
- 原 `0002_edit_log.sql` / LogRecord 已存 def_version，idempotency 测试明确允许定义变化后回放原 receipt；monitor 没有 storage audit handle。Grok 对这三点的扩展建议已排除，未修改既有合同。

## 规划中的 L2 参数差距与第二意见

- 2026-10-29：planner 发现 ADR-0019 / `ref-mcp-trino.md` §2、§6.1 已要求 `explain_query(query,format?)` 的 `LOGICAL|DISTRIBUTED|VALIDATE|IO`，但当前 `L2Service::explain_query(sql)` 仅调用 `run(sql,true)`。主会话亲读两处，确认是已有范围差距；通过独立兼容扩展计划覆盖，不以 adapter 拒绝合法参数缩减范围。
- Grok session 87503：55 秒超时，ETIMEDOUT / SIGTERM，无正文；session 78659：exit 0，无 signal/error。第二意见支持 typed ExplainType 和兼容 wrapper，共用 AST、授权、attestation、审计和限制管线，避免 MCP adapter 自行拼 SQL。
- 采用的测试提醒：默认 wrapper 回归、非法 format 拒绝、四 format 安全门禁、DDL/DML/注释注入拒绝、超时/取消/并发一致。其建议新增 audit format 字段没有当前需求依据，不采纳；VALIDATE/IO 执行语义属于目标 Trino 的实测问题，不要求用户以偏好裁决。没有新增用户决策。

## 计划审查与文档边界

- 第一轮独立审查真相源为 `reviews/07-PLAN-CHECK.md`；完整确定性输入为 `reviews/plan-probes.json`。修订包括所有 warning 和有实际测试依赖影响的 info，不把严重度分类替代工程修复。
- 修正 orchestrator 输入：ROADMAP Phase 7 已持久设置 `Mode: mvp`，应按该模式审查；首次 planner prompt 的 false 不覆盖已存模式。保留既有范围，只补足用户故事和纵向切片表达。
- 按项目三层文档纪律，完成规划时 wave 依赖和跨切约束保存在 T2 PLAN/task_plan，不将 GSD 13c 的完整 wave 注释内联 ROADMAP；T1 仅更新计划数量、状态与 phase 指针。
