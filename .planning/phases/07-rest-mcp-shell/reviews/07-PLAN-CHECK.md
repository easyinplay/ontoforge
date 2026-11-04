## ISSUES FOUND

Phase: 07 REST & MCP Shell。检查 15 份 PLAN、31 个 task。结论：0 blocker、2 warning、1 info，返回 planner 修订。

本次只读规划和相关合同，未运行业务应用、未改计划或共享状态。使用 PowerShell 与 GSD query；专用 Read 工具不可用，以 Get-Content 读取。根 AGENTS.md 和项目 skills 目录不存在，遵守会话提供的 AGENTS 规则；agent-skills gsd-plan-checker --skills-only 未返回额外 skill。

### Structured Issues

```yaml
issues:
  - plan: null
    dimension: mvp_compliance
    severity: warning
    required_property: "ROADMAP 中 Mode=mvp 的 Phase goal 使用 As a / I want to / so that 用户故事格式，并保留全部已批准能力。"
    description: "ROADMAP.md Phase7 标记 **Mode:** mvp，但 Goal 为‘应用经 REST（OpenAPI 3.1、Rust / TS client），agent 经 MCP（按主体过滤的 toolset）使用全部能力；ontoforge serve --dev 单二进制在本机跑通端到端演示，并发布三平台二进制’，不具备规定的用户故事格式。planner-mvp-mode.md 明确要求该格式；不能按此前传入的标准模式忽略持久 Mode。"
    fix_hint: "将同一已批准目标重述为 As a ... , I want to ... , so that ... .，引用原六项 SC，不引入新产品选择。"
  - plan: "07-08"
    dimension: scope_sanity
    severity: warning
    required_property: "单份计划的修改范围保持在可控 context 预算内，达到文件阈值时予以收敛。"
    description: "07-08 的 files_modified 列出 11 个文件，跨 server MCP catalog/adapter/session/context/state、CLI 参数与 serve 接线及三个测试入口；超过文件数 10 的 warning 阈值。3 个 task 同时交付六工具、transport/session 和部署默认值，33000 tokens 估计为 low confidence，sample_count=0，不能以未校准估计抵消已观测文件跨度。"
    fix_hint: "例如将可信部署默认值 task 08-3 划为后续有依赖 slice，保留六工具及全部安全验收；其他能降低单 plan 跨度的切分同样可接受。"
  - plan: "07-06 / 07-10 / 07-11"
    dimension: dependency_correctness
    severity: info
    required_property: "同 wave 计划之间的具体前置产物依赖显式声明。"
    description: "三份计划同为 wave6，07-10/11 仅依赖07-05。07-10-1 的 rust_client_metadata_pk_query_aggregate_health 与07-11-1 的 ts_client_generated_metadata_pk_query_aggregate_health 必须真实调用 /health；该生产端点由07-06-1首次接线。07-05明确仅约定health schema，不能视为实现。当前client验收依赖未声明的同wave端点产物，先执行client时不能可靠完成；按Dimension3b规则该项固定为INFO。"
    fix_hint: "例如让完整client contract所在计划显式依赖07-06并调整wave；也可将health contract拆成在07-06后执行的明确任务，保留完整真实HTTP覆盖。"
```

### Coverage

18 个 requirement 均出现在 frontmatter 且有具体 executable action：API-01/03→01/03/04/13；API-02→05；API-04→10；API-05→11；MCP-01→07/08/09；MCP-02/03/04/05→07；MCP-06→02/04/07/09/13；MCP-07→07/08/15/13；MCP-08→07/08/13；DX-01/02→12/14；DX-04→14；L2-04→06/08/13；SCOPE-07→01/02/04/07/08/09/13。

D-01/02/03 分别由09、12、06的task action覆盖，未发现缩减决策或加入deferred ontology SDK。tracer为真实auth→TCP HTTP→同锁metadata snapshot→Query→关闭后durable audit。后续task总体新增可使用能力或细化既有能力安全边界，不构成schema/API/UI层蛋糕；本阶段不是项目首phase，无Walking Skeleton要求。

Action/history snapshot-aware wrapper、共同锁短事务、锁外compile/network、validate-only fence/CAS/guard、当前授权replay保留原receipt/time都有实现动作和失败断言。MCP六工具完整名称参数、四typed EXPLAIN、可信defaults、逐请求认证、session owner/TTL/capacity、禁redirect/reinit、真实stdio取消/EOF、管理工具不可达均已承接。15→08依赖有效；没有发现循环或同wave写文件交集。

health的200 degraded/503 core、先等待Action及依赖再取消独立read/SSE和排空audit/monitor/DB、TS整数token与decimal保真、持久dev/reset、真实PG/Trino和三native artifact smoke均有明确任务；未运行不能标passed的限制已进入执行合同。07-14可创建workflow而不主动发布，PR#7保持Draft OPEN。

### Verification Evidence

实际运行15次 verify.plan-structure：全部valid=true，任务数均2，08为3。05/12的one-way通用warning已由ADR和D-02解释，不重复要求批准。所有task有automated和fails_when；新测试创建责任写入所属task，PG ignored测试由02创建、13正式执行。Nyquist sampling无连续缺失；60秒真实TTL归独立有界integration，不将mock/compile作为最终服务证据。

消费原有plan-probes.json，未重派生其结论：verify-command-paths为pending_creation，39 commands、0 blocker/0 warning；failure-directions为ok，39 commands、0 blocker/0 warning。pending_creation不是问题，not_applicable不等同Rust路径已验证。没有发现需要执行read-only grep/count gate的automated命令。

Architectural Responsibility Map核对通过。RESEARCH无未决产品问题；其‘Sources / Confidence / Open Questions’中未验证实现列举是执行验收清单，不是未决研究选项。PATTERNS的主要analog通过read_first和共享context导入，原子锁、认证、生命周期、serde和真实进程约束落实在action。Dimension10: SKIPPED（根AGENTS.md不存在，已另外核对会话规则）。

实际运行 estimate-check --calibrated，budget=100000，每份confidence=low、sample_count=0，以下估计均不过预算，不能视为项目实测校准：

| Plan | Tokens | Budget | Over Budget |
|---|---:|---:|---|
| 01 | 28000 | 100000 | false |
| 02 | 26000 | 100000 | false |
| 03 | 28000 | 100000 | false |
| 04 | 25000 | 100000 | false |
| 05 | 23000 | 100000 | false |
| 06 | 29000 | 100000 | false |
| 07 | 28000 | 100000 | false |
| 08 | 33000 | 100000 | false |
| 09 | 24000 | 100000 | false |
| 10 | 23000 | 100000 | false |
| 11 | 27000 | 100000 | false |
| 12 | 23000 | 100000 | false |
| 13 | 26000 | 100000 | false |
| 14 | 21000 | 100000 | false |
| 15 | 17000 | 100000 | false |

建议处理2项warning并显式处理health依赖advisory后复核。所有fix_hint均非绑定，满足required_property的其他修改同样有效。
