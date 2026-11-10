---
phase: 08-production-webhook-end-to-end-delivery
plan: "02"
subsystem: testing
tags: [webhook, ci, evidence, archive, sha256, documentation, receipt]

requires:
  - phase: 08-production-webhook-end-to-end-delivery
    provides: "plan 08-01 交付的 `phase8-webhook-e2e` CI job、`phase8-webhook-ledger` artifact 与三次成功的 CI run（RED 42505885675 / GREEN 42508002310 / 多事件矩阵 42510584474）"
provides:
  - "冻结的 CI 证据归档三件套：README（归档说明）+ `ci-evidence-index.json`（逐成员 path/size/sha256）+ `ci-evidence.tar.gz`（6 个普通文件成员）"
  - "`ci/verify-evidence-archive.sh`：解包到临时空目录后逐成员复算 sha256 与字节数、核对成员数、拒绝对对路径/路径穿越/非普通文件成员/重复成员/未列出成员的归档完整性校验器，带四个篡改场景的自检"
  - "`crates/ontoforge/WEBHOOK-WORKER.md` 的「端到端联调（CI）」小节：CI job 名、构建与用例命令、三个环境变量、artifact 名与接收端 harness 的边界"
affects: [phase-09-webhook-supervision, phase-12-first-public-release, gsd-verify-work-08]

actuals:
  tokens: 6092
  tasks: 2
  commits: 2

plan_head_before: 30a05a8a43f7a8235ca8db7cc715d39e277bc8b5
plan_head_after: 26531c8
commits: 2

tech-stack:
  added: []
  patterns:
    - "证据归档三件套：README + index JSON + tar.gz；index 逐成员记录 path/size/sha256 并记录归档自身的 sha256 与来源边界"
    - "归档用显式文件清单打包（逐个列出成员），成员路径保持仓库根相对路径；只产生普通文件条目，成员数与 index 的 file_count 一一对应"
    - "「索引 ↔ 归档」的一致性由脚本化判定承担（解包复算 + 成员数核对 + 篡改自检），不作为人工步骤留在叙述里"

key-files:
  created:
    - ci/verify-evidence-archive.sh
    - .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/README.md
    - .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json
    - .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence.tar.gz
  modified:
    - crates/ontoforge/WEBHOOK-WORKER.md

key-decisions:
  - "归档只收 plan 预期的六个成员（ledger JSONL、受信配置快照、两个子进程各一份 stdout/stderr 日志）。artifact 实际含七个成员，多出的 `receiver.ready`（接收端 ready-file，只含绑定的回环 target URL）不在 plan 的证据枚举内，被显式排除并逐条记入索引的 `source_boundary.excluded`，不静默丢弃"
  - "归档用显式文件清单打包，成员路径保持仓库根相对路径（`target/phase8-webhook/ci/*`），使归档只含普通文件条目、实际成员数与索引 `file_count` 一一对应"
  - "归档完整性由 `ci/verify-evidence-archive.sh` 机器判定；该脚本不被任何 workflow 引用、不进发行包，其可信度由自身的四个篡改场景自检保证"
  - "校验器的 symlink 自检用逐字节构造的 tar 头生成 symlink 成员，不依赖宿主文件系统支持符号链接（Windows 本机不支持）"
  - "校验器对 jq 的输出统一做 CR 归一化（Windows 构建的 jq 以 CRLF 结束行）"

requirements-completed: [REL-01, REL-02]

coverage:
  - id: D1
    description: "CI 证据冻结成可独立复核与恢复的归档：README、index JSON、tar.gz 三件套，逐成员 path/size/sha256 可复算"
    requirement: REL-01
    verification:
      - kind: other
        ref: "bash ci/verify-evidence-archive.sh .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence.tar.gz .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json（退出码 0，输出 `OK: 6 members verified against ...`）"
        status: pass
      - kind: other
        ref: "归档成员表：tar -tvzf 全为普通文件、无绝对路径与 `..` 成员；`tar -xzOf | grep -cE 'postgres://[^[:space:]]*:[^[:space:]]*@'` 为 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "校验器不是空壳：四个篡改场景（成员 sha256 被改 / 索引少列成员 / 成员被换成 symlink / 成员路径重复）分别以退出码 10、12、4、5 被拒"
    requirement: REL-01
    verification:
      - kind: other
        ref: "bash ci/verify-evidence-archive.sh --selftest（退出码 0，输出 `SELFTEST OK: 4 tamper scenarios rejected`）"
        status: pass
    human_judgment: false
  - id: D3
    description: "归档确实来自一次成功的 `phase8-webhook-e2e` run（run id、head sha、job 结论可交叉确认）"
    requirement: REL-01
    verification:
      - kind: other
        ref: "run 42515808955 / job `phase8-webhook-e2e`（job id 128838450761，结论 success），head sha 30a05a8a43f7a8235ca8db7cc715d39e277bc8b5；三条谓词由 gh run list / gh run view + git merge-base --is-ancestor 机器判定"
        status: pass
    human_judgment: true
    rationale: "FA-03 backstop：本地命令能证明「该 run 含一个成功的 `phase8-webhook-e2e` job、且 head_sha 是当前 HEAD 的祖先、且晚于首次引入该 job 的 commit」，但归档内容是否真出自该 run 只能由 CI run 的 URL 与 run id 人工交叉确认。已登记到 `.planning/WINDOWS.md`（第 20 条，unrun-verify），待人工复核后关闭"
  - id: D4
    description: "运维者文档给出端到端联调的入口与边界，且既有 worker 契约逐字未改"
    requirement: REL-01
    verification:
      - kind: other
        ref: "crates/ontoforge/WEBHOOK-WORKER.md：含 `phase8-webhook-e2e` / `phase8-webhook-ledger` / `ONTOFORGE_WEBHOOK_RECEIVER_BIN` / `ONTOFORGE_WEBHOOK_EVIDENCE_DIR` 四个字面量；既有 `X-Ontoforge-Event-Id` 与 `至少一次` 两处措辞仍在；`git diff $(git merge-base main HEAD) --shortstat` 为 25 行新增、零删除行"
        status: pass
    human_judgment: false

# Metrics
duration: 10 min
completed: 2026-11-09
status: complete
---

# Phase 08 Plan 02: CI 证据归档与运维文档入口 Summary

**把 `phase8-webhook-e2e` 的一次成功 run 冻结成逐成员可复算的归档三件套，并给运维者留下这条联调的入口**

## Performance

- **Duration:** 10 min
- **Started:** 2026-11-08T20:08:19Z
- **Completed:** 2026-11-08T20:18:04Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- **来源 run 由三条谓词选出，不是凭列表挑选**：本分支上按「含一个结论为 success 的 `phase8-webhook-e2e` job（该 job 是本 phase 独有的锚）」「`head_sha` 是当前 HEAD 的祖先」「`head_sha` 晚于首次把该 job 名引入 `.github/workflows/ci.yml` 的 commit（`f95767a`）」三条同时命中，选出 run **42515808955**（head sha `30a05a8a`，job id `128838450761`，结论 success）。整条选择循环作为 `<automated>` 门跑通，输出 `EVIDENCE_RUN=42515808955 head_sha=30a05a8a...`。
- **归档三件套**：`08-evidence/ci-evidence.tar.gz`（6 个普通文件成员，751 字节）+ `ci-evidence-index.json`（逐成员 path/size/sha256 + 归档自身摘要 + 来源边界）+ `README.md`（来源、成员构成、字段含义、核对与恢复步骤、边界声明）。
- **校验器 `ci/verify-evidence-archive.sh`**：解到临时空目录后逐成员复算 SHA256 与字节数、核对成员数、拒绝对对路径 / 路径穿越 / 非普通文件成员 / 重复成员 / 索引未列出的成员；退出码 0 且输出 `OK: 6 members verified against ...`。它自己的可信度由四个篡改场景的自检保证。
- **运维者文档**：`crates/ontoforge/WEBHOOK-WORKER.md` 文末追加「端到端联调（CI）」小节，既有段落逐字未改（diff 为 25 行新增、零删除行）。

## CI 证据（来源）

| 项 | 值 |
|---|---|
| run | `42515808955` — https://github.com/easyinplay/ontoforge/actions/runs/42515808955 |
| head sha | `30a05a8a43f7a8235ca8db7cc715d39e277bc8b5`（= 本 plan 执行时的 HEAD） |
| job | `phase8-webhook-e2e`（结论 `success`，job id `128838450761`） |
| artifact | `phase8-webhook-ledger` |
| 收集目录 | `target/phase8-webhook/ci`（`gh run download 42515808955 --name phase8-webhook-ledger`） |

三条谓词的判定命令（可复算）：

```bash
landing=$(git log --format=%H --reverse -S'phase8-webhook-e2e' -- .github/workflows/ci.yml)  # f95767a
gh run list --workflow ci.yml --branch "$(git rev-parse --abbrev-ref HEAD)" --limit 20 --json databaseId
gh run view <id> --json headSha --jq .headSha
gh run view <id> --json jobs --jq '[.jobs[]|select(.name=="phase8-webhook-e2e" and .conclusion=="success")]|length'
```

## 归档成员与复算结果

| 成员（仓库根相对路径） | 字节数 | SHA256（前 16 位） | 内容 |
|---|---|---|---|
| `target/phase8-webhook/ci/ledger.jsonl` | 1866 | `65acd8a17c5fece0` | 接收端逐请求写入的原始 JSONL：6 行 = 3 条事件 × 2 次投递 |
| `target/phase8-webhook/ci/ontoforge-server.toml` | 302 | `787a20532e7eca86` | 受信配置快照；`[store]` 只含 `url_env = "ONTOFORGE_PG_URL"` 变量名 |
| `target/phase8-webhook/ci/receiver.log` | 0 | `e3b0c44298fc1c14` | 接收端 stdout（空） |
| `target/phase8-webhook/ci/receiver.err.log` | 0 | `e3b0c44298fc1c14` | 接收端 stderr（空） |
| `target/phase8-webhook/ci/worker.log` | 0 | `e3b0c44298fc1c14` | worker stdout（空） |
| `target/phase8-webhook/ci/worker.err.log` | 0 | `e3b0c44298fc1c14` | worker stderr（空） |

归档自身：`ci-evidence.tar.gz`，751 字节，SHA256 `3bad98b398b3d800e4e1e38a1e517b97509f0d98a0ce7b694ad9d56894dd7ac9`。

复算输出：

```
$ bash ci/verify-evidence-archive.sh .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence.tar.gz .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json
OK: 6 members verified against .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json
$ echo $?
0

$ bash ci/verify-evidence-archive.sh --selftest
SELFTEST OK: 4 tamper scenarios rejected
$ echo $?
0
```

归档成员的 ledger 是 08-01 那次多事件矩阵 run 的原始记录：三条事件的第 1 次投递均 503、第 2 次均 204，`header_event_id == payload_event_id == event_id`，逐行不含连接串与密码。

**排除项**（记在索引的 `source_boundary.excluded`，非静默丢弃）：`receiver.ready`（接收端 ready-file，只含绑定的回环 target URL，不属 plan 的证据枚举）、CI 的私有 env / state / secret、auth 状态文件、其它全部文件。

## 校验器的判定链

| 退出码 | 判定 | 自检覆盖 |
|---|---|---|
| 2 | 输入文件缺失或不可读 | — |
| 3 | 成员是绝对路径或含 `..` 段 | — |
| 4 | 存在非普通文件成员（symlink / 硬链接 / 目录条目） | 场景 (c) |
| 5 | 成员路径重复 | 场景 (d) |
| 6 | 列表或解包失败 | — |
| 7 | 索引 `file_count` 与 `files` 长度不一致 | — |
| 8 | 索引记录的归档自身 sha256/size 与文件不符 | — |
| 9 | 索引列出的成员在归档里不存在 | — |
| 10 | 某成员复算 sha256 与索引不符 | 场景 (a) |
| 11 | 某成员复算字节数与索引不符 | — |
| 12 | 归档里有未被索引列出的成员 | 场景 (b) |
| 13 | 实际成员数与 `file_count` 不一致 | — |

## 运维文档的字面量清单

`crates/ontoforge/WEBHOOK-WORKER.md` 新增小节里出现的字面量，逐个与 `.github/workflows/ci.yml` / `crates/ontoforge/tests/webhook_e2e_pg.rs` 对齐：

| 字面量 | 仓库出处 |
|---|---|
| `phase8-webhook-e2e` | `.github/workflows/ci.yml` 的 job 名 |
| `phase8-webhook-ledger` | 该 job 的 `upload-artifact` 的 `name` |
| `cargo build -p ontoforge --example webhook_receiver --locked` | 该 job 的构建步骤 |
| `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture` | 该 job 的测试步骤 |
| `ONTOFORGE_PG_URL` / `ONTOFORGE_WEBHOOK_RECEIVER_BIN` | `ci/query-services.sh postgres` 与该 job 的 step env |
| `ONTOFORGE_WEBHOOK_EVIDENCE_DIR`（默认 `target/phase8-webhook`） | 该 job 的 job 级 env 与测试里的 `evidence_dir()` |
| `bash ci/query-services.sh check-artifacts target/phase8-webhook` | 该 job 的凭据扫描步骤 |
| `crates/ontoforge/examples/webhook_receiver.rs` | 接收端 harness 源码 |

该小节同时写明：接收端是 CI 内部的测试 harness，以 example target 形式构建，**不是** `ontoforge` 的子命令、不进发行包、不作为对外能力。既有段落的 `X-Ontoforge-Event-Id` 与 `至少一次` 两处措辞逐字保留。

## Task Commits

Each task was committed atomically:

1. **Task 08-02-01: 把 phase8-webhook-e2e 的 CI 证据冻结成可复核的归档** - `3301850` (feat)
2. **Task 08-02-02: 在 worker 运维契约里补上端到端联调的入口** - `26531c8` (docs)

## Files Created/Modified

- `ci/verify-evidence-archive.sh` — 归档完整性校验器：解包复算、成员数核对、四类篡改自检；不被任何 workflow 引用，不进发行包
- `.planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/README.md` — 归档说明：来源 run、成员构成、索引字段、核对与恢复步骤、边界声明
- `.planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json` — 逐成员 path/size/sha256 + 归档自身摘要 + `source_boundary`
- `.planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence.tar.gz` — 冻结的 CI 证据（6 个普通文件成员）
- `crates/ontoforge/WEBHOOK-WORKER.md` — 追加「端到端联调（CI）」小节（既有段落零改写）

本 plan 未改任何生产代码、CI 配置或测试：`git diff --name-only 30a05a8..HEAD` 只有上述 5 个文件。

## Decisions Made

- **归档成员集合按 plan 的证据枚举取六个**，不整包收 artifact：artifact 实际含七个成员，多出的 `receiver.ready` 被显式排除并写进索引的 `excluded`，这样「归档里是什么、不是什么」在机器可读的索引里一目了然。
- **打包用显式文件清单**（逐个列出成员），而不是对目录整包打：这样归档里只有普通文件条目、没有目录条目，实际成员数与索引 `file_count` 一一对应，退出码 12 / 13 才有意义。
- **归档一致性由脚本判定，不留给人工步骤**：把「索引 ↔ 归档」的逐成员复算脚本化，同时让脚本用四个篡改场景自证不是空壳。
- **校验器的 symlink 场景不依赖宿主文件系统**：Windows 本机无法创建符号链接，改用逐字节构造 tar 头生成 symlink 成员，同一份脚本在 Cygwin 与 ubuntu-latest 上行为一致。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修正 jq 的 CRLF 输出污染校验器比较值**
- **Found during:** Task 08-02-01（写完校验器后跑 `--selftest`）
- **Issue:** 本机 jq 1.8.2（Windows 构建）以 CRLF 结束行。`index_member_rows` 的 TSV 行尾因此带上 `\r`，最后一段（成员的 sha256）被污染成 `…\r`，自检场景 (b) 期望退出码 12 却拿到 10（成员摘要“不符”），且这类假阴性会掩盖真正的篡改。
- **Fix:** 抽出 `jq_text` 助手，所有 jq 标量输出与 TSV 行统一经 `tr -d '\r'` 归一化。
- **Files modified:** `ci/verify-evidence-archive.sh`
- **Verification:** `bash ci/verify-evidence-archive.sh --selftest` 退出码 0、输出 `SELFTEST OK: 4 tamper scenarios rejected`；真实归档校验输出 `OK: 6 members verified against …`、退出码 0。
- **Committed in:** `3301850`

**2. [Rule 3 - Blocking] 自检的 symlink 变体改为逐字节构造 tar 头**
- **Found during:** Task 08-02-01（实现自检的四个篡改场景）
- **Issue:** plan 要求自检派生一个「成员被换成 symlink」的变体并期望退出码 4。本机（Windows / Cygwin，无符号链接权限）`ln -s` 只会复制文件内容，`test -L` 恒假，GNU tar 因此把它记成普通文件成员，场景 (c) 永远无法触发退出码 4。
- **Fix:** `craft_symlink_tar` 直接按 tar 头格式（typeflag `2` + linkname + 校验和）逐字节写出一个 symlink 成员，不再依赖 `ln -s`；脚本因此在不支持符号链接的宿主上仍能验证「非普通文件成员被拒」这条判定。
- **Files modified:** `ci/verify-evidence-archive.sh`
- **Verification:** 自检场景 (c) 实际以退出码 4 被拒（`tar -tvzf` 显示 `lrw-r--r-- … link -> one.txt`）；四个场景整体以退出码 0 通过。
- **Committed in:** `3301850`

---

**Total deviations:** 2 auto-fixed（1 bug、1 blocking）
**Impact on plan:** 两处都只改校验器自身的实现细节，未触碰 plan 规定的判定语义、退出码表或归档内容构成。无 scope creep。

## TDD Gate Compliance

- **本 plan 不是 TDD plan，两个 task 也没有 `tdd="true"`**，因此不适用 RED → GREEN 序列。`gsd_run check tdd.review-checkpoint 08 --raw` 返回 `"No type:tdd plans found in phase 08. TDD review skipped."`（`tddPlans: 0`、`violations: 0`）。
- **未伪造 RED 提交**：本 plan 的产物是一个 shell 脚本加归档文件，不匹配门禁的任何测试文件 pathspec；计划级的 TDD 门不适用，因此没有 `test(08-02)` 类型的提交。校验器自身的可信度改由 `--selftest` 的四个篡改场景承担（退出码 10 / 12 / 4 / 5），这是机器可判定的等价证明。
- **本 plan 对 REL-01 / REL-02 的机器可判定结论仍是 plan 08-01 的 CI 证据**（run 42515808955 / job `phase8-webhook-e2e`）；本 plan 只把它冻结下来，不产生新的行为结论。

## Issues Encountered

- **jq 的 CRLF 行为与命令替换的差异**：同一台机器上，`jq … | od` 看到 CRLF，而 `$(jq …)` 的结果不带 CR，而经进程替换喂给 `read` 的行仍带 CR。为免依赖这一不一致，脚本统一用 `tr -d '\r'` 归一化 jq 输出，而不是依赖 bash 的隐式处理。
- **tar `--transform` 的转义差异**：GNU tar 的 `--transform` 不接受 sed 风格的 `\.` 转义（会把成员名改成 `one\\.txt`），自检的重复成员变体改用无转义的 `s|^two|one|`。

## Known Stubs

无。新增的脚本与归档文件没有 `TODO` / `FIXME` / `placeholder` / 空实现残留；`ci/verify-evidence-archive.sh` 的每条判定分支都有对应的退出码与错误信息。

## User Setup Required

None - no external service configuration required.

## 未验证边界

- **FA-03（backstop）**：归档内容是否真正来自一次成功 run，本地命令无法完全证明——本地没有那次 run 的 PostgreSQL 凭据，也没有 CI 的写权限。本地已机器判定的只是三条谓词（run 含一个成功的 `phase8-webhook-e2e` job、其 `head_sha` 是当前 HEAD 的祖先、且晚于首次引入该 job 的 commit `f95767a`）。归档来源的最终确认只能由人工对照 CI run 的 URL 与 run id 完成。该条已登记到 `.planning/WINDOWS.md`（第 20 条，`unrun-verify`），供 ship 前人工复核。
- **REL-03（systemd 自动拉起）** 不属本 phase（Phase 9）；本归档不关闭它，Phase 8 的结论位保持 `VERDICT: PENDING`。

## Next Phase Readiness

- **Phase 8 的两个 plan 均已落盘**：REL-01 / REL-02 的机器可判定部分由 run 42515808955 承担并已冻结进 `08-evidence/`，人工复核只需对照 run URL、ledger 原始记录与索引摘要。
- **Phase 9 可以起手**：worker 崩溃信号、恢复投递与 systemd unit（REL-03）不依赖本 plan 的任何未完成项。
- **Phase 12（首次对外发布）** 需等 Phase 8-11 全部验收通过；本 plan 未引入任何对外能力面（接收端仍是 example target）。
- **阻塞项**：无。

## Self-Check: PASSED

- `ci/verify-evidence-archive.sh`、`08-evidence/README.md`、`08-evidence/ci-evidence-index.json`、`08-evidence/ci-evidence.tar.gz` 四个文件存在；`crates/ontoforge/WEBHOOK-WORKER.md` 的「端到端联调（CI）」小节存在
- `3301850`、`26531c8` 两个提交均在 HEAD 的祖先链上（`git rev-list --count 30a05a8..HEAD` = 2）
- 两个 task 的全部 `<automated>` 门重跑通过（23 条全绿）：归档/索引/README 三件套存在、verifier 输出 `OK: 6 members verified`、selftest 输出 `SELFTEST OK: 4 tamper scenarios rejected`、toolchain 门、无绝对路径与路径穿越成员、ledger 成员存在、归档内容无 PostgreSQL 连接串、索引含 sha256 字段与 ledger 成员、索引 `source_boundary.acceptance` 含「不代替独立 phase 验收」、README 含 job 名、文档四个字面量齐备、既有两处措辞未丢、文档 diff 零删除行、本 plan 只改 5 个文件（无生产代码 / CI 配置 / 测试改动）

---
*Phase: 08-production-webhook-end-to-end-delivery*
*Completed: 2026-11-09*
