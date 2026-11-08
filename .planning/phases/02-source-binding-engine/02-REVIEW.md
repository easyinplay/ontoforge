---
phase: 02-source-binding-engine
reviewed: 2026-11-08T09:07:18Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - .planning/phases/02-source-binding-engine/02-10-SUMMARY.md
  - .planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md
  - .planning/phases/02-source-binding-engine/02-11-PLAN.md
  - .planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md
  - .planning/phases/02-source-binding-engine/02-11-SUMMARY.md
  - .planning/phases/02-source-binding-engine/02-11-source-scan.txt
  - .planning/phases/02-source-binding-engine/02-REVIEW-FIX.md
  - .planning/phases/02-source-binding-engine/02-SECURITY.md
  - .planning/phases/02-source-binding-engine/02-UAT.md
  - .planning/phases/02-source-binding-engine/02-VALIDATION.md
  - .planning/phases/02-source-binding-engine/02-VERIFICATION.md
  - .planning/phases/02-source-binding-engine/progress.md
  - .planning/phases/02-source-binding-engine/task_plan.md
findings:
  critical: 1
  warning: 9
  info: 3
  total: 13
  necessity: 1
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-11-08T09:07:18Z
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

**审查对象**：`.planning/phases/02-source-binding-engine/` 下的 13 份计划 / 证据类文档（`02-10-SUMMARY.md`、`02-11-PLAN.md`、`02-11-RECONCILIATION.md`、`02-11-source-scan.txt`、`02-11-HUMAN-ACCEPTANCE.md`、`02-11-SUMMARY.md`、`02-REVIEW-FIX.md`、`02-SECURITY.md`、`02-UAT.md`、`02-VALIDATION.md`、`02-VERIFICATION.md`、`progress.md`、`task_plan.md`）。**本次审查的不是产品代码**：`crates/`、`ci/`、`.github/`、`examples/`、`docs/` 下的文件一律只读，仅作为核对基准被引用。

**审查性质**：文档事实核对 + 跨文档自洽性核对。逐条复验文档中的 `file:line` 引用、行数 / 计数、符号名、命令、测试名、commit hash、门禁编号与 CI run 号是否与**当前仓库源码、CI 配置与 git 历史**一致；核对 `02-SUMMARY` / `02-VERIFICATION` / `progress.md` / `task_plan.md` / `02-VALIDATION.md` / `02-SECURITY.md` / `02-UAT.md` 之间是否互相矛盾；核对结论强度是否匹配证据强度（结构门禁通过 ≠ 实跑通过）；扫描 `TODO` / 待补充 / 暂定 / `TBD` / 空节残留；核对机器可判定与人工可判定的切分是否守住（三处 `VERDICT: PENDING` 必须仍为 `PENDING`）。

**结论**：13 份文档的**主干结论成立**——三处 `VERDICT: PENDING`（`02-11-HUMAN-ACCEPTANCE.md:44/:60/:78`）未被代签，RECONCILIATION 与扫描文件的 45 词中性词表逐行相等（双向 diff 为空），`## SUSPECTED NON-NEUTRAL` 为空，`## This Round Did Not Execute` 在相关文档中齐备且具体，没有把任何未执行的 Docker / Trino / MinIO / PostgreSQL / SQL Server / nightly / CI 运行写成已通过。问题集中在**引用精度与状态一致性**上：5 条标识符来源引用指向了完全无关的行（CR-01），`02-VERIFICATION.md` 的 MinIO 断言与分数存在自相矛盾且已被后续文档判定为陈旧（WR-01 / WR-02），`02-10-SUMMARY.md` 的测试计数与预测值失配（WR-03 / WR-08），以及若干元数据陈旧与门禁编号误标（WR-04 / WR-05 / WR-07 / IN-01）。

**本次审查明确没有做的事**：**没有运行任何构建或测试命令**（`cargo build` / `cargo test` / `cargo fmt` / `cargo clippy` 一律未执行）；**没有启动 Docker 或任何容器**；**没有连接任何外部服务**（Trino / MinIO / PostgreSQL / SQL Server / 公司环境一个都没有连）；**没有触网**，因此文中出现的 CI run 号（`39219827829`、`39220997866`、`39221684753`、`41833307824` 等）只能按文档转述采信，无法在本轮复跑验证；**没有读取** `ONTOFORGE_PRIVATE_FIXTURES` 指向的仓库外目录；**没有修改除本文件外的任何文件**，没有改 git 历史，没有 commit、没有 push。因此请勿把「文档审查通过」读成「代码真的跑过并通过」——本文档只断言文档里写的东西与仓库里现存的东西是否对得上。

## Critical Issues

### CR-01: 标识符来源表 5 条 `file:line` 引用指向完全无关的行

**File:** `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:84`、`:95`、`:110`、`:116`、`:124`

**Issue:** 「标识符逐条来源（file:line）」表里有 5 条引用解析到的行与所声明的标识符毫无关系：

| 表行 | 声明 | 实际该行内容 | 正确位置 |
|---|---|---|---|
| `:84` | `Orders` → `ci/trino/l2/rules.json:19` | `:19` 是 `{ "user": "l2", "catalog": "system", "schema": "runtime", "table": "queries", ... }` | `ci/trino/l2/rules.json:18`（`"table": "orders\|Orders\|shipments\|items\|ordеrs"`） |
| `:95` | `cardinality_records` → `ci/pg/seed.sql:32` | `:32` 是 `CREATE TABLE aviation.aircraft (` | `ci/pg/seed.sql:172`（`CREATE TABLE aviation.cardinality_records (`） |
| `:110` | `orders` → 第二处 `ci/trino/l2/rules.json:19` | 同上，`system/runtime/queries` 规则行 | `ci/trino/l2/rules.json:18` |
| `:116` | `queries` → `ci/trino/l2/rules.json:20` | `:20` 是 `{ "privileges": [] }` | `ci/trino/l2/rules.json:19` |
| `:124` | `system` → 第二处 `:20` | 同上，`{ "privileges": [] }` | `ci/trino/l2/rules.json:19` |

核验依据（只读）：`ci/trino/l2/rules.json` 全文 33 行逐行打印，`ci/pg/seed.sql:25-40` 与 `:165-205` 逐行打印。影响面：这张表正是 `02-11-HUMAN-ACCEPTANCE.md:34` 作为 G-02-B 机器可判定一半交给人工验收人的证据入口，四条引用解析到的行里根本没有所声称的标识符，一条指向了另一张表。**45 词清单本身、`## SUSPECTED NON-NEUTRAL` 为空与「词表全部中性」这一结论不受影响**（每个标识符在改正后的位置都真实存在，例如 `runtime` 那一行 `:119` 引用的 `rules.json:19` 就是对的），受损的是逐行可追溯性。

**Fix:** 把上述 5 处行号改为正确值（`:84` → `rules.json:18`；`:95` → `ci/pg/seed.sql:172`；`:110` → `rules.json:18`；`:116` → `rules.json:19`；`:124` 的第二处 → `rules.json:19`）。改完重跑一次「每条 `file:line` 指向的行确实包含该标识符」的机械核对，避免同类偏移再次进入下一轮验收人的视野。

## Warnings

### WR-01: `02-VERIFICATION.md` 仍以现在时断言 Iceberg warehouse 是本地文件系统、MinIO 缺失

**File:** `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:37`、`:39`、`:215`、`:224`、`:234`、`:235`、`:236`、`:311`

**Issue:** 这些行把「Iceberg 只有 local warehouse、MinIO 路径 MISSING / BLOCKER」写成当前事实：

- `:37` `issue: "fs.local.enabled=true、local:///warehouse；无MinIO/S3 endpoint与warehouse"`
- `:39` `issue: "当前Iceberg fixture挂载本地warehouse，没有MinIO启动/验证路径"`
- `:215` `| MinIO/S3 warehouse | current local路径没有S3配置与MinIO服务 | MISSING（合同特定路径，BLOCKER） |`
- `:224` `| Iceberg行数 | … JDBC catalog+local文件 … | FLOWING（local，非MinIO） |`
- `:234` / `:235` `| Trino483 Iceberg local |` / `| Trino484 Iceberg local |`
- `:236` `| MinIO/S3 | 当前fixture无该路径，本轮未执行 | FAILED（实现/合同缺口） |`
- `:311` `**1个BLOCKER**：accepted ADR0017与02-10.6仍要求MinIO，而当前只有local warehouse。`

与当前仓库相反：`ci/trino/iceberg/iceberg.properties:9-13` 是 `default-warehouse-dir=s3://warehouse/`、`fs.s3.enabled=true`、`s3.endpoint=http://minio:9000`、`s3.path-style-access=true`、`s3.region=us-east-1`；`ci/iceberg-services.sh:431` 设置 `S3_ENDPOINT=http://minio:9000`；`ci/minio/Dockerfile` 与 `ci/minio/bucket.go` 存在。该文件 frontmatter 的 `source_head: f8851ca`（`:4`）早于 `da5b151`，`02-VALIDATION.md:89` 已明写「2026-11-04 本报告记录的『当前是 JDBC+local warehouse』是 `source_head f8851ca` 时的快照，已随 `da5b151` 失效」。但 `02-VERIFICATION.md` 本身从未标注为快照——它是下一轮 `/gsd-verify-work 02` 最先读的判定文件，若照字面采信，会重新推导出同一个已经关闭的 MinIO BLOCKER。

**Fix:** 把 `02-VERIFICATION.md` 的 `source_head` 推进到当前 HEAD 并重写上述 8 行为 MinIO/S3 现状 + 07.2 的 run `41833307824`；若不打算重跑判定，至少在这些行加上「本节为 `source_head f8851ca` 快照，已被 `02-VALIDATION.md:89` 与 `02-11-RECONCILIATION.md` A3 取代」的前置声明，并把 `:215` / `:236` 的 `MISSING` / `FAILED` 改为 `SUPERSEDED`。

### WR-02: `02-VERIFICATION.md` 的分数与 gap 状态自相矛盾（101/103 vs 100/103；partial vs FAILED (BLOCKER)）

**File:** `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:6`、`:33`、`:91`、`:194`、`:199`、`:311`

**Issue:** 同一文件给出两个互斥的分数与两种互斥的状态：

- frontmatter `:6` `score: 101/103 must-haves verified`，正文 `:199` `**Score:** 100/103`；而 `:91` 明写「共103条…1条FAILED、2条数据来源judgment为UNCERTAIN，其余100条含2 override计入verified」——按其自述 103 − 1 − 2 = 100，`:6` 的 101 无法成立。
- `gaps[0].status`（`:33`）是 `partial`，同一条 truth 在判定表 `:194` 是 `FAILED (BLOCKER)`，在 `:311` 又被计入「1个BLOCKER」。`partial` 与 `FAILED (BLOCKER)` 是两种不同的判定，会让下一轮 verifier 不知道该按「未闭合但部分成立」还是按「已判失败的阻塞项」处理。

**Fix:** 统一到一组数字与一组词：frontmatter `:6` 改为 `score: 100/103 must-haves verified`；`gaps[0].status` 与 `:194` / `:311` 统一为同一种判定（若保留 `FAILED (BLOCKER)` 则把 `:33` 改为 `failed`；若按 `02-VALIDATION.md:89` 认为 MinIO 缺口已由 07.2 关闭，则三处一并改为 `closed` 并引用 `41833307824`）。

### WR-03: `02-10-SUMMARY.md` 的 ignored / 本机测试计数与源码不符且内部加总不上

**File:** `.planning/phases/02-source-binding-engine/02-10-SUMMARY.md:14`、`:84`、`:88`

**Issue:** 两处计数与被审查的源码对不上：

- `:14` `trino_it（10 个 ignored + 2 个本机测试）`——`crates/ontoforge-engine/tests/trino_it.rs` 实际只有 **9** 个 `#[ignore]` 属性（`:134`、`:146`、`:229`、`:296`、`:427`、`:467`、`:552`、`:582`、`:625`），其中 `:625` 正是 Iceberg 用例 `iceberg_table_is_readable`。该数字在 SUMMARY 自身所在提交 `174b997d` 上也是 9，在当前 HEAD 上仍是 9。
- `:84` `（trino_it 9 + 1 Iceberg、cli_trino_it 1、private_fixtures 1）`——按字面读是 10 + 1 + 1 = 12，而 `:88` 报告 `cargo test --workspace --locked（573 passed, 0 failed, 11 ignored）`；实际总数 11 正确（9 + 1 + 1），说明「+ 1 Iceberg」把已经含在 9 里的 Iceberg 用例重复计了一次。
- `:14` `cli_trino_it（1 + 1）`——在 `174b997d` 上成立（1 ignored + 1 非忽略），但当前 `crates/ontoforge/tests/cli_trino_it.rs` 已是 **4 个 ignored（`:130`、`:187`、`:279`、`:365`）+ 3 个非忽略**（07.1 / 07.2 与 WR-C07 期间新增），属于陈旧快照。

**Fix:** `:14` 改为 `trino_it（9 个 ignored + 2 个本机测试）、cli_trino_it（4 + 3）、private_fixtures（1 + 2）`；`:84` 改为 `（trino_it 9 个 ignored，其中 1 个为 Iceberg 用例；cli_trino_it 1；private_fixtures 1）`，使加总与 `:88` 的 11 一致。

### WR-04: 两份产出记录的 `git_head_at_generation` 不是生成时的 HEAD

**File:** `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:4`、`.planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md:7`

**Issue:** 两处都记 `f2e73edce3e8d97934e1a7704db8a519b85a5f1d`，但那只是 plan 的基线 HEAD，不是生成时的 HEAD：

- `02-11-source-scan.txt:4` `# git HEAD: f2e73ed…`——该文件是 **Task 2** 的产出，Task 1 的提交 `a135a0e` 在它之前就已存在，生成时 HEAD 应为 `a135a0e`。
- `02-11-HUMAN-ACCEPTANCE.md:7` `git_head_at_generation: "f2e73ed…"`——该文件是 **Task 3** 的产出，生成时 HEAD 应为 `e915a34`（Task 2 的提交）。

对照：`02-11-RECONCILIATION.md:7` 记 `f2e73ed` 是对的（Task 1 确实在 `f2e73ed` 上执行）。这两处元数据的作用就是钉住证据对应的源码版本；钉错版本会让「证据与哪次提交对应」这一追溯链断在起点。（本轮不认为扫描内容因此失真：Task 1 / Task 2 只改动 `.planning/` 下的文档，源码树未变。）

**Fix:** `02-11-source-scan.txt:4` 改为 `a135a0e265828d770452939627494ae414481abe`；`02-11-HUMAN-ACCEPTANCE.md:7` 改为 `e915a349b9d7558a4a83550f15681ee1d2d3476f`。若希望两处都保留 plan 基线，改成 `plan_base_head: f2e73ed…` + `generated_at_head: <实际>` 两个字段，不要二选一。

### WR-05: 三处「（命令）」引用指向门控标题行而非 `COMMAND` 行

**File:** `.planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md:52`、`:70`、`:71`

**Issue:** 三条 `EVIDENCE` 把门控标题行标注成「（命令）」，实际命令在其下一行：

- `:52` `门控 C-1 在 \`:161\`（命令）、\`:165\`（\`RESULT: PASS\`）`——`02-11-source-scan.txt:161` 是标题 `GATE C-1 — 公开工作流不得引用私有 fixture 目标或私有环境变量`，`COMMAND:` 在 `:162`；`:165` 的 `RESULT: PASS` 正确。
- `:70` `门控 D-1（命令）与 \`:181\`（\`RESULT: PASS\`）`——D-1 的标题在 `:169`，首个 `COMMAND:` 在 `:170`；`:181` 的 `RESULT: PASS` 正确。
- `:71` `门控 D-2（命令）与 \`:191\`（\`RESULT: PASS\`）`——D-2 的标题在 `:185`，`COMMAND:` 在 `:186`；`:191` 的 `RESULT: PASS` 正确。

验收人按 `:161` / `:169` / `:185` 去找命令会落到标题行，需要多跳一行才能拿到可复核的命令原文。

**Fix:** 把 `:52` 的 `:161` 改为 `:162`，`:70` 的 `:169` 改为 `:170`，`:71` 的 `:185` 改为 `:186`；或者在扫描文件里给 `COMMAND:` 行也加锚点并同时引用两行。

### WR-06: `02-10-SUMMARY.md` 的 PR `integration` 路径描述已漂移，且未被 02-11 的偏差台账登记

**File:** `.planning/phases/02-source-binding-engine/02-10-SUMMARY.md:11`、`:143`、`:147`、`:161`、`:193`

**Issue:** 这些行描述的是 PR `integration` 作业与 Iceberg fixture 的当前形态，与仓库现状不符，且 02-11 的偏差台账没有覆盖：

- `:11` `postgres:17 service（发布 5432:5432）` 与 `:143` `\`integration\` 作业：\`services.postgres\` 用 \`postgres:17\`、健康检查与 \`ports: ["5432:5432"]\``——当前 `.github/workflows/ci.yml` 全文**没有 `services:` 块**（`rg -n "services" .github/workflows/ci.yml` 无命中）；PostgreSQL 由 `ci/query-services.sh:33-38` 的 `docker run -d --name postgres … -p 127.0.0.1:5432:5432 … postgres:17.11@sha256:2d2b8998…` 启动，即端口只绑回环（不是「发布」到所有接口），镜像标签是钉 digest 的 `17.11`（不是裸 `postgres:17`）。
- `:12`、`:147`、`:161`、`:193` 把 Iceberg warehouse 写成 Trino 本地文件系统（`fs.local.enabled`、`local:///tmp/iceberg`、`默认本地 warehouse`）——当前 `ci/trino/iceberg/iceberg.properties:9-13` 是 MinIO/S3。

关键点：`02-11-RECONCILIATION.md:47-57` 的 D-01..D-04 只登记了 **nightly.yml** 的迁移（D-01 原文即「`.github/workflows/nightly.yml` 已无 `services.postgres` 与 `5432:5432`」），PR `integration` 侧与 `ci/trino/iceberg/iceberg.properties` 的形态变化在整个 Phase 02 的偏差台账里查不到记录。

**Fix:** 在 `02-11-RECONCILIATION.md` 的 `## Implementation Path Deviations` 增补 D-05（`ci.yml` 的 `services.postgres` → `ci/query-services.sh postgres`，含 `ci/query-services.sh:33-38` 的 `file:line`）与 D-06（PR 侧 Iceberg warehouse 由 local 改 MinIO/S3，含 `ci/trino/iceberg/iceberg.properties:9-13`）；同时在 `02-10-SUMMARY.md` 的上述行加「已由 07.2 / `ci/query-services.sh` 取代」的标注。

### WR-07: `02-VERIFICATION.md:313` 与 `progress.md:20` 描述的 `02-VALIDATION.md` 状态是旧的

**File:** `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:313`、`.planning/phases/02-source-binding-engine/progress.md:20`

**Issue:**

- `:313` `02-VALIDATION现为validated、26 COVERED/1 PARTIAL/1文档SKIP、nyquist_compliant=false`——当前 `02-VALIDATION.md:14` 写的是「共 31 个 task：30 个 COVERED、1 个纯文档 SKIP」，任务表 31 行中 30 行含 `COVERED`、无一行标 `PARTIAL`（`rg -n "PARTIAL" 02-VALIDATION.md` 只命中 `:82` 的 Wave 0 勾选项，见 WR-09）。`26 COVERED/1 PARTIAL` 与现在的 `30 COVERED/0 PARTIAL/1 SKIP` 对不上。
- `progress.md:20` 把 `02-VALIDATION.md` 列为「后续项（不阻塞）」并写「`02-VALIDATION.md` 待 `/gsd-validate-phase 2` 回填」——但该文件已存在且 frontmatter 为 `status: validated` / `updated: 2026-11-08` / `source_head b61fe09`。这一行会让读状态的人以为验证还没跑。

**Fix:** `02-VERIFICATION.md:313` 改为「`02-VALIDATION` 现为 validated、31 task：30 COVERED / 1 文档 SKIP、nyquist_compliant=false」；`progress.md:20` 删除该条，或替换为指向当前 `02-VALIDATION.md` 的指针。

### WR-08: `02-10-SUMMARY.md` 的「首跑待确认」预测值与实际首跑记录矛盾

**File:** `.planning/phases/02-source-binding-engine/02-10-SUMMARY.md:155`

**Issue:** `:155` 写「`trino_it` 显示 `running 10 tests`（含 `iceberg_table_is_readable`，PR 中只检查环境变量）、`cli_trino_it` 显示 `running 1 test`，全部通过；数量门禁打印的两个计数（预期 10 与 1）均大于 0」，而 `02-UAT.md:18` 记录的实际首跑是「CI run 39219827829：trino_it 9 passed、cli_trino_it 2 passed」。两个数字都错：写的时候 `trino_it.rs` 就只有 9 个 `#[ignore]`（Iceberg 用例已含在内，见 WR-03）；`cli_trino_it` 的 1 是在 `02-REVIEW-FIX` 补上 WR-C07 用例之前的状态。该「待确认」清单在首跑实际发生后从未回填，读者把两份文档并排看会得到互相矛盾的门禁计数。

**Fix:** 把 `:155` 的预期值改写为实测值并指向 `02-UAT.md:18`（`trino_it` 9、`cli_trino_it` 2），或明确标注「本条已由 `02-UAT.md:18` 实测取代」。

### WR-09: Phantom requirement — `02-VALIDATION.md` 的 Wave 0 勾选项要求一个已不存在的 PARTIAL 分类

**File:** `.planning/phases/02-source-binding-engine/02-VALIDATION.md:82`

**Category:** necessity

**Issue:** `:82` `- [x] ci/trino/catalog、seed、nightly文件存在并接线；MinIO特定合同另列PARTIAL。` 的后半句要求把 MinIO 特定合同「另列 PARTIAL」。这个状态在本文件里已经不可能出现：`:89` 明写「`02-10-T3` 的 MinIO 合同缺口已关闭（源码侧）」，`:107` 重述「MinIO 合同缺口已由 `da5b151` 关闭」，任务表 31 行中没有任何一行是 `PARTIAL`（唯一的 `PARTIAL` 字样就是 `:82` 这一句本身）。它原本要覆盖的「当前只有 local warehouse」形态已由 `da5b151` 换成 MinIO/S3，该分支在当前仓库状态下已失效。

**Fix:** 删除 `:82` 的分句「；MinIO特定合同另列PARTIAL」，保留前半句（前半句仍成立）；若要保留 MinIO 的历史痕迹，改写为「MinIO 特定合同原列 PARTIAL，已由 `da5b151` 关闭（见 `## Gap and Manual Boundary`）」的陈述句，而不是继续作为一条待满足的验收要求。

## Info

### IN-01: 门禁编号误标：「13 条 G-A1..G-A5」与未定义的「G-A2b」

**File:** `.planning/phases/02-source-binding-engine/02-VALIDATION.md:72`、`:89`、`.planning/phases/02-source-binding-engine/02-11-SUMMARY.md:64`

**Issue:**

- `02-VALIDATION.md:72` 「本轮 13 条 G-A1..G-A5 结构门禁全过」与 `02-11-SUMMARY.md:64` 「每条配一条可失败只读门禁 G-A1..G-A5，本轮 13 条门禁全部通过」把两类门禁混为一谈。`02-11-PLAN.md` Task 1 的 `<verify>` 共 13 条（`:185`、`:187`、`:189`、`:191`、`:193`、`:195`、`:197`、`:199`、`:201`、`:203`、`:205`、`:207`、`:209`），其中只有前 9 条属于 G-A1..G-A5，另外 4 条是 `T1-evi` / `G-PROD` / `G-DOCS` / `G-OWNED`——`02-VALIDATION.md:28` 自己枚举得是对的。
- 「G-A2b」（`02-VALIDATION.md:72`、`:89`）在 `02-11-PLAN.md` 与 `02-11-RECONCILIATION.md` 中都没有定义。对应的事实是 `02-11-PLAN.md:187` 的 `bash -c '! rg -q "fs\.local\.enabled|local:///warehouse" ci/trino/iceberg/iceberg.properties'`，它是 G-A2 块（`02-11-RECONCILIATION.md:36`）里的第二条断言，但从未被命名为独立门禁。

**Fix:** `:72` 改为「本轮 13 条门禁全过（G-A1..G-A5 九条结构门禁 + `T1-evi` / `G-PROD` / `G-DOCS` / `G-OWNED`）」，`02-11-SUMMARY.md:64` 同步；把「G-A2b」改称「G-A2 的第 2 条断言（`02-11-PLAN.md:187`）」，或在 `02-11-PLAN.md` 中把它正式登记为 G-A2b。

### IN-02: 「plan 起始 HEAD」在同一 plan 的两份产出里取值不同

**File:** `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md:8`、`:23`、`.planning/phases/02-source-binding-engine/02-11-SUMMARY.md:37`

**Issue:** `02-11-RECONCILIATION.md:8` 写 `plan_start_head: "9a6eaffe060e93df1c19c79c7b25d3946a7d3229"`，`:23` 重述「本 plan 的起始 HEAD 为 `9a6eaffe…`」；而 `02-11-SUMMARY.md:37` 写 `plan_head_before: "f2e73edce3e8d97934e1a7704db8a519b85a5f1d"`。`git merge-base --is-ancestor 9a6eaffe f2e73ed` 成立，即 `9a6eaffe` 是 `f2e73ed` 的祖先，plan 实际是在 `f2e73ed` 上起草并执行的。`9a6eaffe` 在 `02-11-PLAN.md:207` / `:336` 只被用作 `docs/` 差异门禁的基线——那是一个更宽（更保守）的比对起点，不是 plan 的起始 HEAD。

**Fix:** 把 `02-11-RECONCILIATION.md:8` 的字段改名为 `docs_diff_baseline`（或 `gate_baseline`），并在 `:23` 写成「plan 起始 HEAD = `f2e73ed`；`docs/` 差异门禁基线 = `9a6eaffe`」。

### IN-03: `commits: 3` 的「实测」方法在文中给出的时点复现不出 3

**File:** `.planning/phases/02-source-binding-engine/02-11-SUMMARY.md:36`、`:42`、`:49`

**Issue:** `:49` 写「`commits: 3` 为本 plan 的三个 task 提交数…由持久化 ledger `git rev-list --count f2e73ed..HEAD` 实测」。该命令只有在以 SUMMARY 自身所在提交 `73cfb2f` 为 HEAD 时才返回 3（`git rev-list --count f2e73ed..73cfb2f` → 3）；以同一段给出的 `plan_head_after`（`e915a34`，`:38`）为 HEAD 时返回 2，以当前分支 HEAD 时返回 8。此外，仓库中不存在可核对的「持久化 ledger」产物：`.planning/` 下没有名为 ledger 的文件，`.planning/state.json` 的键为 `contract` / `flavor` / `milestone` / `next` / `phases` / `updated_at`，无 ledger 项。数字本身对得上（三个 task 提交 `a135a0e` / `e915a34` / `73cfb2f`），但出处写法无法复核。

**Fix:** 改为可复现的形式「`git rev-list --count f2e73ed..73cfb2f` = 3（`a135a0e` / `e915a34` / `73cfb2f`）」，并删去「持久化 ledger」这一指向不存在的引用；若确有外部 ledger，请写明其路径。

---

_Reviewed: 2026-11-08T09:07:18Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
