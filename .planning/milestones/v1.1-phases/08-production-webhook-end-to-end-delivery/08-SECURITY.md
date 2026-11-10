---
phase: "08"
slug: "production-webhook-end-to-end-delivery"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-09"
---

# Phase 08 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| 接收端进程 → ledger 文件 | 独立进程写入、CI 侧读取的证据文件 | 请求事实（方法 / 路径 / 事件 id / 状态码 / 回环来源地址），无凭据 |
| ledger 文件 → CI 判定 | 原始记录被当作交付结论的依据 | 判定输入；接收端自身不做断言 |
| CI job → PostgreSQL 容器 | job 级凭据经环境变量注入容器与连接串 | PostgreSQL 密码（仅环境变量；配置文件里只写 `url_env` 变量名） |
| worker 子进程 → 接收端 | 受信配置里的 target 指向回环地址上的接收端 | outbox 事件的 JSON 负载与 `X-Ontoforge-Event-Id` 请求头 |
| 本地开发机 → 本机验证 | Windows 上无 Docker，端到端用例只能编译不能运行 | 编译级证据；端到端裁决只在 Linux CI 上产生 |
| CI run → 本地归档目录 | 从远端拉取的 artifact 进入工作区并被打包提交 | ledger JSONL、两个子进程日志、不含秘密的配置快照 |
| 归档 → git 历史 | 冻结的证据进入版本库，永久可读 | 归档成员（逐成员 SHA256 索引）；不含凭据 |
| 文档 → 运维者 | 文档里的命令与环境变量名会被运维者照着执行 | 字面量（job 名 / artifact 名 / 环境变量名），必须与实现逐字一致 |

---

## Threat Register

| Threat ID | Category | STRIDE | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|--------|----------|----------|-------------|------------|--------|
| T-08-01 | Tampering | Tampering | `crates/ontoforge/examples/webhook_receiver.rs` 的 ledger 写入 | medium | mitigate | 逐条无缓冲写入原始记录（`OpenOptions::append` + `write_all` + `flush`，无 `BufWriter`）；判定在 CI 侧由 ledger 与 `of_webhook_outbox` 终态两处独立来源交叉比对；接收端不做断言、不暴露聚合端点（实测：`assert!` 0 处、`counts` 0 处、`BufWriter` 0 处） | closed |
| T-08-02 | Information Disclosure | Information Disclosure | ledger、worker 日志、CI artifact | high | mitigate | 接收端只记录请求事实；上传前跑 `bash ci/query-services.sh check-artifacts target/phase8-webhook` 扫描 PG 密码（`.github/workflows/ci.yml` 内该行存在）；用例断言 ledger 每行不含连接串与密码 | closed |
| T-08-03 | Spoofing | Spoofing | `X-Ontoforge-Event-Id` 与 payload `event_id` | medium | mitigate | 断言 `header_event_id == payload_event_id == of_webhook_outbox.event_id`；worker 的 target 只来自受信配置，Action 请求改不了 | closed |
| T-08-04 | Elevation of Privilege | Elevation of Privilege | 接收端监听地址 | medium | mitigate | 只绑 `127.0.0.1`（实现中 `TcpListener::bind(("127.0.0.1", args.port))`，全文无 `0.0.0.0`），不发布端口，job 结束即杀进程 | closed |
| T-08-05 | Denial of Service | Denial of Service | CI job 挂起、端口占用、退避等待 | medium | mitigate | `--port 0` 由 OS 分配；进程生命周期收进 Rust 测试（`Child` + `Drop` kill/wait），CI shell 不起后台进程；job `timeout-minutes: 30`；轮询带 deadline | closed |
| T-08-06 | Tampering | Tampering | `outbox.rs` / `config.rs` / `commit.rs` / `of_webhook_outbox` 表结构 | high | mitigate | 被验证对象的投递语义在本 phase 一律未改动：实测 `git diff --name-only main...HEAD -- crates/ontoforge-action/src/outbox.rs crates/ontoforge-engine/src/config.rs crates/ontoforge-storage/` 输出为空 | closed |
| T-08-07 | Information Disclosure | Information Disclosure | `[store] url_env` 解析与错误输出 | medium | mitigate | 连接串只经环境变量传入，配置文件里只写变量名；测试侧用 `PgConnectOptions` 连接，连接串在错误输出里不回显密码 | closed |
| T-08-SC | Tampering | Tampering | npm / pip / cargo installs | high | mitigate | 本 phase 未引入任何新外部包：`crates/ontoforge/Cargo.toml` 的 `[dev-dependencies]` 只多 `axum.workspace = true` 一行（`axum` 已是 workspace pin `=0.8.9` 且被 `ontoforge-server` 使用），依赖图不变；cargo 一律带 `--locked` | closed |
| T-08-08 | Information Disclosure | Information Disclosure | `08-evidence/` 归档内容与索引 | high | mitigate | 只收 ledger、进程日志与不含秘密的配置快照；本地复扫禁止出现 PostgreSQL 连接串（实测 `tar -xzOf … \| grep -cE 'postgres://…'` 为 0）；CI 侧上传前已用 `check-artifacts` 用真实密码扫过一遍 | closed |
| T-08-09 | Tampering | Tampering | `ci-evidence-index.json` 的成员清单与归档本体 | medium | mitigate | 逐成员记录 path、size 与 sha256，由 `ci/verify-evidence-archive.sh` 解到新空目录后机器复算并核对成员数量（实测 `OK: 6 members verified`，退出码 0）；拒绝绝对路径、路径穿越、非普通文件成员与重复成员（实测绝对路径 / `..` 成员计数为 0；自检 `SELFTEST OK: 4 tamper scenarios rejected`） | closed |
| T-08-10 | Denial of Service | Denial of Service | CI 的 PostgreSQL 容器与临时目录 | medium | mitigate | 复核 `.github/workflows/ci.yml` 的 `if: always()` 清理分支仍在（该 job 内 `always()` 出现 4 处），`cleanup` 对不存在的容器有 `\|\| true` 兜底 | closed |
| T-08-11 | Spoofing | Spoofing | `WEBHOOK-WORKER.md` 里给出的 job 名、artifact 名与环境变量名 | low | mitigate | 字面量与实现逐字一致：`phase8-webhook-e2e` / `phase8-webhook-ledger` / `ONTOFORGE_WEBHOOK_RECEIVER_BIN` / `ONTOFORGE_WEBHOOK_EVIDENCE_DIR` 在文档中均存在（grep 计数均 ≥ 1），且与 `.github/workflows/ci.yml`、测试文件一致 | closed |
| T-08-12 | Repudiation | Repudiation | 归档与文档对「结论」的表述 | medium | mitigate | 索引的 `source_boundary.acceptance` 与 README 的边界声明写明「不代替独立 phase 验收」（两处均命中）；REL-03 的结论位保持 `VERDICT: PENDING` | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|

*Accepted risks do not resurface in future audit runs.*

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-09 | 13 | 13 | 0 | /gsd-secure-phase 08（L1 grep-depth 核验，asvs_level 1） |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-09 — 13 条威胁全部 CLOSED，无 accept / transfer 项；`threats_open: 0`，未阻塞 phase 推进。

> 审计深度说明：ASVS level 1，按 `secure-phase` 的短路规则（`threats_open: 0` 且 register 在 plan 期撰写且 level 1）以 L1 grep-depth 核验缓解措施是否存在于实现，未派发 `gsd-security-auditor` 做 L2 边界放置或 L3 端到端 trace 检查。两处 backstop（FA-01 接收端对非 JSON 请求体的容忍性、FA-02 第 17 次失败起的 300 秒退避上限）不属本 phase 的自动化验证面，已登记在 `.planning/WINDOWS.md` 第 19 条待人工复核。

## Security Audit 2026-11-08

| Metric | Count |
|---|---|
| Threats found | 13 |
| Closed | 13 |
| Open | 0 |
