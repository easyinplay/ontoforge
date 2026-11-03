---
phase: 07-rest-mcp-shell
fixes: [RF-01, RF-10]
status: accepted_for_implementation
reviewed: 2026-11-03T07:41:57Z
source_head: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
implementation_started: false
tests_executed: false
---

# RF-01 drift 修复设计

本文件是只读实现设计，唯一写入物为本文件。以下符号名称是建议，不表示源码已有实现；开放合同由主会话结合 Grok 第二意见决定。RF-10 可共用 health wire 变更，但不得由两个 lane 并发修改 DTO/server/client 文件。

主会话2026-11-03收敛：用户授权Claude Code同方向意见自动推进，完整exit0意见见`target/phase7-claude-contract-review.json`及`target/phase7-claude-followup.json`。最终产品合同以ADR-0030为准，覆盖本文早期待决草案。采用既有health字段、30s/20s常量、raw host engine和原有drain；不新增独立Trino client。ScopedEngine可在owned Action async块内借用captured host并持固定view，无需强制新增第二wrapper。保留单slot与exact-key隔离，可用必要短锁观察标记拒绝旧轮发布，不建立多版本缓存。下列测试矩阵继续适用，新增source摘要wire测试不适用。工程增量四维审查见07-DRIFT-ENG-REVIEW.md；全部target降级仍200 degraded，monitor异常退出转CheckFailed；现有Query读审计保留，不为Action发明额外read_audit合同。

## 1. 已有合同与真实调用链

- `.planning/REQUIREMENTS.md:31`、ADR-0015:21、ADR-0021:32/44 要求启动及周期检查、按目标降级，其余照常。`docs/designs/trino-engine-sql-tiers.md:176/268` 要求受影响目标返回 `SourceSchemaDrift`。ADR-0015:20 明确自动 drift 不做 pk 全表查重。
- `07-06-PLAN.md:120` 要求 owner、启动/周期检测、限时 probe 与失败清理；本修复使用 interval 默认 30s、每轮总预算默认 20s，配置须为正且不超过二者。既有 L2 的 60s proof TTL 不是已批准的 drift TTL，不将它复制成新失效 SLA。
- `07-CONTEXT.md:29` 只明确 L2 单独故障时 200 degraded、核心故障 503；没有明确“部分 source target 漂移”的 HTTP 状态或新版本尚未检查的行为。
- 当前不存在 `EngineRegistry`（生产源码全树符号搜索无结果）。真实对象为 `serve.rs:153-212` 构造 `AnyEngine`、`ServerHost<E>` 和独立 L2 engine；`ServerState` 只持 L2 monitor、取消与任务状态。无需新增全局多引擎注册体系。
- `bind_check.rs:519-578` 按 catalog 调 `Engine::describe_tables`；`is_engine_wide()` 为 true 则整轮 Err，其余 catalog 错误成为相应 target 的 `SOURCE_UNREADABLE`。`:770-777` 的 `check_drift` 返回不带版本的 `DriftReport`。当前函数仅收 `&AnyEngine`，可将它和 `describe_bindings` 最小泛型化为 `E: Engine + Sync`，保持现有调用语法。
- `store.rs:256` 的 `active_definition()` 只读本进程缓存；外部 reload 后不能用它发现新版本。`request_snapshot.rs:22-45` 在共同 metadata 锁内读取已提交定义/scope并释放事务，随后才能发网络请求。`reload.rs:155-169` 只在提交/发布后更新执行 reload 的 store 缓存。
- 真实 source 出口包括 PK `query/src/lib.rs:461-518`，列表首批 `execute.rs:743-757`、后续/backfill `:885-903`，聚合 `aggregate.rs:773-796`，M:N `link.rs:289-363`。Action `action/src/source.rs:96-123` 仅在实际需要 source 时经 `read_pk_source` 读取。只在 REST handler 检查顶层 objectType 会漏集合、link、聚合及 Action。
- 当前 `health.rs` 每次执行 `engine.status()`；Trino 的实现是 `SELECT 1`（`trino/mod.rs:516-544`），成功不能证明 source metadata 可读。drift 的 engine-wide error不能被这一次 SELECT 1 静默清除。

## 2. 最小 owner 与调度

建议新增 `ontoforge-server/src/drift.rs`，定义 `SourceDriftMonitor`、`DriftOptions`、`VersionedDriftState`；由 `ServerState` 持有唯一 monitor handle、只读当前状态及唤醒信号。raw ontology engine 供只读 probe 使用；不用 L2 engine，不通过带 drift gate 的请求 wrapper，避免递归准入。

启动顺序：验证参数/limits → store/auth/engine/固定定义 → 建立可回收 owner → 完成有界首次检查 → 安装后台周期任务 → 接受 HTTP。首次检查返回 target degradation 或运行时 engine failure是可观察状态，允许服务暴露 `/health`；配置错误、无法取得活动定义等初始化错误应清理并非零退出。对首次失败是否允许监听、pending 错误码的最终决定见第 6 节。

共享 `ServerHost::serve_with_limits` 必须走同一个启动 seam，不能只在 CLI 特例安装而真实 listener fixture继续绕过。CLI 最好先成功 bind 再启动后台 owner，避免端口冲突后回收窗口；若保留现顺序，则所有启动失败分支显式 shutdown 既有 L2/drift owner、drain engine cancellations与 audit。运行时失败不能仅依赖 Drop。

调度使用单个 task + Tokio interval（Skip missed ticks），同时最多一轮；不是每轮结束再 sleep 30s，防止20s检查加30s sleep变成50s启动间隔。每轮总20s包含 snapshot及所有catalog元数据请求；每个 QueryContext timeout取剩余预算。禁止每catalog各享有独立20s而使整轮无界。请求发现新 def_version 时只唤醒同一 task，不 spawn 重叠 probes、不新增健康去抖。

owner在首次 await 前建立；shutdown future取消时仍可定位任务及 engine cleanup。正常停机沿既有顺序：停止准入 → 等已接受 Action/依赖 → 取消独立 read/SSE → audit → 停止/join drift与L2 monitor → engine cancellations drain。drift与L2共享现有截至8s的 monitor阶段，最终截至10s drain raw ontology engine，失败不吞。启动失败清理使用同一原则，不直接增加新的总关闭窗口。probe不写业务数据、不追加 read_audit；用户因drift被拒绝的读取仍需恰好一条原有审计。

## 3. 定义版本绑定与旧报告隔离

建议状态身份为 `(def_version, effective_profile)`，effective_profile使用与当前 Query一致的 `active.ontology.default_profile`；不私自引入第二套 CLI/env profile优先级。检查对象包含该固定 ontology 内的 object 和 M:N link bindings。纯editOnly且无profile时为 notApplicable；有source却缺profile/binding应报告配置/绑定失败，不能以空report认定healthy。

每轮步骤：

1. 从 `store.request_snapshot()` 获取 Arc<ActiveDefinition>，事务结束后取 `(version, profile)`，标记本轮身份。
2. 对该 Arc 的 ontology执行 `check_drift`。结果以 `Checked(report)`、`EngineFailed(safe_code)`、`CheckFailed(safe_code)`区分。外层超时没有完整report，应是 CheckFailed；不能构造“全部漂移”或“全部健康”。单catalog timeout沿现有函数成为对应target的 SourceUnreadable。
3. 发布时在短 mutex/RwLock 内比较已观察到的最高版本与profile；旧轮结果不得覆盖已观察到的新版本。不要在这个锁内访问storage或网络。
4. 请求的 `RequestContext` 和匿名health均读取真实已提交snapshot，再选择完全相同key的状态。没有匹配状态就显式 Pending，并唤醒检查；不得借用另一版本的healthy/degraded报告。
5. 新请求观察到更高版本时更新单调版本标记。即使外部reload在本轮最后一次观察后提交，也只能暂存带旧key结果，下一请求的精确key检查会隔离它；不需要假装publish与跨进程DB提交可以原子同步。
6. 在途请求保留已经捕获的不可变 Arc状态与定义。slot只保留当前版本，旧报告由在途Arc延长寿命；不建无界按版本缓存。若旧请求尚未取得旧报告而slot已前进，它只能得到Pending，不能读取新报告。

本方案不重读/替换在途定义，不触碰reload发布协议或Action DefinitionStale fence。周期恢复由新的完整检查结果自然替换，不做自动reload、reset、DDL、绑定纠偏或dev锁。

## 4. 状态消费与 wire

需要明确的逻辑目标 gate，不能靠解析 PrintedSql 或 TableRef反推：同一物理表可以绑定到多个逻辑target，而它们要求的列不同。

推荐工程落点：给 Engine trait新增默认可通过的只读 `check_source_target` hook，参数包含版本、profile与逻辑 BindingTarget，返回独立的有限 `SourceTargetError`。server的请求 Engine wrapper持有第3节不可变状态并实现hook；普通引擎/库调用沿已有无宿主monitor行为。Query在每个实际source发送前调用hook，且保留 QueryError/AggregateError 中明确的 drift/unreadable/pending类别；Action source错误映射必须保留这些类别。此为建议内部接口，executor应先核对现有trait实现与调用者，不建立第二权限体系。

`ScopedEngine`已经封装请求limits/trace，可复用其职责，而不是再包三层adapter。REST及MCP的owned Action任务必须使用同样的固定状态wrapper；当前 `.with_engine(&*host.engine)`不得继续绕过。采样gate发生在现有授权和audit reserve之后，网络发送之前；保持审计状态分类与响应request_id，并防止隐藏target名称通过错误泄露。

消费规则：实际source访问边界与匿名脱敏已获主会话确认；Pending/CheckFailed准入及HTTP细节仍为第6节的保守草案。

| 状态 | source依赖操作 | 其他对象/纯edit路径 | 对外信息 |
|---|---|---|---|
| target Healthy（可含warning） | 允许 | 允许 | 状态；完整mismatch文本只在operator日志 |
| target SourceSchemaDrift | 仅匹配target拒绝，零上游发送 | 允许 | `SOURCE_SCHEMA_DRIFT` + caller可见apiName |
| target SourceUnreadable | 仅匹配target拒绝 | 允许 | 稳定code、已过滤error_name；无detail |
| 当前key EngineFailed | source依赖操作拒绝 | 保留无需source路径 | 引擎故障；不伪造各target漂移 |
| Pending / CheckFailed | 建议只拒绝需要该key检查结果的source路径 | 允许 | 明确检查状态；不宣称已知schema不匹配 |

匿名 `/health` 不应列出未授权ontology对象名或物理名；建议新增脱敏source检查摘要（状态、defVersion、目标计数、稳定原因），逐对象/links状态只在认证metadata/describe按scope投影。部分target degraded时是否HTTP200见开放决策。已有 L2故障D-03和engine.status核心故障503保持原合同。

RF-10不必等待以上新增source wire选择：`HealthResponse`增加可选L2状态DTO，完整保留现有 `l2.status`/`l2.reason`；Rust decode_health显式解析，OpenAPI/TS重生成并真实解码验证。若与source摘要同次改动，最终只由一个wire owner编辑DTO与schema。`DegradeReason::mismatches`、`OperatorDetail`不得直接Serialize到匿名health或L0。

## 5. 文件/符号所有权与TDD顺序

建议一个RF-01 executor负责完整core链，避免monitor与gate分叉后互相声称完成；wire lane串行接入，主会话负责集成。所有文件路径以 `crates/` 为前缀，除特别说明。

| Owner | 文件及符号责任 |
|---|---|
| RF-01 core | 新`ontoforge-server/src/drift.rs`；`state.rs` owner/state；`lib.rs`启动；`context.rs`固定view和ScopedEngine hook；`lifecycle.rs`有界join；`ontoforge/src/serve.rs`启动失败清理 |
| RF-01 core | `ontoforge-engine/src/lib.rs` hook/error类型；`bind_check.rs`泛型化；`ontoforge-query/src/{lib,execute,aggregate,link}.rs`全部实际source出口、错误/审计映射；`ontoforge-action/src/{source,lib}.rs`保留安全错误 |
| 集成指定单一owner | `ontoforge-server/src/{rest,mcp,error}.rs` wrapper接线/错误映射；这些与RF-02/03/06/07/09/12重叠，禁止并行写。core先提供符号合同及独立patch，主会话安排顺序 |
| RF-01/RF-10 wire单一owner | `ontoforge-server/src/health.rs`；`ontoforge-api/src/{dto,openapi}.rs`；`ontoforge-client/src/lib.rs`；`clients/typescript/src/index.ts`；生成`docs/openapi.json`和`clients/typescript/src/generated.ts`，与RF-03 DTO工作串行 |
| 相应owner | 新server drift测试、既有health_lifecycle/真实Trino/真实PG测试；query/action定向测试；API snapshot、Rust/TS contract。测试support的describe_tables需真实返回fixture元数据，不能把所有host默认禁用monitor来维持旧绿 |

TDD必须先以真实listener/真实服务seam证明行为RED；本设计没有执行下列测试：

1. **启动及周期**：首次metadata probe先于接受source请求；非默认物理列名的真实Trino表先正常、ALTER后目标返回SOURCE_SCHEMA_DRIFT、恢复后可读；另一个对象与editOnly全过程正常。核实调用的是生产serve owner，不是直接调用check_drift。
2. **所有出口**：PK、列表首批/后续/backfill、aggregate、source M:N及searchAround、Action source/validate-only、REST/MCP同target拒绝；零上游业务SQL、零Action edit/log/outbox。source-independent完整patch路径继续可用。schema probe可发送，需区分probe与业务SQL计数。
3. **reload隔离**：两store真实PG，共同锁；阻塞v1探测，外部reload至v2，新请求看到v2 Pending/新结果；随后释放v1，不能污染v2。再验证在途v1 Arc固定、撤权新请求不泄露target；用barrier，不靠sleep竞争。
4. **分类及脱敏**：某catalog PERMISSION_DENIED只影响其绑定；其他catalog成功。engine Unauthorized/Unavailable是round failure，不把所有target记作drift；正常SELECT1不能抹掉失败检查。warnings不降级；无source不发metadata SQL。匿名health与L0中无catalog/schema/table、mismatch message、operator detail。
5. **预算及重复**：paused time或可控clock验证start间隔<=30s、整轮<=20s、多catalog共享预算、无重叠probe；错误恢复无需两次成功/去抖；重复安装owner拒绝；新版本唤醒不产生后台task风暴。
6. **关闭与启动失败**：活跃metadata请求阻塞时shutdown仍在原总10s预算结束，已接受Action先完成；drift task停止/join，Trino DELETE由同raw engine tracker排空。DELETE失败使非零。端口冲突、初始snapshot失败、首次probe取消后无后台task/活动query，audit正确结束。
7. **RF-10**：健康、L2 enabled、expired/disabled reason及未配置省略分别经真实HTTP→Rust/TS完整断言；OpenAPI snapshot与生成漂移门同步。若加入source摘要，再为每状态做同样wire断言。

完成证据须包含实际命名用例非零执行、RED归因、GREEN日志及独立复签；不能以原1373项通过代替新增链路证据。Windows本地的fixture测试与Linux真实Trino/PG覆盖各自记录，未运行保持pending。

## 6. 已收敛的合同

以下原开放项已由主会话与Claude Code同方向收敛，最终行为见ADR-0030：

1. 部分target degraded为HTTP200 degraded，匿名health保持既有字段。
2. 精确版本无report为SOURCE_CHECK_PENDING，整轮失败为SOURCE_CHECK_UNAVAILABLE；source fail-closed/HTTP503，合并唤醒同一monitor，无新增TTL。
3. 初次runtime probe失败仍提供HTTP/health503并周期恢复；结构配置/活动定义读取失败退出并清理。

已决：只拦实际source依赖，完整patch且无需source的既有QUERY-01/ACTION-12路径继续可用，不因对象type存在source binding就一律拒绝。

其余可立即落定的技术边界：不新增EngineRegistry、不自动纠偏、不做pk扫描、不加dev锁、不隐藏单次故障、不改变L2 proof TTL与既有关闭总预算、不允许旧版本报告匹配新请求。
