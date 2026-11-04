# Phase 01 / 03 当前源码重新验证

日期：2026-11-04。执行者：`gsd-verifier`。仅写本文件与两份目标 `VERIFICATION.md`，不修改实现、不提交、不运行 Cargo 或服务。

## 初始审计范围与状态（下方最终复签为当前结论）

核对旧 VERIFICATION / UAT / VALIDATION、ROADMAP 五条 SC、PLAN artifact / key link / prohibition、当前生产调用链和已保存的原始 CI 输出。SUMMARY 只用于定位文件与历史提交，不作为行为证据。旧报告的历史结果保留为历史，不冒称本次运行。

| Phase | 代码结论 | 待完成 |
|---|---|---|
| 01 | passed，11/11（含1个既有用户接受override）；编辑器未逐项观察的限制保留 | VERIFICATION已写，canonical status=passed、exit0 |
| 03 | passed，8/8；当前源码、真实SQLite / PostgreSQL / Trino行为及新TLS guard有证据 | VERIFICATION已写，canonical status=passed、exit0 |

当前没有新增功能 BLOCKER。完整新 HEAD CI 由 orchestrator 执行，本文件不会把旧 run 算作新改动的通过证据。

## canonical stale 与差异

`query verification.status` 返回的初始 `moved_files`：

- Phase 01：`.github/workflows/ci.yml`、`.planning/REQUIREMENTS.md`、`CONTRIBUTING.md`、`Cargo.toml`、`crates/ontoforge-schema/src/lib.rs`、`crates/ontoforge-schema/tests/schema_export.rs`、`examples/ontologies/aviation/objects.yaml`、`schemas/ontoforge-definition.schema.json`。
- Phase 03：`.github/workflows/ci.yml`。

这只是覆盖文件变化提示，不是生产功能失败。旧 Phase 03 fingerprint 只覆盖 CI、cache fixture 和一份测试，覆盖过窄；本轮扩至真正的 storage/auth/expr 实现、测试与调用边界。全局 STATE / ROADMAP / REQUIREMENTS 的状态 digest 不进入 fingerprint；本 Phase PLAN、SC 在报告内的契约、实际实现和 fixture 都保留。ROADMAP 的既有 `mode: mvp` 与非 User Story goal 格式冲突继续记录，本次是历史合同重新验证，不伪称新 MVP UAT。

`git diff adc0227 -- crates/ontoforge-schema crates/ontoforge-expr crates/ontoforge-auth crates/ontoforge-storage crates/ontoforge/src` 在检查时为空。`81788e4` 新增 CLI 颜色测试及 VALIDATION，未改生产代码。当前 engine 新增 `l2_edit_catalog` accessor 与 MCP L2 修复属于 INT-01，不能由旧 CI 证明已通过。

## 原始行为证据

已亲读 `target/phase7-ci-41669982375.json`：head `adc02278b5cc55111def9a30a1c3b778485e8279`，四个实际 job success，isolated benchmark 按条件 skipped。将原始日志限定在 `Run cargo test --workspace --locked -- --test-threads=1` 的 step 内重新计数，Windows 为 149 suites / 1490 passed / 0 failed / 117 ignored，Ubuntu 为 149 / 1497 / 0 / 117。整个 job 后续再次运行 Action 测试，不能把重复结果累计为 workspace 数量。

| 来源 | 已亲核行为 |
|---|---|
| `-windows.log:3295,3370,3373,3376` | YAML/JSON 同 IR；逐 document modeline；入库 schema 等于 exporter；编辑器映射覆盖 |
| `-windows.log:2690,3305,3306` | CEL / JSON / YAML 深嵌套隔离测试通过 |
| `-windows.log:2241,2119,2133` | 多 role 不产生列×行泄露；整对象 criteria 列覆盖；配置不能独自启用 dev token |
| `-windows.log:3918` | `model_matches_store` 1/1，通过随机提交逐步比较独立内存 model、versions、行和 replay |
| `-integration.log:579-628` | PostgreSQL conformance 45/45，含 append-only UPDATE/DELETE/TRUNCATE、log/history、decimal 精确往返、版本栅栏 |
| `-integration.log:629-684` | history 1、import 2、migrate 4、reload 11、scope 3，全部实际运行通过 |
| `-integration.log:1083-1091` | Trino 行谓词 corpus、seeded differential 3/3 |
| `-integration.log:1111-1119` | reload Trino 3/3，含 `cached_column_requires_flush_before_definition_switch` |
| `-services.log:611` | `pg_http_mixed_snapshot_reload_revoke_and_history` 通过；源码确实暂停请求、reload/revoke、释放请求并检查旧请求成功、新请求失权及 audit 旧版本 |
| `01-VALIDATION.md` | 独立 Nyquist 执行单元记录 schema/expr 399、CLI 52，共 451/0/0；本 verifier 复用其余记录，不冒称重跑 |
| 本 verifier 命令 | `target/debug/deps/cli_color-4d01ddf867a0fcb3.exe --exact color_environment_controls_human_output_and_never_colors_json --nocapture` 实跑 exit 0，1 passed / 0 failed / 0 ignored，0.64s；直接运行既有编译产物，未运行 Cargo |

## 代码链路核验

- Phase 01：CLI `pipeline::check_with_profile` → `load_dir` → discover/parse/dispatch/merge/validate → `check_ontology`；lint 先拒绝结构错误，随后读配置、运行 registry、从 SourceMap 回查诊断位置。render 的 JSON 分支序列化真实诊断，human 分支经 miette 输出真实源文本。IR 测试加载真实 aviation YAML/JSON 并断言 5 object / 5 link / 3 action 与完整相等。
- Phase 03：`reload` 在 definition transaction 内 derive/diff/role-check，commit additive DDL 后等待 probe，再 publish active；`migrate::apply_in` 先锁版本并 `load_scope_in + compile_scope`，再执行变更。`commit` 同一事务 append header/entry/history。history 对当前 scope 遮蔽真实 DB log；不是静态数组。
- 后继接线：`request_snapshot` 在同一锁内读 active definition 与 scope，结束事务后进入网络；server `context::build` 使用 snapshot 编译 scope；`history_page` 把 active snapshot 传入 `history_with_snapshot`。本轮确认后继真实测试的断言与这些行为吻合。
- 自动扫描 Phase 01 58 个 artifact / 32 个 key link 均通过；01-13 未声明两类 block，记 N/A。Phase 03 唯一 key-link pattern miss 是 03-03 的直接 `ddl.rs → sql_exit.rs`，手工追到 `reload::execute_steps/apply_step → sql_exit::execute_ddl`，真实链接存在。工具 pattern 不匹配不等于缺实现。

## 禁止项与判断边界

1. 01-01 禁参考仓库标识符 / 代码复制：实际对 `crates`、`examples`、`schemas`、`CONTRIBUTING.md`、`docs/edit-store.md`、`docs/scope.md` 的列举禁名扫描零命中。原创出处无法仅由 grep 完全证明，属于 non-authoritative 自动判断。
2. 01-01 禁绝对路径/用户名：实际诊断 `file` 由相对 root 路径生成，loader/CLI 测试断言该协议；产品快照扫描零命中。构建日志中的 checkout 路径不是产品诊断。
3. 01-08 禁关闭/降级 ONT-V：`lint_cmd.rs` 在配置处理前返回结构错误，`lint/config.rs` 拒绝 ONT-V ID；CLI negative tests 已通过。
4. 01-13 backstop：`git diff --name-only d021687 07a0e06 -- ':(glob)**/*.snap'` 零输出；该提交段的 `diag.rs` 只加位置换算函数与测试，JSON Diagnostic 形状没有改变；当前诊断/快照测试也通过。后继 phase 的显式 schema 演进不倒算为 01-13 违反。
5. Phase 03 test-tier prohibition：dev token 显式启用、SQLite 字符串谓词差分、import source-backed/create-only 拒绝均有真实执行测试。旧 OpenSSL guard 的 `! cargo tree` fail-open 已由独立执行单元修复，见下面的最终闭环；旧 CI 原始输出明确两个包均 `did not match any packages`，不是禁依赖已进入产品。

`target/milestone-tls-and-judgment-review.json` 已读，Claude Code 第二意见为 `AGREE`，认可上述证据与限制，并依据用户已授权的同向自动推进处理 judgment 项。报告保留 non-authoritative 标识与原创出处限制，不制造新的重复人工审批，也不把 UAT Test 2 的三个未逐项观察项目写成已观察。

## 其他 gate

- 两个 CONTEXT decision coverage 均 18/18 honored、0 not_honored（工具实跑）。
- 核心生产目录与 CI 未检出未引用的 `TBD` / `FIXME` / `XXX`。
- schema/expr 核心测试没有 ignore；auth 中外部 Trino 三个 ignored tests 在 integration job 明确运行。PG conformance 宏接到相同 suite，缺服务会 panic，不会静默通过。
- 参考模型独立维护行状态与错误类，SQL differential 实际执行 SQLite/Trino；不存在将被测输出保存为本次 expected 的循环证明。
- 旧 Windows SQLite race flaky 风险保留：旧 UAT 的首跑锁冲突真实存在；新的冻结 CI 同名测试通过只证明这次执行，不能证明永不再现。
- 01-UAT 3/3、03-UAT 2/2 保留原用户/外部系统结论。本次不启动编辑器/服务、不重做用户已接受步骤。

## TLS闭环与最终交付边界

2026-11-04T02:08Z恢复后已亲读 `ci/check-tls-deps.sh`、`ci/test_tls_deps.py`、CI step、`TLS-GUARD-FIX.md` 与 `target/milestone-tls-real-tree.log`。原始RED为6失败，GREEN由本verifier独立复跑：`python -B -m unittest discover -s ci -p test_tls_deps.py -v`，6/6、0.439s；Bash syntax与diff-check均exit0。主会话真实helper记录 `tls_real_exit=0`，本verifier未调用Cargo。

测试会从实际workflow取TLS命令，fake Cargo提供neither/openssl/native-tls/both/查询失败/相似名称六类结果，验证门禁退出码且恰好一次完整graph调用。helper与tests的blob分别为 `dd70463dd72b164e2a66a520f7ffd673cff56362`、`6019ab3bf77e8a90b3a166e9b34624ed284e3e46`，CI blob `5a0ff8327a0305b0972d9652cce5672330b96656`。当前新HEAD远端CI尚未运行完，旧41669982375不会被扩称为这些新增step已通过。

Phase 03自动声明总数为 **89/89 artifacts、54/55 regex key links**；唯一regex未命中已手工证明真实链路，最终55/55 wired。Phase 01为58/58 artifacts、32/32 links，01-13两项N/A（工具exit66，非错误验证通过）。两报告于2026-11-04T02:11:14Z写入，canonical fingerprint覆盖分别312/314个文件；`query verification.status` 两者均返回passed、exit0；三份交付文档的`git diff --check`通过。完整新HEAD CI终局与INT-01仍由orchestrator追踪。

补充本verifier只读spot-check：既有CLI对aviation `validate --format json` 输出 `[]`、exit0；invalid/v04输出ONT-V04、objects.yaml、3:1、span34/9及primaryKey建议、exit1；`lint --deny-warnings --format json` 输出 `[]`、exit0。这些是本次真实执行，不属于历史日志复用。

## 2026-11-04 最终复签

最终CI41759804733，branch head f8851ca073fed34e80e605f0ac91880259df7e12，PR merge22913c8e79ae98b6604109d6e05a15f27cd50292。verifier独立gh查询completed/success，Windows/Ubuntu/integration/phase7-real-services全部success。亲读新原始日志，只在workspace step计数：W150组1496passed/0failed/117ignored，U150组1503/0/117；后续corpus/fault不叠加。TLS hermetic双平台6/6、实际helper成功。

| 文件 | 本次重算SHA256 |
|---|---|
| target/milestone-ci-41759804733-windows.log | b6ecb8fa1fb5c4459c9e7c8e01074d99c142456d65ab76afd66ccac613679eaa |
| target/milestone-ci-41759804733-ubuntu.log | dd5f4c4165adb6792daa3889c85418d50a14821adf2171352492d57775b27975 |
| target/milestone-ci-41759804733-integration.log | 6ecde94b35302e75b4484d4268281d5216fb09f8f2856013075d081050ca6f25 |

本地动态原runner exit1由HEAD改变触发，独立final-check exit0证明唯一区别是cli_color tuple→type alias、动态生产输入一致，原runner失败记录保留。未重新运行Cargo/Docker/服务、未改实现或共享STATE/ROADMAP/REQUIREMENTS、未提交。用户后续要求第二意见独立提方案；这里旧AGREE记录仅作历史，没有沿旧协议新增请求。

Phase01当前为 **human_needed，11/11 truth（1个既有用户editor override），3个judgment flag**。这是高优先级verifier状态规则要求，不是发现功能回归或要求重做用户已接受的编辑器测试。原01-01:104、111及01-08:81均明确 `verification: "judgment"`，`status: "resolved"`描述规划时决策，不能当作验收时逐项人工resolution。

| 原PLAN禁止项 | 自动证据已证明什么 | 未完备边界 |
|---|---|---|
| 01-01 DX-03：禁参考库标识符/复制代码 | 列举产品目录禁名扫描零命中 | 原创代码出处无法由grep完备证明；NON-AUTHORITATIVE |
| 01-01 SCHEMA-06：禁路径/用户名泄漏 | file相对root生成链路、loader/CLI路径断言与产品快照扫描通过 | 未覆盖产物和完整输出空间不是已逐项观察；保留judgment处置flag，不能说缺少自动测试 |
| 01-08 LINT-04：ONT-V不得关闭降级 | lint先结构校验、config拒绝V ID、validate_codes_cannot_be_turned_off_by_config实跑通过 | 该行为已有明确自动证明；flag是原judgment-tier显式resolution未闭环，不是功能无证据 |

历史用户同向自动推进授权与Claude Code意见保留，但不伪称逐项人工见证；本worker仅报告flag，不向用户重复提问。

Phase03当前 **passed，8/8**；全需求STORE-05保持PARTIAL，edit子合同通过，source报告缺失按ROADMAP Phase7.1 SC1/SC2显式deferred。VALIDATION为validated、wave0=true、nyquist=false：55task=50可执行COVERED+5静态SKIP；不能把task覆盖当全需求覆盖。五份SUMMARY六项dated metadata差异已逐项核对，原completed历史保留、没有补STORE-05。

当前ROADMAP合法User Story已重新读取，01/03补User Flow Coverage；旧“非User Story格式冲突”属于历史，不继续作为当前事实。

最终自检：GSD verification.fingerprint生成01覆盖315文件、03覆盖318文件，原样复制covered_files/covered_digest；verification.status分别返回human_needed、passed，均exit0。四个当前User Story由canonical user-story.validate检查均true；git diff --check通过。01没有新增功能gap；03的STORE-05全需求仍PARTIAL。交付文件已落盘，未提交。
