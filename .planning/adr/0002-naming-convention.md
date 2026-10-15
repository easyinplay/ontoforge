# ADR-0002 命名规范

- status: accepted
- date: 2026-10-15

## Context
设计参考了 bkn-foundry / open-ontology / open-foundry 三个项目。只借鉴设计，不沿用它们的标识符（如 `kn_id`、KN、BKN、ODL、Domain Pack、`_tenant_id` 系统列等），以避免出处与许可混淆（bkn-foundry 含 OpenBKN License 文件）。

## Decision
1. **概念词汇** 以 Foundry Ontology 官方术语为准：ontology、object type、property、link type、action type、object set、function、interface、shared property、value type、branch、edit。
2. **schema 实体标识**：
   - `apiName`：用户定义、ontology 内唯一、可读；object type / link type / action type 用 PascalCase 或 camelCase 按 Foundry 惯例（object type `PascalCase`，property / link 端 / parameter `camelCase`）。
   - `rid`：系统生成的不可变资源 ID，格式 `ontoforge.<kind>.<ulid>`，`<kind>` 取 `ontology`、`object-type`、`link-type`、`action-type`、`edit`、`branch` 等。
3. **wire 格式（REST / MCP / SDK JSON）**：字段 `camelCase`。
4. **Rust 代码**：遵循 Rust API Guidelines（类型 `UpperCamelCase`，函数/字段/模块 `snake_case`，crate 名 `ontoforge-<area>` kebab-case）。
5. **数据库**：表名/列名 `snake_case`；系统表前缀 `of_`（如 `of_object`、`of_link`、`of_edit`）；不使用前导下划线列名。
6. **错误码**：`UPPER_SNAKE_CASE`，wire 上为 `{ errorCode, errorName, parameters }`。

## Consequences
review 时出现参考仓库标识符即视为违规。
