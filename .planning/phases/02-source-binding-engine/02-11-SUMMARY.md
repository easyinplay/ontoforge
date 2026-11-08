---
phase: "02"
plan: "11"
subsystem: source-binding-engine
type: evidence
status: complete
wave: 6
gap_closure: true
tags: [reconciliation, evidence, human-acceptance, gap-closure, no-product-code-change]
requires: [02-10, 07.2-minio-contract-gap-closure]
provides:
  - "G-02-A 的源码级调和证据（02-11-RECONCILIATION.md）"
  - "G-02-B / G-02-C / G-02-D 的机器可判定部分原始输出（02-11-source-scan.txt）"
  - "G-02-B / G-02-C / G-02-D 的人工验收问题清单（02-11-HUMAN-ACCEPTANCE.md）"
affects: [02-VERIFICATION.md, /gsd-verify-work-02]
tech-stack:
  added: []
  patterns:
    - "只读源码核验 + 可失败结构门禁（G-A1..G-A5 / C-1 / D-1 / D-2）"
    - "机器可判定部分与人工验收部分显式切分，结论位固定为 VERDICT: PENDING"
key-files:
  created:
    - .planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md
    - .planning/phases/02-source-binding-engine/02-11-source-scan.txt
    - .planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md
    - .planning/phases/02-source-binding-engine/02-11-SUMMARY.md
  modified: []
decisions:
  - "G-02-B / G-02-C / G-02-D 的结论一律保持 VERDICT: PENDING，不因机器门禁通过而代签"
  - "标识符词表按实测的 45 个登记（plan 基线 23 个的严格超集），偏差如实登记而非静默收敛"
metrics:
  duration: "约 40 分钟（2026-11-08）"
  completed: 2026-11-08
  tasks: 3
  files: 4
  commits: 3
  plan_head_before: "f2e73edce3e8d97934e1a7704db8a519b85a5f1d"
  plan_head_after: "e915a349b9d7558a4a83550f15681ee1d2d3476f"
actuals:
  tokens: 9635
  tasks: 3
  commits: 3
---

# Phase 02 Plan 11: 残余 gap 闭合（证据 + 人工验收清单） Summary

**一句话**：把 Phase 02 遗留的 4 个残余项拆成「G-02-A 用当前源码重新核验并留下带 `file:line` 的调和证据」与「G-02-B / G-02-C / G-02-D 转写成机器可判定的原始输出 + 结论待人工填写的验收问题」，全程零产品代码变更、零外部连接、零伪造闭合。

> `commits: 3` 为本 plan 的三个 task 提交数（Task 1 / Task 2 / Task 3），由持久化 ledger `git rev-list --count f2e73ed..HEAD` 实测。`plan_head_after` 取 SUMMARY 生成时刻的分支 HEAD（即 Task 2 提交），SUMMARY 自身所在的 Task 3 提交在此之后产生。

## 产出索引

| 文件 | 作用 |
|---|---|
| `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md` | G-02-A 的调和证据：五个要素 A1..A5 的逐条判定表（每条带 `file:line` 与可失败门禁 G-A1..G-A5）、与 02-10 计划文本的偏差 D-01..D-04、07.2 交叉引用与边界声明、45 词中性词表、本轮未执行清单 |
| `.planning/phases/02-source-binding-engine/02-11-source-scan.txt` | 一次只读扫描的原始输出：五个规定分段（IDENTIFIERS / ENDPOINTS / PRIVATE PATH GATE / PROVENANCE BOUNDARY / NOT SCANNED）+ `## SUSPECTED NON-NEUTRAL`（空）+ 标识符与端点的逐条 `file:line` 来源 |
| `.planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md` | G-02-B / G-02-C / G-02-D 的人工验收问题清单：每条四字段（`QUESTION:` / `EVIDENCE:` / `NOT VERIFIABLE BY MACHINE:` / `VERDICT: PENDING`），共 9 条带 `file:line` 的证据引用 |
| `.planning/phases/02-source-binding-engine/02-11-SUMMARY.md` | 本文件：产出索引与四个 gap 的状态声明 |

## 四个 gap 的最终状态

| Gap | 对应禁止项 | 状态 | 说明 |
|---|---|---|---|
| **G-02-A** | 02-10.6 nightly Iceberg 合同 | **已调和**（待 verifier 重新判定） | `02-11-RECONCILIATION.md` 用当前源码（HEAD `f2e73ed`）逐条重核 02-10.6 的五个要素：A1 483/484 矩阵与两条镜像 digest（PASS）、A2 JDBC catalog 六键（PASS）、A3 MinIO 自建镜像与 warehouse、无本地回退（PASS）、A4 `report-failure` 三步行为（REFINED，accepted by 07.2）、A5 权限范围（PASS）。每条配一条可失败只读门禁 G-A1..G-A5，本轮 13 条门禁全部通过 |
| **G-02-B** | 02-03 公开数据来源 | **未关闭** | 仅完成机器可判定部分（45 个物理标识符全为中性词，`## SUSPECTED NON-NEUTRAL` 为空）与人工验收问题准备。结论 `VERDICT: PENDING` |
| **G-02-C** | 02-10 公开 CI 不接公司源 | **未关闭** | 仅完成机器可判定部分（7 条 `jdbc:` / `s3://` 端点串全部指向 loopback / 容器内 `minio` / 仓库内桶前缀；门控 C-1 PASS）与人工验收问题准备。结论 `VERDICT: PENDING` |
| **G-02-D** | 02-10.8 / ENGINE-05 私有 SQL Server 边界 | **未关闭** | 仅完成机器可判定部分（门控 D-1 环境变量 + `#[ignore]` 双门控 PASS、门控 D-2 贡献文档 PASS）与人工验收问题准备。结论 `VERDICT: PENDING` |

**G-02-B / G-02-C / G-02-D 未关闭，仅完成机器可判定部分与人工验收问题准备。** 本 plan 不声称这三项已关闭，也不因门禁通过而把它们吸收为 closed。

## 本 plan 未引入任何产品代码变更

`crates/`、`ci/`、`.github/`、`examples/`、`docs/` 下的任何**已跟踪**文件在本 plan 全程未被修改。三条门禁在三个 task 结束时分别实测通过：

- `test -z "$(git status --porcelain -- crates ci .github examples)"` — 空
- `test -z "$(git diff 9a6eaffe060e93df1c19c79c7b25d3946a7d3229 --name-only -- docs)"` — 空（该比对按设计只覆盖已跟踪文件）
- `test -z "$(git diff --name-only HEAD -- .planning/REQUIREMENTS.md .planning/ROADMAP.md .planning/phases/02-source-binding-engine/02-VERIFICATION.md .planning/phases/02-source-binding-engine/02-UAT.md .planning/phases/02-source-binding-engine/02-CONTEXT.md .planning/phases/02-source-binding-engine/02-RESEARCH.md .planning/phases/02-source-binding-engine/02-PATTERNS.md)"` — 空

本 plan 只新建上述 4 个 `.planning/phases/02-source-binding-engine/` 下的文件。

## 本轮未执行的动作

与 `02-11-RECONCILIATION.md` 的 `## This Round Did Not Execute` 一致：

- 没有启动 Docker / 任何容器，没有拉取任何镜像
- 没有运行 `ci/iceberg-services.sh`（任何模式）
- 没有触发、重跑或以任何方式干预 GitHub nightly / ci / release workflow
- 没有连接任何 Trino（含 483 / 484）、MinIO、PostgreSQL、SQL Server 实例
- 没有连接任何公司环境或私有环境，没有读取 `ONTOFORGE_PRIVATE_FIXTURES` 指向的仓库外目录
- 没有修改 git 历史（无 amend / rebase / reset / tag / push）
- 没有修改 `02-VERIFICATION.md`、`02-UAT.md`、`02-CONTEXT.md`、`02-RESEARCH.md`、`02-PATTERNS.md`、`.planning/REQUIREMENTS.md`、`.planning/ROADMAP.md`
- 没有新增任何依赖，没有执行任何包管理器安装

唯一执行的实际命令是仓库已锁定的 `cargo test -p ontoforge --test private_fixtures --locked --offline`（`--locked` 保证 `Cargo.lock` 未被改动，`--offline` 保证不触网），结果 `test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out`；被忽略的正是需要私有 fixture 的在线用例 `private_fixtures_validate_online`。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修正词表条目计数**
- **Found during:** Task 2 写入扫描文件后复核
- **Issue:** 首次在扫描文件与调和证据中把「相对 plan 基线新增的词」写成 23 个，但 45 − 23（基线）= 22，条目清单中误含基线内的 `other`。
- **Fix:** 两处清单删去 `other`，计数改为 22。
- **Files modified:** `02-11-source-scan.txt`、`02-11-RECONCILIATION.md`
- **Commit:** `e915a34`

### 计划与实测的差异（如实登记，非本 plan 的改动）

**1. 中性词表实测结果是 plan 基线的严格超集**
- `02-11-PLAN.md` 的基线列出 23 个词（catalog 4、schema 2、table 12、哑元 5）。一次只读扫描实测出 45 个，额外 22 个全部来自 `ci/trino/l2/rules.json`、`ci/trino/l2/seed.sql`、`ci/query-services.sh:81`、`ci/trino/catalog/tpch.properties`、`ci/pg/seed.sql` 的视图与 Rust 测试夹具，均为公开通用名或哑元，`## SUSPECTED NON-NEUTRAL` 仍为空。
- 基线的 23 个词在最终词表中全部在位，无遗漏。已在 `02-11-RECONCILIATION.md` 的 `## Machine-Checkable Portions of G-02-B / G-02-C / G-02-D` 节与本文件登记。

**2. `02-11-PLAN.md` 的登记口径未覆盖全部来源位置**
- plan 只列出「`catalog:` / `schema:` / `table:` 键名的取值」与「绑定 YAML 中的三元组位置」。实测中 `ci/trino/l2/rules.json` 的 JSON 键、`ci/pg/seed.sql` 与 `ci/trino/l2/seed.sql` 的 `CREATE SCHEMA/TABLE/VIEW`、`.properties` 的 `catalog-name` 与库名、`ci/trino/catalog/<name>.properties` 的 catalog 名同样是 catalog / schema / table 三元组位置。扫描文件头部把完整登记口径写死（6 条），并据此取全量。
- 后果是词表比 plan 基线多，但不产生任何非中性词。

**3. 执行环境与 plan 描述的分支名不一致**
- plan 的「已知的工作树基线」写的是分支 `agent-quick-261107-q0k-phase-1`、HEAD `9a6eaffe060e93df1c19c79c7b25d3946a7d3229`；实际执行时分支为 `gsd/phase-02-source-binding-engine`，HEAD 为 `f2e73edce3e8d97934e1a7704db8a519b85a5f1d`（含 3 个计划文档提交）。
- 影响：无。plan 的门禁以 `9a6eaffe` 为固定比对基点，产品目录在本 plan 开始前已无改动，门禁实测仍为空。

**4. `gsd_run --version` 不受支持**
- 该 CLI 无 `--version` 标志（报 `Unknown flag: --version`）。改用其 `query` 子命令调用，不影响任何门禁。

## 未采纳 plan 预期但成立的事实

- plan 预期 `## SUSPECTED NON-NEUTRAL` 为空——实测确为空。
- plan 预期「容器编排已迁到 `ci/iceberg-services.sh`」——实测确认为该形态，`nightly.yml` 中 `services.postgres` 与 `5432:5432` 均无匹配。
- plan 预期 `iceberg.properties` 无本地文件系统回退——实测 `fs.local.enabled` / `local:///warehouse` 无匹配。

## Commits

| Commit | 说明 |
|---|---|
| `a135a0e` | `docs(02-11): G-02-A 调和证据，逐条 file:line 重核 02-10.6 nightly Iceberg 合同` |
| `e915a34` | `docs(02-11): G-02-B/C/D 机器可判定部分落盘只读扫描原始输出` |
| （本 SUMMARY 所在提交） | `docs(02-11): G-02-B/C/D 转写为人工验收清单并交付 SUMMARY` |

## 下一步

1. `/gsd-verify-work 02` 消费 `02-11-HUMAN-ACCEPTANCE.md`：向人提出 G-02-B / G-02-C / G-02-D 的三个问题，取得「是 / 否 / 不适用 + 理由」的答复后，把三处 `VERDICT: PENDING` 替换为接受或拒绝结论并登记到 `02-VERIFICATION.md` 的 `unverified_prohibitions`。
2. 同一轮 `/gsd-verify-work 02` 用 `02-11-RECONCILIATION.md` 重新判定 `02-VERIFICATION.md:194` 的 `02-10.6 | FAILED (BLOCKER)` 一行。
3. 07.2 的 `passed` 只覆盖 MinIO/S3 + PostgreSQL JDBC Iceberg 子范围，**不闭合** ENGINE-05 的私有 SQL Server 边界（`07.2-VERIFICATION.md:136`、`:174`）。

## Self-Check

- [x] `02-11-RECONCILIATION.md` 存在，含六个规定节
- [x] `02-11-source-scan.txt` 存在，含五个规定分段与两个定界标记
- [x] `02-11-HUMAN-ACCEPTANCE.md` 存在，三个子节各四字段，`VERDICT: PENDING` 恰好 3 处
- [x] `02-11-SUMMARY.md` 存在
- [x] 扫描文件与调和证据的标识符集合双向 diff 为空
- [x] `cargo test -p ontoforge --test private_fixtures --locked --offline` 通过（2 passed / 0 failed / 1 ignored）
- [x] 三个 commit 均存在于本分支 HEAD 的祖先链上
- [x] 产品目录与被其他流程拥有的文件零改动

## Self-Check: PASSED
