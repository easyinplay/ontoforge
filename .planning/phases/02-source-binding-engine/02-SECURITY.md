---
phase: "02"
slug: "source-binding-engine"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-17"
updated: "2026-11-08"
source_head: "2c78b7a92620a1c3522c09567da713b90d68eda5"
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> 威胁模型原文见 `02-01-PLAN.md` ~ `02-11-PLAN.md` 的 `<threat_model>`；本文件只记录核验结论与证据指针。

## 本轮（2026-11-08）复核范围

**做了**：对既有 50 条威胁逐条复核 mitigation 构造在 `source_head` 上是否仍然存在（47 项语义检查，全部命中），并把 18 条因 Phase 07.1 / 07.2 重构而失效的 `file:line` 引用重新定位；同时把 02-11 引入的 4 条新威胁登记进册（全部 closed）。

**没做**（本轮是静态复核，边界如实登记）：

- 没有启动 Docker 或任何容器，没有运行 `ci/iceberg-services.sh` / `ci/query-services.sh` 的任何模式
- 没有连接任何 Trino（含 483 / 484）、MinIO、PostgreSQL、SQL Server 实例
- 没有触发或重跑 GitHub `ci` / `nightly` workflow
- 没有读取 `ONTOFORGE_PRIVATE_FIXTURES` 指向的仓库外目录
- 没有修改任何产品代码（`crates/`、`ci/`、`.github/`、`examples/`）或已跟踪的 `docs/` 文件

因此本轮结论只能证明「mitigation 构造在当前源码中成立」，**不**证明「某次运行中该 mitigation 真的生效」。凡需实跑证据的判断项，见 `02-11-HUMAN-ACCEPTANCE.md` 与 `02-VERIFICATION.md`。

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| 定义 / 绑定 / fixture / ontoforge.toml → loader、lint | 来自可能不可信的 ontology 仓库作者 | 结构定义、物理名、fixture 路径与内容 |
| 调用方的值 → SQL 文本 | Trino 无带外参数绑定，转义是唯一防线 | 过滤值、表名、列名 |
| ontoforge → Trino（HTTP） | 服务凭据跨网络；`nextUri`、响应头、错误文本不可信 | 凭据（高敏）、查询结果 |
| `ontoforge-server.toml` / 进程环境 → engine | 部署方配置，`*_file` / `*_env` 指向文件系统与环境变量 | 密钥（高敏） |
| fixture → SQLite 替身 | fixture 内容进入替身表 | 虚构测试数据 |
| 物理元数据 / 引擎错误 → 终端输出 | 远端列名、类型、错误文本可能含控制字符 | 诊断文本 |
| CLI 参数 → 引擎 | `--server-config`、`--check-pk` 决定连接目标与是否全表扫描 | 操作者输入 |
| GitHub Actions / 第三方 action 与镜像 → CI | 工作流 token 权限与供应链 | 仓库内容、issue 写权限 |
| crates.io → 构建 | 新增依赖（部分带 build script） | 依赖代码 |
| 只读源码扫描 → 公开面结论 | `crates/`、`ci/`、`.github/`、`examples/` 的当前内容被写成可公开的结论 | 物理标识符、端点串、CI 编排形态 |
| 机器可判定部分 → 人工验收部分 | 机器门禁的 PASS 只构成**证据**，不构成 judgment **结论** | 结论位（`VERDICT`）的填写权 |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation（证据） | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-01 | Tampering | ir/object.rs backing 缺省 | high | mitigate | `ontoforge-schema/src/ir/object.rs:29`；`ontoforge-schema/src/validate/links.rs:44-53`；快照 `invalid_examples__v05-missing-backing.snap` | closed |
| T-02-02 | Repudiation | 迁移脚本改写快照语义 | medium | mitigate | `git show -U0 bcafe8d -- '*.snap'` 只改位置字段；02-01-SUMMARY.md:226 | closed |
| T-02-03 | Tampering | 迁移脚本自身 | low | mitigate | bcafe8d 无脚本文件；规则记于 02-01-SUMMARY.md:171 | closed |
| T-02-48 | Tampering | `loader::read_limited` 跟随符号链接 | low | mitigate | `ontoforge-schema/src/loader/mod.rs:245-256`（`check_beneath` 拒绝 symlink）；`ontoforge-schema/src/lint/config.rs:331-333` | closed |
| T-02-04 | Tampering | printer/escape.rs（SQL 注入） | high | mitigate | `ontoforge-engine/src/printer/escape.rs:11`（`quote_string`）；`printer/mod.rs:19-24`（`PrintedSql` 不透明封装）；测试 `printer_prop.rs:238`（`string_literal_keeps_statement_structure`）、`sql_text_only_from_printer.rs:27` | closed |
| T-02-05 | Tampering | 数值字面量 | medium | mitigate | `printer/dialect.rs:219-262`；`tests/printer_literals.rs:77-86,178-186` | closed |
| T-02-06 | Denial of Service | SqliteEngine::execute 结果规模 | low | mitigate | `ontoforge-engine/src/sqlite/mod.rs:417-436`（结果行数与字节上限） | closed |
| T-02-SC | Tampering | `Cargo.toml` 新增依赖 | high | mitigate | `Cargo.toml:34-44` ⊂ `02-RESEARCH.md:152-172` 审计列表；`Cargo.lock` 入库；CI 全部 `--locked`；02-11 轮未新增依赖（`git diff 9a6eaffe060e93df1c19c79c7b25d3946a7d3229 --name-only -- Cargo.toml Cargo.lock` 实测为空） | closed |
| T-02-07 | Tampering | discover::fixture_path（路径遍历） | high | mitigate | `loader/discover.rs:87-139`；`loader/mod.rs:225-285`；`sqlite/fixture.rs:159`；测试 `bindings.rs:777`、`sqlite_engine.rs:260` | closed |
| T-02-08 | Tampering | profile 名进入路径 | medium | mitigate | `ir/names.rs:206-242`；`ontoforge/src/pipeline.rs:64-87` | closed |
| T-02-09 | Spoofing | 物理标识符含控制字符 | low | mitigate | `ontoforge-schema/src/validate/bindings.rs:192-215`（V16）；`ontoforge/src/render.rs:100`（`sanitize`） | closed |
| T-02-10 | Information Disclosure | 示例中的物理命名 | medium | mitigate | `examples/ontologies/aviation*/bindings/dev/*` 只用 `demo` / `aviation` 与虚构数据 | closed |
| T-02-11 | Information Disclosure | trino nextUri 跟随 | high | mitigate | `ontoforge-engine/src/trino/mod.rs:177-198`（`checked_next_uri`：拒绝带 userinfo 的 nextUri + 校验 origin）；`trino/mod.rs:271-277`（`same_origin`）；测试 `trino_client.rs:275` | closed |
| T-02-12 | Information Disclosure | 密钥进入错误 / Debug | high | mitigate | `trino/mod.rs:200-213`（`Debug` 只输出 origin / user / 认证方式，不含密钥）；`config.rs:463-474`（拒绝明文密钥）；`config.rs:1064-1100`（`SecretString`）；测试 `server_config.rs:422` | closed |
| T-02-13 | Spoofing | TLS 校验被关闭 | high | mitigate | `crates/ontoforge-engine/src/config.rs:34`（`deny_unknown_fields`）；`clippy.toml:3-8`（禁 4 个 `danger_*` 方法）；`trino/mod.rs:109-119`（CA bundle 合并，注释明确 "certificate checks stay on"，源码中无 `danger_accept_invalid_certs`） | closed |
| T-02-14 | Information Disclosure | basic / bearer 走明文 http | high | mitigate | `trino/mod.rs:222-258`（`check_transport`：`http` + basic/bearer 被拒）；`trino/mod.rs:103-105`（`new()` 是唯一构造入口）；测试 `server_config.rs:189,218` | closed |
| T-02-44 | Information Disclosure | origin / nextUri https→http 降级 | high | mitigate | `trino/mod.rs:241-252`（extra origin 必须与主 url 同 scheme）；`trino/mod.rs:259-269`（`statement_url`）；测试 `server_config.rs:237`、`trino_client.rs:406` | closed |
| T-02-15 | Tampering | 头注入 | medium | mitigate | `trino/headers.rs:19-43`；测试 `trino_client.rs:363-387` | closed |
| T-02-16 | Denial of Service | 配置 / `*_file` / `ca_file` 读取 | low | mitigate | `config.rs:30`（`MAX_SECRET_FILE_BYTES = 64 KiB`）、`config.rs:1081-1087`；`ontoforge-schema/src/loader/mod.rs:211-231`（`read_limited`） | closed |
| T-02-17 | Tampering | 响应头改写会话状态 | low | mitigate | `trino/client.rs:48-52`；`headers.rs:1-2`；测试 `trino_client.rs:325-360` | closed |
| T-02-18 | Tampering | fixture → INSERT 值注入 | high | mitigate | `ontoforge-engine/src/printer/sqlite_ddl.rs:80`（`insert` 用带位参数）；`sqlite/mod.rs:302`（替身装载只走 printer 生成的 SQL） | closed |
| T-02-19 | Tampering | 替身加载后被写入 | medium | mitigate | `sqlite/mod.rs:119-123`；`sqlite_ddl.rs:98-100`；测试 `sqlite/tests.rs:151` | closed |
| T-02-20 | Denial of Service | 超大 fixture / 过多 ATTACH | medium | mitigate | `sqlite/fixture.rs:258`（`MAX_RECORDS = 100_000`）；`sqlite/mod.rs:130-168`（`MAX_ATTACHED = 10` + 装载前检查） | closed |
| T-02-45 | DoS / Repudiation | 单 catalog 错误影响整轮 | medium | mitigate | `bind_check.rs:546-562`（按 catalog 分组描述）；`error.rs:100-108`（`is_engine_wide`）；`ontoforge/src/online.rs:84,93-101`（`online_exit_code` 遇 V19 → `EXIT_ENGINE`）；测试 `bind_check_isolation.rs:112,164` | closed |
| T-02-21 | Spoofing | 物理名含终端控制序列 | low | mitigate | `ontoforge/src/render.rs:64-95`（诊断渲染全部经 `sanitize`）、`render.rs:100` | closed |
| T-02-22 | Information Disclosure | 诊断泄露数据值 | low | mitigate | `ontoforge-engine/src/sqlite/fixture.rs:461-471`（fixture 值只转为 `SqlValue`，不进诊断文本）；`ontoforge/src/render.rs:100`（输出前 `sanitize`）；测试 `sqlite_engine.rs:338`（`fixture_errors_never_repeat_cell_contents`） | closed |
| T-02-23 | Denial of Service | 失控查询占用共享 Trino | high | mitigate | `trino/cancel.rs:26-96`（`CancellationTasks` / `spawn` / `drain`）；`trino/cancel.rs:115-178`（`QueryGuard` + `Drop` 发 `DELETE`）；`headers.rs:93-101`；测试 `cli_online_trino.rs:455`。范围限定：首个 POST 响应解析出 `nextUri` 前 guard 无 DELETE 目标（`cancel.rs:9-10`、`trino/mod.rs:177-198`），该窗口由服务端 `query_max_execution_time` 兜底（Codex 二审） | closed |
| T-02-24 | Denial of Service | 超大响应页 | high | mitigate | `trino/client.rs:108-125`（`read_body` 逐块读，越限即停）；`ontoforge-engine/src/lib.rs:35`（`DEFAULT_MAX_PAGE_BYTES = 16 MiB`） | closed |
| T-02-25 | Denial of Service | 恶意 Retry-After | medium | mitigate | `trino/classify.rs:29`（`RETRY_AFTER_CAP = 5s`）、`classify.rs:55-61`（`retry_after_delay`）；`trino/client.rs:50`（Retry-After 头读取） | closed |
| T-02-26 | DoS / Tampering | 503 后重复提交查询 | medium | mitigate | `trino/classify.rs:80-107`（`classify_status`）、`classify.rs:133-146`（`classify_transport`）；测试 `trino_resilience.rs:443,465` | closed |
| T-02-27 | Information Disclosure | DELETE 发往异源 | medium | mitigate | `trino/mod.rs:177-198`（nextUri 必须先过 origin 校验）；`trino/cancel.rs:9-10`（guard 只持有同源 nextUri） | closed |
| T-02-28 | Tampering | ontoforge.toml 关闭类型契约规则 | low | accept | 见 Accepted Risks Log AR-02-01 | closed |
| T-02-29 | Denial of Service | fixture 表头解析 | low | mitigate | `loader/mod.rs:427-474`；测试 `loader/mod.rs:781,802` | closed |
| T-02-49 | Repudiation | 损坏表头被静默跳过 | low | mitigate | `lint/binding/w17.rs:47-55`；测试 `w17.rs:212` | closed |
| T-02-30 | Spoofing | fixture 表头控制字符 | low | mitigate | `loader/fixture.rs:20-69`；`render.rs:100` | closed |
| T-02-31 | Tampering | 有损类型映射 / 大小写歧义 | medium | mitigate | `bind_check.rs:96-104`（`Compat`：只接受无损读法）、`bind_check.rs:232-240`（不支持的远端类型显式报 unsupported）；`bind_check/compat_tests.rs`；`tests/doc_type_table.rs` | closed |
| T-02-47 | DoS / Repudiation | drift 单目标失败 | medium | mitigate | `bind_check.rs:740-800`（`drift_report` / `check_drift`）；测试 `drift.rs:232,386` | closed |
| T-02-32 | Denial of Service | drift 触发全表扫描 | medium | mitigate | `bind_check.rs:778-798`（`check_drift` 文档明确 "drift never scans a table"）、`bind_check.rs:519-562`（`describe_bindings` 只读元数据） | closed |
| T-02-33 | Spoofing | 远端类型字符串控制字符 | low | mitigate | `ontoforge/src/render.rs:64-95,100`（所有人类可读诊断与文件片段经 `sanitize`）；`ontoforge/src/lib.rs:209`（JSON 输出同样 `sanitize`） | closed |
| T-02-34 | Information Disclosure | 密钥经 CLI 输出泄露 | high | mitigate | `ontoforge/src/online.rs:359-367`（样例值只经 `display_value` 渲染）；`render.rs:64-75`（server config 的行不回显）；测试 `cli_online_trino.rs:594,620` | closed |
| T-02-35 | Elevation of Privilege | Trino 不可用时静默改用替身 | high | mitigate | `ontoforge/src/online.rs:477-520`（`select_backend`：引擎由 profile 的 `engine` 决定，Trino 不可用时报 ONT-V19 而非改用替身）；`online.rs:84,93-101`（`online_exit_code`）；`ontoforge/src/lib.rs:97`；测试 `cli_online_trino.rs:512-541` | closed |
| T-02-36 | Tampering | SQLite 库文件路径逃逸 | high | mitigate | 攻击面已消除：`sqlite/mod.rs:45`（`sqlite::memory:`）；`sqlite_ddl.rs:44-50`；`server_config.rs:278`（拒绝 `sqlite_dir`） | closed |
| T-02-37 | Denial of Service | 全表查重占用共享集群 | medium | mitigate | `ontoforge/src/cli.rs:74-77`；`ontoforge-engine/src/pk_check.rs:78,120-121` | closed |
| T-02-46 | Repudiation | NULL 主键被报为重复 | medium | mitigate | `pk_check.rs:56-59`（`IsNull` 取反，只看非空主键）、`pk_check.rs:80-91`（独立 NULL 统计）；测试 `pk_check.rs:159`、`cli_check_pk.rs:225` | closed |
| T-02-38 | Spoofing | 远端错误文本 / 主键样例控制字符 | low | mitigate | `pk_check.rs:20,194-199`；`render.rs:100` | closed |
| T-02-39 | Elevation of Privilege | nightly.yml 开 issue 权限 | medium | mitigate | `.github/workflows/nightly.yml:10-11`（workflow 级 `contents: read`）、`nightly.yml:81-84`（只有 report-failure job 提权到 `issues: write`）；`ci.yml:15-16`；无 `pull_request_target` | closed |
| T-02-40 | Tampering | 第三方 action / 镜像供应链 | high | mitigate | action 全部钉 40 位 SHA（`ci.yml:34-39`、`nightly.yml:33-37`）；镜像钉 digest（`ci/iceberg-services.sh:51-56`、`ci/query-services.sh:38,70,96,122`）；CI 全部 `--locked` | closed |
| T-02-41 | Information Disclosure | CI 一次性口令写在仓库 | low | accept | 见 Accepted Risks Log AR-02-02 | closed |
| T-02-42 | Information Disclosure | 公司数据进入公开 CI | high | mitigate | `.github/`、`ci/` 无 private 引用；`ci/pg/seed.sql` 只用 aviation fixture；`crates/ontoforge/tests/private_fixtures.rs:69` `#[ignore]` 且不在 CI 目标内 | closed |
| T-02-43 | Repudiation | 集成测试静默跳过 / golden 未经真实运行 | medium | mitigate | `crates/ontoforge-engine/tests/trino_it.rs:40`（缺环境变量 panic）、`crates/ontoforge/tests/cli_trino_it.rs:105`（同上）；`ci.yml:135-139` 执行数门禁；golden 标 PROVISIONAL，`02-VERIFICATION.md` 保持 `human_needed`。CI 首跑 run 39219827829 通过，golden 已定稿（2026-10-19） | closed |
| T-02-50 | Repudiation | 调和证据被误读为本轮实跑证据 | high | mitigate | `02-11-RECONCILIATION.md:159`（`## This Round Did Not Execute` 节）；`02-11-SUMMARY.md:81-94`（本轮未执行清单）；`02-11-RECONCILIATION.md:41` 显式区分「结构门禁通过」与「实跑通过」 | closed |
| T-02-51 | Repudiation | 三个 judgment 项被源码扫描的绿灯静默吸收为 closed | high | mitigate | `02-11-HUMAN-ACCEPTANCE.md:44,60,78`（三处结论位固定为 `VERDICT: PENDING`）；`02-11-HUMAN-ACCEPTANCE.md:96`（登记不得预填）；反向门禁禁止出现 `VERDICT: PASS/ACCEPTED/REJECTED/CLOSED` | closed |
| T-02-52 | Tampering | 审计过程中误改产品代码或他流程拥有的文件 | medium | mitigate | `git status --porcelain -- crates ci .github examples` 实测为空；`git diff 9a6eaffe060e93df1c19c79c7b25d3946a7d3229 --name-only -- docs` 实测为空；2026-11-08 复核时再次实测为空 | closed |
| T-02-53 | Information Disclosure | 审计过程本身把私有环境信息写进公开仓库 | medium | mitigate | `02-11-source-scan.txt:208`（`==== NOT SCANNED ====` 段）；私有目录内容一律不读；未连接任何 Trino / MinIO / PostgreSQL / SQL Server 实例 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-01 | T-02-28 | E13-E16 / W17 是可配置 lint（LINT-04 既定设计）；结构性 ONT-V 不可配置（`lint/config.rs:152-157`）；关闭 E 规则后 `--online` 的 V17 仍报告真实不匹配 | plan-time（02-07-PLAN.md） | 2026-10-17 |
| AR-02-02 | T-02-41 | CI PostgreSQL / Trino catalog 口令 `ontoforge-ci-only` 以字面量入库（`ci/trino/catalog/demo.properties:6`、`ci/trino/catalog/editstore.properties:6`），只用于 CI 临时容器；`5432:5432` 只发布到临时 runner 本机；不保护任何真实数据。2026-11-08 复核：该字面量**已不在** `.github/workflows/ci.yml` 中；nightly 的 Iceberg 路径在运行时生成随机凭据（`ci/iceberg-services.sh:348` `secrets.token_hex(24)` ×3），并把 catalog 的 `connection-password` 在运行时改写（`ci/iceberg-services.sh:441`），仓库内不含该路径的凭据字面量 | plan-time（02-10-PLAN.md） | 2026-10-17（2026-11-08 复核） |

*Accepted risks do not resurface in future audit runs.*

---

## Observations（非阻塞，后续 phase 可跟进）

1. T-02-07：`check_beneath` 与后续 open 之间仍有 TOCTOU 窗口（02-REVIEW-FIX CR-S01 已记录），根治需 beneath-open（如 `cap-std`）。
2. T-02-43：已于 2026-10-19 CI 首跑通过后关闭；nightly（Trino 483 / 484）已于 2026-10-19 通过（run 39221684753，`02-UAT.md` 测试 2）。
3. T-02-13：`clippy.toml` 只禁 `danger_*`，`use_preconfigured_tls` + 自定义 verifier 不在禁令内（REVIEW IN-C05）；当前代码无此用法。
4. T-02-39：测试 job 的 `actions/checkout` 未设 `persist-credentials: false`（REVIEW IN-C04），token 只有 `contents: read`。
5. T-02-09 / 21 / 38：`sanitize` 只作用于 human 输出，JSON 输出保留原文；物理名只拒绝 Cc 类控制字符，不拒绝 bidi Cf 字符（REVIEW IN-S01）。
6. T-02-12：`config.rs:270` 原样转发 toml 类型错误文本，密钥写成非字符串值时可能出现在错误中（未实测）；密钥中间副本未清零（REVIEW IN-E08）。
7. **（2026-11-08 新增）** 本轮复核发现 18 条既有威胁的 `file:line` 引用因 Phase 07.1 / 07.2 重构而失效（原引用指向无关代码）。失效本身不是安全缺陷（对应 mitigation 构造均仍存在，47 项语义检查全部命中），但说明 SECURITY.md 的行号引用会随代码漂移，下轮复核必须重新核，不能默认沿用。
8. **（2026-11-08 新增）** T-02-22 / T-02-33 的 mitigation 实际落在统一渲染层 `render::sanitize`，旧引用分别指向 fixture 转换与 online 收集处（语义不对位）；已改为指向渲染层。其余 16 条失效引用属于「构造还在、位置漂移」，语义判断未变。
9. **（2026-11-08 新增）** T-02-50..T-02-53 是 02-11 的审计期威胁，防护手段是**文档结构**（未执行清单、`VERDICT: PENDING` 结论位、零产品代码改动门禁）而非代码。其有效性依赖后续流程不代签结论；`02-11-HUMAN-ACCEPTANCE.md` 被 verifier 消费前，这四条不应被视为「已完成验收」。

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-17 | 50 | 50 | 0 | gsd-security-auditor（Claude Opus；Codex 因 sandbox 故障与 429 限流未能运行） |
| 2026-10-19 | 50 | 49 | 0（1 条 medium open，低于阈值） | Codex 二审（静态核查；修正 T-02-43 状态、T-02-23 范围、AR-02-02 措辞） |
| 2026-10-19 | 50 | 50 | 0 | orchestrator（CI 首跑通过后关闭 T-02-43） |
| 2026-11-08 | 54 | 54 | 0 | orchestrator（静态复核：47 项 mitigation 构造存在性 + 54 条 register 的 `file:line` 重定位；登记 02-11 的 4 条新威胁；未运行容器 / 未连接任何外部服务 / 未实跑 CI 或 nightly） |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter
- [x] 2026-11-08 复核：54 条威胁全部有当前可定位的证据指针，0 条 open
- [ ] 2026-11-08 复核的边界：未做实跑核验（无容器 / 无外部连接 / 无 CI 运行），实跑类结论仍归 `02-VERIFICATION.md` 与 `02-11-HUMAN-ACCEPTANCE.md`

**Approval:** verified 2026-10-17；re-verified 2026-11-08（静态复核，范围见「本轮复核范围」）

## Security Audit 2026-11-08

| Metric | Count |
|---|---|
| Threats found | 54 |
| Closed | 54 |
| Open | 0 |
