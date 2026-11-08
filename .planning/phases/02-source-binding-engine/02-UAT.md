---
status: testing
phase: 02-source-binding-engine
source: [02-VERIFICATION.md]
started: 2026-10-17T00:00:00Z
updated: 2026-11-08T09:20:00Z
---

## Current Test

number: 9
name: "G-02-D：私有 SQL Server 边界与仓库外目录"
expected: |
  人工核对后对断言作答「是 / 否 / 不适用 + 理由」：SQL Server 只经仓库外私有 fixture 验证、公开 CI 不运行该目标、仓库内无任何私有绑定或数据。
  机器已证部分（不必重复核对）：门控 D-1（`crates/ontoforge/tests/private_fixtures.rs:14/44/69` 的环境变量 + `#[ignore]` 双门控）与门控 D-2（`CONTRIBUTING.md:89/91` 约定）均 PASS；见 `02-11-HUMAN-ACCEPTANCE.md` G-02-D 节。
  人工要看的范围：`ONTOFORGE_PRIVATE_FIXTURES` 指向的仓库外目录当前实际内容（是否含公司绑定数据）、该入口的实际部署形态是否与 `CONTRIBUTING.md` 约定一致、git 历史中是否曾提交后删除私有绑定。
awaiting: user response

## Tests

### 1. PR `integration` 作业首跑
expected: `trino_it` 与 `cli_trino_it` 的 `--ignored` 测试实际执行并全部通过
result: pass
note: "CI run 39219827829：trino_it 9 passed、cli_trino_it 2 passed（首跑 39218898185 因 seed.sql 引号 bug 失败，修复 a4fdfea）"

### 2. 首次 nightly（Trino 483、484 矩阵）
expected: 手动触发 nightly，两个版本的 Iceberg 作业都通过
result: pass
note: "首跑 run 39220997866 失败（iceberg.properties 的 local.location 写成 URI，Trino 无法启动，issue #3）；修复后 run 39221684753 中 483 / 484 各 Engine 9 passed、CLI 2 passed"

### 3. E21 golden 定稿
expected: 按首跑日志修正 `describe_types_golden` 与 `decode_golden`，删除 `crates/ontoforge-engine/tests/trino_it.rs` 中的 `PROVISIONAL` 注释并提交
result: pass
note: "两个 golden 在首跑原样通过，PROVISIONAL 标记已移除"

### 4. WR-C06 镜像 digest 可拉取
expected: CI 中按 digest 钉住的 `postgres:17.11` 与 `trinodb/trino:483/484` 均拉取成功
result: pass
note: "run 39219827829 按 digest 拉取 postgres / trino 镜像成功"

### 5. WR-C07 真实 Trino 上的 `--check-pk`
expected: `check_pk_finds_duplicate_and_null_keys_on_ci_trino` 在 CI 中通过
result: pass
note: "check_pk_finds_duplicate_and_null_keys_on_ci_trino ok（run 39219827829）"

### 6. WR-C03 决策
expected: 决定是否修订 ADR-0021 第 7 条（推荐：带 fixture 的绑定映射到 Trino 时降为 Info），并按决定落地
result: pass
note: "选定方案 (a)：ADR-0022 修订 ADR-0021 第 7 条；落地 commit 4d681aa / 05362e5 / 4950d00"

### 7. G-02-B：公开仓库数据来源
expected: |
  核对公开仓库的示例 ontology、绑定文件、CI 种子与测试夹具，确认其数据（不只是标识符）确为可公开的虚构数据，不含任何公司业务数据
result: pass
why_human: "标识符中性是必要条件而非充分条件；机器无法判定数据行的来源与授权"
note: "人工接受（2026-11-08）：公开仓库的示例 ontology、绑定文件、CI 种子与测试夹具数据均为可公开虚构数据，不含公司业务数据"
evidence: "机器可判定部分已落盘：`02-11-source-scan.txt` 的 45 个物理标识符全为中性词、`## SUSPECTED NON-NEUTRAL` 为空；问题与证据见 `02-11-HUMAN-ACCEPTANCE.md` G-02-B 节（结论位仍为 `VERDICT: PENDING`）"

### 8. G-02-C：公开 CI 与公司源的隔离
expected: |
  核对公开 CI 与公司源的隔离，确认部署环境、runner 配置与私有 fixture 目录均未被公开 workflow 触达
result: pass
why_human: "workflow 文本可证明「未引用」，不能证明 runner 上的环境变量注入、secret 与网络出口"
note: "人工接受（2026-11-08）：公开 workflow 不连接公司 Trino 集群、不读取公司 catalog / 表名 / 数据"
evidence: "机器可判定部分已落盘：门控 C-1（`.github/` 内无 `private_fixtures` / `ONTOFORGE_PRIVATE_FIXTURES` 引用）PASS，7 条 `jdbc:` / `s3://` 端点串全指向 loopback / 容器内 `minio` / 仓库内桶前缀；见 `02-11-HUMAN-ACCEPTANCE.md` G-02-C 节"

### 9. G-02-D：私有 SQL Server 边界与仓库外目录
expected: |
  核对私有 SQL Server 边界与仓库外目录的真实内容，确认仓库外目录当前不含公司绑定数据、且 `ONTOFORGE_PRIVATE_FIXTURES` 入口的约定与实际部署一致
result: pending
why_human: "仓库外目录内容、git 历史中是否曾提交后删除私有绑定、以及文档与实际部署形态的一致性都不能由仓库内源码证明"
evidence: "机器可判定部分已落盘：门控 D-1（`private_fixtures.rs:14/44/69` 的环境变量 + `#[ignore]` 双门控）与门控 D-2（`CONTRIBUTING.md:89/91` 约定）均 PASS；见 `02-11-HUMAN-ACCEPTANCE.md` G-02-D 节"

## Summary

total: 9
passed: 8
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps

无 BLOCKER、无 gaps。上一轮 `02-VERIFICATION.md` 的 1 个 BLOCKER（`02-10.6` nightly Iceberg 合同）经逐条 `file:line` 复核已判定 VERIFIED；剩余 3 条为原理上无法机器判定的 judgment 项（对应 02-03 / 02-10 禁止项与 ENGINE-05 私有 SQL Server 子句），需人工签字后由 `/gsd-verify-work 02` 回填 `02-11-HUMAN-ACCEPTANCE.md` 的 `VERDICT` 并同步 `02-VERIFICATION.md`。
