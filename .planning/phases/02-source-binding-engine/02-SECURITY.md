---
phase: "02"
slug: "source-binding-engine"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-17"
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> 威胁模型原文见 `02-01-PLAN.md` ~ `02-10-PLAN.md` 的 `<threat_model>`；本文件只记录核验结论与证据指针。

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

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation（证据） | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-01 | Tampering | ir/object.rs backing 缺省 | high | mitigate | `ontoforge-schema/src/ir/object.rs:29`；`validate/links.rs:44-53`；快照 `invalid_examples__v05-missing-backing.snap` | closed |
| T-02-02 | Repudiation | 迁移脚本改写快照语义 | medium | mitigate | `git show -U0 bcafe8d -- '*.snap'` 只改位置字段；02-01-SUMMARY.md:226 | closed |
| T-02-03 | Tampering | 迁移脚本自身 | low | mitigate | bcafe8d 无脚本文件；规则记于 02-01-SUMMARY.md:171 | closed |
| T-02-48 | Tampering | `loader::read_limited` 跟随符号链接 | low | mitigate | `ontoforge-schema/src/loader/mod.rs:181-190,253-256`；`lint/config.rs:331-333` | closed |
| T-02-04 | Tampering | printer/escape.rs（SQL 注入） | high | mitigate | `ontoforge-engine/src/printer/escape.rs:10-26`；`printer/mod.rs:19-24`；`lib.rs:95-99`；测试 `printer_prop.rs:177`、`sql_text_only_from_printer.rs:27` | closed |
| T-02-05 | Tampering | 数值字面量 | medium | mitigate | `printer/dialect.rs:219-262`；`tests/printer_literals.rs:77-86,182` | closed |
| T-02-06 | Denial of Service | SqliteEngine::execute 结果规模 | low | mitigate | `sqlite/mod.rs:417-436` | closed |
| T-02-SC | Tampering | `Cargo.toml` 新增依赖 | high | mitigate | `Cargo.toml:34-44` ⊂ `02-RESEARCH.md:152-172` 审计列表；`Cargo.lock` 入库；CI 全部 `--locked` | closed |
| T-02-07 | Tampering | discover::fixture_path（路径遍历） | high | mitigate | `loader/discover.rs:87-139`；`loader/mod.rs:225-285`；`sqlite/fixture.rs:159,368`；测试 `bindings.rs:777`、`sqlite_engine.rs:260` | closed |
| T-02-08 | Tampering | profile 名进入路径 | medium | mitigate | `ir/names.rs:206-242`；`ontoforge/src/pipeline.rs:64-87` | closed |
| T-02-09 | Spoofing | 物理标识符含控制字符 | low | mitigate | `validate/bindings.rs:192-215`（V16）；`render.rs:100-123` | closed |
| T-02-10 | Information Disclosure | 示例中的物理命名 | medium | mitigate | `examples/ontologies/aviation*/bindings/dev/*` 只用 `demo` / `aviation` 与虚构数据 | closed |
| T-02-11 | Information Disclosure | trino nextUri 跟随 | high | mitigate | `trino/mod.rs:63,104-124,226-246`；测试 `trino_client.rs:275-294` | closed |
| T-02-12 | Information Disclosure | 密钥进入错误 / Debug | high | mitigate | `trino/mod.rs:29-33,127-140`；`config.rs:463-474,496-543`；测试 `server_config.rs:370` | closed |
| T-02-13 | Spoofing | TLS 校验被关闭 | high | mitigate | `config.rs:30-63`（`deny_unknown_fields`）；`clippy.toml:3-8`；`trino/mod.rs:74-75` | closed |
| T-02-14 | Information Disclosure | basic / bearer 走明文 http | high | mitigate | `config.rs:438-442`；`trino/mod.rs:57-61,149-167`；测试 `server_config.rs:166` | closed |
| T-02-44 | Information Disclosure | origin / nextUri https→http 降级 | high | mitigate | `config.rs:383-395`；`trino/mod.rs:168-202`；测试 `server_config.rs:185`、`trino_client.rs:406` | closed |
| T-02-15 | Tampering | 头注入 | medium | mitigate | `trino/headers.rs:19-43`；测试 `trino_client.rs:363-387` | closed |
| T-02-16 | Denial of Service | 配置 / `*_file` / `ca_file` 读取 | low | mitigate | `config.rs:27,209,518,627`；`loader/mod.rs:43,191-218` | closed |
| T-02-17 | Tampering | 响应头改写会话状态 | low | mitigate | `trino/client.rs:48-52`；`headers.rs:1-2`；测试 `trino_client.rs:325-360` | closed |
| T-02-18 | Tampering | fixture → INSERT 值注入 | high | mitigate | `sqlite/mod.rs:296-309`；`printer/sqlite_ddl.rs:79-95` | closed |
| T-02-19 | Tampering | 替身加载后被写入 | medium | mitigate | `sqlite/mod.rs:119-123`；`sqlite_ddl.rs:98-100`；测试 `sqlite/tests.rs:151` | closed |
| T-02-20 | Denial of Service | 超大 fixture / 过多 ATTACH | medium | mitigate | `sqlite/fixture.rs:258,368-376,440-488`；`sqlite/mod.rs:131,158-168` | closed |
| T-02-45 | DoS / Repudiation | 单 catalog 错误影响整轮 | medium | mitigate | `bind_check.rs:546-562`；`error.rs:100-108`；`online.rs:81-85`；测试 `bind_check_isolation.rs:112,164` | closed |
| T-02-21 | Spoofing | 物理名含终端控制序列 | low | mitigate | `render.rs:64-91` | closed |
| T-02-22 | Information Disclosure | 诊断泄露数据值 | low | mitigate | `sqlite/fixture.rs:469,560-666`；测试 `sqlite_engine.rs:338` | closed |
| T-02-23 | Denial of Service | 失控查询占用共享 Trino | high | mitigate | `trino/mod.rs:358-383`；`trino/cancel.rs:57-70`；`headers.rs:93-101`；测试 `cli_online_trino.rs:455`。范围限定：首个 POST 响应解析出 `nextUri` 前 guard 无 DELETE 目标（`cancel.rs:22`、`trino/mod.rs:214,231`），该窗口由服务端 `query_max_execution_time` 兜底（Codex 二审） | closed |
| T-02-24 | Denial of Service | 超大响应页 | high | mitigate | `trino/client.rs:108-125`；`lib.rs:33` | closed |
| T-02-25 | Denial of Service | 恶意 Retry-After | medium | mitigate | `trino/classify.rs:29,55-61`；`trino/mod.rs:358` | closed |
| T-02-26 | DoS / Tampering | 503 后重复提交查询 | medium | mitigate | `classify.rs:93-107,133-141`；测试 `trino_resilience.rs:443,465` | closed |
| T-02-27 | Information Disclosure | DELETE 发往异源 | medium | mitigate | `trino/mod.rs:226-235`；`cancel.rs:8-9` | closed |
| T-02-28 | Tampering | ontoforge.toml 关闭类型契约规则 | low | accept | 见 Accepted Risks Log AR-02-01 | closed |
| T-02-29 | Denial of Service | fixture 表头解析 | low | mitigate | `loader/mod.rs:427-474`；测试 `loader/mod.rs:781,802` | closed |
| T-02-49 | Repudiation | 损坏表头被静默跳过 | low | mitigate | `lint/binding/w17.rs:47-55`；测试 `w17.rs:212` | closed |
| T-02-30 | Spoofing | fixture 表头控制字符 | low | mitigate | `loader/fixture.rs:20-69`；`render.rs:100` | closed |
| T-02-31 | Tampering | 有损类型映射 / 大小写歧义 | medium | mitigate | `bind_check.rs:100-101,235-236,401-470`；`bind_check/compat_tests.rs`；`tests/doc_type_table.rs` | closed |
| T-02-47 | DoS / Repudiation | drift 单目标失败 | medium | mitigate | `bind_check.rs:703-778`；测试 `drift.rs:232,386` | closed |
| T-02-32 | Denial of Service | drift 触发全表扫描 | medium | mitigate | `bind_check.rs:766-778`；`trino/mod.rs:457-511` | closed |
| T-02-33 | Spoofing | 远端类型字符串控制字符 | low | mitigate | `online.rs:570-580`；`render.rs:100` | closed |
| T-02-34 | Information Disclosure | 密钥经 CLI 输出泄露 | high | mitigate | `ontoforge/src/online.rs:359-367`；`render.rs:64-75`；测试 `cli_online_trino.rs:594,620` | closed |
| T-02-35 | Elevation of Privilege | Trino 不可用时静默改用替身 | high | mitigate | `online.rs:81-85,200-210`；`lib.rs:97`；测试 `cli_online_trino.rs:512-541` | closed |
| T-02-36 | Tampering | SQLite 库文件路径逃逸 | high | mitigate | 攻击面已消除：`sqlite/mod.rs:46`（`sqlite::memory:`）；`sqlite_ddl.rs:44-50`；`server_config.rs:227` 拒绝 `sqlite_dir` | closed |
| T-02-37 | Denial of Service | 全表查重占用共享集群 | medium | mitigate | `ontoforge/src/cli.rs:75-76`；`ontoforge-engine/src/pk_check.rs:78,120-121` | closed |
| T-02-46 | Repudiation | NULL 主键被报为重复 | medium | mitigate | `pk_check.rs:56-59,80-91`；测试 `pk_check.rs:159`、`cli_check_pk.rs:225` | closed |
| T-02-38 | Spoofing | 远端错误文本 / 主键样例控制字符 | low | mitigate | `pk_check.rs:20,173-199`；`render.rs:100` | closed |
| T-02-39 | Elevation of Privilege | nightly.yml 开 issue 权限 | medium | mitigate | `.github/workflows/nightly.yml:10-11,113-122`；`ci.yml:8-9`；无 `pull_request_target` | closed |
| T-02-40 | Tampering | 第三方 action / 镜像供应链 | high | mitigate | action 全部钉 40 位 SHA；镜像钉 digest（`ci.yml:39,68`、`nightly.yml:23,25,30,70`）；`--locked` | closed |
| T-02-41 | Information Disclosure | CI 一次性口令写在仓库 | low | accept | 见 Accepted Risks Log AR-02-02 | closed |
| T-02-42 | Information Disclosure | 公司数据进入公开 CI | high | mitigate | `.github/`、`ci/` 无 private 引用；`ci/pg/seed.sql` 只用 aviation fixture；`tests/private_fixtures.rs:69` `#[ignore]` 且不在 CI 目标内 | closed |
| T-02-43 | Repudiation | 集成测试静默跳过 / golden 未经真实运行 | medium | mitigate | `trino_it.rs:40`、`cli_trino_it.rs:19`（缺环境变量 panic）；`ci.yml:90-100`、`nightly.yml:96-106` 执行数门禁；golden 标 PROVISIONAL，`02-VERIFICATION.md` 保持 `human_needed`。CI 首跑 run 39219827829 通过，golden 已定稿（2026-10-19） | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-01 | T-02-28 | E13-E16 / W17 是可配置 lint（LINT-04 既定设计）；结构性 ONT-V 不可配置（`lint/config.rs:152-157`）；关闭 E 规则后 `--online` 的 V17 仍报告真实不匹配 | plan-time（02-07-PLAN.md） | 2026-10-17 |
| AR-02-02 | T-02-41 | CI PostgreSQL / Trino catalog 口令 `ontoforge-ci-only` 以字面量入库（`ci/trino/catalog/demo.properties:6`、`.github/workflows/ci.yml:43`），只用于 CI 临时容器；`5432:5432` 只发布到临时 runner 本机；不保护任何真实数据。威胁描述中的 MinIO 当前 workflow 未使用 | plan-time（02-10-PLAN.md） | 2026-10-17 |

*Accepted risks do not resurface in future audit runs.*

---

## Observations（非阻塞，后续 phase 可跟进）

1. T-02-07：`check_beneath` 与后续 open 之间仍有 TOCTOU 窗口（02-REVIEW-FIX CR-S01 已记录），根治需 beneath-open（如 `cap-std`）。
2. T-02-43：已于 2026-10-19 CI 首跑通过后关闭；nightly（Trino 483 / 484）已于 2026-10-19 通过（run 39221684753，`02-UAT.md` 测试 2）。
3. T-02-13：`clippy.toml` 只禁 `danger_*`，`use_preconfigured_tls` + 自定义 verifier 不在禁令内（REVIEW IN-C05）；当前代码无此用法。
4. T-02-39：测试 job 的 `actions/checkout` 未设 `persist-credentials: false`（REVIEW IN-C04），token 只有 `contents: read`。
5. T-02-09 / 21 / 38：`sanitize` 只作用于 human 输出，JSON 输出保留原文；物理名只拒绝 Cc 类控制字符，不拒绝 bidi Cf 字符（REVIEW IN-S01）。
6. T-02-12：`config.rs:270` 原样转发 toml 类型错误文本，密钥写成非字符串值时可能出现在错误中（未实测）；密钥中间副本未清零（REVIEW IN-E08）。

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-17 | 50 | 50 | 0 | gsd-security-auditor（Claude Opus；Codex 因 sandbox 故障与 429 限流未能运行） |
| 2026-10-19 | 50 | 49 | 0（1 条 medium open，低于阈值） | Codex 二审（静态核查；修正 T-02-43 状态、T-02-23 范围、AR-02-02 措辞） |
| 2026-10-19 | 50 | 50 | 0 | orchestrator（CI 首跑通过后关闭 T-02-43） |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-17
