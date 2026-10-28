# Phase 7 Planning Preflight

生成日期：2026-10-29。此文件记录 GSD plan:pre 输入，不替代产品合同或阶段验收。

## Gate Results

- UI：frontend=false，block=false，无产品前端。
- AI：服务与 MCP transport，不包含 LLM 推理，不生成 AI-SPEC。
- drift：codebase skipped(no-structure-md)；context 无 stale artifacts。
- assumption-delta：detected=false。
- SPEC：不存在，edges/prohibitions 均 supplied=false，启用 spec-less fallback。
- edge probe：使用原需求中文与显式 shapes，确定性 engine 输出 46 applicable、0 unclassified；planner 必须逐项写验收，不得把 unresolved 当已验证。
- stall watcher：Codex spawn 不返回 outputFile，无法执行原 Bash marker watcher；采用平台终态消息与有界 wait，保留 10 分钟无活动阈值并显式报告，不伪造 outputFile。
- model resolver 返回旧模型 ID，当前平台无对应模型；沿用 session 模型并按 resolver 设置 effort，未修改全局配置。

## Edge Probe Input And Report

```json
{
  "requirements": [
    {
      "id": "API-01",
      "text": "服务暴露 REST API，覆盖 ontology 元数据、对象读取、object set 查询、action 提交与 edit 历史",
      "shapes": [
        "io",
        "collection"
      ]
    },
    {
      "id": "API-02",
      "text": "服务导出 OpenAPI 3.1 文档，并纳入快照测试",
      "shapes": [
        "io",
        "text"
      ]
    },
    {
      "id": "API-03",
      "text": "错误响应统一为结构化错误码 + apiName 级描述 + request_id；L0 不含物理名",
      "shapes": [
        "io",
        "text"
      ]
    },
    {
      "id": "API-04",
      "text": "Rust 开发者可以用 `ontoforge-client` 调用全部 REST 能力",
      "shapes": [
        "io",
        "stateful"
      ]
    },
    {
      "id": "API-05",
      "text": "TypeScript 开发者可以使用由 OpenAPI 生成的平台层 client 调用全部 REST 能力",
      "shapes": [
        "io",
        "stateful",
        "numeric-range"
      ]
    },
    {
      "id": "MCP-01",
      "text": "agent 可以通过 stdio 或 streamable HTTP 连接 MCP server",
      "shapes": [
        "io"
      ]
    },
    {
      "id": "MCP-02",
      "text": "agent 可以调用 describe 工具获取 ontology 结构（object types、属性、links、actions 及其描述）",
      "shapes": [
        "io",
        "collection"
      ]
    },
    {
      "id": "MCP-03",
      "text": "agent 可以通过查询工具执行 object set 查询（filter / search / searchAround / aggregate）",
      "shapes": [
        "io",
        "collection"
      ]
    },
    {
      "id": "MCP-04",
      "text": "每个 action type 自动暴露为一个 MCP 工具，输入 schema 由参数定义派生",
      "shapes": [
        "io",
        "text"
      ]
    },
    {
      "id": "MCP-05",
      "text": "agent 可以以 validate-only 方式调用 action 工具（dry run）",
      "shapes": [
        "io",
        "stateful"
      ]
    },
    {
      "id": "MCP-06",
      "text": "agent 通过 MCP 的读写与 REST 走同一套授权与 edit log",
      "shapes": [
        "io",
        "stateful"
      ]
    },
    {
      "id": "MCP-07",
      "text": "`list_tools` 按主体过滤 toolset（core / full / sql:raw）；L2 的 6 个工具名称与参数对齐 mcp-trino",
      "shapes": [
        "io",
        "collection"
      ]
    },
    {
      "id": "MCP-08",
      "text": "L3 与任何定义 / 授权变更工具不出现在 agent MCP 中",
      "shapes": [
        "io"
      ]
    },
    {
      "id": "DX-01",
      "text": "开发者用一个二进制 `ontoforge serve --dev` 即可在本机（含 Windows）启动完整服务（SQLite 替身引擎 + dev token），无需 Trino",
      "shapes": [
        "io",
        "stateful"
      ]
    },
    {
      "id": "DX-02",
      "text": "仓库附带至少一个示例 ontology，可完成查询、执行 action、查看历史的端到端演示",
      "shapes": [
        "io",
        "stateful"
      ]
    },
    {
      "id": "DX-04",
      "text": "release workflow 发布 Linux / macOS / Windows 单二进制到 GitHub Releases",
      "shapes": [
        "io"
      ]
    },
    {
      "id": "L2-04",
      "text": "启动自检无法证明 L2 身份在 Trino 侧受限时禁用 L2 并告警（fail-closed）",
      "shapes": [
        "io",
        "stateful"
      ]
    },
    {
      "id": "SCOPE-07",
      "text": "角色 / scope 只能经管理通道或 L3 修改并审计；ontology 定义只经部署方 reload；agent MCP 无变更工具；每个请求固定使用开始时的定义版本",
      "shapes": [
        "io",
        "stateful"
      ]
    }
  ],
  "report": {
    "items": [
      {
        "requirement_id": "API-01",
        "category": "adjacency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When two things are exactly equal or just touch, do they merge, collide, or separate?"
      },
      {
        "requirement_id": "API-01",
        "category": "empty",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What is the result for empty, single-element, or null input?"
      },
      {
        "requirement_id": "API-01",
        "category": "ordering",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When elements compare equal, is output order specified and stable?"
      },
      {
        "requirement_id": "API-01",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "API-02",
        "category": "empty",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What is the result for empty, single-element, or null input?"
      },
      {
        "requirement_id": "API-02",
        "category": "encoding",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "Whose definition of length/equality applies — bytes, code points, grapheme clusters, or normalized form?"
      },
      {
        "requirement_id": "API-02",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "API-03",
        "category": "empty",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What is the result for empty, single-element, or null input?"
      },
      {
        "requirement_id": "API-03",
        "category": "encoding",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "Whose definition of length/equality applies — bytes, code points, grapheme clusters, or normalized form?"
      },
      {
        "requirement_id": "API-03",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "API-04",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "API-04",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "API-05",
        "category": "boundary",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens exactly at each min/max/threshold — and one step either side?"
      },
      {
        "requirement_id": "API-05",
        "category": "precision",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "Where can precision loss, overflow, or rounding/tie-breaking occur — and what is the exact contract (e.g. half-up vs half-to-even, ceil/floor/truncate)?"
      },
      {
        "requirement_id": "API-05",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "API-05",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-01",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-02",
        "category": "adjacency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When two things are exactly equal or just touch, do they merge, collide, or separate?"
      },
      {
        "requirement_id": "MCP-02",
        "category": "empty",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What is the result for empty, single-element, or null input?"
      },
      {
        "requirement_id": "MCP-02",
        "category": "ordering",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When elements compare equal, is output order specified and stable?"
      },
      {
        "requirement_id": "MCP-02",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-03",
        "category": "adjacency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When two things are exactly equal or just touch, do they merge, collide, or separate?"
      },
      {
        "requirement_id": "MCP-03",
        "category": "empty",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What is the result for empty, single-element, or null input?"
      },
      {
        "requirement_id": "MCP-03",
        "category": "ordering",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When elements compare equal, is output order specified and stable?"
      },
      {
        "requirement_id": "MCP-03",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-04",
        "category": "empty",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What is the result for empty, single-element, or null input?"
      },
      {
        "requirement_id": "MCP-04",
        "category": "encoding",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "Whose definition of length/equality applies — bytes, code points, grapheme clusters, or normalized form?"
      },
      {
        "requirement_id": "MCP-04",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-05",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "MCP-05",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-06",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "MCP-06",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-07",
        "category": "adjacency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When two things are exactly equal or just touch, do they merge, collide, or separate?"
      },
      {
        "requirement_id": "MCP-07",
        "category": "empty",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What is the result for empty, single-element, or null input?"
      },
      {
        "requirement_id": "MCP-07",
        "category": "ordering",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "When elements compare equal, is output order specified and stable?"
      },
      {
        "requirement_id": "MCP-07",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "MCP-08",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "DX-01",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "DX-01",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "DX-02",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "DX-02",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "DX-04",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "L2-04",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "L2-04",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      },
      {
        "requirement_id": "SCOPE-07",
        "category": "idempotency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "What happens if this runs twice on the same input?"
      },
      {
        "requirement_id": "SCOPE-07",
        "category": "concurrency",
        "status": "unresolved",
        "verification": null,
        "resolution": null,
        "reason": null,
        "probe": "If interrupted or run in parallel, what is guaranteed?"
      }
    ],
    "coverage": {
      "applicable": 46,
      "resolved": 0,
      "unresolved": 46,
      "unclassified": 0,
      "byVerification": {
        "explicit": 0,
        "backstop": 0
      }
    }
  }
}
```

## Planner Contributions


### ai-integration

# API Coverage Decision Checkpoint

> Full API Coverage by Default — Opt Out, Never Opt In. Fires when a phase
> integrates an external API / SDK / service. Most non-API phases will not fire
> it — that is the point.

## Why this exists

"We integrated the API" too often silently means "we integrated whatever the
first use case exercised." Every un-built capability is then an invisible hole,
discovered later by a user who reasonably expected it to work. The phase sealed
green because its tasks completed; nobody decided the gaps were acceptable,
because nobody enumerated them. This checkpoint makes the surface **visible and
decided** before the phase can seal.

## Detect whether this phase integrates an external API

The detector is a deterministic scan over the phase scope. It strips fenced
code blocks first, so a trigger term inside a code snippet does not fire. It
returns a typed result: `{ detected, signals[], terms }`. Run it on the phase
scope (the concatenation of this phase's ROADMAP section + the PLAN body):

```bash
SCOPE="$(cat "${PHASE_DIR}"/*-PLAN.md 2>/dev/null) $(gsd_run query roadmap.get-phase "${PHASE}" 2>/dev/null || true)"
API_COVERAGE_JSON=$(printf '%s' "$SCOPE" | node gsd-core/bin/lib/api-coverage.cjs --json 2>/dev/null) || true
[ -n "$API_COVERAGE_JSON" ] || API_COVERAGE_JSON='{"skipped":true,"reason":"probe_unavailable"}'
```

The `|| true` neutralizes the assignment's status without discarding the
detector's own payload: the detector exits **1** for a real "no integration"
verdict, so treating any non-zero exit as failure would throw away a correct
answer. Emptiness — not exit status — is what proves the probe never ran, and
the second line is the only place the fragment manufactures a payload of its
own — one that records the *absence* of a verdict rather than asserting one.

The detector's exit code and `--json` payload now distinguish a real negative
from an unexamined input (ADR-3889 Phase 3, #3907): empty/whitespace-only
`$SCOPE` or a stdin read failure emit `{"skipped":true,"reason":"no_input"|
"stdin_error"}` — no `detected` key at all. **Check for `skipped` before
reading `detected`**: a `skipped` payload is not a confirmed "no API
integration" verdict, it means the detector never examined real input. Do not
treat it as `detected:false`. Read `API_COVERAGE_JSON.detected` only when
`skipped` is absent — act on it only, do **not** pattern-match the prose
yourself.

**If `skipped` is `true`:** the detector could not establish a scope (empty
`$SCOPE`) or failed to run (stdin read error). Skip the checkpoint for this
run rather than asserting a verdict about input that was never examined; do
not raise it with the user.

**If `detected` is `false`:** this phase does not integrate an external API. Skip
the checkpoint entirely and continue planning. Do not raise it with the user.

**If `detected` is `true`:** an external-API integration is in scope. You MUST
produce a **coverage matrix** before the plan is finalized.

**If `detected` is `true` but the phase genuinely integrates no external API**
(the detector is deterministic, not infallible — confirm by re-reading the phase
scope, not by preference): do NOT fabricate a matrix row for a capability that
does not exist. Write a reasoned declaration to `${PHASE_DIR}/COVERAGE.md`
instead:

```markdown
No external API integration: <one-line reason — what the phase touches instead>.
```

The reason is required, exactly like an `OPT-OUT` reason. The seal-time gate
accepts this declaration in place of a matrix.

## Produce the coverage matrix

Enumerate the external API's full **capability surface** — the verb/endpoint/method
list (e.g. for a music service: `search`, `play`, `pause`, `skip`, `set_volume`,
`get_playlist`, `create_playlist`, `add_to_playlist`, …). For each capability
record a decision, starting from **full coverage** as the default:

| capability | decision | reason |
|---|---|---|
| `<capability-id>` | `INTEGRATE` \| `OPT-OUT` | `<one-line reason if OPT-OUT>` |

Rules:

- **`INTEGRATE` is the default.** Every capability starts as INTEGRATE; the
  matrix is the *subtraction record*.
- **Every `OPT-OUT` MUST carry a one-line reason** (`not needed`, `not needed
  yet`, `explicitly out of scope`, …). An opt-out without a reason is an
  un-decided hole — the exact failure mode this gate exists to close.
- **A second integration against the same need** (e.g. a second platform for the
  same capability) starts from the **same full-coverage baseline** as the first.
  Do not carry over the first integration's opt-outs silently — re-decide each
  capability for the new surface, so a first-class/fallback asymmetry cannot
  accumulate.

Write the matrix to `${PHASE_DIR}/COVERAGE.md` (canonical markdown-table form):

```markdown
# API Coverage — <service>

> Full coverage by default. Opt-outs are explicit, reasoned decisions.

| capability | decision | reason |
|---|---|---|
| search | INTEGRATE | |
| playlists | INTEGRATE | |
| skip | OPT-OUT | not needed yet — tracked for follow-up phase |
```

A fenced ` ```coverage ` JSON block is also accepted for machine-generated
matrices; the markdown table is preferred (human-editable, diff-friendly).

## The seal-time gate

This checkpoint is enforced. At `verify:pre` the `api-coverage.verify-pre` gate
runs `check api-coverage.verify-pre <phase-dir>`:

- If `COVERAGE.md` exists, it is validated — every row needs a valid decision and
  every `OPT-OUT` a reason. A malformed/partial matrix **blocks the seal**. A
  reasoned `No external API integration: …` declaration (and no rows) passes.
- If `COVERAGE.md` is absent, the detector runs again over the phase scope. If a
  strong external-API-integration signal is found, the seal is **blocked** until a
  matrix is produced. If no signal is found, the phase is treated as a non-API
  phase and the seal proceeds.

So: an API-integrating phase cannot seal without a decided matrix. Produce it at
plan time; do not leave it for seal time.

## Tuning the vocabulary (optional)

The trigger vocabulary is a curated, additive-only set in
`gsd-core/bin/lib/api-coverage.cjs` (`DEFAULT_API_COVERAGE_TERMS`). To widen it
for a project, override at the call site:

```bash
printf '%s' "$SCOPE" | node gsd-core/bin/lib/api-coverage.cjs --json \
  --verbs integrate,wrap,connect,embed --nouns api,sdk,rest,grpc,webhook,plugin
```

The whole checkpoint is toggleable via `workflow.api_coverage_gate` in
`.planning/config.json`.


### assumption-delta

# Assumption-Delta Architecture Checkpoint

> Advisory, non-blocking. Fires **only** when the phase scope shows a singular→plural / required→optional / derived→chosen transition. When it fires, it surfaces ONE identity-model question before the plan is finalized. Most phases will not fire it — that is the point.

## Why this exists

Most quietly-imported architectural debt does not come from a missing upfront design phase. It comes at the *seam*: a later phase introduces a second case (a second platform, auth method, tenant, region, source of truth) and nobody re-asks whether the original abstraction still names the right thing. The phase that adds the second case is exactly the 20-minute conversation that prevents an afternoon of later cleanup.

## Run the detector

The detector is a deterministic scan over the phase scope text. It strips fenced code blocks first, so a trigger word that appears only inside a code snippet does not fire. It returns a typed result: `{ detected, signals[], terms }`. Resolve it through the `assumption-delta scan` query (same phase-section resolver as `roadmap.get-phase`):

```bash
ASSUMPTION_DELTA_JSON=$(gsd_run query assumption-delta scan "${PHASE}" --json 2>/dev/null) || true
[ -n "$ASSUMPTION_DELTA_JSON" ] || ASSUMPTION_DELTA_JSON='{"skipped":true,"reason":"probe_unavailable"}'
```

> If the phase section cannot be resolved (no `ROADMAP.md` / unknown phase, or a section with no body), the query emits `{ "skipped": true, "reason": "phase_unresolved" }` — **not** `detected:false`. A probe that never had input does not get to assert that this phase changes no core assumption. The checkpoint does not fire either way; the difference is that a skip is now distinguishable from a real negative. Do not block on it.
>
> Optional tuning — pass `--terms <comma-list>` to replace the curated pluralization cues for this project (the `optional`/`chosen` cues keep their defaults): `gsd_run query assumption-delta scan "${PHASE}" --json --terms second,alternative,fallback`.

## Decision branch

Read `ASSUMPTION_DELTA_JSON`. Act on `detected` only — do **not** pattern-match the human prose.

**If `skipped` is `true`:** the detector never examined a phase section — it could not resolve one (`phase_unresolved`) or could not run at all (`probe_unavailable`). Skip the checkpoint for this run rather than asserting a verdict about input that was never examined; do not raise it with the user. **Check for `skipped` before reading `detected`** — a skipped payload carries no `detected` key, and treating its absence as `false` re-creates the fabrication this branch exists to prevent.

**If `detected` is `false`:** this phase does not change a core assumption. Skip the checkpoint entirely and continue planning. Do not raise it with the user.

**If `detected` is `true`:** a core assumption may have lost its monopoly. The `signals[]` array tells you which family fired:

| `kind` | What changed | The question to answer |
|---|---|---|
| `pluralization` | A second X was introduced where there was one (second platform / auth method / tenant / region / source of truth) | Does the current primary key / identity model still name the right noun? |
| `optional` | A required / `only` field became optional | Is the field still the right anchor, or has the anchor moved? |
| `chosen` | A derived value became chosen, or a constant became a parameter | Has a configuration decision become a modeling decision? |

Before finalizing the plan, answer this for the user and record the decision explicitly:

> **Promote vs. add-alongside.** The usual correct move when a generalization occurs is to **promote** the new general representation to the primary and **demote** the old specific one to a detail of one variant — *not* to add the new one alongside the still-required old one. Adding alongside silently contradicts the generalized intent (a later variant that does not fit the old primary can be stored but never confirmed as a default).

Record the outcome in the PLAN.md front matter / a `<assumption_delta_decision>` block:

- The **noun** that is now primary (the generalized identity).
- The **decision**: `promote` | `add-alongside` | `no-change`, with a one-line rationale.
- If `add-alongside`: call it out as accepted debt and note what would force a later promote.

## Optional companion: an invariant test

When `detected` is `true`, suggest (do not require) a contract/invariant test that encodes the now-generalized intent — e.g. *"every confirmed default round-trips through the primary use-path, for every supported variant."* That test goes red the instant a future phase reintroduces the singular assumption, so the regression cannot land silently. If the user accepts, add the test as a task in the plan.

## Tuning the vocabulary (optional)

The trigger vocabulary is a curated, additive-only set in `gsd-core/bin/lib/assumption-delta.cjs` (`DEFAULT_ASSUMPTION_DELTA_TERMS`). Bare "or" is intentionally excluded — it is too common in prose and would make the gate fire constantly. To widen or narrow the cues for a project, override at the call site with `--terms <comma-list>` (replaces the pluralization cues; `optional`/`chosen` keep defaults). The whole checkpoint is toggleable via `workflow.assumption_delta` in `.planning/config.json`.

This checkpoint is advisory: it informs and records; it never blocks the phase.


### schema-gate

# Schema Push Detection Gate

> Detects schema-relevant files in the phase scope and injects a mandatory `[BLOCKING]` schema push task into the plan. Prevents false-positive verification where build/types pass because TypeScript types come from config, not the live database.

Check if any files in the phase scope match schema patterns:

```bash
PHASE_SECTION=$(gsd_run query roadmap.get-phase "${PHASE}" --pick section 2>/dev/null)
```

Scan `PHASE_SECTION`, `CONTEXT.md` (if loaded), and `RESEARCH.md` (if exists) for file paths matching these ORM patterns:

| ORM | File Patterns |
|-----|--------------|
| Payload CMS | `src/collections/**/*.ts`, `src/globals/**/*.ts` |
| Prisma | `prisma/schema.prisma`, `prisma/schema/*.prisma` |
| Drizzle | `drizzle/schema.ts`, `src/db/schema.ts`, `drizzle/*.ts` |
| Supabase | `supabase/migrations/*.sql` |
| TypeORM | `src/entities/**/*.ts`, `src/migrations/**/*.ts` |

Also check if any existing PLAN.md files for this phase already reference these file patterns in `files_modified`.

**If schema-relevant files detected:**

Set `SCHEMA_PUSH_REQUIRED=true` and `SCHEMA_ORM={detected_orm}`.

Determine the push command for the detected ORM:

| ORM | Push Command | Non-TTY Workaround |
|-----|-------------|-------------------|
| Payload CMS | `npx payload migrate` | `CI=true PAYLOAD_MIGRATING=true npx payload migrate` |
| Prisma | `npx prisma db push` | `npx prisma db push --accept-data-loss` (if destructive) |
| Drizzle | `npx drizzle-kit push` | `npx drizzle-kit push` |
| Supabase | `supabase db push` | Set `SUPABASE_ACCESS_TOKEN` env var |
| TypeORM | `npx typeorm migration:run` | `npx typeorm migration:run -d src/data-source.ts` |

Inject the following into the planner prompt (step 8) as an additional constraint:

```markdown
<schema_push_requirement>
**[BLOCKING] Schema Push Required**

This phase modifies schema-relevant files ({detected_files}). The planner MUST include
a `[BLOCKING]` task that runs the database schema push command AFTER all schema file
modifications are complete but BEFORE verification.

- ORM detected: {SCHEMA_ORM}
- Push command: {push_command}
- Non-TTY workaround: {env_hint}
- If push requires interactive prompts that cannot be suppressed, flag the task for
  manual intervention with `autonomous: false`

This task is mandatory — the phase CANNOT pass verification without it. Build and
type checks will pass without the push (types come from config, not the live database),
creating a false-positive verification state.
</schema_push_requirement>
```

Display: `Schema files detected ({SCHEMA_ORM}) — [BLOCKING] push task will be injected into plans`

**If no schema-relevant files detected:** Skip silently.


### security

Each PLAN.md must include a <threat_model> block when security enforcement is active. Use the configured ASVS level and blocking threshold from workflow.security_asvs_level and workflow.security_block_on.

### tdd

<tdd_mode_active>
**TDD Mode is ENABLED.** Apply TDD heuristics to all eligible tasks:
- Business logic with defined I/O → type: tdd
- API endpoints with request/response contracts → type: tdd
- Data transformations, validation, algorithms → type: tdd
- UI, config, glue code, CRUD → standard plan (type: execute)
Each TDD plan gets one feature with RED/GREEN/REFACTOR gate sequence.
</tdd_mode_active>

Security resolved config: ASVS level 1, block on high.

