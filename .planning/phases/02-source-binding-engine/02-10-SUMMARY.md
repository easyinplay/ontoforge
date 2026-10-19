---
phase: 02-source-binding-engine
plan: 10
subsystem: ci
tags: [github-actions, trino, postgresql, iceberg, integration-tests, insta, ci-security]

requires:
  - phase: 02-source-binding-engine
    provides: "02-04 的 TrinoEngine / describe_tables / 值解码；02-06 的超时取消（DELETE nextUri）；02-07 的 check_drift；02-09 的 validate --online --check-pk --server-config"
provides:
  - "PR 的 `integration` 作业：postgres:17 service（发布 5432:5432）+ checkout 后 docker run 的 trinodb/trino:483，种入 aviation fixture，`--ignored` 运行 trino_it 与 cli_trino_it，并门禁两个目标的 ignored 测试数 > 0"
  - "nightly.yml：Trino 483 / 484 矩阵 + Iceberg JDBC catalog（本地文件系统 warehouse），独立 `report-failure` job（唯一拥有 issues: write）开 / 评论 `nightly-failure` issue"
  - "ci/：demo / editstore / tpch / iceberg catalog、seed.sql（fixture → staging → 类型转换）、iceberg_catalog.sql、ontoforge-server.ci.toml"
  - "trino_it（10 个 ignored + 2 个本机测试）、cli_trino_it（1 + 1）、private_fixtures（1 + 2），缺环境变量即 panic"
  - "CONTRIBUTING「集成测试」节：本机复现命令、nightly、私有 fixture 目录约定"
affects: [02-verification, phase-03, phase-07]

plan_head_before: c5e97e99f285a3b2506ad03d8c20a8525f398bc8

actuals:
  tokens: 13584
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "真实引擎测试一律 #[ignore] + 环境变量缺失即 panic；读取环境的逻辑拆成接收 Option 的纯函数，用 catch_unwind 在本机验证 panic"
    - "推导出的真实引擎 golden 写成 insta 内联快照并标 `// PROVISIONAL: confirm against first CI run (E21)`，首跑后人工定稿"
    - "Trino 用 docker run 挂载仓库 catalog 目录（service container 无法挂载仓库文件），/v1/info 的 starting 轮询 180 s 上限"
    - "workflow 权限：顶层 contents: read，写权限只给不 checkout 的独立 job"

key-files:
  created:
    - .github/workflows/nightly.yml
    - ci/trino/catalog/demo.properties
    - ci/trino/catalog/editstore.properties
    - ci/trino/catalog/tpch.properties
    - ci/trino/iceberg/iceberg.properties
    - ci/pg/seed.sql
    - ci/pg/iceberg_catalog.sql
    - ci/ontoforge-server.ci.toml
    - crates/ontoforge-engine/tests/trino_it.rs
    - crates/ontoforge/tests/cli_trino_it.rs
    - crates/ontoforge/tests/private_fixtures.rs
  modified:
    - .github/workflows/ci.yml
    - CONTRIBUTING.md
    - .planning/WINDOWS.md

key-decisions:
  - "nightly 的 Iceberg warehouse 改用 Trino 本地文件系统（fs.local.enabled），不用 MinIO：minio/minio 在 Docker Hub 已不存在（API 返回 object not found），quay.io/minio/minio 拒绝匿名拉取；ontoforge 只与 Trino 通信，不接触对象存储，本地 warehouse 覆盖同一个 Iceberg connector 且不引入新的第三方镜像"
  - "PostgreSQL 镜像钉 postgres:17（Docker Hub API 2026-10-17 确认 active，digest sha256:2d2b8998…df9e3）；Trino 483 / 484 标签同样确认 active"
  - "system.runtime.queries 按 source = 'ontoforge' 与查询文本中的唯一 marker 定位被超时的查询（不依赖 trace token 列，A5），断言 state = FAILED 且 error_code = USER_CANCELED，且在超时后 2 s 内观察到，以区别于 session 的 query_max_execution_time（超时 + 5 s）"
  - "golden 用 insta::assert_snapshot! 的文本内联快照，而不是计划写的 assert_json_snapshot!：PhysicalColumn / Value 没有 Serialize，为测试给库类型加 derive 属于范围外改动"
  - "种子表的主键列与必填列加 NOT NULL / PRIMARY KEY，使 describe golden 也固定 nullable；pilots 的 \"licenseNumber\" 保留驼峰以在真实 Trino 上验证小写化比对"
  - "nightly 除 trino_it 外也运行 cli_trino_it 与数量门禁，使 E21 的「两个目标在 483 / 484 上各执行 ≥ 1」能由 nightly 直接证明"

patterns-established:
  - "CI 一次性口令 `ontoforge-ci-only` 只写在 catalog properties 与工作流中（T-02-41 accept）"
  - "新增 Linux 集成测试目标时，同时加进 integration 作业与 nightly 的数量门禁循环"

requirements-completed: [ENGINE-05, ENGINE-01, ENGINE-03]

coverage:
  - id: D1
    description: "集成测试入口：ONTOFORGE_TRINO_URL / ONTOFORGE_PRIVATE_FIXTURES 缺失时 panic（不静默通过），私有 fixture 集合需同时有 ontology/ 与 ontoforge-server.toml"
    requirement: ENGINE-05
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/tests/trino_it.rs#missing_env_panics"
        status: pass
      - kind: unit
        ref: "crates/ontoforge/tests/cli_trino_it.rs#missing_env_panics"
        status: pass
      - kind: unit
        ref: "crates/ontoforge/tests/private_fixtures.rs#missing_env_panics"
        status: pass
      - kind: unit
        ref: "crates/ontoforge/tests/private_fixtures.rs#a_set_needs_ontology_and_server_config"
        status: pass
    human_judgment: false
  - id: D2
    description: "Windows 本机 cargo test --workspace 不需要 Docker 即全绿，全部真实引擎测试为 ignored（trino_it 9 + 1 Iceberg、cli_trino_it 1、private_fixtures 1）"
    requirement: ENGINE-05
    verification:
      - kind: other
        ref: "cargo test --workspace --locked（573 passed, 0 failed, 11 ignored）"
        status: pass
    human_judgment: false
  - id: D3
    description: "PR integration 作业在真实 PostgreSQL + Trino 483 上跑通 trino_it 与 cli_trino_it（含 describe / decode golden、printer 语料、超时取消、catalog 可见性、drift、CLI validate 退出码 0），数量门禁 > 0"
    requirement: ENGINE-05
    verification:
      - kind: other
        ref: "rg 结构检查：trinodb/trino:483、ci/trino/catalog:/etc/trino/catalog、--test trino_it --locked -- --ignored、--test cli_trino_it --locked -- --ignored、--ignored --list、contents: read、5432:5432"
        status: pass
    human_judgment: true
    rationale: "工作流只能在 GitHub Linux runner 上运行，本机无 Docker / Trino；golden 预期值为推导值（PROVISIONAL），首跑通过前不算验收证据（E21）"
  - id: D4
    description: "nightly：Trino 483 / 484 上 Iceberg 表可读且行数与源表一致；失败时 report-failure 开 / 评论 nightly-failure issue，测试 job 无 issues 权限"
    requirement: ENGINE-05
    verification:
      - kind: other
        ref: "rg 结构检查：schedule、report-failure、failure()、非注释行 issues: write 恰好 1 处、5432:5432、\"483\"、\"484\"、ONTOFORGE_TRINO_ICEBERG、iceberg.catalog.type=jdbc；YAML 可解析"
        status: pass
    human_judgment: true
    rationale: "需要 GitHub 托管 runner 与 issue 权限；Iceberg 本地 warehouse 与 JDBC catalog 属性（A12）首次 nightly 才能调通"
  - id: D5
    description: "ENGINE-01 / ENGINE-03 在真实 Trino 上的验证：超时取消服务端查询、printer 固定语料往返"
    requirement: ENGINE-01
    verification: []
    human_judgment: true
    rationale: "测试已写好且为 ignored，只能在首次 CI 运行中执行"
  - id: D6
    description: "公开 CI 不运行私有测试、仓库内无私有绑定或数据（ENGINE-05 prohibition）"
    requirement: ENGINE-05
    verification:
      - kind: other
        ref: "rg -n \"private_fixtures\" .github/workflows 无匹配；rg -n \"password|token\" ci/ontoforge-server.ci.toml 无匹配；rg -n \"pull_request_target\" .github/workflows 无匹配"
        status: pass
    human_judgment: false

duration: 11min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 10: Linux CI 集成测试与 nightly Iceberg Summary

**PR 新增 `integration` 作业（postgres:17 发布 5432 + docker run 的 Trino 483，种入 aviation fixture 后以 `--ignored` 运行 10 个引擎测试与 1 个 CLI 测试并门禁执行数），nightly 在 Trino 483 / 484 上加跑 Iceberg（JDBC catalog + 本地 warehouse），失败由唯一拥有 `issues: write` 的独立 job 开 issue；公司环境只经 `ONTOFORGE_PRIVATE_FIXTURES` 本地验证。所有真实引擎结果待首次 CI 运行确认。**

## Performance

- **Duration:** 约 11 min（自 ledger 记录起，不含开工前的上下文阅读）
- **Started:** 2026-10-17T00:54:43Z
- **Completed:** 2026-10-17T01:05:49Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 3（Task 1 tracer、Task 2 auto/tdd、Task 3 auto）
- **Files modified:** 13（11 个新文件，修改 `ci.yml` 与 `CONTRIBUTING.md`；另 `.planning/WINDOWS.md` 随 SUMMARY 提交）

## Accomplishments

- `integration` 作业：`services.postgres` 用 `postgres:17`、健康检查与 `ports: ["5432:5432"]`（E25）；checkout 后用 runner 自带 `psql` 建 `demo` / `editstore` 库并执行 `ci/pg/seed.sql`，`docker run -d --network host -v "$PWD/ci/trino/catalog:/etc/trino/catalog:ro" trinodb/trino:483`，轮询 `/v1/info` 的 `starting` 至多 180 s（超时打印 `docker logs trino`），然后运行两个测试目标并对两者的 `--ignored --list` 计数，任一为 0 即失败；失败时输出 Trino 日志。触发器仍只有 `push` 与 `pull_request`，沿用顶层 `contents: read` 与三个钉 SHA 的 action。
- `ci/pg/seed.sql`：`\cd :'fixtures'` 后逐表 `\copy` CSV 到 text staging 表，再按类型转换插入（`timestamp(3)`、`timestamptz(3)`、`numeric(12,2)`、`double precision`、`integer`、`date`、`varchar(n)` / `text`，`tags` 由 JSON 数组文本经 `jsonb_array_elements_text ... WITH ORDINALITY` 转 `text[]`）。fixture 仍是唯一数据来源；`\copy` 不展开 psql 变量，所以用 `\cd` 切到 fixture 目录。
- `trino_it.rs`：`select_one`、`describe_demo_flights`（列集合等于 Flight 绑定的物理列）、`describe_types_golden`（五张表一次批量描述，E22）、`decode_golden`（integer、double、decimal(12,2)、date、timestamp、timestamp with time zone、varchar、boolean、array(varchar) 与 NULL）、`printer_corpus_round_trips`（16 条固定语料 + 10 000 字符串，作字面量与别名各回读一次）、`timeout_cancels_server_query`（tpch.sf1000.lineitem 求和、1 s 超时）、`catalogs_visible`、`drift_is_healthy_on_ci_trino`、`iceberg_table_is_readable`。
- `cli_trino_it.rs`：`validate --online --check-pk --server-config ci/ontoforge-server.ci.toml examples/ontologies/aviation --format json` 退出码 0，诊断恰为 5 条「fixture 被忽略」的 ONT-V17 warning；另断言 `ONTOFORGE_TRINO_URL` 与配置文件中的 url 一致。本机对无 Trino 的 `localhost:8080` 运行同一命令得到 V19 + 退出码 3（不是 V20），证明 CI 配置文件可被解析且 profile 映射到 Trino，JSON 形状与测试断言一致。
- `nightly.yml`：每日 `schedule` + `workflow_dispatch`；`iceberg` 矩阵 job（483 / 484，`fail-fast: false`）复用 integration 步骤，另建 `iceberg_catalog` 库与 JDBC catalog 表，把两个 catalog 目录复制到同一临时目录后挂载，Trino 就绪后 `docker exec trino trino --execute` 建 `iceberg.aviation` 并 CTAS 复制 `demo.aviation.flights`，以 `ONTOFORGE_TRINO_ICEBERG=1` 运行全部 ignored 测试；`report-failure` job（`needs: iceberg`、`if: ${{ failure() }}`、job 级 `permissions: issues: write`、不 checkout）用 `gh` 找未关闭的 `nightly-failure` issue，有则评论运行链接，无则创建标签与 issue。
- `private_fixtures.rs`：遍历 `$ONTOFORGE_PRIVATE_FIXTURES/*/`，对同时含 `ontology/` 与 `ontoforge-server.toml` 的子目录运行 `validate --online --server-config`，汇总所有非 0 退出后断言；两个工作流都不引用该目标。
- 最终门（本机 Windows）：`cargo test --workspace --locked` 573 passed、0 failed、11 ignored；`cargo clippy --workspace --all-targets --locked -- -D warnings` 与 `cargo fmt --all --check` 通过；两个 workflow 文件经 PyYAML `safe_load` 解析通过。

## 首跑待确认

以下各项本机无法执行，推送分支（由用户执行）后必须在 GitHub Actions 上实际观察到，**全部满足前不得写 `02-VERIFICATION.md` 的 passed，ROADMAP 的 Phase 2 不得勾选**（E21）：

1. **PR `integration` 作业绿**：日志中 seed 成功、Trino 483 在 180 s 内就绪、`trino_it` 显示 `running 10 tests`（含 `iceberg_table_is_readable`，PR 中只检查环境变量）、`cli_trino_it` 显示 `running 1 test`，全部通过；数量门禁打印的两个计数（预期 10 与 1）均大于 0。
2. **`describe_types_golden` 定稿**（PROVISIONAL）：预期 PG connector 报告 `varchar(16)` / `varchar(3)` / `varchar(32)`、`text` → `varchar`、`timestamp(3)`、`timestamp(3) with time zone`、`decimal(12,2)`、`double`、`integer`、`date`、`array(varchar)`，主键与 NOT NULL 列 `is_nullable = NO`，`"licenseNumber"` 显示为 `licensenumber`（A3）。不一致时按日志修正内联快照、删除 `PROVISIONAL` 注释并提交。
3. **`decode_golden` 定稿**（PROVISIONAL）：预期 timestamp 线格式 `2026-03-28 09:15:00.000`，timestamp with time zone 为 UTC 形态 `2026-02-01 10:00:00.000 UTC`（`+02:00` 的源值换算到 UTC），decimal 保留两位小数（`0.00`、`-9999999999.99`），NULL 字面量列类型为 `unknown`（A11）。处理方式同上。
4. **`timeout_cancels_server_query`**：确认 `system.runtime.queries` 有 `source` / `query` / `state` / `error_code` 列，超时后 2 s 内该查询为 `FAILED` + `USER_CANCELED`（A5）。若 Trino 实际报告不同的错误码或需要更久，按日志调整并在下一个 SUMMARY / VERIFICATION 记录。
5. **`printer_corpus_round_trips`**：别名回读假设 Trino 保留带引号别名的大小写与原样字符（含换行、10 000 字符的别名）；如 Trino 对别名有长度或字符限制，按日志收窄别名语料并说明原因。
6. **`drift_is_healthy_on_ci_trino` 与 CLI 退出码 0**：确认种子表类型对 D-13 类型表无 error（`scheduled_departure` 无时区 + `Europe/Berlin`、`performed_at` 带时区 + `aware`），且 `--check-pk` 无 ONT-V18。
7. **首次 nightly（手动 `workflow_dispatch`）**：（2026-10-19 更正：本条原为预期，写成了已发生。实际首跑 run 39220997866 两个 job 均失败，原因是 `local.location` 写成 URI；修复后 run 39221684753 两个 job 均通过，见 `02-UAT.md` 测试 2）原文：483 与 484 两个矩阵 job 都绿；Iceberg JDBC catalog（`ci/pg/iceberg_catalog.sql` 的 V1 表结构）与本地 warehouse 属性（`fs.local.enabled`、`local.location=local:///tmp/iceberg`、`default-warehouse-dir=local:///tmp/iceberg/warehouse`）能建 schema 与 CTAS（A12）；两个目标的 ignored 执行数各 ≥ 1。
8. **`report-failure`**：人为制造一次失败（例如临时改坏一个 nightly 断言后手动触发）时创建或评论 `nightly-failure` issue；`iceberg` job 的 token 没有 issues 权限。

上述 1、7 已登记为 `.planning/WINDOWS.md` 第 5、6 条（`unrun-verify`，open）。

## Task Commits

1. **Task 1: 端到端 tracer（tdd）**
   - RED `6b9b0df`（test）：`missing_env_panics` 因 `trino_url(None)` 不 panic 在断言上失败（`a missing ONTOFORGE_TRINO_URL must panic: ""`），其余为 ignored
   - GREEN `8fb8fb8`（feat）：`trino_url`、三个 catalog、`seed.sql`、`integration` 作业
2. **Task 2: golden、语料、超时、catalog、drift 与 CLI（tdd）**
   - RED `0c39836`（test）：`cli_trino_it::missing_env_panics` 在断言上失败；新增的引擎测试均为 ignored（本机无法 RED）
   - GREEN `5b5962e`（feat）：CLI 环境 helper、`ci/ontoforge-server.ci.toml`、CLI 测试步骤与数量门禁
3. **Task 3: nightly、私有 fixture 与 CONTRIBUTING** - `f937f5c`（feat）

**Plan metadata:** 本文件所在的 `docs(02-10)` 提交。无 REFACTOR 提交。

## TDD Gate Compliance

- Task 1：RED `6b9b0df` → GREEN `8fb8fb8`；Task 2：RED `0c39836` → GREEN `5b5962e`。
- RED 证据：`gsd_run check tdd-red-evidence` 只解析 node TAP，不适用于 `cargo test`（与 02-01 至 02-09 相同），因此人工核对目标测试在断言上失败而非编译错误。
- 真实 Trino 上的行为测试（ignored）在本机无法经历 RED；它们的首次执行就是「首跑待确认」中的 CI 运行。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase`、auto 模式未开，但 Task 1 的 `<verify>` 含 `<human-check>`（推送后看 GitHub Actions）。按 checkpoints.md 第 4 行本应停下；编排器明确要求连续执行全部任务并把需要真实 CI 的项列为首跑待确认，且该人工检查只有在用户推送整个分支后才能进行，中途停下得不到任何新证据。因此 Task 1 提交前重跑了全部 `<automated>`（`trino_it` 2 passed / 2 ignored、五条 rg 结构检查）后继续展开，人工检查并入上面的清单第 1 项。

## Files Created/Modified

- `.github/workflows/ci.yml` - 新增 `integration` 作业
- `.github/workflows/nightly.yml` - `iceberg` 矩阵 job 与 `report-failure` job
- `ci/trino/catalog/{demo,editstore,tpch}.properties` - PR 与 nightly 共用的 catalog
- `ci/trino/iceberg/iceberg.properties` - Iceberg JDBC catalog（本地 warehouse）
- `ci/pg/seed.sql` - aviation fixture 导入
- `ci/pg/iceberg_catalog.sql` - Iceberg JDBC catalog 表（计划外新增，见偏差 1）
- `ci/ontoforge-server.ci.toml` - CI Trino 的服务配置，无密钥
- `crates/ontoforge-engine/tests/trino_it.rs` - 真实 Trino 集成测试
- `crates/ontoforge/tests/cli_trino_it.rs` - CLI 端到端集成测试
- `crates/ontoforge/tests/private_fixtures.rs` - 私有 fixture 入口
- `CONTRIBUTING.md` - 「集成测试」节
- `.planning/WINDOWS.md` - 第 5、6 条 unrun-verify

## Decisions Made

见 frontmatter `key-decisions`。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - 阻塞] MinIO 镜像不可用，nightly 改用 Trino 本地文件系统作 Iceberg warehouse**
- **Found during:** Task 3
- **Issue:** 计划要求 `docker run` MinIO 并用 AWS CLI 建 bucket。`https://hub.docker.com/v2/repositories/minio/minio/` 返回 `object not found`，Docker Hub registry 与 `quay.io/minio/minio` 的匿名 pull token 都没有 pull 权限（401）。
- **Fix:** 经 Context7 查 Trino 文档确认 Iceberg connector 支持 `fs.local.enabled=true` + `local.location=local://...`；`iceberg.properties` 用本地 warehouse，nightly 把 `$RUNNER_TEMP/iceberg` 挂到容器 `/tmp/iceberg`，删去 MinIO 与 AWS CLI 步骤。Trino 文档同时说明 JDBC catalog 要求元数据表已存在，因此新增 `ci/pg/iceberg_catalog.sql`（V1 表结构）并在 nightly 中执行。没有改用另一家 S3 兼容镜像，避免在未经人工确认的情况下引入新的第三方镜像（T-02-40）。
- **Files modified:** `ci/trino/iceberg/iceberg.properties`、`ci/pg/iceberg_catalog.sql`、`.github/workflows/nightly.yml`、`CONTRIBUTING.md`
- **Verification:** 结构检查与 YAML 解析通过；实际可用性见首跑待确认第 7 项
- **Committed in:** `f937f5c`
- **用户可复核：** 若希望 nightly 覆盖 S3 路径，需要选定一个可信的 S3 兼容镜像（这是供应链决策），届时改 `iceberg.properties` 与 nightly 的启动步骤即可。

**2. [Rule 3 - 范围对齐] golden 用文本内联快照而非 `assert_json_snapshot!`**
- **Found during:** Task 2
- **Issue:** `PhysicalColumn` 与 `Value` 不实现 `Serialize`，JSON 快照需要给库类型加 derive。
- **Fix:** 把结果渲染成稳定文本（类型用 `PhysicalType::render`，值用 `Debug`），用 `insta::assert_snapshot!(text, @r"...")` 内联固定；预期值仍写在源码里，CI 下 insta 不写快照文件，不一致即失败。
- **Committed in:** `0c39836`

**3. [Rule 2 - 补充] 计划外的少量加固**
- `cli_trino_it` 断言 `ONTOFORGE_TRINO_URL` 与 `ci/ontoforge-server.ci.toml` 的 url 一致，避免环境变量指向别处而 CLI 实际连另一个 Trino。
- 超时测试在查询文本中放唯一 marker 并断言 `error_code = USER_CANCELED`，区分客户端 DELETE 与 session 上限造成的结束。
- nightly 也运行 `cli_trino_it` 与数量门禁（E21 要求两个目标在首次 nightly 中各执行 ≥ 1）。
- `private_fixtures` 增加两个本机测试（缺变量 panic、子目录必须同时有两部分）。

---

**Total deviations:** 3（2 个阻塞 / 范围对齐、1 个补充）
**Impact on plan:** 公开行为与 must_haves 一致；唯一实质变化是 nightly 的 Iceberg 存储从 MinIO 改为本地文件系统，原因是 MinIO 镜像已无法公开拉取。

## Issues Encountered

- Git Bash 把 rg 模式 `ci/trino/catalog:/etc/trino/catalog` 当成 POSIX 路径改写成 Windows 路径，导致 rg 正则报错；加 `MSYS_NO_PATHCONV=1` 后检查通过（与 02-04 的 openssl 问题同源）。
- 本机对无监听的 `localhost:8080` 运行 CLI 时每个绑定得到 `ENGINE_TIMEOUT` 的 V19（而不是整轮 `ENGINE_UNAVAILABLE`），这是 Windows 上连接被拒的重试耗尽超时（02-06 已记录），不影响 CI 上的真实 Trino。

## Known Stubs

None。`iceberg_table_is_readable` 在没有 `ONTOFORGE_TRINO_ICEBERG=1` 时只检查 `ONTOFORGE_TRINO_URL` 后返回，这是计划规定的行为（PR 作业没有 Iceberg catalog），已在 doc 注释说明。

## Threat Flags

None。T-02-39（`issues: write` 只在不 checkout 的 `report-failure` job，非注释行恰好 1 处）、T-02-40（action 沿用钉 SHA，镜像用固定版本标签，未新增第三方镜像，`--locked`）、T-02-41（CI 口令只在一次性容器内，accept）、T-02-42（工作流不引用 `private_fixtures`）、T-02-43（缺变量 panic、数量门禁、golden 标 PROVISIONAL）均已落地。nightly 的 `gh` 只使用 `github.token`，无其他 secret。

## User Setup Required

None - 无需外部服务配置。需要用户执行的是推送分支并手动触发一次 nightly，见「首跑待确认」。

## Next Phase Readiness

- Phase 2 的全部 plan 已有 SUMMARY，但 **Phase 2 尚不能判定完成**：E21 门禁要求首跑待确认的各项实际通过、golden 定稿并删除 `PROVISIONAL` 注释后，才能写 `02-VERIFICATION.md` 的 passed。
- Phase 3 可直接使用 CI 中的 `editstore` catalog（同一 PostgreSQL 实例的 `editstore` 库，目前为空）。

## Self-Check: PASSED

- 计划列出的 12 个文件与新增的 `ci/pg/iceberg_catalog.sql` 均存在于磁盘。
- 提交 `6b9b0df`、`8fb8fb8`、`0c39836`、`5b5962e`、`f937f5c` 均存在，`git rev-list --count c5e97e9..HEAD` 为 5；每个提交 `git diff --diff-filter=D` 无删除。
- Task 1 acceptance：`array-mapping=AS_ARRAY`、`editstore.properties` 与 `tpch.properties` 存在、`CREATE SCHEMA` 与 `copy` 有匹配、`--ignored --list` 计数 2（≥ 2）、无 `pull_request_target`。
- Task 2 acceptance：`trino_it --ignored --list` 计数 9（Task 2 时 8，≥ 7）、`10_000` 有匹配、`ci/ontoforge-server.ci.toml` 无 `password|token`。
- Task 3 acceptance：nightly 的 action 钉 40 位 SHA、`.github/workflows` 无 `private_fixtures` 与 `pull_request_target`、CONTRIBUTING 含 `ONTOFORGE_PRIVATE_FIXTURES` 与 `trino_it`；nightly 结构检查（含非注释行 `issues: write` 恰好 1 处）通过。
- 最终门：`cargo test --workspace --locked`（573 passed、0 failed、11 ignored）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 通过；`Cargo.toml` / `Cargo.lock` 未改。
- 未在本机观察到任何 CI 运行，所有 CI 结果标为待确认，未声称通过。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
