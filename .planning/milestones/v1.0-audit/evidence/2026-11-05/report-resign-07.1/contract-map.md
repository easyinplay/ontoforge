# Phase 07.1 当前合同重新验收

## 结论与证据边界

**当前结论：passed，18/18 must-haves verified，behavior_unverified=0，overrides_applied=0。** 本节是2026-11-05重新签署的权威判定；报告下方2026-11-04正文仅保留为历史审计上下文，不把其中旧HEAD、旧测试数量或旧命令执行人冒充本轮事实。旧报告逐字节快照为`.planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-07.1/previous-VERIFICATION.md`，原文件与快照SHA256均为`824303b284cf21feddfc38b7e31bd06f1153ceb2ef8dbc22b2eba1d2d557fa23`。

**Phase Goal:** As a ontology维护者, I want to 显式运行source OneToOne基数检查并获得有界诊断, so that 我能发现违反声明的FK唯一性而不改变默认在线验证和业务读取行为.

本轮读取原完整报告、三个PLAN全部must_haves、REQUIREMENTS中STORE-05、CONTEXT、VALIDATION、UAT、GATES、REVIEW、REVIEW-DISPOSITION、PARANOID-REVIEW、SECURITY及独立意见材料。SUMMARY仅定位证据，不作行为证据。主会话已调用planning-with-files，计划为`.planning/milestones/v1.0-audit/task_plan.md`的“追加报告门收口”。本轮不改该计划、STATE、ROADMAP、需求矩阵、共享SUMMARY或其他agent文件。

验收源码为`060019fe52005c6b85ae5c5f59da9868f464e579`。旧报告native路由为stale且唯一moved_file为`crates/ontoforge/tests/cli_trino_it.rs`。逐项复核全部18项而非只接受此提示。对旧covered集合中的全部实现文件比较`01aa2df..HEAD`，仅此测试文件变化；scanner、online、printer、storage约束、seed和用户文档均未变，当前实现无未提交diff。完整差异在`source-since-previous.diff`。

变化实质是将服务配置模板复制至每次测试独占的TempDir，使用实际`ONTOFORGE_TRINO_URL`，拒绝空白、控制字符、引号和反斜杠注入，不修改共享模板。四个真实服务case继续实际spawn CLI，原source断言未放宽。新增配置隔离及注入拒绝测试在当前workspace各实际通过。

证据简称：`W`=`.planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-workspace.log`；`I`=同级`ci-41994479295-integration.log`；`J`=同级`ci-41994479295-final.json`；`S`=`crates/ontoforge-engine/src/cardinality_check.rs`；`O`=`crates/ontoforge/src/online.rs`；`C`=`crates/ontoforge/tests/cli_check_cardinality.rs`；`E`=`crates/ontoforge-engine/tests/cardinality_check.rs`；`T`=`crates/ontoforge/tests/cli_trino_it.rs`。下文W/I数字为原始日志行号；全部输入hash、具名匹配、命令、stderr及exit记录在`source-log-audit.json`、`native-gates.json`。

## Observable Truths

前三项保留ROADMAP原文，其后为三个PLAN的全部15项新增truth；无缩减、override、prohibition或backstop。每项行为判断同时核对生产代码、测试断言和真实通过行，不以符号存在或总CI绿替代行为。

| ID | Truth | 判定 | 当前源码与行为证据 |
|---|---|---|---|
| SC1 | 显式online基数检查对source-backed link.to的非空FK按不同对象PK计数并报告OneToOne冲突；NULL和重复物理行语义明确。 | VERIFIED | S:34-73候选及物理映射，S:83-104非NULL FK与COUNT DISTINCT/HAVING；E:157/164/172断言2、去重和NULL；W:779/780/784；I:1085真实source目标。 |
| SC2 | 默认online与check-pk无额外FK扫描；基数warning默认0/deny-warnings1，未完成3；有整个检查总deadline及真实CLI测试。 | VERIFIED | O:291-298独立flags、O:309-338共享计时，CLI requires online；C的flag/默认/退出码/deadline/cleanup实际通过W:61-73；unit W:14-17。 |
| SC3 | typed SQL、限额、取消与错误边界通过SQLite/Trino验证；STORE-05以独立验收结果更新。 | VERIFIED | S typed Select经print_select→Engine::execute，truncated先于解码；W:65-67、777-787与729；I:1085/1087真实Trino。REQUIREMENTS已将STORE-05映射Phase3 edit+Phase7.1 source，本轮独立分别核验，不以旧勾选当证据。 |
| P01-1 | 显式 validate --online --check-cardinality 经选定 profile、元数据预检和真实 SQLite，报告两个不同 to-object PK 共享非NULL FK的 OneToOne 冲突。 | VERIFIED | cli.rs:233 requires online，O两backend实传flag，C:732实际CLI/SQLite及binding文件/行号/两种输出断言；W:64。 |
| P01-2 | 同PK同FK的重复物理行只计一个对象，NULL FK没有连接，NULL PK不增加distinct对象数。 | VERIFIED | S过滤FK并COUNT DISTINCT PK；E:164/172固定输入断言；W:780/784；T真实专用表排除ROW-DUP/NULL-PK/ALL-NULL-PK，I:1085。 |
| P01-3 | warning保留在human/JSON输出，默认exit0且摘要不声称无违规；诊断明确未检查referential integrity。 | VERIFIED | O:345起W18/help，lib.rs:123-143按warning分支打印；C:194/732检查exit0/1、两格式、不含clean摘要、help及定位；W:73/64。 |
| P01-4 | 冻结同一fixture连续检查两次，warning、样本顺序相同；source文件/行及edit存储没有写入。 | VERIFIED | E:218比较两finding、source八行及文件内容/数量；C:804比较两次stdout/stderr和全部文件字节；W:785及72。 |
| P02-1 | 只检查OneToOne的source-backed link.to；from为editOnly仍被检查；PK/FK使用to-object当前profile的物理列映射。 | VERIFIED | S:46-71按to backing及usable BindingKey筛选并column_for，不要求from source；E:252测试dev/prod映射与隔离；C tracer的from为editOnly；W:783/64。 |
| P02-2 | 默认online、仅check-pk、OneToMany、ManyToMany及to为editOnly都没有cardinality SQL；check-pk与check-cardinality可组合且各保留自己的诊断。 | VERIFIED | O独立if，S候选过滤；C:261/298实际metadata/PK请求计数与cardinality零请求，组合V18+W18；W:69/70/62。 |
| P02-3 | warning默认exit0，deny-warnings为1；离线schema或metadata预检失败不会扫描失败binding，也不声称完整通过。 | VERIFIED | O离线错误出口与usable过滤；C:456无效schema、缺表/列/类型以及401，明确扫描0和exit1/3；C:194 warning；W:71/73。 |
| P02-4 | 最多20个稳定排序的有界样本；仅在实际获得第21组且结果完整时more=true；LIMIT不代表限制了扫描成本。 | VERIFIED | S ORDER BY输出alias n DESC/fk ASC、LIMIT21，完整性优先后判断more和truncate20；E:310/528断言20/21/22及索引alias；C:337 Unicode64字符；W:786/777/68，I:1087；docs明确全表成本。 |
| P02-5 | 超时、rows/bytes截断（包括0行）、查询/解码/cleanup失败都报告ONT-V19及exit3，保留此前诊断并在runtime内drain取消。 | VERIFIED | S:134-147、O:250-265/325-338 await drain并保留诊断；E:452零行/坏行，C:492/547/646故障及DELETE完成/DELETE500；W:781、65-67、14-15。 |
| P02-6 | cardinality整轮共享由QueryContext.timeout确定的总deadline，默认30秒；后续link仅取得剩余时限，耗尽停止新扫描并报告V19/exit3，metadata和独立check-pk保留原时限语义。 | VERIFIED | O:309单次started，311扣elapsed，target_ctx只变timeout；metadata/PK先完成；unit核30k/12k/1k、remaining及零预算，CLI核第三项0次、DELETE1及预检/PK四次180ms；W:16/17/63/61。 |
| P02-7 | 每次查询按当次source观测报告，外部可并发变化；不保证跨表快照或持续唯一性，中断/半结果/未执行不能冒充完整合规。 | VERIFIED | 每次S:133实际执行，无finding cache；E:402改变source后第二次查询无旧冲突，W:782；故障/预检完整性见W:65-67/71；docs明确无跨表快照/未来承诺。 |
| P03-1 | 真实Trino对专用source数据执行新CLI检查，报告不同PK冲突且不误报重复物理行/NULL；至少1个目标integration test实际通过。 | VERIFIED | T:270起绑定demo.aviation.cardinality_records、spawn真实CLI、精确一个W18/CONFLICT=2并排除五种clean/more；I:1085实际ok，I:1090该harness为4/0/0，不是ignored；真实环境见下。 |
| P03-2 | 用户文档与CLI一致说明独立opt-in、source to-object映射、单列distinct语义、warning退出码、未完成V19、整轮默认30秒总deadline、20+1样本与全表扫描成本。 | VERIFIED | docs/source-binding.md:244-294逐条对照cli.rs字段、O和S实现；W:794文档诊断注册校验及W:61-73实际CLI输出/参数合同；旧help记录保留，不冒称本轮重跑help。 |
| P03-3 | 文档和诊断明确本检查没有验证referential integrity，不保证外部source未来持续唯一性或跨表快照；报告不阻塞读取。 | VERIFIED | O help与docs:292/294边界，scanner的生产消费者仅online CLI；S/online/业务调用边界与旧实现未变；C:194及真实T断言help，W:73与I:1085。 |
| P03-4 | 缺Trino环境会失败，wiremock/ignored/零用例不被记为真实Trino通过；SQLite及相关PK/printer/CLI回归有当前tree证据。 | VERIFIED | T trino_url缺失/空值panic，W:303实际missing_env_panics；W:298-299的两个ignored不计通过，改用I:1085/1087的真实执行；W:301/302配置安全回归，PK/printer具名结果及engine/CLI harness均非零。 |

**Score:** 18/18。已观察行为测试均在当前HEAD真实日志中通过；本轮是独立读取和核验既有执行记录，并未新运行Rust测试。不把复用执行日志描述为本verifier再次运行测试。未发现需追加测试才能判断的合同项。

## Required Artifacts 与 Key Links

native `verify.artifacts`三个PLAN均exit0，分别4/4、3/3、3/3，去重8项全部有实质实现与消费者：S候选/查询/解码、printer/ast.rs的CountDistinct、C进程级测试、E真实SQLite、printer/tests.rs双dialect测试、T真实Trino、seed专用表及views、docs完整操作合同。见`native-gates.json`，不是以文件存在代替行为。

native `verify.key-links`三个PLAN均exit1，合计1/9命中，其余8项均为`Target not referenced in source`。这不是自动绿，逐条以真实Rust module/调用和行为消解如下；没有override。

| PLAN/link | 真实接线 | 判定 |
|---|---|---|
| 01 cli.rs→online.rs | ValidateArgs.check_cardinality→validate_online→run_online→Trino/Fixtures两个compare_all实参；W:62/64与I:1085 | WIRED |
| 01 online.rs→S | import cardinality_targets/check_cardinality，usable BindingKey→check_cardinalities→execute；W:783/71 | WIRED |
| 01 S→printer/dialect.rs | Select/CountDistinct→print_select→dialect递归expr→Engine::execute；W:729/778及I:1087 | WIRED |
| 02 online.rs→S | metadata diagnostics筛usable，独立check_pk/check_cardinality分支；W:62/69-71 | WIRED |
| 02 S→online.rs | truncated/解码先验，Result逐target累加并保留此前warning，V19→exit3；W:65-67/781 | WIRED |
| 02 online.rs→Engine::drain_cancellations | runtime block_on内await finish_cardinality_check，再await drain；W:14/15/65/66 | WIRED |
| 03 seed.sql→T | cardinality_records及三个alias view与T绑定同名；I:540-543 seed完成，I:1085/1087实际查询断言 | WIRED |
| 03 CI→T | ci.yml:185-188设置ONTOFORGE_TRINO_URL并cargo test --test cli_trino_it -- --ignored；I:1066-1090实际运行 | WIRED |
| 03 docs→cli.rs | 独立flag/requires online/timeout/deny-warnings字段、真实CLI输出及退出码逐项一致；W:61-73 | WIRED |

## Data-Flow Trace (Level 4)

| 输出 | 实际来源与传递 | 判定 |
|---|---|---|
| W18 samples/more | selected profile→metadata usable keys→source.to mapped PK/FK→typed SQL→真实SQLite/Trino rows→完整性/解码→20+1→diagnostics human/JSON | FLOWING，W:64/777-787与I:1085/1087 |
| warning摘要/exit | 实际diagnostics severity计数，V19优先，再deny-warnings；C核stdout/stderr、0/1/3与定位 | FLOWING，W:65-73 |
| clean结果与后续观测 | 每轮真实execute返回完整空rows，不是static空数组；改变source后新查询返回新结果 | FLOWING，W:784/782 |

## 真实服务身份及执行边界

J的headSha与当前完整HEAD相同，CI run为41994479295。I:107-125证明checkout该HEAD；I:445为PostgreSQL17.11固定digest镜像，I:496运行`psql ... -f ci/pg/seed.sql`，I:540实际`INSERT 0 14`、541-543三个CREATE VIEW。I:865-895为Trino483固定digest镜像，I:922服务ready。I:1069设置`ONTOFORGE_TRINO_URL: http://localhost:8080`，I:1082运行真实`cli_trino_it` binary，1084起running4，1085/1087分别是两个source检查，1090为4 passed/0 failed/0 ignored/3 filtered out。不能拿另两个PK/online目标替代这两个source目标，也没有拿wiremock冒充真实服务。

当前T配置辅助函数要求模板URL恰好一次，替换到每次独占TempDir，四个case实际使用该config；环境不存在或为空时panic，主机URL含TOML特殊字符时拒绝。W:301/302/303对应这三个active安全/环境回归；其断言检查8个并行配置目录不同、只有URL变化且模板字节不变。真实T仍验证精确W18 warning、文件/行号、source counts及20组稳定顺序，不是仅HTTP200或“CLI能启动”。

W原始头部绑定HEAD060019f与独占CARGO_TARGET_DIR，原命令`cargo test --workspace --locked -- --test-threads=1`，执行人日志由主会话提供，exit0记录见`reverify-local-results.json`。本verifier重新从原始`test result`行计算152 harness、1528 passed、0 failed、119 ignored，仅用于回归背景；每项合同依赖上述具名行。13项CLI、11项engine均0 ignored；真实Trino在W中ignored，明确不算通过。

本轮独立实际运行的是Node只读源码/日志审计和GSD query，不新增Cargo、现成Rust binary执行、容器操作、服务、CI触发或外部请求。旧原始RED/GREEN、失败日志、monitor竞态修复、TDD gate、fmt/Clippy日志全部仍在旧covered集合内。没有删除失败历史、降低测试断言或把未执行项改成manual-only。

## Behavioral Spot-Checks 与24项矩阵

下表是读取当前HEAD原始日志的行为证据映射，不是本轮重跑命令。`node .../report-resign-07.1/audit.cjs inspect`独立校验33项本地具名通过记录和2项真实Trino通过记录，并保存命令exit/具体日志行；重复name按所在harness人工区分，不能只接受跨harness同名。

| 行为ID | 当前named test/关键断言 | 原始执行证据 | 判定 |
|---|---|---|---|
| A1/A2/A3 | distinct_objects_sharing_fk_are_reported、duplicate_physical_rows_are_one_object、null_keys_and_clean_links_have_no_finding；distinct/NULL/空表 | W:779/780/784 | VERIFIED |
| A4 | cardinality_flag_contract；缺online2、独立0、组合V18+W18 | W:62 | VERIFIED |
| B1/B2/B3/B4 | mapped_to_binding_is_the_only_source、non_candidates_are_not_scanned、no_flag_means_no_cardinality_scan、warning_output_and_exit_codes | W:783/70/69/73 | VERIFIED |
| B5/B6 | samples_have_twenty_plus_sentinel_bound、long_sample_values_are_bounded；20/21/22、Unicode64字符 | W:786/68 | VERIFIED |
| C1 | count_distinct_quotes_identifiers、count_distinct_quotes_identifiers_on_sqlite；两dialect安全引用/单SQL及真实SQLite | W:729/778 | VERIFIED |
| C2/C3/C4 | precheck_errors_skip_cardinality_scan、incomplete_scan_is_not_clean、tighter_budgets_are_not_widened、incomplete_or_malformed_results_are_not_clean、failure_preserves_findings_and_drains | W:71/67/787/781/66 | VERIFIED |
| D1/T0 | check_cardinality_counts_distinct_objects_on_ci_trino、cardinality_tracer_reports_conflict；source实际冲突/输出 | I:1085、W:64 | VERIFIED |
| C5a/C5b | online::tests::cardinality_total_deadline_is_shared、同名CLI；remaining递减/第三项不启动/DELETE1 | W:17/63 | VERIFIED |
| C6 | online::tests::cardinality_total_deadline_configuration、cardinality_budget_starts_after_precheck_and_pk；30k/12k/1k/0、预检及PK不扣预算 | W:16/61 | VERIFIED |
| C7 | cardinality_cleanup_failure_after_success/after_metadata_error、cleanup_transport_failure_is_reported；await与原diagnostic保留 | W:14/15/65 | VERIFIED |
| E1/E2 | repeated_check_is_read_only的E/C两个harness、independent_runs_observe_current_source | W:785/72/782 | VERIFIED |
| R1/R2 | count_alias_cannot_change_indexed_fk_sample_order、check_cardinality_aliases_on_ci_trino；SQLite四组合及Trino三组合20个A..T | W:777、I:1087 | VERIFIED |

24个原行为ID全部保留映射，没有新增测试数量要求。Probe Execution为N/A：三个PLAN没有声明可执行probe-*.sh；monitor术语不等于独立shell probe。

## Requirements Coverage

| Requirement | 来源 | 实现与独立执行证据 | 判定 |
|---|---|---|---|
| STORE-05 source | 三个07.1 PLAN | 上述18项/24项全部，source OneToOne distinct FK warning；真实Trino的两个具名target | SATISFIED |
| STORE-05 edit既有合同 | Phase03，不代替source证据 | storage/physical.rs:329-366在to对象FK构造UniqueWhenSet；ddl.rs:105-117限定live/set部分UNIQUE；commit.rs:1497-1517映射CardinalityViolation；conformance:1842/1991断言拒写无row/log/history、并发恰一胜；W:2249/2259/2403实际SQLite，I:603/612真实PG对应两项 | SATISFIED |
| STORE-05完整需求 | REQUIREMENTS Phase3+7.1映射 | edit强制约束与source显式报告各有自己的实现和执行记录；OneToMany的to-object单值FK与既有IR不变 | SATISFIED |

所有三个PLAN都声明STORE-05，无本phase额外orphaned requirement。REQUIREMENTS/ROADMAP仅作为合同输入读取并在source-log-audit中保存hash，不修改。源码后的治理文件不能被忽略或从covered集合删除。

## 治理门与 Test Quality Audit

| Gate | 本轮核查与边界 | 判定 |
|---|---|---|
| 原覆盖与治理 | 原256项全保留，包括全部.planning治理及200个旧证据；旧报告完整快照hash一致；新增当前日志与独立审计 | PRESERVED；covered-diff.json记录差集 |
| code-review/Paranoid/security | 原REVIEW、DISPOSITION、PARANOID、SECURITY均读取；CR-01/WR-01的fixed不能当当前实现证明，复核alias两个输出排序键、cleanup等待及原真实RED/GREEN；本次唯一增量ci_config隔离/注入拒绝有当前具名测试，无旧生产增量 | 当前合同内无未关闭finding；不伪称重跑外部Review Army或安全工具 |
| D-01至D-11 | CLI opt-in/默认成本/候选/COUNT DISTINCT/RI边界/typed SQL/W18/未完成/20+1/不改业务路径/整轮deadline逐项对应P01/P02/P03表 | 11/11 VERIFIED |
| UAT/Nyquist | 原10项UAT与24项VALIDATION逐项映射本轮W/I；PLAN02/03 deferred human-check的文档、diff、证据归属由本次独立审阅完成 | 无待人工判定项；不声称用户手工见证 |
| TDD/失败历史 | 原TDD native日志、SQLite/Trino alias RED/GREEN、monitor旧竞态记录和修复后日志全部保留覆盖；本轮不改实现，TDD不适用 | 无新增实施义务；不以历史失败作为当前通过 |
| native artifacts/key-links | artifacts全过；8项文件路径启发式miss由上述9项真实调用链逐条消解 | 未遗留未接线WARNING |

测试强度：CLI为真实子进程退出码/诊断值/请求与DELETE数量/文件不变；engine为真实SQLite固定输入的数值及排序；unit为remaining/调用数/await完成标志；printer为quoted SQL/单Query AST；Trino为精确W18/CONFLICT=2/排除clean组及稳定20个A..T。expected来自固定独立输入，不从scanner输出生成oracle；wiremock仅证明错误/取消边界，不冒充真实Trino。33项具名本地记录包含同名case时按harness复核，两个Trino必需target实际0 ignored，无只有disabled测试支撑的truth。

## Anti-Patterns、人工项及未验证边界

对旧集合全部实现文件的TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER扫描无命中，source-log-audit保存空结果；合法空candidate/空finding都有明确筛选条件或实际完整查询，不是stub。新增ci_config不存在提前return成功，缺环境失败，URL插入限制明确；无HOLLOW_PROP/static动态输出。未发现BLOCKER、WARNING、coincidental-reliance或需要override的偏差。

**Human Verification Required:** 无。PLAN延后文档/边界/差异审查已独立完成；本phase无UI/视觉目标，CLI和真实服务交互有行为证据。没有PRESENT_BEHAVIOR_UNVERIFIED、UNCERTAIN或未执行必需合同项。

明确不验证或承诺：持续唯一性、跨表一致快照、referential integrity、复合键、跨backend JOIN、数据库物理扫描行数上界、cleanup包含在30秒扫描deadline内；这些均为锁定合同排除项，不是隐藏缺口。不把Phase03完整安全范围或整个v1.0里程碑重新认证为本07.1报告的成果。

**Deferred / Advisory:** 无。已检查milestone roadmap，无需把本phase必需项移交未来phase来获得passed。旧报告没有gaps前置优化，本轮是全合同重新验收；不能把新发现压成advisory来保留旧passed。

## 独立gate闭环

旧native状态、覆盖集合及初始git状态在before.json；旧完整报告在previous-VERIFICATION.md；源码增量、当前输入hash、逐项日志行及六条artifact/link原生命令在本目录持久化。使用`query verification.fingerprint`对原完整256项与新增证据取并集，直接复制其covered_files/covered_digest，不手算、不移除.planning。最终`covered-diff.json`记录added/removed及全部保留治理项，`after-gate.json`记录本verifier实际`query verification.status ...`与`--pick status`的exit和输出，并检查验收期间原256项无漂移。

本报告及fingerprint/covered-diff/after-gate这三个派生receipt不进入自己的指纹，避免自引用；所有原covered项不受影响。未commit、push、发布、归档，也未触碰容器或其他agent变更。ralph-loop/Warp不可用，未声称调用；完成依据真实终端记录、逐合同判断与独立native gate闭环。
