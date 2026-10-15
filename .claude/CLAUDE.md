<!-- GSD:project-start source:PROJECT.md -->

## Project

**ontoforge**

ontoforge 是一个用 Rust 实现、可自托管的开源 Ontology 操作层，复刻 Palantir Foundry Ontology 的核心能力：用 YAML/JSON 定义 object type / link type / action type（ontology-as-code），对象数据只能通过带校验的 Action 写入并留下不可变 edit log，再用 object set 查询（过滤、遍历、集合运算、聚合）读取。同一份 ontology 同时通过 REST（配 Rust / TypeScript client）服务业务应用，通过 MCP 服务 AI agent。

**Core Value:** 定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 两个同等地位的出口，对同一份对象图做受控的读写：写入一律经 Action 校验与授权，每次写入都可审计。

### Constraints

- **Tech stack**：Rust 1.98，edition 2024，cargo workspace（ADR-0011）；axum + utoipa、sqlx、sea-query、cel、rmcp — 已在 ADR-0003～0011 锁定
- **Platform**：必须在 Windows 11 上 `cargo build` / `cargo test` 通过（本机不依赖 Docker），生产部署目标 Linux
- **Testing**：Windows 本机默认测试只依赖 SQLite；Postgres 测试本机用 `postgresql_embedded`，CI 用 service container
- **Naming**：项目 / crate 前缀 `ontoforge`；标识符遵循 ADR-0002，禁止沿用参考仓库的标识符
- **License**：不复制参考仓库代码（bkn-foundry 含 OpenBKN License 文件）

<!-- GSD:project-end -->

<!-- GSD:stack-start source:STACK.md -->

## Technology Stack

Technology stack not yet documented. Will populate after codebase mapping or first phase.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
