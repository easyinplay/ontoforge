---
phase: 01-workspace-schema-lint
plan: 14
subsystem: schema-editor-integration
tags: [yaml-language-server, json-schema, zed, vscode, modeline, gap-closure]

requires:
  - phase: 01-workspace-schema-lint
    provides: "schemas/ontoforge-definition.schema.json 与 ontoforge-manifest.schema.json（plan 01-09 导出）"
provides:
  - "项目级 yaml.schemas 配置（.zed/settings.json、.vscode/settings.json）"
  - "aviation 定义文件每个 YAML document 头部的定义 schema modeline（新增 10 行）"
  - "两个回归测试守住编辑器配置与 modeline"
  - "CONTRIBUTING.md「编辑器接入」一节"
affects: [phase-01-uat, SCHEMA-07]

gap_closure: true
gap_ids: [G-1-2]

actuals:
  tokens: 3967
  tasks: 3
  commits: 3
plan_head_before: b56e2ddfa7b2cc04f9034f13fd52a122fc519f9c

tech-stack:
  added: []
  patterns:
    - "多文档 YAML 的 schema 关联：项目级 yaml.schemas 为主，modeline 逐 document 兜底"
    - "编辑器配置只含 schemas 映射，测试断言键集合与路径，防止混入可执行设置或远程 URL"

key-files:
  created:
    - .zed/settings.json
    - .vscode/settings.json
  modified:
    - crates/ontoforge-schema/tests/schema_export.rs
    - examples/ontologies/aviation/objects.yaml
    - examples/ontologies/aviation/links.yaml
    - examples/ontologies/aviation/actions.yaml
    - CONTRIBUTING.md

key-decisions:
  - "在锁定 glob examples/ontologies/**/*.yaml 之后追加排除项 !examples/ontologies/**/ontology.yaml。依据：规划期 headless LSP 实测，只用单个 glob 时 28 个没有 modeline 的 manifest 各报 4 条误报；加排除项后误报为 0，aviation/ontology.yaml 仍由自己的 manifest modeline 校验"
  - "不改 Cargo.toml / Cargo.lock：测试只用 ontoforge-schema 已有的 serde_json、globset、walkdir"

requirements-completed: [SCHEMA-07]

coverage:
  - id: D1
    description: "项目级编辑器配置把定义 schema 关联到 examples/ontologies 下的全部 document，manifest 被排除"
    requirement: SCHEMA-07
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/tests/schema_export.rs#editor_configs_associate_definition_schema_with_examples"
        status: pass
    human_judgment: false
  - id: D2
    description: "aviation 定义文件每个 document 以定义 modeline 开头，validate / lint / IR 等价不变"
    requirement: SCHEMA-07
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/tests/schema_export.rs#aviation_definition_documents_start_with_modeline"
        status: pass
      - kind: integration
        ref: "cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json"
        status: pass
    human_judgment: false
  - id: D3
    description: "CONTRIBUTING.md「编辑器接入」一节说明 modeline 作用域、项目配置、JetBrains 映射、Ctrl+Space"
    verification:
      - kind: other
        ref: "rg 校验六个必需事实均存在于 CONTRIBUTING.md"
        status: pass
    human_judgment: false
  - id: D4
    description: "Zed / VS Code 中在文件末尾新追加的 document 输入 kind: 后 Ctrl+Space 列出 ObjectType / LinkType / ActionType，未知字段标红（UAT Test 2 复测）"
    verification: []
    human_judgment: true
    rationale: "补全弹窗与编辑器内诊断只能在真实编辑器中观察；按 human_verify_mode=end-of-phase 在 phase 末统一复测"

duration: 12min
completed: 2026-10-16
status: complete
---

# Phase 1 Plan 14: 多文档 YAML 的编辑器 schema 关联 Summary

**通过项目级 `yaml.schemas`（Zed 与 VS Code）加逐 document 的 modeline，让 aviation 示例每个 YAML document 都按导出的 JSON Schema 补全和校验，关闭 UAT gap G-1-2。**

## Performance

- **Duration:** 12 min
- **Started:** 2026-10-16T01:54Z（近似）
- **Completed:** 2026-10-16T02:06Z
- **Tasks:** 3
- **Files modified:** 7（新建 2、修改 5）

## Accomplishments

- G-1-2 根因：yaml-language-server 只把 modeline 作用于它所在的那个 document，aviation 文件仅第 1 行有 modeline，后续 document 没有 schema。
- `.zed/settings.json` 与 `.vscode/settings.json` 的 `yaml.schemas` 映射逐字一致，把 `./schemas/ontoforge-definition.schema.json` 关联到 `examples/ontologies/**/*.yaml`，排除 `ontology.yaml`。
- aviation 的 objects.yaml（4）、links.yaml（4）、actions.yaml（2）在每个 `---` 之后补定义 modeline，共 10 行纯注释，IR、诊断、快照不变。
- 两个回归测试：配置形状 / schema 路径 / glob 覆盖 / manifest 排除；modeline 数量等于 document 数。
- CONTRIBUTING.md 新增「编辑器接入」一节（共 7 节）。

## Task Commits

1. **Task 1（tracer）：编辑器配置 + 配置回归测试** - `a839ac2` (feat)。RED：`.zed/settings.json` 缺失时测试失败；GREEN：创建两份配置后 `schema_export` 8 个测试全过。
2. **Task 2：aviation modeline + 回归测试** - `3908f7b` (feat)。RED：在 `actions.yaml:21` 失败；GREEN：补 10 行后通过。
3. **Task 3：CONTRIBUTING.md「编辑器接入」** - `82ca319` (docs)

## Verification

- `cargo test -p ontoforge-schema --test schema_export`：8 passed。
- `cargo test -p ontoforge-schema --test schema_export --test load_examples --test lint_examples`：通过（8 / 3 / 9，另含 1 个 target）。
- `cargo test -p ontoforge --test cli --test cli_lint --test cli_schema` 与 `cargo test -p ontoforge-expr --test criteria`：通过。
- `ontoforge validate examples/ontologies/aviation --color never`：`ok: ontology aviation with 5 object type(s)`，退出码 0。
- `ontoforge lint examples/ontologies/aviation --deny-warnings --format json`：输出 `[]`，退出码 0。
- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace --locked`：全部通过，无 FAILED。
- 验收标准：jq 四条断言通过；modeline 计数 5 / 5 / 3，`---` 计数 4 / 4 / 2；`git diff --numstat` 为 `4 0` / `4 0` / `2 0`；快照目录无变更；`rg -c '^## ' CONTRIBUTING.md` 为 7；`git check-ignore` 对两个配置文件退出码均为 1（未被忽略）。

## Decisions Made

- 排除 glob `!examples/ontologies/**/ontology.yaml` 是对编排决策 1 的补充而非替换，锁定的键与第一个 glob 原样保留。依据是规划期 headless LSP 实测（plan `<context>`）：不加排除项，28 个没有 modeline 的 manifest 各报 4 条误报。
- Task 1 的 human-check（Zed 中追加 document 验证补全与诊断）留待 phase 末 UAT 复测，不阻塞本 plan。
- debug session `.planning/debug/g-1-2-yaml-schema-completion.md` 的归档由 `/gsd-verify-work` 处理，本 plan 未改动。

## Deviations from Plan

None - plan executed exactly as written。

小的实现细节（非偏差）：plan 验收标准中的 `git check-ignore -q a b` 在 git 中对多路径带 `-q` 报错（`--quiet is only valid with a single pathname`），改为对两个文件分别运行，均退出码 1。

## Issues Encountered

None。

## Known Stubs

None。

## Threat Flags

None。T-01-42 / T-01-43 / T-01-44 已由 `editor_configs_associate_definition_schema_with_examples` 与 `aviation_definition_documents_start_with_modeline` 落地。

## Next Phase Readiness

Plan 01-14 为 gap-closure plan。phase 末需人工复测 UAT Test 2（Zed / VS Code 中的补全与标红）。

## Self-Check: PASSED

- 文件存在：`.zed/settings.json`、`.vscode/settings.json`、`crates/ontoforge-schema/tests/schema_export.rs`、三个 aviation 定义文件、`CONTRIBUTING.md`。
- 提交存在：`a839ac2`、`3908f7b`、`82ca319`（`git rev-list --count b56e2dd..HEAD` 为 3）。
