# Phase 7 findings

## 真实服务前置预查

- 2026-10-29：只读核查Docker/PG/Trino/WSL与native runner前置，详 `reviews/07-REAL-SERVICE-READINESS.md`。资源可用不等于真实服务验收通过；07-13/14执行时重新核对。

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
- 2026-10-29：plan:pre UI gate 返回 frontend=false、block=false；本阶段是服务外壳与平台 clients，不含产品 UI 或 LLM 推理，跳过 UI/AI 设计生成。仅规划文档同步，本次跳过新增业务 TDD；执行 PLAN 继续要求适用任务 Red-Green-Refactor。ralph-loop 未安装，不能声称运行；实际终端为 PowerShell。

- 2026-10-28：两项用户决策已蒸馏到 `07-CONTEXT.md`，完整选项见 `07-DISCUSSION-LOG.md`，不重复询问。ADR-0007/0009/0019/0028 已固定框架、DTO、toolset 和 ACL 生命周期边界。
- Grok 上轮有界咨询超时，但返回了建议正文，仅作为部分第二意见：建议 stdio 认证桥接同一服务、dev 数据默认持久化、部分 L2 降级在健康正文表达。来源为 Phase 6 findings 的 Phase 7 预查条目。health HTTP 状态现已由用户接受，见 CONTEXT D-03；不采纳“所有非 200 都会触发重启”的泛化推断。
- 本轮仅记录讨论和状态，不修改业务实现，按项目判据跳过新 TDD；以 JSON 解析、git diff --check 和 Phase 6 verification.status 验证。ralph-loop 本机此前确认不可用，本轮未运行或声称运行该工具。
- 跳过 office-hours/plan-ceo-review：本轮延续已获批 milestone 的既定 Phase 7 范围，不涉及新产品方向。Phase 层 discuss 正在进行，复杂架构在 P2 进入 plan-eng-review；实现前按任务判据判断 brainstorming。

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
