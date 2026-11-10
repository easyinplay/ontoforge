---
phase: 07-rest-mcp-shell
plan: "03"
subsystem: api
tags: [rest, axum, sqlite, read-audit, cancellation, tdd]
requires:
  - phase: 07-02
    provides: 既有认证宿主、固定RequestSnapshot和durable read audit接口
provides:
  - 真实listener覆盖完整L0 REST、授权metadata与分页快照
  - 每请求可信预算、统一拒绝envelope和typed超时取消映射
  - Query单次reservation ID观察接口与HTTP/durable audit关联
affects: [07-02, 07-04, 07-13, 07-14]
tech-stack:
  added: []
  patterns: [request-scoped Engine adapter, synchronous reservation observer, real-listener contract tests]
key-files:
  created:
    - crates/ontoforge-server/tests/support/mod.rs
    - crates/ontoforge-server/tests/unit/rest.test.rs
    - crates/ontoforge-server/tests/unit/http_contract.test.rs
    - .planning/phases/07-rest-mcp-shell/07-03-evidence/
  modified:
    - crates/ontoforge-api/src/dto.rs
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-query/src/execute.rs
    - crates/ontoforge-query/src/aggregate.rs
    - crates/ontoforge-query/tests/audit_pk.rs
    - crates/ontoforge-server/src/lib.rs
    - crates/ontoforge-server/src/context.rs
    - crates/ontoforge-server/src/rest.rs
    - crates/ontoforge-server/src/error.rs
    - crates/ontoforge-server/tests/http_contract.rs
key-decisions:
  - 兼容audited wrappers返回自身reservation ID；observed入口成功reserve后、异步执行前仅同步回调一次，不改变guard的finish/Drop。
  - 仅按EngineError类型记录Timeout/Cancelled，并且只覆盖对应Query/Aggregate Source失败；不按后端错误文本判错。
  - HTTP deadline优先使用observer记录的真实audit ID，未reserve时仅回退transport ID。
  - GET PK truncated映既有400 COST_EXCEEDED；保留Query bounded结果既有audit success/0语义，HTTP错误状态不反写Query审计。
  - 独立REST读取通过drop传播取消；后续Action必须保留自身依赖读取的生命周期所有权。
requirements-completed: [API-01, API-03]
coverage:
  - id: D1
    description: 真实REST metadata、完整L0、分页和固定快照
    requirement: API-01
    verification:
      - kind: integration
        ref: crates/ontoforge-server/tests/unit/rest.test.rs
        status: pass
    human_judgment: false
  - id: D2
    description: 真实HTTP拒绝、编码脱敏、可信预算、超时取消及fail-closed审计
    requirement: API-03
    verification:
      - kind: integration
        ref: crates/ontoforge-server/tests/unit/http_contract.test.rs
        status: pass
    human_judgment: false
actuals:
  tokens: 69470
  tasks: 2
  commits: 20
plan_head_before: 0a84fee3d9a69bd1777f35f2f580a5ddc1efb1b1
plan_head_after: e2b8b8cddf5847807abc557c7ab1af1d40cb0b7c
duration: 58min
completed: 2026-10-29
status: complete
---

# Phase 07 Plan 03: 真实REST验收修复 Summary

真实TCP listener、SQLite EditStore与aviation SqliteEngine证明完整L0、固定快照、统一拒绝和取消审计；HTTP ID关联同一次Query reservation，metadata只公开有限engineStatus。

## 范围与计量

- 本文记录2026-10-29验收修复，替代旧完成主张。历史 `07-03-IMPLEMENTATION-RECORD.md` 不改写；编译失败、路由常量和helper响应不充当有效行为RED。
- 首个保留命令UTC03:27:41，最终全量测试UTC04:22:52结束，收口约UTC04:25。实际使用PowerShell；Warp和ralph-loop未暴露，未声称运行其completion promise。
- `actuals.tokens` 为本执行者13个代码/测试文件、专属execution台账和evidence目录的realized diff共277878字符除4后向上取整，包含原始证据，排除共享主会话文档及本SUMMARY，非harness token数。共83个已提交文件。
- `commits: 20` 实测自repair ledger基线到 `plan_head_after`，包含主会话并行提交的授权/审查文档；不把20宣称为本执行者独占提交数。SUMMARY metadata commit位于该测量之后。
- `requirements-completed` 表示本PLAN负责的API-01/API-03 slice验收。整phase REQUIREMENTS、STATE、ROADMAP、task_plan/progress/findings及WINDOWS登记由主会话按独立验证维护；本执行者未修改共享指针。07-02仍需修复，07-04未恢复。

## 六项验收

| 指定测试 | 实际证明 | 最终证据 |
|---|---|---|
| `rest_metadata_and_all_l0_operations` | metadata授权过滤、description UTF-8、available/unavailable与reason sentinel脱敏；GET PK、where/search/searchAround、union/intersect/subtract、aggregate；关服后每数据操作单条durable audit | `07-03-evidence/rest-final.json`，3项之一，passed |
| `rest_equal_sort_keys_empty_nullable_and_pagination` | 同country值由PK稳定破同，空/单项/null、重复集合不重复对象；真实9MiB对象形成短页truncated/hasMore并可继续cursor | 同上，passed |
| `rest_reload_and_revoke_freeze_inflight_only` | 阻塞source时另一store完成reload/revoke，不等待metadata锁；在途请求保留旧snapshot/def_version，新请求404且metadata采用新定义与空授权集合 | 同上，passed |
| `http_framework_rejections_are_structured` | 实际invalid JSON、unknown fields、bad UTF-8 path/query、404、413、401和405都含三字段JSON及同ID header；GET query/POST ontology保留正确Allow；无engine dispatch或业务写入 | `07-03-evidence/http-final.json`，3项之一，passed |
| `http_error_utf8_ids_and_l0_redaction` | Unicode保留；16并发ID不同；Config空信息、typed Timeout/Cancelled覆盖PK/list/aggregate并映503/504/408；物理表/SQL/token/sentinel不进入响应或durable audit | 同上，passed |
| `http_rows_bytes_concurrency_timeout_and_audit_fail_closed` | rows=3等值与2越界；decoded JSON精确字节等值/少1，包含北京UTF-8；body等值/少1；editOnly GET PK完整ObjectRow含version字节等值成功、少1返回400；第33个阻塞请求429且只32条audit；三入口deadline关联cancelled audit；真实TCP断开drop；audit shutdown后三入口503且无engine调用 | 同上，passed |

所有fixture均真实认证、真实TcpListener和真实文件SQLite，关服drain后重开数据库读取audit/业务表。Notify只控制真实Engine边界，未伪造Response。指定六项均实际运行，0 ignored；source-backed路径同时使用aviation SQLite源。远程Trino/PG并未因此获得证明。

## TDD Gate Compliance

| 有效RED记录 | 可归因失败 | GREEN |
|---|---|---|
| `rest-audit-red.json` | HTTP query ID未匹配durable query.list reservation | `d31cf91`，最终rest-final |
| `http-framework-red.json` | 非法UTF-8 path使用纯文本，非统一JSON | `93fad96`，最终http-final |
| `http-concurrency-red.json` | 第33个阻塞请求不能及时返回准入拒绝 | `93fad96`，最终http-final |
| `http-fault-red.json` | typed Timeout被折叠为503，合同要求504 | `93fad96`，最终http-final |
| `rest-metadata-status-red.json` | metadata缺少逻辑engineStatus | `93fad96`，最终rest-final |
| `http-method-red-junit.json` | GET query返回Axum默认405空body | `93fad96`，最终http-final |
| `http-pk-budget-red.json` | editOnly对象存在，完整ObjectRow字节少1时REST误报404 | `93fad96`，最终http-final |

七份原生nextest JUnit RED均经 `gsd-tools.cjs check tdd-red-evidence <record> --raw` 得到 `RED_EVIDENCE_OK`，目标测试真实运行并在预期行为断言失败，无编译/fixture/0-test冒充RED。`rest-initial` 三个既有操作/分页/snapshot测试初轮GREEN如实保留，没有为已通过行为制造RED。HTTP历史record的targetFile字段沿早期脚本默认路径；实际归属以未改写JUnit中的 `tests/unit/http_contract.test.rs` 与targetTest为准。

REFACTOR仅拆分长预算测试、调整imports和Clippy建议；同一指定测试集合在最终命令重跑。证据脚本空日志初始化是可逆记录修正，跳过独立TDD，实际fmt运行验证静默命令也留下log。

## 验证命令

| 实际命令 | 结果 | 原始证据 |
|---|---|---|
| `cargo test -p ontoforge-server --lib --locked rest_ -- --nocapture` | 3 passed / 0 ignored | `rest-final.log/json` |
| `cargo test -p ontoforge-server --test http_contract --locked -- --nocapture` | 3 passed / 0 ignored | `http-final.log/json` |
| `cargo test -p ontoforge-server --lib --locked tracer_ -- --nocapture` | 3 passed / 0 ignored | `tracer-final.log/json` |
| `cargo test -p ontoforge-query --test audit_pk --locked -- --nocapture` | 13 passed / 0 ignored；observer三入口各一次，reserve失败零回调/无ID | `query-audit-final.log/json` |
| `cargo fmt --all -- --check` | exit0 | `fmt-final.log/json` |
| `cargo clippy -p ontoforge-server --all-targets --all-features --locked -- -D warnings` | exit0 | `server-clippy-final.log/json` |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | exit101，仅Action三处既有large_futures | `workspace-clippy-fixed.log/json` |
| `cargo test --workspace --locked -- --test-threads=1` | exit0；121组总1279 passed / 0 failed / 107 ignored / 0 filtered | `workspace-tests-jobs2.log/json`及`workspace-tests-jobs2-totals.json` |
| `cargo tree -i native-tls --workspace --locked` | exit101，`package ID specification`不匹配；依赖不存在 | `tls-native-final.log/json` |
| `cargo tree -i openssl-sys --workspace --locked` | exit101，`package ID specification`不匹配；依赖不存在 | `tls-openssl-final.log/json` |

全量测试仅工具子进程设置 `CARGO_BUILD_JOBS=2`，测试线程为1。UTC04:06:49.6138073至04:22:52.7004087，构建11m45s，总16m03s。此前 `workspace-tests-final` 因15个并发link严重换页，在测试运行前由主会话授权中止；通过CIM确认本次PowerShell64880 -> Cargo38284/39320命令后，仅结束该进程树。首次exit1记录为resource-aborted，非测试失败；保留增量target，不改flags或全局配置。

## Task Commits

- 03-1：`8fda43c` 真实REST测试及audit ID RED；`d31cf91` 单次audited wrappers与HTTP ID GREEN；`e5372f4` metadata状态RED，最终GREEN见 `93fad96`。
- 03-2：`107401e` framework RED；`b4bb975` concurrency/typed fault RED；`a546635` 405 RED；`ed050e3` PK byte RED；`93fad96` 可信limits、reservation observer、deadline/cancel、405/Allow、PK截断映射及最终GREEN。
- 证据：`110a181` 定向RED/GREEN、fmt/Clippy/TLS和资源中止记录；`e2b8b8c` 最终workspace原始日志与统计。
- 主会话批准的合同和独立验收见本phase PLAN及 `reviews/07-03-ACCEPTANCE-AUDIT.md`，不复制其完整叙事。

## Deviations from Plan

### Auto-fixed Issues

1. [Rule 1 - Bug] framework默认path/405绕过统一envelope，修复结构化path rejection与method fallback，保留Axum Allow；真实HTTP RED/GREEN覆盖。
2. [Rule 1 - Bug] 并发未实际准入、typed超时未分类、deadline前audit ID不可观察。按主会话批准的兼容observed wrappers和request-local Engine adapter修复，原guard finish/Drop保持不变。
3. [Rule 1 - Bug] GET PK把bounded truncated空rows当不存在，现映既有400 COST_EXCEEDED；同一ID和Query原audit success/0有显式真实断言。该audit是Query领域的bounded结果，不是HTTP状态，不扩展审计状态合同。
4. [Rule 3 - Blocking] Windows全量并发链接导致严重换页，核验进程树后按主会话批准中止并以jobs2重跑，保留所有原始证据。

metadata状态投影、真实fixture、audited wrappers均在修订PLAN授权范围内，未新增依赖、网络路由或storage schema。修复代码与测试通过stub扫描，无阻碍目标的占位数据或TODO/FIXME；未发现威胁模型之外的新trust boundary。无authentication gate。

## Deferred Issues

- all-features Clippy当前未绿：`crates/ontoforge-action/tests/idempotency_pg.rs:242/257/398` 三处16464-byte `large_futures`。不属于07-03所有权，主会话已纳入下一07-02修复，不添加allow掩盖。
- 107 ignored不是passed：真实PG/Trino、独立Trino 483 ACL、private fixtures、实际backend p95及1个Engine doctest未执行；完整名称/原因保留在workspace原始log。default-feature的Action PG/idempotency PG套件0 tests也未获验收。环境证明留给07-13/14，未声称真实外部服务通过。
- Warp、ralph-loop不可用；未运行其completion promise。后续Action接线必须自行拥有其依赖读取生命周期，不能机械采用独立REST查询drop策略。
- 本执行者不写共享WINDOWS、STATE或ROADMAP；上述未运行/既有lint项已回传主会话登记。PR保持Draft、OPEN，未合并或发布。

## Self-Check: PASSED

已核验11个代码/台账/证据文件存在，10个本执行者RED/GREEN/证据commit均为当前HEAD祖先；workspace统计与原始结果一致，git diff --check无错误。实际指标固定在plan_head_before至plan_head_after区间，metadata commit不纳入自引用计量。共享状态更新回交主会话。
