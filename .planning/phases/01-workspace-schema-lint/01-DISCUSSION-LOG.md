# Phase 1: Workspace, Schema & Lint - Discussion Log

> **只是审计记录。** 不要作为规划、研究、执行 agent 的输入。
> 决策写在 CONTEXT.md 里，这份日志只保存当时考虑过的备选方案。

**Date:** 2026-10-15
**Phase:** 01-workspace-schema-lint
**Areas discussed:** 文件布局、定义文档形状、action 值来源写法、lint 配置格式、示例领域

本 phase 的灰色地带都是技术实现问题，没有逐项询问用户。依据是用户 2026-10-15 的指示：「需要抉择的自行调研得出方案，必须由我决定的才来问，问的时候给出每个选项的优缺点和推荐项」。

---

## 文件布局

| Option | Description | Selected |
|--------|-------------|----------|
| manifest + 任意 glob | 根目录放 `ontology.yaml`，其余文件位置自由 | ✓ |
| 固定目录约定 | `object-types/`、`link-types/`、`action-types/` 分目录存放 | |
| 单文件 | 整个 ontology 写在一个文件里 | |

**Choice:** manifest + 任意 glob。固定目录给开发者的约束太多，单文件在规模变大后无法维护。

## 定义文档形状

| Option | Description | Selected |
|--------|-------------|----------|
| `kind` 判别的扁平文档 | 每个文档一个实体，类似 k8s 的写法 | ✓ |
| 按类型分组的嵌套 map | `objectTypes: {...}`、`linkTypes: {...}` | |

**Choice:** `kind` 判别。每个文档独立，合并时不会冲突，JSON Schema 用 `oneOf` 表达也更清楚。

## Action 值来源写法

| Option | Description | Selected |
|--------|-------------|----------|
| 结构化 `{ param: x }` | 可以静态做类型检查 | ✓ |
| 字符串模板 `${params.x}` | 写起来短，但要再引入一种语法 | |
| 全部写成 CEL | 统一，但对简单赋值来说太重，也很难静态判断值的来源 | |

**Choice:** 结构化写法。CEL 只用于 submission criteria。

## Lint 配置格式

| Option | Description | Selected |
|--------|-------------|----------|
| `ontoforge.toml` | 符合 Rust 工具链习惯，和 YAML 定义文件区分开 | ✓ |
| 写在 manifest 的 `lint:` 段 | 少一个文件，但工具配置和 ontology 契约混在一起 | |

**Choice:** `ontoforge.toml`。

## 示例领域

| Option | Description | Selected |
|--------|-------------|----------|
| 航空（Airport / Aircraft / Flight / Pilot） | 与 Foundry 文档一致，三种基数都能覆盖 | ✓ |
| 通用 Todo / 项目管理 | 简单，但覆盖不到 M:N 等复杂关系 | |

**Choice:** 航空。

## Claude's Discretion

crate 内部模块划分、IR 的 Rust 表示、CLI 参数命名、JSON Schema 导出命令的形式。

## Deferred Ideas

- ONT-W06 基于空值率的检测：要等存储层就位。
- LSP / 编辑器插件。
- `ontoforge init` 脚手架命令。
