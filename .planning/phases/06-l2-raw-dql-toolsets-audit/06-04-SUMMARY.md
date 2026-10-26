---
phase: 06-l2-raw-dql-toolsets-audit
plan: 04
subsystem: engine
tags: [trino, l2, acl, authorization, tdd]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "06-03 的 L2 SQL gate 与独立身份"
provides:
  - "L2 默认禁用、ACL 正反探针、活跃 query 隔离与周期复检健康门禁"
  - "独立 Trino 483 file ACL fixture 与宽松规则对照测试"
affects: [06-06, 06-08]
actuals:
  tokens: 11475
  tasks: 2
  commits: 4
commits: 4
plan_head_before: 6a2795cf6ab12be52e13a036c4019327ea4892f7
plan_head_after: 253da90ab41d5a856fd9df21f715eb73e887e456
tech-stack:
  added: []
  patterns: ["库级 L2 健康快照与读写门禁", "独立 Trino 483 ACL 验证"]
key-files:
  created:
    - crates/ontoforge-engine/src/trino/l2_health.rs
    - crates/ontoforge-engine/tests/l2_health.rs
    - crates/ontoforge-engine/tests/trino_l2_acl_it.rs
    - ci/trino/l2/rules.json
  modified:
    - crates/ontoforge-engine/src/config.rs
    - crates/ontoforge-engine/src/printer/l2.rs
    - crates/ontoforge-engine/src/trino/mod.rs
    - ci/query-services.sh
key-decisions:
  - "仅 ACL 显式声明且独立身份、正反对象和活跃 query 探针均通过时启用 L2。"
  - "复检失败、超时或超过 60 秒未复检即禁用 L2。"
requirements-completed: [L2-03, L2-04]
coverage:
  - id: D1
    description: "默认禁用、精确权限错误和活跃 query 隔离的库级健康门禁"
    requirement: L2-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/l2_health.rs#l2_health_"
        status: pass
    human_judgment: false
  - id: D2
    description: "真实 Trino 483 ACL 正反对象和宽松 ACL 对照"
    requirement: L2-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_l2_acl_it.rs#trino_l2_acl_"
        status: pass
    human_judgment: false
duration: 54min
completed: 2026-10-26
status: complete
---

# Phase 6 Plan 04: L2 ACL 健康门禁 Summary

L2 只有在独立 Trino 483 ACL 正反探针及活跃查询隔离通过后可用，撤权或复检失败立即禁用。

## Performance

- **Duration:** 54 min（16:55 至 17:49，Asia/Shanghai）
- **Tasks:** 2
- **Files modified:** 14（含两份 RED JUnit 报告和证据记录）

## Accomplishments

- L2 默认 disabled。ACL 声明、独立身份、已存在的正反对象及已知活跃 query 探针均须通过；仅精确 PERMISSION_DENIED 算负探针成功，健康原因脱敏。
- 请求与复检共用读写门禁。撤权、错误、超时以及超过 60 秒未复检均使 L2 disabled。
- 独立 ontoforge-trino-l2 的 file ACL 允许正对象数据，拒绝两个确实存在的越界对象并隐藏另一主体活跃 query ID；ontoforge-trino-l2-loose 作为 fail-closed 对照。

## Task Commits

- Task 1 RED：fb15f80；GREEN：7a1116f。
- Task 2 RED：63ced52；GREEN：253da90。

## TDD Gate Compliance

- Task 1：l2_health_unverified_identity_never_submits_a_statement 在未校验身份时提交 SQL 并得到 HTTP 404，身份复用断言也失败。真实 JUnit 记录 06-04-RED-task1-junit.json 返回 RED_EVIDENCE_OK；目标因预定的默认禁用断言失败。GREEN 后健康门禁测试通过。
- Task 2：无 ACL 的独立 Trino 483 上，L2 读取已存在的 outside.tiny.nation 得到 FINISHED，预期的 PERMISSION_DENIED 断言失败。真实 JUnit 记录 06-04-RED-task2-junit.json 返回 RED_EVIDENCE_OK；目标因预定的越界拒绝断言失败。GREEN 后真实集成测试 3/3 通过。
- REFACTOR：无独立重构提交。两个 RED 均经机器分类和语义检查，不以编译或 fixture 失败充当 RED。

## Verification

- gsd-tools check tdd-red-evidence：两份记录均为 RED_EVIDENCE_OK（本次复核）。
- cargo test -p ontoforge-engine --locked、cargo clippy -p ontoforge-engine --all-targets --locked -- -D warnings：通过。
- cargo test -p ontoforge-engine --test trino_l2_acl_it --locked -- --ignored --nocapture：真实 Trino 483 上 3/3 通过。允许对象有数据；outside 和 editstore 存在且返回 PERMISSION_DENIED；已知活跃 query ID 不可见；宽松 ACL 对照使复检 disabled。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 调整本机容器运行入口**
- **Found during:** Task 2。
- **Issue:** WSL 未启用 Docker 集成。
- **Fix:** 在 Git Bash 执行 ci/query-services.sh trino-l2，使用本机 Docker daemon。
- **Verification:** 两容器健康且真实集成测试 3/3 通过。
- **Committed in:** 253da90。

**2. [Rule 3 - Blocking] 适配现有 reqwest feature**
- **Found during:** Task 2。
- **Issue:** 当前 reqwest 未启用 .json() feature。
- **Fix:** 读取响应文本后以 serde_json 解析，不增加依赖。
- **Verification:** 目标集成测试编译并通过。
- **Committed in:** 253da90。

## Next Phase Readiness

06-06 可复用库级健康状态接入公开 execute/explain；06-08 可纳入独立 ACL fixture。两台本机 L2 容器在撰写时仍运行，检查完成后清理。

## Self-Check: PASSED

任务提交 fb15f80、7a1116f、63ced52、253da90 均在 HEAD 历史中；健康模块、两个测试 target、ACL 规则与本 SUMMARY 均存在。
