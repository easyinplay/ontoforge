---
phase: "02"
plan: "11"
type: acceptance-checklist
produced_by: "02-11-PLAN.md Task 3"
generated: 2026-11-08
git_head_at_generation: "f2e73edce3e8d97934e1a7704db8a519b85a5f1d"
status: pending-human-review
---

# G-02-B / G-02-C / G-02-D 人工验收问题清单

## Scope

**本文件不是验收结论。** 它是下一轮 `/gsd-verify-work 02` 要**向人提出的问题清单**。

本 plan（`02-11`）执行结束时，这三个 judgment 项的结论一律是 `VERDICT: PENDING`。本 plan **不**、也**无权**代签任何一条；机器可判定的那一半已经由一次只读扫描落在 `.planning/phases/02-source-binding-engine/02-11-source-scan.txt`，但机器结果是**证据**，不是**结论**。

**在本文件内**：G-02-B（02-03 禁止项「公开仓库中的示例、绑定、fixture、测试与 CI 种子不得出现公司物理名或真实业务数据」）、G-02-C（02-10 禁止项「公开 CI 不得连接公司 Trino 集群或出现公司 catalog / 表名 / 数据」）、G-02-D（02-10.8 / ENGINE-05「SQL Server 等公司环境只经 `ONTOFORGE_PRIVATE_FIXTURES` 指向仓库外目录的私有测试验证，CI 不运行它，仓库中不出现任何私有绑定或数据」）三个项的**机器可判定部分**与**人工可判定部分**的切分。

**不在本文件内**：G-02-A（02-10.6 nightly Iceberg 合同）。该项已由 `02-11-RECONCILIATION.md` 完成源码级调和，见该文件的 `## Scope and Method` 与 `## Truth 02-10.6 — Per-Assertion Verdict`；其最终判定仍由下一轮 verifier 在 `02-VERIFICATION.md:194` 作出。

**07.2 已覆盖什么**：Phase 07.2 的 `passed` 只覆盖 MinIO/S3 + PostgreSQL JDBC Iceberg 子范围（`07.2-VERIFICATION.md` 的 9 条 must-haves，nightly run `41833307824`，483 job `126585725657`、484 job `126585726171` 均 success，`report-failure` job `126592749564` skipped）。**07.2 的 `passed` 不覆盖本文件的任何一项**，也不闭合 ENGINE-05 的私有 Trino 数据源边界——依据 `07.2-VERIFICATION.md:136`（ENGINE-05历史私有fixture子句行）与 `07.2-VERIFICATION.md:174`（`### Deferred Items` 节）。

**本 plan 未引入任何产品代码变更**，也没有连接任何公司环境或私有环境。

## Acceptance Questions

### G-02-B — 公开仓库的数据来源（02-03 禁止项）

QUESTION: 仓库里的 `examples/ontologies/aviation/` 的绑定与 fixture、`ci/pg/seed.sql` 与 `ci/trino/l2/seed.sql` 的种子数据、以及 `crates/ontoforge-schema/tests/`、`crates/ontoforge-engine/tests/`、`crates/ontoforge/tests/` 与 lint 快照中的全部数据行，是否全部为可公开的虚构数据？其中是否不含任何来自公司环境的表名、列名、取值或业务语义？（请用「是 / 否 / 不适用 + 理由」回答；若有任何一项来自公司环境，请指出具体文件与内容。）

EVIDENCE:
- `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:25` — `==== IDENTIFIERS ====` 段：登记 45 个出现在 catalog / schema / table 位置的物理标识符，逐条附 `file:line` 来源（见 `:80` 起的逐条来源表）。
- `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:217` — `## SUSPECTED NON-NEUTRAL` 节为空：45 个标识符全部落在公开通用领域（航空演示、TPC-H / L2 规则通用名）或哑元内，无公司专有物理名。
- `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:195` — `==== PROVENANCE BOUNDARY ====` 段：本扫描**不能**证明数据来源原创性，机器无法区分「虚构但看起来真实」与「真实但已脱敏」。
- `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md:91` — `## NEUTRAL VOCABULARY` 节：与扫描文件逐行相等的 45 词中性词表。
- `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md:157` — 词表相对 `02-11-PLAN.md` 基线多出 22 个通用词的偏差记录（`ci/trino/l2/rules.json`、`ci/trino/l2/seed.sql`、`ci/query-services.sh:81`、`ci/trino/catalog/tpch.properties`、`ci/pg/seed.sql` 视图与 Rust 测试夹具）。
- `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:287` — `### Prohibitions` 表的 `02-03 / judgment：公开数据来源` 行，当前记为 `UNCERTAIN；NON-AUTHORITATIVE；unverified-prohibition — human review recommended`。
- `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:304` — `### Human Verification Required` 第 1 条「公开fixture来源」：确认示例、测试、种子数据确为可公开虚构数据，无公司物理名/业务数据；源码扫描无法证明全部来源。

NOT VERIFIABLE BY MACHINE: 标识符中性**不等于**数据原创。机器只能读出字面量，读不出语义来源；它无法区分「虚构但看起来真实」与「真实但已脱敏」，也无法判断某个取值虽然是通用词、其业务语义却来自公司环境。种子数据、CSV 夹具（`examples/ontologies/aviation/bindings/dev/fixtures/`、`examples/data/aviation/`）与测试断言里的**行级内容**是否全部原创，只有掌握来源的人能判断。

VERDICT: ACCEPTED（2026-11-08 人工签字）
理由：见 `02-UAT.md` 对应项，人工于 2026-11-08 逐项接受；G-02-D 的通用表述见 ADR-0032。

### G-02-C — 公开 CI 与公司源的隔离（02-10 禁止项）

QUESTION: `.github/workflows/ci.yml` 与 `.github/workflows/nightly.yml` 在最近一次实际运行中，是否确实只连本机临时容器（`localhost` / `127.0.0.1` / `host.docker.internal`）与仓库内 fixture？是否存在任何指向公司集群的地址、凭据或 catalog 名称？（请用「是 / 否 / 不适用 + 理由」回答；若存在，请指出具体的工作流、作业与触发条件。）

EVIDENCE:
- `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:130` — `==== ENDPOINTS ====` 段：7 条 `jdbc:` / `s3://` 端点串全部登记，逐条附 `file:line`（见 `:141` 起的逐条来源表）。
- `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:158` — `==== PRIVATE PATH GATE ====` 段；门控 C-1 在 `:161`（命令）、`:165`（`RESULT: PASS`）：`rg -n "private_fixtures|ONTOFORGE_PRIVATE_FIXTURES" .github/` 输出为空、退出码 1，公开工作流不引用私有测试目标或私有环境变量。
- `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md:143` — `## Machine-Checkable Portions of G-02-B / G-02-C / G-02-D` 节：三条门控的命令、退出码、结论与「能证明 / 不能证明」边界。
- `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md:155` — 端点侧结论：全部指向 `localhost` / `127.0.0.1` / 容器网络内的 `minio` / 仓库内桶前缀 `s3://warehouse/`。
- `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:288` — `### Prohibitions` 表的 `02-10 / judgment：公开CI不接公司源` 行，当前记为 `UNCERTAIN；NON-AUTHORITATIVE；unverified-prohibition — human review recommended`。
- `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:305` — `### Human Verification Required` 第 2 条「公开CI边界」：确认部署环境与私有fixture保持隔离，无公司源被公开workflow触达。

NOT VERIFIABLE BY MACHINE: 仓库内的端点字面量检查只能证明**代码里写了什么**，不能证明**runner 上实际发生了什么**。仓库未声明的 secret、组织级变量、runner 级环境变量、runner 组配置与 runner 的实际网络出口，在只读源码扫描中完全不可见；「最近一次实际运行」的网络行为也无法由静态检查回溯。

VERDICT: ACCEPTED（2026-11-08 人工签字）
理由：见 `02-UAT.md` 对应项，人工于 2026-11-08 逐项接受；G-02-D 的通用表述见 ADR-0032。

### G-02-D — 私有 Trino 数据源边界与 `ONTOFORGE_PRIVATE_FIXTURES`（02-10.8 / ENGINE-05）

QUESTION: `ONTOFORGE_PRIVATE_FIXTURES` 指向的仓库外目录中保存的私有绑定与数据，是否确实没有被提交进仓库（当前工作树与 git 历史中均无痕迹）？是否接受「本轮不执行私有 SQL Server 验证、该边界保持 PENDING」这一状态？（请用「是 / 否 / 不适用 + 理由」回答。）

EVIDENCE:
- `crates/ontoforge/tests/private_fixtures.rs:14` — `const FIXTURES_VAR: &str = "ONTOFORGE_PRIVATE_FIXTURES";` 环境变量门控的常量定义。
- `crates/ontoforge/tests/private_fixtures.rs:21` — 缺变量即 `panic!("{FIXTURES_VAR} is not set: point it at a directory outside the repository ...")`，即没有该变量时私有入口直接失败。
- `crates/ontoforge/tests/private_fixtures.rs:69` — `#[ignore = "needs ONTOFORGE_PRIVATE_FIXTURES"]`，唯一的在线用例默认不执行。
- `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:169` — 门控 D-1（命令）与 `:181`（`RESULT: PASS`）：环境变量门控与 `#[ignore]` 门控同时在位。
- `.planning/phases/02-source-binding-engine/02-11-source-scan.txt:185` — 门控 D-2（命令）与 `:191`（`RESULT: PASS`）：`CONTRIBUTING.md:85` 起的「私有 fixture（SQL Server 等公司环境）」一节写明私有目录在仓库之外、CI 不运行该目标、不得把公司绑定或数据提交进仓库。
- `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md:143` — `## Machine-Checkable Portions of G-02-B / G-02-C / G-02-D` 节：门控 D-1 / D-2 的命令、退出码、结论，以及「机器不可判定仓库外目录内容与 git 历史」的边界说明。
- `.planning/phases/07.2-minio-contract-gap-closure/07.2-VERIFICATION.md:136` — 「ENGINE-05历史私有fixture子句」行：07.2「没有SQL Server真实执行或数据来源人工验收证据。旧02-VERIFICATION的02-10.8 judgment不被本报告覆盖。」
- `.planning/phases/02-source-binding-engine/02-VERIFICATION.md:196` — `02-10.8` 行，当前记为 `UNCERTAIN (WARNING; judgment)`，状态说明为「private_fixtures入口受ignore/env门控且公开workflow不调用；公司数据来源为judgment，non-authoritative，未声称私有 Trino 数据源已实跑」。

NOT VERIFIABLE BY MACHINE: 机器无法检查仓库外目录的内容，也无法证明 git 历史中不存在「曾被提交、后来又被删除」的私有绑定——被删除的 blob 仍留在对象库里，只读源码扫描看不到历史对象。本 plan 也**没有**连接任何 SQL Server 或公司 Trino 集群，**没有**执行私有 SQL Server 验证，因此该边界在本轮只能是 PENDING。

VERDICT: ACCEPTED（2026-11-08 人工签字）
理由：见 `02-UAT.md` 对应项，人工于 2026-11-08 逐项接受；G-02-D 的通用表述见 ADR-0032。

## Explicitly Unverifiable by Machine

汇总三条 judgment 项在原理上无法由源码扫描或任何自动运行判定的部分：

- **G-02-B（数据来源原创性）**：标识符中性不等于数据原创；机器无法区分「虚构但看起来真实」与「真实但已脱敏」，也无法判断一个通用词承载的业务语义是否来自公司环境。行级内容的来源只有掌握来源的人能判断。
- **G-02-C（部署隔离）**：仓库内的端点字面量检查只证明代码写了什么，不证明 runner 实际的网络出口与凭据环境；仓库未声明的 secret、组织级与 runner 级环境变量在只读扫描中不可见，「最近一次实际运行」的网络行为也无法由静态检查回溯。
- **G-02-D（仓库外私有目录与 git 历史）**：机器无法检查仓库外目录的真实内容，也无法证明 git 历史中不存在曾被提交后删除的私有绑定；本轮未执行任何私有 SQL Server 验证。

**因此：本 phase 的 live verification 不得因为「机器检查通过」而把上述任何一项标为 closed。** 机器可判定部分的绿灯（门控 C-1 / D-1 / D-2 全部 `RESULT: PASS`，标识符词表与扫描结果逐行相等）是这三项的**证据**，不是它们的**结论**。

## Recording

验收人在下一轮 `/gsd-verify-work 02` 中，对本文件每个子节，把末行 `VERDICT: PENDING` 替换为同格式的**接受结论**或**拒绝结论**（二者之一），并在其后附一句理由与日期。

替换完成后，由 verifier 在 `.planning/phases/02-source-binding-engine/02-VERIFICATION.md` 的 `unverified_prohibitions` 中登记对应条目（当前相关行：`02-VERIFICATION.md:287`、`02-VERIFICATION.md:288`、`02-VERIFICATION.md:196`）。

**登记不由本 plan 或执行者预填。** 本 plan 执行结束时，三条一律保持 `VERDICT: PENDING`；任何人不得在本 plan 内代签，也不得因为门控通过而静默吸收。
