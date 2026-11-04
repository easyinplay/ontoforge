---
phase: 07-rest-mcp-shell
review_head: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
status: closed
---

# Phase 7 review fix contract

来源为同目录`07-STANDARDS-REVIEW.md`和`07-SPEC-REVIEW.md`，两个审查轴保持原独立结论。本文件仅统一去重后的执行范围，不用已有绿色测试宣称新缺口已关闭。Windows helper方案A本地完整脚本与最新CI已通过，此处为Phase7既有合同的后续审查修复；07-14仍未实施。

| ID | 来源 | 修复范围 | 必需行为证据 | 状态 |
|---|---|---|---|---|
| RF-01 | Spec CR-01 | 启动/周期drift owner、版本绑定、target与engine-wide健康分开 | 真实schema drift/恢复、reload期间陈旧报告不污染新版本、shutdown清理 | 已关闭：27/27+fresh36/36、独立复签与10175f6 CI41638673666四必需job通过 |
| RF-02 | Spec CR-02 | MCP describe补scope过滤links与engine状态 | REST/MCP同主体metadata字段与授权一致 | 已关闭：27项tranche通过，复签clean |
| RF-03 | Spec CR-03 | REST安全Action参数诊断及criteria消息，DTO/SDK合同 | 逐参数错误、配置criteria消息可达，不暴露底层异常 | 已关闭：server11/Rust17/API7/TS14，复签clean |
| RF-04 | Spec CR-04、Standards CR-03 | MCP schema从实际PK/数组元素类型生成 | integer/boolean ObjectReference与float数组真实调用 | 已关闭：27项tranche通过，复签clean |
| RF-05 | Spec CR-05、Standards CR-04 | 控制/业务参数命名空间隔离 | 同名commandId/validateOnly业务参数完整送达 | 已关闭：36/36及lint通过，fresh复签clean |
| RF-06 | Spec CR-06 | MCP穷举安全错误envelope与请求ID | 底层storage错误不泄露physical relation，code/apiName/request_id保留 | 已关闭：MCP46项及全目标Clippy通过，复签clean |
| RF-07 | Spec CR-07 | REST/MCP history完整记录bytes/条数预算与真实继续信息 | 边界记录、limit>500、MCP truncated、继续取回无丢失 | 已关闭：REST20/MCP46项tranche与双接线复签通过 |
| RF-08 | Spec CR-08、Standards CR-02 | 生命周期OS文件锁代替PID删除协议 | 竞争启动/reset拒绝、崩溃后重新获取、无活跃inode删除 | 已关闭：Windows2+8、Linux2+12，复签clean |
| RF-09 | Spec CR-09 | MCP PK超预算不能伪装null | 区分真实不存在与truncated，保留审计关联 | 已关闭：27项tranche通过，复签clean |
| RF-10 | Spec CR-10 | health DTO/OpenAPI/Rust/TS保留L2原因 | 真实healthy/degraded/disabled字段完整解码 | 已关闭：Rust13/API6/TS10通过，独立复签clean |
| RF-11 | Standards CR-01 | Unix私有dev目录与两把key权限 | 新目录0700、新key0600；不削弱既有key安全边界 | 已关闭：Linux4项实际通过，复签clean |
| RF-12 | Standards CR-05 | history审计ID与错误类别传播 | 响应ID对账durable audit，Source/Store/AuditUnavailable不伪装404 | 已关闭：7/7+相邻6/6通过，独立复签clean |

## 已锁定边界

- RF-08使用Rust标准库`File::try_lock`持有整个DevState生命周期，不删除正在使用的lock inode，不引入依赖/unsafe；本机Rust1.98独占/释放API probe已exit0。PID文本仅可作诊断，不能作为删除或占有证明。
- RF-11复用既有Unix安全创建模式。Windows既有ACL限制不扩大本轮范围。核心并发和权限修复必须先建立可归因RED，再GREEN；Unix测试必须实际在Linux执行才算通过。
- RF-11已有Unix key如group/other任意权限位非零，fail-closed拒绝启动并提示chmod 600，保留字节/权限不变，不擅自迁移现有文件权限；新建目录0700/key0600。
- RF-11已有Unix dev根目录如group/other任意权限位非零，同样fail-closed并提示chmod 700，不修改其权限，不扩大到父目录全树或Windows ACL。已加入定向测试，须Linux实际运行。
- RF-05已按用户最新同方向自动推进授权与Claude Code第二意见接受统一嵌套方案；最终合同以ADR-0029为准，旧等待用户确认已解除。
- Grok第二意见正常exit0（`target/phase7-gap-second-opinion-20261103.log`）支持嵌套params、OS锁和既有明确缺口修复。其关于B必需双读、drift必须持dev锁、探测故障需健康去抖的泛化未采纳；生产drift为只读检测，不得新增自动纠偏/reset或隐藏engine故障。
- 原12项发现当前为静态证据，实际RED/GREEN与独立复签需逐项补齐。已批准scope内已知根因修复不重复战略讨论；新产品合同灰区返回主会话。
- RF-12沿用REST object既有execute_audited_observed/request_id/query_failure模式。history显式区分QueryError与HistoryError::NotFound/Store，PK截断为COST_EXCEEDED，不伪装404；不改分页形状、不扩大storage或DTO接口。所有已保留audit ID的结果以同一ID响应。
- RF-07主会话设计：REST/MCP共用规范化HistoryPage完整记录组预算，复用execution.max_decoded_bytes并含规范化页JSON开销；同seq不可拆，条数为1..500软界限，末组可跨界。使用现有history_with_snapshot(beforeSeq=末组seq, limit=1)额外lookahead确定真实继续状态，避免扩大storage接口。若首个完整组即超bytes，返回脱敏COST_EXCEEDED而非空成功页/跳过记录；不得泄露条目内容或尺寸。待RF-12冻结后接入，保持授权与固定definition。
- RF-07终页按真实继续信息返回truncated=false/nextBeforeSeq=null，非终页返回末个完整已返回组seq；延续Option游标类型，不保留指向空下一页的伪继续信息。helper合同为read_history_page(&EditStore, HistoryRequest, &ActiveDefinition, max_bytes) -> Result<wire::HistoryPage, QueryError>，HistoryError仅映射NotFound/Store，预算为CostExceeded。避免每加一条就重序列化整页的二次方成本，bytes必须按规范化JSON实际UTF-8长度计。
- RF-01最终健康/准入合同已按同方向授权接受，见ADR-0030；技术与测试见07-DRIFT-FIX-DESIGN.md。旧等待用户选择已解除，实施与验收仍不可省略。
- RF-10只补当前health已产生的可选l2.status/l2.reason typed解码，不改变server状态判定或新增source摘要。DTO沿现有status字符串风格，reason可选，未配置仍省略；共享OpenAPI、Rust/TS解码/真实HTTP合同同步，未知底层detail不得新增输出。
- RF-03诊断DTO锁定为可选parameterErrors[{itemOrdinal:u32,name,reason}]和message，None省略；只投影ParameterErrors与CriteriaFailed，后者只来自已配置failureMessage。只保留领域已经返回的诊断，不改批量验证短路/聚合语义。Rust/TS typed envelope保留新增字段；TS Error.message继续原code/status展示，新配置消息通过typed envelope.message访问，不增改展示行为。参数reason可含调用者提交的非法字面量，不能称为完全脱敏；Store/Commit/InvalidRequest原始Display不输出。
- RF-06追加已确认L2路径：mcp_l2.rs:116/211将EngineError::to_string装入Source(String)，call_l2再次Display，可能输出protocol/config/认证失败原始文本。与L0分开建立真实RED，改穷举安全映射及保留已经reserve的audit ID；不能用新ID冒充原audit ID。不修改L2授权/SQL策略/健康判断。先冻结当前L0 tranche再接此扩展，以便其他owner获得稳定接口。
- RF-06 L2最小方案：Source保存EngineError，reserve后产生的source/finish错误附Audited{error,request_id}；code()/request_id()穷举映射，call_l2直接返回ErrorData，禁止Display到wire。前置Denied/Unavailable/Invalid/MissingContext/AuditUnavailable保持原语义，无审计时使用请求fallback ID。六工具真实MCP HTTP配可控Trino HTTP故障stub验证泄露sentinel不外发，不将stub冒充真实Trino服务证据。
- RF-06取消边界补充：只在error返回时携带ID不足以覆盖select取消。run_sql/get_table_schema在reserve成功后、首次await前必须同步observer更新RequestTrace，取消/关闭使用已预留ID；没有reserve时仍是transport ID。早期request_context常量错误也需统一安全envelope，并保留内部context Response的安全类别与原request_id，不能把STORE_UNAVAILABLE伪装认证失败。
- RF-08/11复签发现DS-01..03在原owner继续修复：保留只读前检并在持锁后复核；reset之前校验既有key权限；拒绝dangling lock symlink。原Windows 8/8不覆盖这些新增边界。

## 当前执行所有权

RF05由wire worker负责，36项GREEN后冻结交fresh复签；同worker随后独占API WR01的OpenAPI源/精确合同测试和两个生成物。RF01完整链交独立gsd-executor，只能在前owner源码/Cargo冻结后开工。主会话继续维护全部共享planning，Cargo唯一owner且jobs=2。各自独立报告可写，共享rest/mcp文件不得并发编辑。

当前RF05/WR01均已冻结提交。RF01真实服务worker持唯一Cargo，拥有source_drift_trino_it.rs、ci/server-services.sh命名suite接线与health_trino_it.rs的HF01夹具配对修正。source gate可逆mutation恢复后交core执行AB01 Action领域预算回归修复；core在交接前只准备独立测试，不改生产。shared context/rest/mcp仍单owner串行。

## 先前已完成修复的所有权记录

主会话维护本合同、task_plan/progress/findings和T1状态。首个worker仅负责dev.rs及dev生命周期回归/必要对应文档；其他实现待RF-05选择与drift接线细化后明确文件边界，禁止多lane同时编辑mcp.rs/rest.rs/API DTO。

RF-12独立worker仅拥有rest.rs的history函数与专用history错误/审计回归测试；其提交前不分配另一rest.rs owner。dev owner独立继续，Cargo统一排队，CARGO_BUILD_JOBS=2。

RF-10独立wire owner拥有API dto/openapi、Rust client lib及health合同测试、TypeScript生成/解码及对应测试、docs/openapi.json；不碰server health判断/rest/mcp，不与RF-03同时启动。Cargo在RF-12完成后执行。

RF-03在RF-10提交4f9d159后复用wire owner；拥有上述DTO/SDK/生成/对应测试和docs/api.md、server/error.rs新Action安全诊断投影helper、rest.rs仅action_response。既有mcp()不变，rest history由RF-07 owner独占。Cargo顺序当前RF-07→RF-03→MCP L2及分页接线。

提交依赖：MCP最终tranche调用RF-03新增action_envelope，必须先冻结提交RF-03，再提交MCP四文件；不能产生缺少helper的中间commit。当前RF-03 Rust普通JSON兼容修复优先占Cargo，MCP仅独立复签。

RF-07复用RF-12 owner（24422dc已冻结）负责新server/history_page.rs、lib.rs单行模块注册、rest.rs history接线及专用HTTP分页测试，不改storage或mcp。MCP owner在helper冻结后接入同一函数；两owner通过主会话传递合同。Cargo排在当前RF-10和MCP tranche之后。

RF-02/04/06/09复用dev完成后的worker，拥有mcp.rs的L0/Action错误、describe/PK分支，mcp_actions.rs类型schema/校验，mcp_catalog.rs固定ontology接线，对应MCP回归及必要server/error.rs共享安全映射。RF-09与RF-06共用typed failure/ErrorData，保留code/apiName/request_id；禁止字符串编码envelope或原始storage错误Display。穷举ActionError/HistoryError/QueryError并沿用安全错误类别，既有审计ID优先，无ID才生成请求ID。RF-05控制信封仍待答，不写rest/DTO/共享planning。Cargo排在RF-10之后。
