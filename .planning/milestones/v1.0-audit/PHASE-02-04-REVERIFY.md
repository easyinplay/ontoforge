---
status: completed-with-gap
audited: 2026-11-04
source_head: f8851ca073fed34e80e605f0ac91880259df7e12
current_head_observed: f8851ca073fed34e80e605f0ac91880259df7e12
owner: historical_phase02_04_verify
---

# Phase 02 / 04 当前目标重验

## 边界

本轮为独立源码与原始证据复核，未执行 Cargo、Docker 或服务，也未提交。唯一运行 owner 为 `milestone_dynamic_validation`。旧 `02-VERIFICATION.md`、`04-VERIFICATION.md` 的 passed 均不作为当前行为证据；两报告现已按以下最终复验更新，fingerprint由GSD owner query生成。主会话仍保留整个 milestone 的 `gaps_found`。

已读取 mandatory-initial-read、verification-overrides、gates、verifier-phase-gates、verifier-evidence-gate、thinking-models-verification、few-shot verifier、wiring-patterns、MVP reference、project-skills-discovery、agent-skills-bootstrap。项目无本地 AGENTS.md/RTK.md、`.codegraph/` 或项目 skills，遵循会话注入规则；`query agent-skills gsd-verifier --skills-only` 空。已显式使用 GSD verification queries。当前工具没有独立 Write，报告创建使用可用 `apply_patch`，未用 shell heredoc。

上下文：两 phase 的 PLAN frontmatter/行为与 verify 片段、旧 VERIFICATION/UAT/VALIDATION、ROADMAP SC、18 个 BIND/ENGINE/QUERY 需求、INTEGRATION、REQUIREMENTS-AUDIT、NYQUIST-02-03-05-VALIDATION、当前 task_plan/DYNAMIC-VALIDATION。Phase 02 原 15 条是 5 条 SC 加 10 个 plan 的汇总；实际原 PLAN 有 98 条细化 truth（10/11/13/11/13/7/9/7/8/9），不可把旧 15/15 误写成逐条 98 项的新行为证明。Phase 04 5 条 SC 加 31 条 PLAN truth，旧分母 36。

## 首轮必须补齐的证据（历史检查点，最终处置见文末）

1. **Phase 02 SC5 / ENGINE-05：当前 Iceberg 483/484。** `trino_it.rs:626` 的 `iceberg_table_is_readable` 在 `ONTOFORGE_TRINO_ICEBERG != 1` 提前返回；普通 integration 的 ok 不计 Iceberg 证据。需 owner 原 nightly digests + JDBC catalog/CTAS flights，两个环境变量齐备，483/484 每个版本真实 named test 1 passed。测试断言 Iceberg schema 非空、source 非零与 copied/source 行数相同。
2. **Phase 04 SC5 / QUERY-10：当前独立 Linux p95。** 相对 c39a971，query `lib/execute/aggregate/cursor` 与 Trino `mod/cancel` 有实际运行语义演进。旧 `04-PERFORMANCE.md` 明确不覆盖后续源码。普通 push 仅 smoke，`ci.yml` 的 isolated job 只在 workflow_dispatch 且 input `phase4_query_bench=isolated` 执行。主会话已触发 run **41759457559**，head 868643c。验收原始 `phase4-query-bench-isolated/phase4-isolated.json`：4 路各 5 warmups/40 measured，nearest-rank p95 严格小于 500/1000/1500/50 ms，零 errors/limit errors、不升 guardrails、edited 路径全部样本零 source calls、完整 distributed plans 1..=2 fragments。
3. **MVP Goal 格式。** 两 phase 标 `Mode: mvp`，旧 ROADMAP goal 非 canonical User Story；旧 Phase02 曾承认 valid:false 后仍通过，当前不能重演。主会话承诺等义修正 ROADMAP，需随后调用 `user-story.validate` 验证，再按 User Flow Coverage 写报告。该机械格式修正不扩大历史业务范围。
4. **最终全局回归门。** 868 CI Ubuntu 在新 `cli_color` 测试 `clippy::type_complexity` 失败，主会话已提交 f8851ca type alias；后续必需 CI 以该 head 为准。868→f885 只有测试文件 2 行，生产/CI/lock/fixture 无变化时，868 bench 可以附 diff 证明迁移覆盖；不能宣称 f885 新测试已由868执行。

## 当前源码差异与复用边界

- `git diff --name-only adc0227 HEAD -- crates/ontoforge-query crates/ontoforge-storage/src/query_read.rs crates/ontoforge-engine/src crates/ontoforge-engine/tests crates/ontoforge-schema/src/ir/binding.rs crates/ontoforge-schema/src/validate/bindings.rs crates/ontoforge-schema/src/lint/binding crates/ontoforge/src/online.rs` 只输出 `crates/ontoforge-engine/src/trino/mod.rs`。
- 该文件 diff 仅新增 `TrinoEngine::l2_edit_catalog` 只读 accessor（从同一 ACL config 返回 edit catalog）；原 execute/describe/cancel/SQL 路径未改。故可重用 **adc0227** CI **41669982375** 的 Phase02/04 行为日志，明确声明是已存在日志，不假称本 subagent 重跑。
- 历史 Phase02→当前：schema 引入 scope/action 结构；binding 校验/lint核心保持，`online.rs` 仅把 `config_diagnostic` 改成 `pub(crate)`。bind_check 由 AnyEngine 参数泛化为 Engine、drift 使用整轮 deadline；新增 named tests 实际锁定 online 保留每查询 timeout 与 drift 共用 budget。
- 历史 Phase04→当前：HMAC 版本为 `v1.` format，Query/Action共享 VersionClaims；L0 pk/list/aggregate 审计 reserve/finish 包住真实结果、保留 request ID；所有 source lookup 都经 `check_source_target`。没有用仅更新指纹掩盖这些演进。

## Goal-backward 源码与数据流

| 合同 | 当前实现链 | 已亲读的断言/日志 |
|---|---|---|
| BIND-01/02 | `ir/binding.rs`/`validate/bindings.rs` → profile binding；e13..e16/w17 注册与共享 binding_def | 原始 Windows 日志命名 schema/lint/binding GREEN；SOURCE 文件与 adc0227 相同 |
| BIND-03 在线与 PK | CLI `online.rs` 选引擎→`describe_bindings`→`compare_binding`；仅 check_pk flag 进 `pk_check` | integration:1058/1059 `check_pk_finds_duplicate_and_null_keys_on_ci_trino`、`validate_online_against_ci_trino` ok；Windows:1684 `check_pk_without_flag_runs_no_scan` ok |
| BIND-03 drift | `describe_bindings_before` 逐catalog描述、deadline递减；引擎错误与目标降级分离 | `drift_budget.rs` 断言 online `[5s,5s]`、drift `[5s,4980ms]`，零budget不发probe；Windows:2384/2385 named GREEN |
| ENGINE-01/02 | `execute_unchecked` timeout→QueryGuard；run 中 checked_next_uri 后才登记 GET/DELETE；CancellationTasks 追踪并 drain | `trino_resilience.rs:714..797` 亲读实例隔离、等待取消调用者被drop、404/500/超时、保留原 timeout；Windows:2641..2644 GREEN；integration:1023 真Trino取消 GREEN |
| ENGINE-03/04 | `PrintedSql::new` printer私有；quote_string/quote_ident；SQLite 单连接流式读取；Trino真实 corpus | `printer_prop.rs` 独立sqlparser单statement/原值与SQLite roundtrip；`trino_it.rs` 每个字面量具体值/alias断言；integration:1024 GREEN |
| QUERY-01 | `QueryService::execute`→audit reserve→read_object_edit→tombstone/all_patched判断→单表LIMIT2 source→overlay→grant.evaluate→投影→version | pk_read `edited_pk_uses_store_only`、`missing_read_has_no_side_effect`、`scope_runs_after_overlay`、tombstone/release GREEN；`trino_query_it:449` 实值与3页keys/has_more断言 |
| QUERY-02/03/04 | typed compiler→并行source/edit候选→CandidateSet→bounded256 backfill/refill→eligible_row scope/recheck→排序/游标 | `every_where_operator_validates_and_evaluates`、pagination false-positive/refill/budget/cursor tests 原始 Windows:2943..2953/2991 GREEN |
| QUERY-05/06 | link按scoped merged FK或M:N edges/edit读取；set先 `authorized_keys` 后 `execute_list_internal` 正常hydration | searchAround 257 bounded batch、独立 edge oracle、set独立BTreeSet oracle 与结果分页 GREEN；Windows:2927..2929/2967..2969 |
| QUERY-07/08 | aggregate source partial + old/new correction、TopN、scope后的exact group/distinct；无预算时typed error | `aggregate.rs` 独立 CSV 全量reference亲读（未调用production生成expected）；固定值/属性test GREEN；integration:1170 aggregate corpus GREEN |
| QUERY-09 | VersionClaims canonical attrs/scope/type/pk/def/source/edit→HMAC v1；cursor另绑定ordering；public错误固定；最终row/bytes cap | `security.rs:2173` 断言v1及scope变化token变化，Windows:2957..2959 GREEN；physical_names:2954 GREEN |
| 演进：请求snapshot | QueryService持有Arc<ActiveDefinition>/已编译scope；reload途中不改当前请求 | `definition_snapshot.rs:107` 阻塞engine→scope_apply/reload→释放→旧请求仍读旧grant、新请求denied、audit def_version一致；Windows:2838 GREEN |
| 演进：审计取消 | pk/list/aggregate wrapper 保留取消event并释放队列槽 | `audit_l0.rs:703` abort后status cancelled、row_count0、pending0；Windows:2803/2808/2820 GREEN |

日志公共前缀：`target/phase7-ci-41669982375-`，以上 Windows 为 `windows.log`，integration 为 `integration.log`。日志还存在 Ubuntu/services，未将 ignored 行计为已执行。

## Artifact / Key-link 工具结果与人工纠偏

本 verifier 本轮实际执行所有 17 份 PLAN 的 `query verify.artifacts` 和 `query verify.key-links`。Phase02：51/53 artifacts、29/31 links自动匹配；Phase04：24/26 artifacts、18/19 links自动匹配。以下全为确定可追踪的旧pattern/模块拆分，不是实现缺失：

| 静态未匹配 | 实际连接 |
|---|---|
| P02 client.rs缺v1/statement | `trino/mod.rs:268` statement_url→base.join("v1/statement")，client::send接受已构造RequestBuilder |
| P02 AnyEngine::Trino pattern | `engine/lib.rs:181/192/199` 使用Self::Trino(e)逐方法分发；CLI构造AnyEngine::Trino |
| P02 e14内缺DefKind::SourceBinding | e14.rs:23→`binding/mod.rs:33..36 binding_def`→该DefKind |
| P02 ci.yml缺trinodb/trino:483 | workflow调用ci/query-services.sh trino→该脚本:70固定483 digest运行容器 |
| P04 EditStore::read_object_edit exports | `query_read.rs:347` impl公开method→read_object_edits，query/lib消费 |
| P04 QueryService::execute_list exports | `execute.rs:506` impl公开method→audited→with_budget→normal/set/link dispatch |
| P04 set旧函数名pattern | `set.rs:56` authorized_keys→`:68` execute_list_internal，正常授权hydration |

决策覆盖本轮owner query：Phase02 **15/15**、Phase04 **7/7**，均无not_honored，仅advisory gate。两phase的需求集合分别8/8、10/10，无orphan。Phase02 BIND-03启动/周期、ENGINE-02 health/describe、ENGINE-04 serve-dev后继责任在Phase07，不用Phase02局部实现冒充最终scope。

## 反证、测试质量与禁止项

- 反证1：Iceberg普通GREEN实际提前return，保留必须补证项1。
- 反证2：bench smoke每路1+1且不判断p95，不满足QUERY-10；保留补证项2，未降低阈值。
- 反证3：后继取消/drift/audit状态变更不能靠grep推断；已亲读上述新测试具体断言并核对原始named GREEN。
- 当前 `query/src`、`engine/src`、`storage/src/query_read.rs`、`ontoforge/src/online.rs` 扫描 `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER|PROVISIONAL` 无命中。测试数据`Sam Placeholder`为明确虚构人员fixture，非业务stub。
- source→SQL→engine rows→overlay→scope→结果路径无静态空数组断链；初始化空容器均后续由真实fetch/store填充。该基础库无UI人工外观项。
- Phase02 test-tier禁止项具生产enforcement：same-origin/no redirects/HTTPS+auth构造拒绝、SecretString与错误消毒、无TLS绕过配置与Clippy方法禁令、Trino fail-closed、PK扫描flag guard。相应原始Windows named GREEN已确认；不会把声明verification:test直接当成通过。
- Phase02公开CI不接公司集群属judgment-tier：虚构航空fixture、loopback/digest容器、私有fixture只从仓库外env入口且公开CI不调用。按当前用户授权与CC AGREE作**non-authoritative**源码/测试判断，不伪称用户亲测，也不重复索要人工审批。真实SQL Server私有环境仍明确不在本轮证据范围。
- Phase04 PLAN无独立prohibitions块；安全要求已包含于truths及QUERY-09，不据缺块漏检。
- 未发现需要新增scope或推翻先前完成项的无证据架构意见。

## 首轮恢复动作（历史检查点）

主会话已等义规范ROADMAP User Story并报告01..04验证true；恢复时亲核02/04。等待动态owner的Iceberg日志与bench artifact/最终CI。复核源身份等价、独立重算p95和fragment数、MVP goal校验后，写两份canonical VERIFICATION与GSD生成fingerprint，不能提前刷passed。fingerprint只列真实implementation/test/配置和必要证据，排除全局ROADMAP/REQUIREMENTS/STATE与审计digest以免非语义循环。

## 最终复验（2026-11-04，f8851ca）

本次独立续验不重跑Cargo/Docker，不改实现，不触发nightly/issue，不提交。当前SOURCE SHA为`f8851ca073fed34e80e605f0ac91880259df7e12`，不与release packaged merge SHA混淆。

| 产物 | 最终结果 | 精确边界 |
|---|---|---|
| 02-VERIFICATION | gaps_found，100/103 | 5SC+98PLAN truth；100 verified（含2个ADR0022历史override）、1 failed MinIO合同、2 uncertain数据来源judgment |
| 04-VERIFICATION | passed，36/36 | 5SC+31PLAN truth；当前真实backend+isolated p95证据 |
| 02-VALIDATION | validated，nyquist_compliant=false，wave_0_complete=true | 28task：26COVERED、1PARTIAL（02-10-T3 MinIO）、1文档SKIP；GSD append-audit已写轨迹 |

### 新证据的独立核验

1. `query user-story.validate`对两个当前ROADMAP goal均返回true；`check.decision-coverage-verify`分别15/15、7/7。格式不再阻塞。
2. `gh run view 41759804733`最终head f8851ca，Windows/Ubuntu/integration/phase7-real-services四必需jobs全部success。已读最终U/I原始日志，所有Phase02 task映射中的已列命名测试都找到实际ok。Windows全文日志也已保存；不把ignored当运行。
3. 483/484专用fixture均真实CTAS29行；runner明确设置两个环境变量；原始各1 passed/0 ignored。亲读trino_it:626后的columns/source/copied断言，排除普通integration提前return。具体hash见02-VERIFICATION。
4. 动态51passed是owner执行，本verifier核其证据不冒充重跑。原runner真实exit1（HEAD改变）保留；`source-equivalence.diff`仅cli_color类型别名；`final-check.log`独立exit0是provenance/凭证扫描/清理结果。
5. `41759457559`仅isolated-query-bench job success，整个run因旧Clippy失败为failure。原artifact SHA256 `13c530fb55b756d01be417926b719801c8544fedfe9a6b049edea1ced572df99`与文件实算一致。
6. 4路各5warmups+40measured；对40个时间排序取第38个：123.194768 / 425.338425 / 374.549715 / 0.913019ms，分别严格低于500/1000/1500/50ms。180样本error均null、limit_error均false。450个完整EXPLAIN文本重数均1 fragment，等于字段且≤2；edited45样本source_calls全0。guardrail32/10000/2048/64MiB及aggregate1024/4096/10000/16MiB均未升。
7. 868→f885完整diff只有`crates/ontoforge/tests/cli_color.rs`，性能与动态输入源码相同；最新普通回归由f885 CI负责。旧04-PERFORMANCE不扩充覆盖新源码。

### 未关闭的确定缺口与独立意见

**BLOCKER：MinIO合同。** accepted ADR0017:16及02-10 PLAN truth6明确JDBC catalog+MinIO；实际`iceberg.properties:13-15`为local warehouse，nightly没有MinIO启动/配置。SUMMARY:52/211-218披露镜像不可取与供应链理由，但未有accepted ADR修订。历史UAT/VERIFICATION仅证实local路径，不构成批准永久替代。

主会话按用户最新要求重新取得不含我方方案的独立盲评，原文在`target/iceberg-fixture-independent-opinion.json`。其结论：“规范偏差”并伴随ADR特定测试覆盖缺口，不是已证产品功能失败；与本次直接源码事实相符。未接受原先“local可以直接等义结案”的假设，未引入替代ADR或新镜像。

**最小收口条件**：需求所有者明确决定保留MinIO并补受信路径的483/484实测，或明确接受local后正式修订历史合同。该核心问题由主会话处理；本worker没有另行向用户重复提问。ROADMAP后续Phase07.1只负责STORE-05，不能拿它defer MinIO。

**2个WARNING：judgment禁止项。** 02-03公开仓库数据来源、02-10公开CI不接公司源。源码/测试与隔离fixture支持non-authoritative判断；用户授权第二意见同向自动推进不代表人工证实来源。保留`unverified-prohibition — human review recommended`，不会写成human pass或静默passed。不存在新增无证据架构advisory。

### GSD产物检查

02/04 fingerprint均由`query verification.fingerprint`直接生成，覆盖implementation/test/CI/fixture，工具自动纳入phase PLAN/SUMMARY；未手写digest、不把共享STATE/ROADMAP/REQUIREMENTS加入循环。写后`query verification.status <dir> --pick status`均exit0，分别gaps_found、passed。

只修改本worker授权的02/04 VERIFICATION、02 VALIDATION和本账本；不更新共享STATE/ROADMAP/REQUIREMENTS/其他worker文件。整个milestone不能由Phase04通过推导全满足，STORE-05仍有独立Phase07.1验收。
