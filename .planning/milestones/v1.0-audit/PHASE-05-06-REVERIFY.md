# Phase 05 / 06 当前源码独立复验

## 初始审计快照（下方最终复签更新状态）

2026-11-04，核查源码 `868643cb0d131b9a7f1fdf2a16a53d1ac445efe8`。本 worker 仅拥有本文件与 `05-VERIFICATION.md`、`06-VERIFICATION.md`，不运行 Cargo、服务，不修改实现、不提交。动态运行由 `milestone_dynamic_validation` 独占。

已读取 GSD verifier 必需参考、MVP 规则、全部 18 个 PLAN 的 frontmatter / 验收指令、两份旧 VERIFICATION、Phase 05 UAT、两份 VALIDATION、ROADMAP、REQUIREMENTS，以及 milestone integration、requirements、Nyquist、L2 catalog、TLS guard 证据。项目无本地 AGENTS.md、RTK.md、`.codegraph` 或本地 skills 目录；遵循会话注入的 AGENTS.md。`query agent-skills gsd-verifier --skills-only` 无额外注入。使用 GSD 核验并持久化本 ledger；没有调用 unavailable 的 Warp 或 ralph-loop。

目前没有新增可确认的实现缺口。尚不刷新两份历史报告的 `passed` 与 fingerprint，等待必要动态证据归齐。历史 UAT 是 17 个 automated 条目加第 18 个用户总体确认，不能表述成用户逐项手测。

## 必须补齐的证据

| 项目 | 当前判定 | 原始证据 / 后续动作 |
|---|---|---|
| SQLite `action_atomic_`，feature `test-support` | 已亲读 named GREEN | `target/milestone-audit-evidence-20261104/sqlite-action-atomic.log`：5 passed / 0 failed / 0 ignored；after_edits、after_command、after_log、success_changes_all_domains_once、after_outbox 全部 ok。命令和 exit 0 由 DYNAMIC-VALIDATION.md 记录。 |
| PostgreSQL `source_guard_absent_target_insert_race` | PENDING | `source.rs:627` 受 embedded-pg 控制且非 ignored。原 CI 默认 feature / `--ignored` 均不能证明该 case。必须在真实 PG fixture 取得 1 named GREEN。 |
| 当前 CI 新 feature 入口 | PENDING | `.github/workflows/ci.yml:73` 新增 SQLite 5 个发现门；`:145` 新增 embedded-pg PG absent-target 精确发现及执行。旧 CI 未覆盖新步骤，不能记为新 CI success。 |
| 当前真实服务矩阵与清理 | PENDING | dynamic owner 正在执行；等待 DYNAMIC-VALIDATION.md 与原始日志、HEAD / exit / cleanup。 |

## 直接源码证据

### Phase 05

- `action/src/lib.rs:288` 的 `execute_inner` 先验证 host limits 与 action grant、使用 `(principal_id, command_id)` 查询旧结果，再 resolve 参数和采样；`prepare_edits` 聚合逐参数错误并复用 `source::prepare`。
- `storage/src/commit.rs:606` 的事务严格为 begin、fence、claim、稳定锁定、CAS、同步授权 guard、log、edit/entries/history、command receipt、outbox、finish_action。validate-only 进入相同 guard 后 rollback；四个故障注入点位于真实写入之后。
- `commit.rs:940` 的 claim 使用 SeaQuery `OnConflict::columns([principal_id, command_id]).do_nothing()`，真实 SQL 经 `sql_exit::build/execute_built`；Existing 分支 rollback 后重读，避免旧 statement snapshot。
- `action/src/version.rs` 只调用共享 `VersionTokenCodec::verify(VersionClaims)`；`fingerprint.rs` 对递归对象排序，原 request params 在 default 填充前参与 hash，不含 command_id 或 server chosen time。
- `action/src/source.rs:35` 每个 alias 独立计算 criteria/row predicate/column predicate/token/release 需求；source snapshot 按 RowKey 去重，`seen` 只去重锁内 guard read。共享 `read_pk_source` 真正查源并拒绝重复 pk。源 I/O 在写事务之前完成。
- `action/tests/source.rs:629` 使用真实 PG 两个 store：准备时读 absent edit，gate 在另一连接插入，再断言 `VersionConflict`、仅对手一条 log、无自己 command/outbox。这是确定性竞态断言，不是顺序 happy-path。
- `action/tests/action_pg.rs:211` 使用两个独立连接、Barrier 与 join；`:255` 精确断言 `(wins, conflicts) == (1, 1)`、版本、log/command/outbox 计数。
- `action/src/outbox.rs:124` 一次 claim 一条，先提交 lease 再发 HTTP；due cutoff 固定，完成时才计算 backoff；store mark/retry 带独立 lease_owner fencing。配置在 worker 构造绑定，request/params 不能覆写 URL。
- CLI worker 真实进程测试覆盖提交后送达、HTTP failure 重试、重启后恢复；历史 WSL systemd 隔离验收仅证明机制。生产接收端部署继续是 release 前已声明 deferred，当前不发布。

### Phase 06

- `query/src/l2.rs:134` discovery 从可信 `config.l2_acl.edit.catalog` 获取硬排除项；`:441` execute policy 使用相同配置。`printer/l2.rs:454` 先排除大小写无关 edit catalog/system，再匹配 catalog/schema grants。
- `validate_l2` 受 parser stack/input cap 约束，单条白名单 AST 经 Display 重序列化后构造 `PrintedSql`。execute/explain 与 discovery 的结果来自 Trino `QueryOutput`，没有静态返回。
- `query/src/lib.rs:265`、`execute.rs:527`、`aggregate.rs:912`、`l2.rs` 的两个统一边界都 reserve 后执行、finish 后返回；aggregate 的内部 list 不重复创建审计。metadata 模型不存 SQL/result 原文，只记录 hash、principal、roles、def_version、request_id 等。
- `storage/src/read_audit.rs:232` 使用共享 `Worker::Running/Finished`，持 async mutex 借用 JoinHandle await；取消保留 handle，结果在后续 await 前缓存。`conformance/mod.rs:2685`、`:2713` 的测试先显式观察 Pending、完成 guard，再查询真实 durable row；JoinError 有实际 abort 测试。
- `l2_monitor.rs:285` run_cycle 获取 recheck/admission 锁，先清理旧 probe，再创建新 probe / run_checks，清理成功才 complete Enabled；health TTL、generation、epoch、stopped 防止过期或旧结果启用。
- `engine/trino/mod.rs` 相对已运行 CI baseline `adc0227` 仅新增只读 `l2_edit_catalog()` accessor；其余 Phase 06 受核实现与 baseline 内容相同。accessor 从同一个私有不可变 attestation 配置读取，不修改 health admission。
- Phase 06 库合同和 Phase 07 MCP transport 合同明确分离；本轮 INT-01 是 Phase 07 adapter 丢失可信配置，不能据此将正确的 Phase 06 library policy 判成未实现。已读 MCP catalog 修复 RED/GREEN / 83 tests、fmt、Clippy 记录，最终 transport 验收由 Phase 07 owner 负责。

## 既有原始动态证据复用

亲读 `target/phase7-ci-41669982375-windows.log`（W）和 `target/phase7-ci-41669982375-integration.log`（I）；CI head `adc0227`。对比 `git diff --name-only adc0227 HEAD -- <phase covered implementation paths>`，Phase 05 仅 CI 文件改变；Phase 06 仅 CI 与上述只读 accessor 改变。旧日志仅用于未变化实现，不代替新 CI 步骤。

| 合同 | 原始 named GREEN 位置 |
|---|---|
| Action 参数、批次、validate-only、scope | W:1947-1952、2029-2038；逐参数/声明顺序/后项失败和无写入由真实 SQLite tests 断言 |
| 同 pk 与 stable lock | I:783、785；同 target 另含 reverse-order 与 absent create，4/4 |
| 幂等与响应丢失 | W:1985-1992；I:798-802 同 target 6/6，实际 PG |
| 按需 source / 重复 pk / offline | W:2047-2056；I:1236-1239 实际 Trino 4/4 |
| outbox lease/crash/retry | W:2004-2014；I:817-818 真实 PG；CLI W:1898-1900 |
| PG atomicity | I:761-765，5/5 实际执行；SQLite 新补证见上表 |
| AST 104-ID 与拒绝零 POST | W:2400-2415、4247-4252；104 个 constructed-equivalent，原始历史 SQL 恢复数为 0，不扩大 provenance |
| L2 health / monitor cleanup / cancellation | W:2343-2344、2422-2456；I:954-991 真实 Trino 六项 |
| L0 / L2 audit / cancellation / errors | W:2803-2831、2846-2875；assert rows/status/zero sends，不仅 HTTP status |
| 请求 definition/scope 快照 | W:2838-2839，L0/L2 两个竞争用例 |
| 双后端 audit durability/shutdown | W:3703、3800-3807；I:709-720 共9个实际PG审计用例 |
| 管理变更 hash、lint、rollback | CLI reload/scope、`scope_store` 目标 W GREEN；W:4029 `audit_insert_failure_rolls_back_scope_and_indexes` |

## Artifact / key-link / quality gate

- 18 个 PLAN 已独立运行 `query verify.artifacts` 和 `query verify.key-links`，非零退出按 JSON 解析，没有吞掉。
- Phase 05：artifact 27/28 自动通过。唯一 `Missing export: EditStore::commit_action` 是 Rust impl 方法启发式误报，`commit.rs:568` 明确为 pub async method 并由 Action 同事务使用。21 个 links 自动 18 passed，三项 pattern 漂移：`params_hash` 实际 `request_hash`、literal `ON CONFLICT DO NOTHING` 实际 SeaQuery `.do_nothing()`、`required_source` 实际 `source::prepare/required_properties`。已核真实数据与调用，不视为断链。
- Phase 06：22/22 artifact 自动通过。12 个 link 全部因 Rust module/type 或 workflow Cargo target 不包含完整目标路径而返回 `Target not referenced in source`；已沿 reserve/finish、migration、validate_l2、Engine.execute、monitor.run_cycle、all_cases! 和 CI target 实际接线复核。
- Decision coverage：Phase 05 `honored=7,total=7`；Phase 06 `honored=6,total=6`；两者 `not_honored=[]`、exit 0。
- 受核实现/测试/CI 文件未发现未引用的 TBD/FIXME/XXX，也无 TODO/HACK/PLACEHOLDER；空集合均为输入初始化或拒绝路径，不是数据源替身。
- 全部 PLAN 无 `prohibitions:`、`verification: backstop`、`<human-check>` 或独立 `probe-*.sh`；隐含负向约束通过 source/test 证据核查。本次不生成不存在的人工 approval。Claude Code 的同向自动推进授权见 `target/milestone-tls-and-judgment-review.json`（AGREE）；自动 judgment 不冒称逐项用户观察。
- Requirements：Phase 05 PLAN union 覆盖 ACTION-01..12；Phase 06 覆盖 L2-01..04/AUDIT-01/SCOPE-07，无 orphan。REQUIREMENTS 中 L2-03 的跨 phase Gap INT-01 尚待主会话整体收口，Phase 06 库级 truth 不受 transport 缺口误归因。
- 反证发现：旧 Phase 05 只给 ROADMAP 5/5，没有清晰列出 PLAN 细化合同；新报告必须显式合并事务顺序、fingerprint、lease、真实 CI、源码写入口等验收项。旧 CI 的 absent-target/SQLite fault coverage 是真实证据缺口，正在补齐；不把 ignored/discovery 算 GREEN。

## 交还条件

等待 PG absent-target 与最终运行/CI 后，按上述源码与原始日志写两份当前报告，保留 release deferred 边界，并仅用 `query verification.fingerprint` 生成覆盖字段，最后 `query verification.status` 检查退出码。此前旧报告不刷新为当前通过。

## 2026-11-04 最终复签

最终CI41759804733，branch head f8851ca073fed34e80e605f0ac91880259df7e12，PR merge22913c8e79ae98b6604109d6e05a15f27cd50292。verifier独立gh查询completed/success，Windows/Ubuntu/integration/phase7-real-services全部success。亲读新原始日志，只在workspace step计数：W150组1496passed/0failed/117ignored，U150组1503/0/117；后续corpus/fault不叠加。TLS hermetic双平台6/6、实际helper成功。

| 文件 | 本次重算SHA256 |
|---|---|
| target/milestone-ci-41759804733-windows.log | b6ecb8fa1fb5c4459c9e7c8e01074d99c142456d65ab76afd66ccac613679eaa |
| target/milestone-ci-41759804733-ubuntu.log | dd5f4c4165adb6792daa3889c85418d50a14821adf2171352492d57775b27975 |
| target/milestone-ci-41759804733-integration.log | 6ecde94b35302e75b4484d4268281d5216fb09f8f2856013075d081050ca6f25 |

本地动态原runner exit1由HEAD改变触发，独立final-check exit0证明唯一区别是cli_color tuple→type alias、动态生产输入一致，原runner失败记录保留。未重新运行Cargo/Docker/服务、未改实现或共享STATE/ROADMAP/REQUIREMENTS、未提交。用户后续要求第二意见独立提方案；这里旧AGREE记录仅作历史，没有沿旧协议新增请求。

Phase05当前 **passed，36/36**：5条ROADMAP+32条PLAN细化，05-01双后端回滚与05-04同一合同去重1条。Phase06当前 **passed，22/22**，全部11份PLAN合同保持。两报告behavior_unverified=0；旧报告历史来源保留，新生产/接线审计和新CI不混淆。

G3 SQLite test-support五项已本地与双平台新CI实际5/5；G4 PG absent-target已本地和I:845精确非ignored1/1；PG atomicity5/race4/idempotency6/outbox2、Trino Action4、PG audit9、真实Trino ACL6均实际通过。fresh43含PG HTTP/请求快照、TrinoACL与SDK，另Iceberg2；本地合计51，不与CI重复加为唯一测试数。

| 本地原始文件（target/milestone-audit-evidence-20261104） | SHA256 |
|---|---|
| sqlite-action-atomic.log | 0bd91dc680d7470a057637ed06619c23ec5681a9f8b870510f1108cdefbd07f8 |
| source-guard.log | 871bf528ec12d40b444acfaaddd6b36b5a78ccf4d5567a533824943c60fb7678 |
| trino_l2_acl_it.log | 676cd510f5b7781cfb236ef705cbfb105fa839c15572ae6c7057c7e01266c576 |
| postgres_http_it.log | 3c621462ea4d9271872d3d88601f07561e2d04955f5f00d2fd24886cbf097466 |
| final-check.log | 4480be4a3312dddf3e5436dbd0c0db02772c88b6c564332af0d9a3c7b2237424 |
| source-equivalence.diff | 76a60b330a343f1aab750e0192999d4c9a0541c9f87d546a6b83c85da9ba9c0a |

05-VALIDATION现validated、nyquist=true、wave0=true，14/14task覆盖，GSD append-audit已落地。06独立报告先完成当前通过判定，随后仅为06-08-SUMMARY补L2-01/02与requirements-reconciled:2026-11-04；原completed与当时CI未运行说明保留。GSD summary-extract实际返回[L2-01,L2-02]。104-ID仍是constructed-equivalent，原文恢复数0；生产webhook联调仍为release前提。

Phase06库从可信config硬排除edit catalog，INT-01是Phase07 adapter，不能据此回退正确的库。跨phase需求整体由主审计对账，本worker不替代Phase07或milestone验收。

最终自检：GSD verification.fingerprint生成05覆盖85文件、06覆盖111文件，包含真实实现/测试、最新原始日志与自身PLAN/SUMMARY；covered_files/covered_digest原样复制。verification.status两者均passed、exit0；summary-extract确认06-08为[L2-01,L2-02]；git diff --check通过。所有owned文档已落盘，未提交。
