# Walking Skeleton — ontoforge

**Phase:** 1
**Generated:** 2026-10-15

> 模板适配说明：本项目 Phase 1 是 Rust 库 + CLI，没有 Web UI、数据库和部署目标。模板中的 Routing / Database / UI / Deployment 四项按下表改写为 CLI 等价物：「路由」= CLI 子命令；「数据层」= 定义文件 → IR 的加载流水线（存储层在 Phase 2，见 ADR-0004）；「UI 交互」= 终端诊断输出；「部署」= 本机可复现的运行命令 + CI。

## Capability Proven End-to-End

开发者在 `examples/ontologies/aviation/` 写一个 ObjectType YAML，运行 `cargo run -p ontoforge -- validate examples/ontologies/aviation` 得到退出码 0；把 `primaryKey` 写错成 `primaryKy` 时，同一命令输出 `ONT-V04`、`objects.yaml` 与写错 key 的 line:col、`did you mean` 建议，退出码 1；Windows 本机 `cargo test --workspace` 全绿，CI 在 windows-latest 与 ubuntu-latest 跑 fmt / clippy / test。

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| 语言与工具链 | Rust 1.98.0（`rust-toolchain.toml` 钉死）、edition 2024、resolver 3 | PROJECT.md Constraints；钉版本让 Windows 本机与 CI 行为一致（RESEARCH A12） |
| Workspace 布局 | `crates/*` 下 10 个 `ontoforge-*` crate，依赖方向 schema ← expr / query ← storage ← action ← server ← cli | ADR-0011；Phase 1 只实现 schema、expr、ontoforge（CLI），其余 7 个为可编译骨架 |
| 依赖管理 | 外部依赖版本集中在根 `Cargo.toml` `[workspace.dependencies]`；crate 间依赖用 `path`；`Cargo.lock` 入库 | D-17；`Cargo.lock` 入库保证可复现，且 01-01/01-02 之后各 plan 不再改依赖，避免并行 worktree 冲突 |
| 定义文件格式 | 一个 ontology 一个目录，根目录 manifest `ontology.yaml`；其余文件为带 `kind` 判别的文档（YAML 多文档 / JSON 对象或数组） | D-01、D-02；字段级形状以 01-01 checkpoint 确认后的 ADR-0012 为准 |
| 解析流水线 | `serde-saphyr` `Spanned` 节点树 → `serde_json::Value`（`kind` 重写为 externally tagged）→ `serde_path_to_error` 类型化反序列化 → 按路径回查 span | ADR-0003 + Spike S1；YAML 与 JSON 同一路径，单一 IR |
| IR 与 span | IR 类型同时是 serde 反序列化目标、`schemars` 导出源和下游契约，不含 span；span 存于并行的 `SourceMap` | 「YAML 与 JSON 得到同一 IR」可直接 `assert_eq!`；下游 phase 只读 IR |
| 诊断模型 | `Diagnostic { code, severity, message, file, line, column, span, help }`；validate 码段 `ONT-V01..V15`（hard，不可配置），lint 码 `ONT-E08/E09/W01..W11`（可配置） | D-06、D-10；码一经发布不复用 |
| 诊断文案语言 | English（与 serde / clap / miette 原生消息一致，便于 LLM 与国际用户） | Claude's Discretion；项目文档仍为中文 |
| CLI | `clap` derive；`ontoforge` crate 的 `lib.rs` 负责编排（`pipeline::check`），`main.rs` 只解析参数并在 32 MiB 栈线程里调用 | Pattern 7；后续 server / MCP 复用同一编排函数 |
| 栈安全 | 库内部在独立大栈线程里执行 YAML 解析与 CEL 编译，并预限深度与长度 | Pitfall 1（Windows debug 1 MiB 主线程栈溢出会直接 abort） |
| 渲染 | 人类可读：`miette` `GraphicalReportHandler`（`--color auto|always|never`，尊重 `NO_COLOR`）；`--format json`：仅 stdout 输出 JSON 数组 | D-10、Pattern 5；库 crate 不依赖 miette |
| 测试 | `cargo test` + `insta` 快照 + CLI 集成测试用 `env!("CARGO_BIN_EXE_ontoforge")`；`examples/ontologies/invalid/<code>-<slug>/` 作为数据驱动诊断 fixture | D-15、D-16；零额外测试依赖 |
| 行尾 | `.gitattributes`：`* text=auto eol=lf`，快照与 fixture 显式 `eol=lf` | D-17；本机 `core.autocrlf=true` |
| CI | GitHub Actions，matrix `windows-latest` + `ubuntu-latest`，`permissions: contents: read` | D-18；DX-03 回归门槛 |
| Data layer | Phase 1 不涉及 | 存储在 Phase 2（ADR-0004） |
| Auth | Phase 1 不涉及 | Phase 5（ADR-0008） |
| Deployment target | 无服务部署；本机运行命令 `cargo run -p ontoforge -- validate <dir>`，CI 为自动化门禁 | Phase 6 才有 `ontoforge serve --dev` |

## Stack Touched in Phase 1

- [ ] Project scaffold：workspace、10 个 crate、`rust-toolchain.toml`、`.gitattributes`、clippy / rustfmt / 测试运行器（01-01、01-02）
- [ ] 「路由」：CLI 子命令 `validate`（01-01）、`lint`（01-08）、`schema export`（01-09）
- [ ] 「数据层」：YAML / JSON → Node 树 → IR 的真实读取，含多文件合并与 SourceMap（01-01 起，01-03 完善）
- [ ] 「UI」：终端诊断输出（miette 人类可读 + JSON），带 file:line:col 与 help（01-01 起）
- [ ] 「部署」：本机运行命令 + GitHub Actions CI（01-02）

## Out of Scope (Deferred to Later Slices)

- 存储、edit log、`ontoforge import`（Phase 2）
- action 执行、CEL 求值（Phase 3；Phase 1 只做 CEL 编译与静态检查）
- object set 查询（Phase 4）、认证授权（Phase 5）、REST / OpenAPI / client（Phase 6）、MCP（Phase 7）
- ONT-W06 基于空值率的检测、LSP / 编辑器插件、`ontoforge init` 脚手架（CONTEXT Deferred Ideas）
- interface、shared property、value type、struct、派生属性（v2，`kind` 预留但报 v1 不支持）
- 项目 license 选择（未决定，`Cargo.toml` 暂不写 `license` 字段）

## Subsequent Slice Plan

每个后续 phase 在此骨架上加一个纵向能力，不改动上表的架构决策：

- Phase 2：对象与 link 持久化到 SQLite / PostgreSQL，不可变 edit log，`ontoforge import`
- Phase 3：经 action 写入（参数校验、rules 单事务、CEL submission criteria 求值）
- Phase 4：ObjectSet 查询（过滤、遍历、集合运算、聚合）
- Phase 5：OIDC / dev token 认证与 RBAC
- Phase 6：REST + OpenAPI + Rust / TS client，`ontoforge serve --dev`
- Phase 7：MCP server（stdio + streamable HTTP）
