---
phase: "02"
plan: "11"
type: evidence
produced_by: "02-11-PLAN.md Task 1"
generated: 2026-11-08
git_head_at_generation: "f2e73edce3e8d97934e1a7704db8a519b85a5f1d"
plan_start_head: "9a6eaffe060e93df1c19c79c7b25d3946a7d3229"
---

# 02-10.6 源码级调和证据

本文件是 Phase 02 残余项 **G-02-A** 的可引用证据，供下一轮 `/gsd-verify-work 02` 重新判定 `02-VERIFICATION.md:194` 的 `02-10.6 | FAILED (BLOCKER)` 一行使用。

本文件**不是**验收结论，**不**修改 `02-VERIFICATION.md`，**不**对 G-02-B / G-02-C / G-02-D 作任何结论。

## Scope and Method

**核验对象**：`02-VERIFICATION.md:194` 正文里仍记为 `FAILED (BLOCKER)` 的 truth `02-10.6`。该 truth 的原文见 `02-10-PLAN.md:36`：

> nightly 工作流在 Trino 483 与 484 上额外跑 Iceberg（JDBC catalog + MinIO），失败时由独立的 `report-failure` job 自动开 issue（同一标签下已有未关闭 issue 则追加评论）；`issues: write` 只授予这个 job（GitHub Actions 的 permissions 只能设在 workflow 或 job 层），测试 job 只有 `contents: read`（E25）

**核验方式**：**当前源码逐行读取**。本轮生成时 `git rev-parse HEAD` = `f2e73edce3e8d97934e1a7704db8a519b85a5f1d`；本 plan 的起始 HEAD 为 `9a6eaffe060e93df1c19c79c7b25d3946a7d3229`。每条断言都由一条**可失败的只读门禁**（G-A1..G-A5）支撑，门禁命令见 `02-11-PLAN.md` Task 1 的 `<verify>` 节，逐条实测输出见本文件 `## This Round Did Not Execute` 之后的说明与 SUMMARY。

**本轮没有做的事**（逐项见 `## This Round Did Not Execute`）：没有启动任何容器、没有运行 nightly、没有连接 Trino / MinIO / PostgreSQL / SQL Server、没有修改 git 历史、没有修改 `02-VERIFICATION.md`。

**关于「实际运行过」的断言**：本文件中的所有「实跑」陈述一律引用 Phase 07.2 的既有原始证据（run 编号、job 编号、headSha），**不冒充本轮执行**。本轮只做了源码结构核验。

## Truth 02-10.6 — Per-Assertion Verdict

02-10.6 的 truth 含五个要素，逐条判定如下。`Verdict` 取值限定为 `PASS` / `REFINED (accepted by 07.2)` / `DEVIATION`。

| Assertion | Current Source Location | Observed Content | Verdict | Machine Gate | 07.2 Evidence |
|---|---|---|---|---|---|
| A1 「在 Trino 483 与 484 上跑 Iceberg」 | `.github/workflows/nightly.yml:20-25`（矩阵 `trino: "483"` 于 :22、digest 于 :23；`trino: "484"` 于 :24、digest 于 :25）；执行入口 `.github/workflows/nightly.yml:45` → `ci/iceberg-services.sh`；env 门控 `ci/iceberg-services.sh:199`，注入 `ONTOFORGE_TRINO_ICEBERG=1` 于 `ci/iceberg-services.sh:476` | `nightly.yml:45` 为 `run: ONTOFORGE_ICEBERG_STATE=... bash ci/iceberg-services.sh nightly --trino "${{ matrix.trino }}"`；`iceberg-services.sh:199` 为 `require(os.environ.get("ONTOFORGE_TRINO_ICEBERG") == "1", "Iceberg environment guard failed")`；`iceberg-services.sh:476` 注入 `ONTOFORGE_TRINO_ICEBERG="1"` | PASS | G-A1 | 07.2-VERIFICATION.md:43（truth 2，483/484 双版真实 MinIO 合同 VERIFIED）；07.2-CI-VERIFICATION.md:15（nightly run `41833307824`，483 job `126585725657`、484 job `126585726171` 均 success） |
| A2 「JDBC catalog」 | `ci/trino/iceberg/iceberg.properties:3`（`iceberg.catalog.type=jdbc`）、`:4`（`iceberg.jdbc-catalog.catalog-name=ontoforge_ci`）、`:5`（`iceberg.jdbc-catalog.driver-class=org.postgresql.Driver`）、`:6`（`iceberg.jdbc-catalog.connection-url=jdbc:postgresql://localhost:5432/iceberg_catalog`）、`:7-8`（`connection-user=postgres`、空 `connection-password`） | 六个 JDBC 键逐字在位：catalog type、catalog name、driver class、connection url、user、password（空值，由 runner 注入临时凭据） | PASS | G-A2 | 07.2-VERIFICATION.md:44（truth 3，PG `metadata_location` 以 `s3://warehouse/` 开头且 bucket 内有非空 metadata/data 对象 VERIFIED） |
| A3 「MinIO」 | `ci/trino/iceberg/iceberg.properties:9`（`default-warehouse-dir=s3://warehouse/`）、`:10`（`fs.s3.enabled=true`）、`:11`（`s3.endpoint=http://minio:9000`）、`:12`（`s3.path-style-access=true`）、`:13`（`s3.region=us-east-1`）；MinIO 容器编排 `ci/iceberg-services.sh:431`（`S3_ENDPOINT=http://minio:9000`）；自建镜像 `ci/minio/Dockerfile`、自建 S3 客户端 `ci/minio/bucket.go`；全文件**无** `fs.local.enabled` / `local:///warehouse` 回退键 | warehouse 指向 `s3://warehouse/`，S3 后端启用并指向仓库内自建的 MinIO；`rg -q "fs\.local\.enabled\|local:///warehouse"` 在该文件上退出码为 1（无匹配），确认没有本地文件系统回退 | PASS | G-A3 | 07.2-VERIFICATION.md:42（truth 1，固定官方源码与固定 Go builder 产生可运行的 scratch MinIO/server 及 bucket client，保留真实版本、checksum 和 provenance VERIFIED）、`:45`（truth 4，失败与清理门控 VERIFIED）、`:47`（truth 6，owner 隔离 VERIFIED） |
| A4 「失败时由独立的 `report-failure` job 自动开 issue，同一标签下已有未关闭 issue 则追加评论」 | `.github/workflows/nightly.yml:79`（`report-failure:`）、`:80`（`needs: iceberg`）、`:81`（触发条件）、`:92`（`gh issue list ... --label nightly-failure --state open`）、`:94`（`gh issue comment`）、`:97`（`gh issue create`） | 三个 `gh` 字面量同时在位，构成「查未关闭 issue → 有则评论 / 否则新建」三步；`report-failure` 是与 `iceberg` 并列的独立 job | REFINED (accepted by 07.2) | G-A4 | 07.2-VERIFICATION.md:49（truth 8，`workflow_dispatch` 的 report-failure 为 skipped，只有 schedule 失败才允许 issue/comment VERIFIED；run `41833307824` event=`workflow_dispatch`，通知 job `126592749564` 实际 skipped）；07.2-CI-VERIFICATION.md:15 |
| A5 「`issues: write` 只授予这个 job，测试 job 只有 `contents: read`」 | workflow 级 `.github/workflows/nightly.yml:10-11`（`permissions:` / `contents: read`）；`iceberg` job 块 `.github/workflows/nightly.yml:14-75` 内**无** job 级 `permissions:` 键；job 级 `.github/workflows/nightly.yml:83-84`（`permissions:` / `issues: write`，全文件非注释行中唯一一处）；普通 CI 工作流 `.github/workflows/ci.yml:15-16`（`permissions:` / `contents: read`） | `test "$(rg -v '^\s*#' .github/workflows/nightly.yml \| rg -c 'issues: write')" = "1"` 成立；`sed -n "/^  iceberg:/,/^  report-failure:/p"` 区间内无 `permissions:` 键 | PASS | G-A5 | 07.2-VERIFICATION.md:49（truth 8，「仅该 job 有 issues:write 且不 checkout」VERIFIED） |

**机器门禁与可判定性的边界（本节不得留白）**：G-A1..G-A5 全部是**结构门禁**，它们证明「合同在当前源码中成立」，**不**证明「nightly 在某次运行中真的跑绿」。后者需要 Docker / GitHub runner / 网络，超出本 plan 的零网络零容器约束，只能引用 07.2 的既有原始 run `41833307824`（483 job `126585725657`、484 job `126585726171`、report-failure job `126592749564`）。本文件**不**把结构门禁的通过表述为实跑通过；`## This Round Did Not Execute` 节与本条保持一致。

## Implementation Path Deviations (vs 02-10 Plan Text)

以下逐条记录与 **02-10 计划文本**（`.planning/phases/02-source-binding-engine/02-10-PLAN.md`）的差异，并标明它是否影响 02-10.6 truth 的成立。

**D-01**：`.github/workflows/nightly.yml` 已无 `services.postgres` 与 `5432:5432`；容器编排整体迁到 `ci/iceberg-services.sh`（`docker` 调用见 `ci/iceberg-services.sh:471`）。02-10 的起草文本见 `02-10-PLAN.md:227`（「services.postgres 同 integration 作业显式 `ports: ["5432:5432"]`」），该起草还要求 nightly 自行起 PostgreSQL service container。
**与 truth 的关系**：**不影响** 02-10.6 truth 的任何要素——矩阵、独立 job、权限范围在当前源码中均成立；这是编排位置的迁移，不是合同缺失。

**D-02**：02-10 Task 3 起草的 S3 属性名 `fs.native-s3.enabled=true` 在当前文件里是 `fs.s3.enabled=true`（`ci/trino/iceberg/iceberg.properties:10`），且 `s3.endpoint` 写的是 `http://minio:9000`（`:11`）而非 02-10 起草的 `http://localhost:9000`（起草文本见 `02-10-PLAN.md:225`）。
**与 truth 的关系**：**不影响** truth 成立。这是属性名与地址的收敛，不是功能缺失；当前形式是被 07.2 用真实 MinIO 验证过的形式（`07.2-VERIFICATION.md:44` truth 3）。同属 REFINED，不是缺口。

**D-03**：`report-failure` 的触发条件从 02-10 起草的 `if: ${{ failure() }}` 收窄为 `if: ${{ github.event_name == 'schedule' && failure() }}`（`.github/workflows/nightly.yml:81`；起草文本见 `02-10-PLAN.md:227`）。
**与 truth 的关系**：**不影响** truth 成立。07.2 truth 8（`07.2-VERIFICATION.md:49`）明确接受这一收紧（手动触发不再开 issue）。属 REFINED，不是缺口。

**D-04**：02-10 起草让 nightly 直接 `docker run` MinIO 与 Trino 并自行灌种子（`02-10-PLAN.md:227`）；当前由 `ci/iceberg-services.sh` 统一编排，并把 `ci/trino/iceberg/iceberg.properties` 复制成私有副本后注入临时凭据，再只读挂载（`ci/iceberg-services.sh:438` 纳入 catalog 目录、`ci/iceberg-services.sh:471` 的 `-v ... :/etc/trino/catalog:ro`、`ci/iceberg-services.sh:491` 的 `test -r /etc/trino/catalog/iceberg.properties`）。
**与 truth 的关系**：**不影响** truth 成立。合同要求的「JDBC catalog + MinIO」语义未变，只是执行主体从 workflow 移入 runner 脚本；07.2 的真实双版运行即在该路径上取得（`07.2-VERIFICATION.md:43`）。

## Phase 07.2 Cross-Reference

### 07.2 的 must-haves 表

`07.2-VERIFICATION.md` 的 `### Observable Truths`（节标题在 `:38`，9 行 truth 在 `:42`-`:50`）逐条覆盖：

| 07.2 truth | 行号 | 覆盖内容 |
|---|---|---|
| 1 | `07.2-VERIFICATION.md:42` | 固定官方源码与固定 Go builder 产生可运行的 scratch MinIO/server 及 bucket client，保留真实版本、checksum 和 provenance |
| 2 | `07.2-VERIFICATION.md:43` | 483 与 484 均通过真实 MinIO 合同 |
| 3 | `07.2-VERIFICATION.md:44` | PG `metadata_location` 以 `s3://warehouse/` 开头，bucket 内有非空 metadata 与 data 对象 |
| 4 | `07.2-VERIFICATION.md:45` | 缺环境、零发现、SDK 错误、泄漏扫描错误及清理错误均阻止成功；owner 之外资源保持原状 |
| 5 | `07.2-VERIFICATION.md:46` | 每版本重启 MinIO 与 Trino 后无需重新 CTAS 仍读 29 行 |
| 6 | `07.2-VERIFICATION.md:47` | 真实部分创建、超时和 TERM 后 owner 资源归零；泄漏证据不进入 CI 日志或上传 |
| 7 | `07.2-VERIFICATION.md:48` | nightly 保留既有 Action/Engine/CLI 完整 corpus 及非零发现 |
| 8 | `07.2-VERIFICATION.md:49` | `workflow_dispatch` 的 `report-failure` 为 skipped；只有 schedule 失败才允许 issue/comment |
| 9 | `07.2-VERIFICATION.md:50` | ENGINE-05 仅在独立 review、安全及目标验收通过后由主会话更新，旧封存指纹与历史 judgment 不被改写 |

### 07.2 的 GitHub 终态

nightly run `41833307824`（483 job `126585725657`、484 job `126585726171`，均 success；`report-failure` job `126592749564` skipped），ci run `41833241053`，release run `41833240915`；三者 headSha 均为 `b7e44fa934b56c9d68620a4ea103d68ee2fb3a92`。见 `07.2-CI-VERIFICATION.md` 的 run 表（`:15` nightly、`:16` ci、`:17` release；`:4` `verified_head`）。`07.2-RUNTIME-EVIDENCE.md` 承载逐行为的原始运行证据。

### 边界声明（原文，不得改写）

07.2 的 `passed` **只覆盖 MinIO/S3 + PostgreSQL JDBC Iceberg 子范围**；它**没有**执行 SQL Server 或任何公司 Trino 集群，**没有**对公司环境做人工核验，因此它**不闭合 ENGINE-05 的私有 SQL Server 边界**（即 `02-VERIFICATION.md:196` 的 `02-10.8 | UNCERTAIN (WARNING; judgment)`），也**不**闭合 02-03 的公开数据来源判断（`02-VERIFICATION.md:287`）与 02-10 的公开 CI 边界判断（`02-VERIFICATION.md:288`）。

依据原文：`07.2-VERIFICATION.md:136`（「ENGINE-05历史私有fixture子句 | 原02-10；非本阶段新增任务 | SQL Server只经仓库外私有fixture，CI不执行公司数据 | 保留原历史边界，不作新增通过声明 | REQUIREMENTS原文和ADR-0017仍保留；CONTRIBUTING:85继续说明私有入口。07.2没有SQL Server真实执行或数据来源人工验收证据。旧02-VERIFICATION的02-10.8 judgment不被本报告覆盖。」）与 `07.2-VERIFICATION.md:174`（`### Deferred Items` 节：「None. … Phase01/02历史judgment和私有SQL Server来源边界从未纳入07.2新增目标，不能用此处passed清除。」）。

### 本文件的效力声明

本文件是对 `02-VERIFICATION.md:194` 中 `02-10.6 = FAILED (BLOCKER)` 一行的**源码级调和**，供下一轮 `/gsd-verify-work 02` 重新判定使用；它本身**不**修改 `02-VERIFICATION.md`，也**不**构成对 G-02-B / G-02-C / G-02-D 的任何结论。这三项的机器可判定部分见 `## Machine-Checkable Portions of G-02-B / G-02-C / G-02-D`，其结论仍为 PENDING，见 `02-11-HUMAN-ACCEPTANCE.md`。

## NEUTRAL VOCABULARY

公开面物理标识符的中性词表，由 Task 2 的一次只读扫描填充。词表与 `02-11-source-scan.txt` 的 `==== IDENTIFIERS ====` 段**逐行相等**（双向 diff 为空）。

<!-- BEGIN IDENTIFIERS -->
<!-- filled by Task 2 -->
<!-- END IDENTIFIERS -->

## Machine-Checkable Portions of G-02-B / G-02-C / G-02-D

本节**只覆盖**三个 judgment 项中**机器可判定的那一部分**，是证据而不是结论。三个项的**结论**仍为 PENDING，见 `02-11-HUMAN-ACCEPTANCE.md`。详细原始输出见 `02-11-source-scan.txt` 的 `==== PRIVATE PATH GATE ====` 段。

| 门控 | 范围 | 命令 | 结果 | 能证明 | 不能证明 |
|---|---|---|---|---|---|
| C-1 | `.github/` 全目录 | `rg -n "private_fixtures\|ONTOFORGE_PRIVATE_FIXTURES" .github/` | RESULT: PASS（退出码 1，输出为空） | 公开工作流文件中不存在私有 fixture 目标名或私有环境变量字面量 | runner 上实际被注入的环境变量、仓库外目录的可达性 |
| D-1 | `crates/ontoforge/tests/private_fixtures.rs` | `rg -n "ONTOFORGE_PRIVATE_FIXTURES" crates/ontoforge/tests/private_fixtures.rs` 与 `rg -n '#\[ignore = "needs ONTOFORGE_PRIVATE_FIXTURES"\]' crates/ontoforge/tests/private_fixtures.rs` | RESULT: PASS（两处均有匹配，退出码 0） | 私有入口同时具备环境变量门控（`:14`、`:21`）与 `#[ignore]` 门控（`:69`） | 仓库外目录的真实内容；git 历史中是否曾提交后删除过私有绑定 |
| D-2 | `CONTRIBUTING.md` | `rg -n "ONTOFORGE_PRIVATE_FIXTURES" CONTRIBUTING.md` | RESULT: PASS（退出码 0，匹配行 `:89`） | 贡献文档写明了私有入口的位置约定 | 文档描述与实际部署形态是否一致 |

## This Round Did Not Execute

本轮（plan `02-11` Task 1 / Task 2 的源码核验部分）**没有**做以下任何一件事：

- 没有启动 Docker / 任何容器
- 没有拉取任何镜像
- 没有运行 `ci/iceberg-services.sh`（任何模式）
- 没有触发、重跑或以任何方式干预 GitHub nightly / ci / release workflow
- 没有连接任何 Trino 实例（含 483 / 484）
- 没有连接任何 MinIO 实例
- 没有连接任何 PostgreSQL 实例
- 没有连接任何 SQL Server 实例或任何公司 Trino 集群
- 没有连接任何公司环境或私有环境
- 没有读取 `ONTOFORGE_PRIVATE_FIXTURES` 指向的仓库外目录
- 没有修改 git 历史（无 amend / rebase / reset / tag / push）
- 没有修改 `02-VERIFICATION.md`、`02-UAT.md`、`02-CONTEXT.md`、`02-RESEARCH.md`、`02-PATTERNS.md`、`.planning/REQUIREMENTS.md`、`.planning/ROADMAP.md`、`.planning/STATE.md`、`.planning/state.json`
- 没有修改 `crates/`、`ci/`、`.github/`、`examples/`、`docs/` 下的任何已跟踪文件

本文件中一切「实际运行过」的陈述，其证据来源是 Phase 07.2 的既有原始 run 与日志（`41833307824` / `41833241053` / `41833240915`，headSha `b7e44fa934b56c9d68620a4ea103d68ee2fb3a92`），**不是本轮执行**。本轮产生的全部证据是**源码结构门禁**（G-A1..G-A5、C-1、D-1、D-2）的通过/失败结果。
