---
status: diagnosed
trigger: "G-1-2: Zed (Windows) objects.yaml 末尾新文档输入 kind: 无补全、无错误标记"
created: 2026-10-16T00:00:00Z
updated: 2026-10-16T02:30:00Z
goal: find_root_cause_only
symptoms_prefilled: true
---

## Current Focus

hypothesis: CONFIRMED H1 — modeline 只作用于其所在的 document；`---` 之后的 document 无 schema
test: done（1.25.0 与 2.0.0 headless LSP，8 个 case + yaml.schemas 关联对照）
expecting: n/a
next_action: 返回 ROOT CAUSE FOUND 给 orchestrator（plan-phase --gaps 处理修复）
bug_class: Bohrbug (deterministic)
candidate_causes:
  - "data/example: modeline 只写在多文档文件第 1 行 → 只覆盖第 1 个 document（CONFIRMED）"
  - "code/schema: anyOf/oneOf+const/$defs/2020-12 不被支持（ELIMINATED）"
  - "environment: Zed 未启动 LSP / node 缺失 / Windows 相对路径解析（ELIMINATED）"
  - "config: Zed 设置覆盖 yaml.validate/completion（ELIMINATED，settings.json 无 yaml 配置）"
and_gate: "no — 单一条件充分：无 schema 时 completion 与 diagnostics 都为空。附带 UX 因素：server completionProvider 无 triggerCharacters，`kind: ` 末尾空格后编辑器不会自动弹窗，需 Ctrl+Space 或键入首字母（不影响 diagnostics，不是根因）"

## Symptoms

expected: objects.yaml 末尾追加 `---` + `kind: ` 时补全 ObjectType / LinkType / ActionType，按 kind 收窄字段，未知字段标红
actual: Zed (Windows) 中末尾输入 kind: 无补全弹窗，也无错误标记
errors: none
reproduction: UAT Test 2
started: UAT 发现

## Evidence

- timestamp: 2026-10-16
  checked: .planning/debug/knowledge-base.md
  found: 不存在，无 KB 匹配
  implication: 无已知模式

- timestamp: 2026-10-16
  checked: examples/ontologies/aviation/*.yaml 首行与 `---` 数量
  found: 四个文件第 1 行均为 modeline；objects.yaml 4 个 `---`（5 个 doc），links.yaml 4 个，actions.yaml 2 个
  implication: 示例是多文档 YAML；modeline 只在第一个 doc 的注释里

- timestamp: 2026-10-16
  checked: schemas/ontoforge-definition.schema.json
  found: 根 anyOf [ $ref Definition, array of Definition ]；Definition = oneOf 三分支，每支 properties.kind.const + additionalProperties:false；draft 2020-12，$defs
  implication: 结构本身是 yaml-language-server 支持的常见形状，需实测

- timestamp: 2026-10-16
  checked: %LOCALAPPDATA%/Zed/logs/Zed.log + %LOCALAPPDATA%/Zed/languages/yaml-language-server + %APPDATA%/Zed/settings.json
  found: Zed 2026-10-16T08:39:58 在 D:\DataSpellProjects\ontoforge 启动 yaml-language-server（node C:\Program Files\nodejs\node.exe，包版本 1.25.0），之后无 lsp 报错；Zed settings.json 的 lsp 块只有 basedpyright，无 yaml 配置
  implication: Zed 侧 LSP 正常运行、用默认设置；"LSP 没启动 / node 缺失" 被排除

- timestamp: 2026-10-16
  checked: yaml-language-server 1.25.0 out/server/src/languageservice/services/modelineUtil.js + parser/yamlParser07.js；2.0.0 out/src/languageservice 同名文件
  found: getSchemaFromModeline(doc) 只查 doc.documentHeaderComments；parse() 中 getDocumentHeaderComments(tokens) 按 document token 切分，第 i 个 document 只拿到紧邻它之前（上一个 doc-end/`---` 之后）的注释。2.0.0 逻辑相同
  implication: modeline 是 per-document 的，只作用于其所在的第一个 document；`---` 之后的 document 没有 schema（除非另有 yaml.schemas 关联）

- timestamp: 2026-10-16
  checked: headless LSP（C:/tmp/of-yamlls/scripts/drive.js，副本 ws 同相对布局，file:///C:/... URI，yaml.validate/completion=true, schemaStore off）对 yaml-language-server 1.25.0 与 2.0.0
  found: |
    两版输出一致：
    append_kind（HEAD objects.yaml + `---\nkind: `）: diagnostics 0, completion 0  ← 精确复现 UAT
    append_unknown（追加 doc 含 primaryKy）: diagnostics 0
    doc1_doc2_unknown: 只报 L3 "Property bogusDoc1 is not allowed."，doc 2 的 bogusDoc2 未报
    single_kind（modeline + `kind: `）: completion ObjectType | LinkType | ActionType；5 条诊断
    append_with_modeline（追加 doc 自带 modeline）: completion 同上；诊断 5 条
    single_key_narrow（kind: ObjectType 后空行）: completion 只含 ObjectType 字段（apiName…properties）
    append_modeline_unknown: L72 "Property primaryKy is not allowed."
    completionProvider = {"resolveProvider":false}（无 triggerCharacters）
  implication: schema 本身与 modeline 相对路径在 Windows 下都正常；缺的只是追加 document 的 schema 关联

- timestamp: 2026-10-16
  checked: yamlSchemaService.getSchemaForResource 优先级 + YAML_SCHEMAS='{"./schemas/ontoforge-definition.schema.json":["examples/ontologies/**/*.yaml"]}' 对照运行
  found: 优先级 modeline > $schema > customSchemaProvider > yaml.schemas glob；glob 关联后 append_kind 有 3 项补全与诊断，bogusDoc2 被报，ontology.yaml 仍用 manifest schema（只报 bogusManifestKey 1 条）
  implication: resource 级 glob 关联覆盖文件内全部 document，且不会把 manifest 误配为 definition schema

- timestamp: 2026-10-16
  checked: 上游 issue（gh api）
  found: redhat-developer/vscode-yaml#1066 [open] "YAML autocompletion and validation works only for the first YAML definition in a file (schema through modeline)"；yaml-language-server#319 [open] "Provide a way to map schemas to multidocument yaml file"
  implication: 已知上游限制，非 ontoforge schema bug，也非 Zed 特有

- timestamp: 2026-10-16
  checked: 01-09-PLAN.md Task 2 step 3、01-RESEARCH.md A9、crates/ontoforge/tests/cli_schema.rs
  found: plan 规定 modeline 只加在第一行；RESEARCH 只把 oneOf+const 收窄列为风险（实测无问题）；没有任何测试覆盖 "多文档文件第 2+ 个 document 是否获得编辑器 schema"（aviation_documents_validate 用 Rust jsonschema 逐 doc 校验，绕过 modeline 机制）
  implication: why-not-caught：编辑器关联机制无自动 gate，headless 执行时 01-09 也明确未做编辑器验证

## Eliminated

- hypothesis: schema 构造（root anyOf / Definition oneOf+const / $defs / draft 2020-12）不被 yaml-language-server 支持
  evidence: single_kind / append_with_modeline 给出 3 个 kind 补全与正确诊断；single_key_narrow 收窄到 ObjectType 字段
  timestamp: 2026-10-16

- hypothesis: Zed 侧 LSP 未运行 / node 缺失 / Windows 下 modeline 相对路径解析失败
  evidence: Zed.log 08:39:58 启动 yaml-language-server 无报错；headless 下 file:///C:/... URI + ../../../schemas 相对路径解析成功
  timestamp: 2026-10-16

- hypothesis: Zed 用户设置关闭了 validate/completion
  evidence: %APPDATA%/Zed/settings.json lsp 块只有 basedpyright
  timestamp: 2026-10-16

## Resolution

root_cause: yaml-language-server（Zed 实装 1.25.0，最新 2.0.0 相同）只从每个 YAML document 自己的 header 注释读取 `# yaml-language-server: $schema=` modeline（modelineUtil.getSchemaFromModeline → doc.documentHeaderComments，按 `---` 切分）。aviation 示例是多文档文件，modeline 只写在第 1 行，因此只有第 1 个 document 关联 schema；UAT 在 objects.yaml 末尾追加的 `---` + `kind:` 是第 6 个 document，没有任何 schema → completion 与 diagnostics 均为空。同一原因下现有 objects.yaml/links.yaml 第 2-5 个 document、actions.yaml 第 2-3 个 document 在编辑器里也从未被校验。schema 本身、Windows 路径、Zed LSP 均无问题。
fix: (未应用，find_root_cause_only)
verification: n/a
files_changed: []
