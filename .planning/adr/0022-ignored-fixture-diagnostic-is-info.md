# ADR-0022 被忽略的 fixture 诊断降为 Info

- status: accepted
- date: 2026-10-19
- decider: 用户（UAT 02 测试 6 选定方案 a）
- 修订：ADR-0021 第 7 条（"配置映射到 Trino 而绑定带 fixture 时给 W" 改为给 Info，code 仍为 ONT-V17）
- 依据：`.planning/phases/02-source-binding-engine/02-REVIEW.md` WR-C03

## Context
ADR-0021 第 7 条规定：`validate --online --server-config` 把 profile 映射到 Trino 而绑定仍带 `fixture` 时给 warning（ONT-V17）。fixture 是给 SQLite 替身用的，同一份绑定在 dev 用替身、在 prod 对 Trino 复核是常态，带 fixture 不是绑定与源表的不一致（drift）。按 warning 处理时，`--deny-warnings` 在任何 Trino 配置下都被迫以退出码 1 结束，CI 无法把该开关用于真实的 warning。

## Decision
该诊断的严重级由 `Severity::Warning` 改为 `Severity::Info`。诊断码保持 `ONT-V17`，消息文本不变（`the fixture of ... is ignored because profile ... is checked against Trino`）。

## Consequences
- `--deny-warnings` 不再因被忽略的 fixture 失败；退出码逻辑本已把 Info 视为不失败，无需改动。
- JSON 输出中该诊断的 `severity` 为 `info`；人类可读输出照常打印该诊断（miette Advice 级），不被过滤；只剩此类 info 时汇总行是 `ok: ... binding(s) checked online`。
- 依赖 `severity == "warning"` 识别该诊断的下游脚本需改按 `info` 识别。
- 其他 ONT-V17 诊断（表缺失、列缺失、类型不兼容等）的 error / warning 严重级不受影响。
