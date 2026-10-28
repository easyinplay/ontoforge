# Phase 6 findings

- 2026-10-28：首轮 GSD verifier 在前置检查停止，未读实现/未跑技术测试：Phase 6 已为 `Mode: mvp`，Goal 却不符合中央 User Story 正则，实际 user-story.validate=false。读取 gsd-mvp-phase 工作流确认其完整转换流程用于规划且需 active-phase force/交互，本轮不重开已执行计划；仅将既有获批 Goal 加上中央校验器要求的 As a/I want to/so that 固定句式，保留原语义、五条 SC、Mode、需求及 ADR-0028。Grok 该次前置咨询未返回，不能当第二意见通过。规范化后先实际校验，再重新委派技术验证。

- 2026-10-28：空闲下漏读后的完整真实 gate 追踪：24 个 Git 调用全部 exit 0、error/signal null、stderr 空，耗时 41–348ms；06-01 三调用 47/43/329ms、log stdout 446 bytes。完整模块 gate 8/8，随后同环境原始 CLI 5.87s、8/8、block:false。没有捕获此前真实失败，因此只确认错误归类机制，不能认定真实根因已查明。更新 Grok 咨询在 55 秒边界 `ETIMEDOUT`、stdout/stderr 空，未返回第二意见；已按本轮两次复验上限停止调查，无安装/配置/历史修改。以最新原始门禁通过继续 Phase 6 verifier，保留工具间歇漏读风险。

- 2026-10-28：只读核对 GSD `phase.cjs:3771-3814`，`phase.complete` 会按本阶段 ROADMAP Requirements 无条件将 Pending/In Progress 改为 Complete，不解析跨 Phase 合同。若 Phase 6 verifier 通过，收尾仍须依 ADR-0028 及 REQUIREMENTS 的既有 Phase 7 归属保留 L2-04、SCOPE-07 整体 Pending，不修改需求合同或 GSD 安装；正式总验收仍由 VERIFICATION 裁定。

- 2026-10-28：Clippy 逐目标检查在首次错误后中止，因此首轮仅显露一项不代表完整 lint 只有一项。第二轮确认 `tests/l2_monitor.rs` 的 responder 长度及 4 处 assigning_clones、`src/trino/l2_monitor.rs` 的 items_after_test_module。仅拆分 fixture helper、替换等价赋值并移动 unit module，跳过新的业务 TDD；必须跑原 monitor unit/integration 与全量 Clippy，不通过 suppress lint 掩盖问题。

- 2026-10-28：最终 Clippy 的唯一错误是 `trino_l2_acl_it.rs:353` 测试轮询中的冗余闭包，改为 `Vec::is_empty`，不改变表达式语义或生产代码。该一行测试风格修正跳过新的业务 TDD；以受影响目标编译和完整 Clippy 验证，真实 Trino 证据仍指向原行为快照。

- 2026-10-28：只读增量调查证明 GSD 错误归类机制：`shell-command-projection.cjs:592,622-627` 的 Git 子进程默认 10 秒 timeout；`gate-evaluation-scope.cjs:141-149,217-218` 转为 unresolvable/git-timeout/空 commits；`gate-tdd-review-checkpoint.cjs:86-88` 忽略 status/reason，并由依赖计划补给 GREEN（186-190），故仅 RED missing。仅内存注入 06-01 git log timeout 已精确复现，无文件改动。真实原命令重跑 7.14 秒、8/8、block:false；真实 trace 三次 Git 调用 216/43/335ms、stderr 空。未捕获此前失败子进程，不能确定资源竞争为根因。Grok 建议区分基础设施错误与缺提交，方向符合源码；其 Cargo RED 验证锁竞争假说不符合此门禁实际只读 Git 的调用链，未采纳，不运行其扩大测试建议。待 workspace/Clippy 结束后空闲复跑原命令，不改安装。

- 2026-10-28：TDD 原始门禁在最终 workspace 编译期间再次返回 `06-01` RED missing（8 个 TDD PLAN、1 violation、block:true），此前同命令重跑曾通过，故不能将首次异常视为已解决。按 diagnosing-bugs 优先核查子进程 timeout/错误吞没，其次解析/缓存与路径上下文；只读调查及 Grok 第二意见进行中，不修改安装或绕过门禁。

- 2026-10-28：最终 workspace 的 SQL 源码边界守卫失败，命中 `src/trino/l2_monitor.rs:258/263` 中 wiremock HTTP `"DELETE"` 字面量。守卫使用 SQL keyword 字符串扫描，最小修复是测试使用 `reqwest::Method::DELETE.as_str()`，保留守卫规则与生产行为。仅修正两行 fixture 表达，跳过新的业务 TDD；现有失败守卫与原 monitor unit 用作回归验证。

- 2026-10-28：默认并行真实 Trino 测试结束后，`docker rm -f ontoforge-trino-l2 ontoforge-trino-l2-loose` 成功，原 58080/55432 及其他容器仍在。自动审批两次拒绝临时 fixture 的删除（含逐文件非递归方案，均仅返回 blocked by policy）；保留 `C:/Users/easyi/AppData/Local/Temp/trino-l2-catalog.UGBYEziV` 的 5 个已核验无链接配置文件，不再尝试绕过。容器删除与文件删除分别记录，不声称清理完全。

- 2026-10-28：新增 `06-10` 后首次原始 `check tdd-review-checkpoint 06` 报 `06-01` RED missing。只读核实该 plan 的 5 个提交仍可达，默认与 1000 上限的 evaluation-scope 均完整返回 `0981150`/`8faa2c9` RED 与 `a531947` GREEN，排除取样上限和历史丢失；原命令重跑为 8/8、block:false。首次漏读的具体瞬态原因未证实，不宣称已根治 GSD 工具，也不改历史/关闭 TDD。Grok 无工具第二意见建议先提取实际 git 命令再改 scope；检查发现 resolver 先按 plan subject 过滤，默认上限 200，该 plan 仅 5 个提交，故未采纳其条件式扩大窗口建议。此调查只读，无工具安装或配置修改。

- 2026-10-28：Grok 第二意见已返回，采纳 monitor owner-only enable 和跨 generation 的 pending 清理义务。同一 engine 的 pending 由共享状态持有，新 monitor 必须先清旧 probe；旧 DELETE 响应按捕获的 query/token 校验后才能清除对应 pending，旧 Drop 只作用于自身 generation。无 monitor 时保持 manual 兼容；注册期间 manual 成功不能写 Enabled，必须明确返回禁用状态。未采纳将 pending 持久化到数据库的扩展建议，进程退出由服务端 execution/run time 上限兜底。新增行为 RED 后再实现。stateful fixture 按 ID/owner/state 观测，成功 DELETE 终结查询，并覆盖 DELETE pending 时不可启用；真实 ACL suite 按 CI 默认并行复跑。

- 2026-10-28：增量 review 确认跨 API 清理绕过：monitor DELETE=500 后为 CleanupFailed，但同 generation 的 manual recheck 可借另一 RUNNING ID 写 Enabled，未清理原 pending probe。按 cleanup-before-enable 合同必须修复。最小边界：无 monitor 时保留 manual；monitor 已注册期间 manual 只能诊断/禁用，不能从 disabled 启用，只有 owner cycle 清理成功后可启用。执行器先补回归 RED 再修复；Grok 第二意见查询中，不重复请求用户批准既定安全合同。

- 2026-10-28：独立 plan checker 无 blocker，指出调度上限和锁等待 deadline 未写进 task action；已明确默认 30 秒周期/20 秒整体 cycle（包含 recheck/admission 等待）和锁竞争测试。主会话追加同 coordinator endpoint 前提，防止传入另一集群 ontology engine 产生无效隔离证据；正常 build_trino_engines 路径不受影响。

- 2026-10-28：Grok 无工具第二意见复核生命周期：enable 与 shutdown 的资格检查/状态写入必须同锁；启动 future 取消须撤销任务；重复 monitor 需原子登记；过期检查独立于 loop 存活；manual 与 periodic 共用 recheck/admission 序列化。主会话采纳这些约束。其认为 DELETE 404 必须失败的建议未采纳：前后真实 RUNNING 已确立观测期间活跃，404 证明清理时已不存在；不以 404 代替活跃证据。探针清理 2xx/404 可以成功，其余失败关闭。

- 2026-10-28：选项 1 已获用户批准。主会话 `superpowers:brainstorming` 复核现有 engine 后选显式 async startup + RAII monitor handle，避免同步构造函数隐式 spawn；约 30 秒复检，小于 60 秒有效期。每轮 fresh ontology query，隔离观测前后检查真实非终态（优先 RUNNING），有界只读 AST、服务端 run/execution 上限、客户端整体 deadline 与 origin 检查；失败/停止/取消失败不得 re-enable。核心安全逻辑执行 TDD，不豁免。既有阶段 scope 明确，跳过新战略 office-hours/CEO gate。ralph-loop 未安装，记录逐项验收而不声称执行。

- 2026-10-28：用户选择方案 A，ADR-0028 将 Phase 6 库级启动/周期 ACL attestation 与 Phase 7 服务及 `/health` 告警分开验收，L2-04 整体保持 Pending。只读代码勘查确认 `recheck_l2_acl` 需调用者传入另一身份 query ID，`QueryResults` 未解析 `id`/`stats`，同步 `build_trino_engines` 不能隐式启动 Tokio monitor。Grok CLI 第二意见：生产没有通用长查询，暂停短 `SELECT 1` 的 `nextUri` 不构成活跃证据；优先部署方配置有界探针，每轮自产 ID、同轮检查、取消，失败维持禁用。是否新增此配置为实施前的开放决策。

- 2026-10-28：Windows 完整 workspace 首轮 `reload_racing_commits_never_lets_an_old_write_through` 在 `WriterStats { committed: 9, stale: 1 }` 失败。共用 helper 把切换后的 stale 拒绝算作 10 次尝试之一，断言却要求至少 10 次成功提交；单独复跑通过，属于测试时序误报。helper 改为累计 10 次成功提交并设 100 次尝试上限，目标连续 6 次及完整 workspace 均通过。只改测试稳定性，无生产逻辑变化，新的 TDD RED/GREEN 不适用。

- 2026-10-28：`06-09` 的两个任务只修改 `tests/`，GSD 执行期 `execute-mvp-tdd.md` 明确豁免 test-only task 的生产 RED/GREEN 提交门禁；阶段 `tdd.review-checkpoint` 仅看 PLAN `type: tdd`，错误要求不存在的 `feat(06-09)`。显式将 PLAN 类型更正为 `execute`，保留 task 级 `tdd="true"`、两份 `RED_EVIDENCE_OK` 和 GREEN 测试记录；该更正不代表跳过测试，门禁需重跑确认。

- 2026-10-28：独立受限 Trino 483 的 memory `hive.sales` fixture 初测让 32 条 allow 全部真实发送，30 条 FINISHED；G38 的 integer `listagg` 为 `TYPE_MISMATCH`，B51 未引用同形异字标识符为 `SYNTAX_ERROR`。两条均为 reconstructed/constructed-equivalent，不是已恢复的历史原文。为实现可执行等价验收，改为显式字符串 cast 与引用标识符，保留 G38 ordered aggregate / B51 同形异字 relation 的意图，并重跑真实 Trino。此任务只修测试 fixture，无生产实现变更，TDD RED/GREEN 不适用；初次真实错误作为探针证据保留。

- 2026-10-28：运行 `cargo test -p ontoforge-engine --test trino_l2_acl_it --locked -- --ignored --nocapture`，3/3 通过；11 条无表 allow 语料经 `validate_l2` 的 Display SQL 发送到独立受限 Trino 483，逐条 FINISHED。其余 21 条需要真实 `hive.sales` 表；拟在两台现有 ACL 容器各使用隔离 memory catalog 和小型 seed，仍保留 outside/editstore 负探针。G38 数值 `id` 与 `listagg` 类型可能冲突，必须记录实际错误而非预设通过。

- 2026-10-28：WR-01：ROADMAP Phase 6 SC-3 写明启动自检和 `/health` 告警，但 `ontoforge-server` 外壳属于 Phase 7；库级 `recheck_l2_acl` 目前只在测试调用，Enabled 60 秒后过期。Grok 建议分离库级 attestation 与 HTTP 暴露，但其 Stale/last-known 建议须服从 ADR fail-closed，不可自行放松。WR-02：32 条 allow constructed-equivalent 中 11 条无物理表依赖，其余 21 条引用 `hive.sales`；独立 ACL fixture 只有 demo/outside/editstore TPCH，不能把 11 条通过等同 32/32。先真实探测再决定合同修订或补 fixture。

- 2026-10-28：CR-01 的 `ReadAudit::shutdown()` 修复让新 `reserve()` fail-closed，同时保留已准入 guard 的 sender 入队直至 worker drain；`State::can_enqueue()` 仍以 sink health 与 pending age 拒绝持续故障。新增 SQLite/PG 共用测试，在关闭异步等待期间 `finish()` 后断言持久行可见。RED/GREEN 提交 `0d3d0bf`/`5fd876d`，审查报告尚未重签。

- 2026-10-28：`06-REVIEW.md` CR-01 与 `06-SECURITY.md` T-06-01 均确认 shutdown 在持有 guard 的读仍在途时将 `available` 置 false，guard 的 `enqueue` 因此丢记录且 shutdown 仍可能成功。最小修复须区分 admission 与 sink 健康，保持 sink 故障/超时 fail-closed，并等待所有已准入 sender 完成后 flush；新增 SQLite/PG 共用回归，先 RED 后 GREEN。WR-01 属启动/周期接线范围疑问，WR-02 是 Trino corpus 实测覆盖缺口，分别复核。

- 2026-10-28：`06-09-PLAN.md` 标记 `type: tdd`，两个任务仅修改 `tests/`，实际没有生产修复。GSD `tdd.review-checkpoint` 对整个 TDD PLAN 要求 `test(06-09)` 后跟 `feat(06-09)`，因此误将 test-only regression 的真实 mutation RED/GREEN 判为缺 GREEN。Grok 第二意见建议保留证据与阻断，显式修门禁或留痕豁免；拒绝空 `feat` 提交和静默回改 PLAN 分类。待独立审计后提交具体选择。

- 2026-10-28：GSD verification.status 返回 missing、route: execute-phase；已有 SUMMARY 的 wave 不会重做。审查完成及 verifier 核对前维持需求 Pending，远端 GitHub Actions 证据尚缺。

- 2026-10-26：ROADMAP Phase 6 范围为 L2-01..04、AUDIT-01 和五条 Success Criteria；spike 2 要在 Trino 483 上复验。ADR-0018/0019 与 `docs/designs/trino-engine-sql-tiers.md` 已锁定双 Trino 身份、SQL AST 白名单、只读权限自检、全级别读审计、管理通道边界。规划前需核对旧语料是否与当前 `sqlparser` 及 Trino 483 一致。
- 2026-10-26：GSD `init.phase-op 6` 的 expected_phase_dir 为 `.planning/phases/06-l2-raw-dql-toolsets-audit/`；ROADMAP 的 `**Details**` 原指向较短的 `06-l2-toolsets-audit/`，现已修正。
- 2026-10-26：代码调查：auth 已有 `sql:raw`/L2 allow-list，engine 已能建独立 L2 Trino 身份，但 `PrintedSql` 私有构造阻止原始 Display SQL 直接执行；L2 校验器、启动自检、读审计表/队列和 `AuditUnavailable` 均尚无。QueryService 的 pk/list/aggregate 是公开读入口，server/api 仍为占位。现有 `reload` 先 lint，`scope apply` 锁内复算计划哈希并写 `of_admin_audit`，可复用，不能把 CLI 环境用户名当远程认证主体。
- 2026-10-26：Trino 483 容器 `ontoforge-trino-05-06` healthy，`127.0.0.1:58080/v1/info` 报 483，临时用户的 `SELECT 1` FINISHED。容器没有 access-control 配置，不能验证 L2 真 ACL；104 条 spike 语料只剩 Markdown，临时实验代码已删除，需重建并区分原文/等价用例。
- 2026-10-26：Grok 第二意见经 ADR/REQUIREMENTS 过滤：拒绝「审计完整 SQL」（AUDIT-01 只要哈希/元数据）、「强制用户身份透传」（设计是可选）、「Phase 6 同时暴露 REST/MCP」（Phase 7）及「现有无 ACL 容器作为授权证据」。采纳固定对抗语料、越界 SQL 到 Trino 零发送、独立受限 ACL、审计队列故障注入。队满拒绝无法再可靠写入同一 sink，不能将此时审计条数等于所有尝试数作为验收。原 `/plan-eng-review` 于 `docs/designs/trino-engine-sql-tiers.md` Eng Review Gate 标记 CLEAR (PLAN)，Phase 6 仍须按现代码做针对性工程复审。
- 2026-10-26：本阶段只整理文档与只读探查，尚未修改业务代码，TDD 实施待 Phase 6 具体任务。`superpowers:brainstorming` 子任务触发判断将在 PLAN 任务切分后逐项执行；当前已由 accepted ADR 锁定对外合同，无新的用户问题。
- 2026-10-26：独立工程复审见 `06-ENG-REVIEW.md`。PLAN 须闭合受控 SQL 的 AST/Display/发送边界，真实受限 Trino ACL 与另一主体已知活跃查询探针，pk/list/set/searchAround/aggregate 及六个 L2 库级入口的审计门禁，PG/SQLite 系统表与故障注入。该报告不是新的正式 gstack gate；既有跨层设计的 `CLEAR (PLAN)` 记录仍在 `docs/designs/trino-engine-sql-tiers.md`。
- 2026-10-26：`06-SPIKE-483.md` 的新构造等价语料 29 条，0 条历史 SQL 原文恢复，75 个历史 ID 未覆盖。21 条解析为单语句且 Display AST 往返相等，7 条解析失败，1 条多语句；Trino 483 的 14 次只读 HTTP 请求中 10 FINISHED、4 FAILED。`G11` 的 ROW 类型 Display 语义损坏导致 TYPE_MISMATCH，`G16` 的 OFFSET/LIMIT Display 导致 SYNTAX_ERROR；AST 往返相等不足以证明 Trino 可执行或语义相等。`B53` 仅发送去除注释后的 Display，CATALOG_NOT_FOUND 不是权限拒绝。固定可执行 corpus 与受限 ACL 自检证据仍是实施和验收任务，不能将此次 spike 视为 L2 安全通过。
- 2026-10-26：PowerShell 管道调用 `apply_patch.bat` 曾报 UTF-8 PATCH 参数错误，随后作为参数传递时又报结尾标记错误；改为直接调用同一 Codex `--codex-run-as-apply-patch` 可执行文件传入 PowerShell here-string，文档修改成功。
- 2026-10-26：研究确定 `G11` 的 ROW 类型 Display 有损应保守拒绝，`G16` 仅在受控改写并经 Trino 483 验证后放行；固定 104-ID 等价 corpus 须记录与历史 G/B 预期的差异，不可假装原文已恢复。`06-PATTERNS.md` 给出代码 analog，ACL fixture、异步有界审计队列和旧 SQL 原文没有可复用实现。spec-less edge probe 报 11 unresolved（L2-01: 3、L2-02: 1、L2-03: 5、L2-04: 1、AUDIT-01: 1），其中 L2-02/04 和 AUDIT-01 的 3 项 unclassified 必须作为 flagged assumption 显式交给 PLAN，不自动视为通过。
- 2026-10-26：第一轮 plan-checker 为 3 blocker/2 warning/1 info；修订后第二轮仅余 `SCOPE-07` frontmatter 覆盖 blocker。`REQUIREMENTS.md:214` 将请求版本固定指向 Phase 6，`ROADMAP.md` Phase 6 的 requirements 列表原漏该 ID，已补摘要指针；`06-09` 的任务和测试已有行为内容，仍需在 PLAN frontmatter 与四源覆盖审计认领。首轮诊断的 ACL 服务前置、pk/L2 审计矩阵、研究决策、零测试门禁与同 wave 依赖已通过复查。
- 2026-10-26：GSD Rust TDD 的 RED commit 门禁只识别测试路径，不识别 `src/lib.rs` 内联测试；`06-01-PLAN.md` 的测试改放 `crates/ontoforge-query/tests/audit_pk.rs`，保持先失败断言、再生产实现。`workflow.use_worktrees=false`，Wave 1 两计划在当前工作树串行执行，避免共享 git index 竞态。`ralph-loop` 命令与 skill 未安装，执行单元仍按明确完成标准、RED/GREEN 与 `COMPLETE` 回报核查，不声称运行过该工具。
- 2026-10-26：Wave 1 子任务层 `superpowers:brainstorming` 设计审查：审计可选在各 QueryService 方法同步写入或由 EditStore 持有有界异步 sink。依 ADR-0016 的批量/故障合同选择后者，公开读先预留 permit、结果释放前提交元数据；内部 read_pk_source 不单独计数。SQL 可选扩展通用 `PrintedSql::from(String)` 或在 printer 内封装校验后构造；依 ADR-0017/0018 选择私有构造与单一受控入口，未知 AST 拒绝且不触发 Trino POST。两项在 `06-01`/`06-03` PLAN 有 RED/GREEN 和故障验收，未新增用户产品决策；已有授权继续执行。
- 2026-10-26：`06-03` 发现 `GenericDialect` 不把 G08 的 `x -> x + 1` 解析为 lambda，而是 `BinaryOperator::Arrow`；生产 gate 因宽泛 `Expr::BinaryOp` 分支误放行。明确白名单后 104-ID 分类测试通过。G16 改写 Display 在 Trino 483 `FINISHED`（`20261026_072806_00018_jtvdc`），B53 仅发送 Display 后因容器无 hive catalog 返回 `CATALOG_NOT_FOUND`（`20261026_072806_00019_jtvdc`）；无 ACL 容器仍不能证明 L2 身份隔离，留给 `06-04` / `06-08`。
- 2026-10-26：`06-02` 真实 embedded PG 与 SQLite 的批量/保留期/失败关闭合同通过，in-flight guard 在 sink 变 unhealthy 后拒绝释放结果。`06-04` 子任务设计复核依既有 ADR 选择独立 file ACL + L2 身份正反探针；只信应用 allow-list 或静态配置不足以证明 Trino 侧隔离。已由 Phase 6 PLAN 锁定，无新增用户决策。
- 2026-10-26：本机 GSD RED classifier 不解析 Rust 原生 libtest console（`Unsupported report format`），不能以断言失败自述代替机器门禁。Rust 1.98 stable 直接 `--format junit` 被拒；本次 `RUSTC_BOOTSTRAP=1` 加 `-Z unstable-options --format junit` 实测得到 JUnit XML 与两条真实断言失败，适配 GSD JUnit parser。GSD RED commit pathspec 只认 `tests/` 等测试路径，Task 1 改成 integration target；此选项仅用于测试 reporter，不改生产依赖或运行配置。
- 2026-10-26：`06-04` 的独立受限 Trino 483 fixture 对正例返回数据，对确实存在的 outside/editstore 返回 PERMISSION_DENIED；另一主体的已知活跃 query ID 在 L2 查询中不可见，松规则对照使自检失败。该证据覆盖库级 ACL 门禁，公开 L2 execute/explain 的接线仍属 `06-06`。
- 2026-10-28：`06-06` 子任务设计复核：Phase 6 的 D-01..D-04 和 ADR-0018/0019 已锁定库级 L2 请求合同。执行器持有固定 principal/scope/definition，复用唯一 AST/Display gate 和独立 L2 Trino 身份，入口预留审计后发送 SQL；explain 只能从已验证语句形成。任意 SQL 的截断不伪造 keyset cursor。没有新增用户决策；两项核心逻辑均按 PLAN 强制 TDD，不能跳过。`ralph-loop` 本机未安装，以逐项 RED/GREEN、真实测试与 SUMMARY 完成标准验收。
- 2026-10-28：`06-08` 只接线 CI 与扩充既有外部服务测试，未改核心业务实现，故按项目判据跳过新的 TDD RED/GREEN；真实受限 Trino 483、PostgreSQL 审计和 workspace 回归已运行。默认并行 workspace 曾受 16 MiB 行计算与 1 秒审计期限竞争影响，CI 暂用 `--test-threads=1`，后续风险只记 `deferred-items.md`。远端 GitHub Actions 仍待 PR/推送后验证；Phase 6 总验收不得提前声明通过。
