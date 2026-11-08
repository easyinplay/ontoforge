---
phase: "01"
slug: "workspace-schema-lint"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-16"
---

# Phase 01 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| 定义文件 → loader | 本地 YAML / JSON ontology 定义、`ontoforge.toml` lint 配置，作者可控、视为不可信输入 | 结构化文本；可能含深层嵌套、alias、超大文件、路径逃逸、控制字符 |
| CEL 表达式 → expr 编译器 | 定义文件中的 submission criteria / 规则表达式 | 表达式源码；本 phase 只编译不求值 |
| CLI → 终端 / JSON 输出 | 诊断渲染到用户终端或机器可读 JSON | 回显的名字、路径、消息；终端转义风险 |
| 依赖供应链 | crates.io 依赖与 GitHub Actions | Cargo.lock 锁定的 crate、CI action |
| 编辑器配置 | `.zed/settings.json`、`.vscode/settings.json` 与 YAML modeline | 本地 schema 路径映射 |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-01-01 | DoS | loader/parse.rs 深层嵌套 | medium | mitigate | max_depth=32；CLI 32 MiB 栈线程；1 MiB 栈回归测试 | closed |
| T-01-02 | Tampering | YAML tag / merge key | low | mitigate | reject_unsupported_tags、Merge/Duplicate=Error、strict_booleans | closed |
| T-01-03 | Spoofing | render.rs 终端转义注入 | medium | mitigate | 等长替换 C0/C1/DEL/bidi；JSON 由 serde_json 转义 | closed |
| T-01-04 | Info Disclosure | Diagnostic.file / 快照 | low | mitigate | 相对路径 + `/`，不 canonicalize | closed |
| T-01-SC (01-01) | Tampering | Cargo.toml 新依赖 | high | mitigate | 仅 Legitimacy Audit OK 的 crate；heck/anstream 经 01-REUSE-AUDIT 核验；Cargo.lock 入库 | closed |
| T-01-05 | EoP | ci.yml | medium | mitigate | permissions contents:read；无 secrets；无 pull_request_target | closed |
| T-01-SC (01-02) | Tampering | cel 依赖树 | high | mitigate | 仅新增 cel 0.15.0；CI `--locked` | closed |
| T-01-06 | Tampering | 第三方 GitHub Action | low | accept | 见 Accepted Risks；实际已钉死 commit SHA | closed |
| T-01-07 | Info Disclosure | include glob 路径逃逸 | medium | mitigate | 拒绝绝对路径、盘符、`..`（V14） | closed |
| T-01-08 | Info Disclosure | 符号链接 | medium | mitigate | follow_links(false) + symlink_metadata 跳过 | closed |
| T-01-09 | DoS | 栈溢出（库内） | medium | mitigate | 32 MiB 作用域线程；1 MiB 栈测试 | closed |
| T-01-10 | DoS | 超大文件 / alias bomb / 文档风暴 | medium | mitigate | 4 MiB 上限；serde-saphyr Budget 默认限额 | closed |
| T-01-11 | Tampering | JSON 宽松解析 | low | mitigate | serde_json 严格预检（V01） | closed |
| T-01-12 | Tampering | 未知字段 | low | mitigate | deny_unknown_fields（V04） | closed |
| T-01-13 | Tampering | 主键不变量 | medium | mitigate | 不可关闭的 V10 | closed |
| T-01-14 | Spoofing | help 回显用户输入 | low | mitigate | render 净化 + serde_json 转义 | closed |
| T-01-15 | Tampering | 外键与主键类型不一致 | medium | mitigate | 不可关闭的 V11 | closed |
| T-01-16 | Tampering | link 端名与属性同名 | low | mitigate | V08 | closed |
| T-01-17 | DoS | link 重名检查 | low | accept | 见 Accepted Risks | closed |
| T-01-18 | Tampering | modifyObject 改主键 | medium | mitigate | 不可关闭的 V12 | closed |
| T-01-19 | Tampering | 字面量校验 | low | mitigate | 按 base type 严格校验 static / default | closed |
| T-01-20 | EoP | currentUser / currentTime 来源 | low | mitigate | 仅允许赋给 string / timestamp | closed |
| T-01-21 | DoS | chrono 日期解析 | low | accept | 见 Accepted Risks | closed |
| T-01-22 | DoS | CEL 深层嵌套 | high | mitigate | 长度 ≤ 4096、嵌套 ≤ 32 预检（按 CEL 词法）；32 MiB 线程 | closed |
| T-01-23 | DoS | walk.rs 递归 / 闭包 panic | low | mitigate | 大栈线程内运行；panic → ExprError::Internal | closed |
| T-01-24 | Tampering | 表达式求值 | low | accept | 见 Accepted Risks | closed |
| T-01-25 | Tampering | cel 传递依赖 antlr4rust | medium | mitigate | Cargo.lock 锁定；仅 compile 与 AST 读取 | closed |
| T-01-26 | Tampering | lint 配置关闭结构性校验 | high | mitigate | ONT-V 键报 V15；validate 诊断不经级别解析 | closed |
| T-01-27 | Tampering | 配置拼写错误静默失效 | medium | mitigate | 未知 section / 规则 / 级别 / 选项均报 V15 | closed |
| T-01-28 | Info Disclosure | `--config` 任意路径 | low | accept | 见 Accepted Risks | closed |
| T-01-29 | DoS | lint 正则 | low | mitigate | 固定 LazyLock 模式，regex crate 线性时间 | closed |
| T-01-30 | Tampering | schema 与 loader 漂移 | medium | mitigate | schema_for! 生成 + committed_schemas_are_up_to_date | closed |
| T-01-31 | Tampering | `--out-dir` 覆盖 | low | accept | 见 Accepted Risks | closed |
| T-01-32 | DoS | jsonschema 远程引用 | low | mitigate | default-features=false；仅本地 `#/` 引用 | closed |
| T-01-33 | DoS | w03 正则 | low | mitigate | 固定模式；名字 ≤ 100 字符 | closed |
| T-01-34 | DoS | w04 跨类型收集 | low | accept | 见 Accepted Risks | closed |
| T-01-35 | DoS | w07 / w08 组合枚举 | low | mitigate | W08 先过滤 ≥ 3 类型的键再枚举 | closed |
| T-01-36 | Tampering | 配置选项值 | low | mitigate | 仅 option_keys + 类型检查（V15） | closed |
| T-01-37 | DoS | w09 / w10 / w11 正则 | low | mitigate | 固定模式 | closed |
| T-01-38 | DoS | infer.rs / check.rs 递归 | low | mitigate | analyze 闭包内大栈线程 | closed |
| T-01-39 | Tampering | V13 消息回显 | low | mitigate | 只回显名字与路径段；render 净化 | closed |
| T-01-40 | Tampering | toml 日期键 | low | mitigate | 公开 DeValue::Datetime | closed |
| T-01-41 | Tampering | JSON 输出混入 ANSI | low | mitigate | 颜色只影响 human；测试检查无 0x1b | closed |
| T-01-42 | EoP | 编辑器配置 | medium | mitigate | 测试断言 Zed / VS Code 配置键精确、只含 schemas | closed |
| T-01-43 | Info Disclosure | yaml.schemas 键 | low | mitigate | 必须 `./schemas/` 开头、文件存在且属于 rendered_schemas() | closed |
| T-01-44 | Tampering | modeline 与 schema 漂移 | low | mitigate | 每个 document 有 modeline 且路径存在 | closed |
| T-01-SC (01-14) | Tampering | 依赖安装 | low | accept | 见 Accepted Risks | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

逐条证据（file:line）见审计记录：gsd-security-auditor 2026-10-16 返回的 Threat Verification 表，要点已压缩进 Mitigation 列。

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01-1 | T-01-06 | 使用官方 / 广泛使用的 action；实际已钉死 commit SHA（ci.yml:19-25），残余风险更低 | 01-02-PLAN | 2026-10-16 |
| AR-01-2 | T-01-17 | 按 object type 分组的哈希检查为线性复杂度，规模受 01-03 文件大小与预算限制 | 01-05-PLAN | 2026-10-16 |
| AR-01-3 | T-01-21 | chrono 解析线性复杂度，输入长度受 01-03 文件大小上限约束 | 01-06-PLAN | 2026-10-16 |
| AR-01-4 | T-01-24 | 本 phase 只编译不求值（D-09），无执行面；workspace 无 `.execute(` 调用 | 01-12-PLAN | 2026-10-16 |
| AR-01-5 | T-01-28 | 本地 CLI 以调用者权限读取；诊断原样显示调用者给出的路径 | 01-08-PLAN | 2026-10-16 |
| AR-01-6 | T-01-31 | 本地 CLI 只写 rendered_schemas() 的固定文件名到调用者指定目录 | 01-09-PLAN | 2026-10-16 |
| AR-01-7 | T-01-34 | 线性遍历全部属性，规模受 01-03 限制 | 01-10-PLAN | 2026-10-16 |
| AR-01-8 | T-01-SC (01-14) | 01-14 不新增 crate；2ba8a1d 之后 Cargo.toml / lock 未变 | 01-14-PLAN | 2026-10-16 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-16 | 47 | 47 | 0 | gsd-security-auditor（opus，ASVS L1，block_on high） |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-16
