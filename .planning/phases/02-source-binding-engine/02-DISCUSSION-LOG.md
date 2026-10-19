# Phase 2: Source Binding & Engine - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-16
**Phase:** 02-source-binding-engine
**Areas discussed:** 绑定写在哪里, Trino 连接与凭据, SQLite 替身的数据来源, validate --online 与 drift

---

## 绑定写在哪里

| Option | Description | Selected |
|--------|-------------|----------|
| 独立绑定文件 | 定义纯语义，`kind: SourceBinding` 按 profile 分目录 | ✓ |
| 内联 + profile 覆盖 | 写在 ObjectType 的 `source:`，profile 文件覆盖 | |
| 只内联 | 严格按设计文档，换环境换定义 | |

| Option | Description | Selected |
|--------|-------------|----------|
| 缺省同名 + 显式覆盖 | 只写不同名的列，配 W 级 lint | ✓ |
| 必须逐列写全 | 无隐含规则 | |

| Option | Description | Selected |
|--------|-------------|----------|
| 只绑单表 | 多表拼接由部署方建 view | ✓ |
| 允许写 SQL 片段 | 灵活但破坏单表形态约束 | |

| Option | Description | Selected |
|--------|-------------|----------|
| ontology 里显式声明 backing | 漏配报 E | ✓ |
| 按绑定有无推断 | 漏配静默变 edit-only | |

| Option | Description | Selected |
|--------|-------------|----------|
| 命令行 / 环境变量 + manifest 缺省 | `--profile` / `ONTOFORGE_PROFILE` | ✓ |
| 只用命令行参数 | 无缺省 | |

**Notes:** 独立绑定文件修订 ADR-0015 的「内联 source」一条。

---

## Trino 连接与凭据

| Option | Description | Selected |
|--------|-------------|----------|
| 独立服务配置文件 | 与定义、lint 配置分开 | ✓ |
| 并入 ontoforge.toml | 单文件 | |

| Option | Description | Selected |
|--------|-------------|----------|
| 环境变量或密钥文件路径 | 禁止明文 | ✓ |
| 允许明文 | | |

认证方式（多选）：用户名密码 ✓、无密码（X-Trino-User）✓、JWT / OAuth2 bearer ✓、Kerberos ✗

| Option | Description | Selected |
|--------|-------------|----------|
| 自定义 CA，不提供跳过校验 | | ✓ |
| 自定义 CA + 不安全开关 | 仅 dev | |

| Option | Description | Selected |
|--------|-------------|----------|
| 两套凭据都支持，L2 自检留 Phase 6 | | ✓ |
| 只做 ontology 身份 | | |

---

## SQLite 替身的数据来源

| Option | Description | Selected |
|--------|-------------|----------|
| 示例自带 CSV / JSON fixture | 测试与 serve --dev 共用 | ✓ |
| 现成 SQLite 文件 | | |
| 只在测试代码里构造 | | |

| Option | Description | Selected |
|--------|-------------|----------|
| 手写小规模、覆盖边界 | 完全虚构 | ✓ |
| 脚本生成较大数据 | | |

---

## validate --online 与 drift

| Option | Description | Selected |
|--------|-------------|----------|
| 只接受无损映射 | 有损 E，精度变化 W | ✓ |
| 宽松：能 cast 就算兼容 | | |

| Option | Description | Selected |
|--------|-------------|----------|
| 复用 Phase 1 诊断体系 | 新诊断码、line:col、human / JSON、独立的不可达退出码 | ✓ |
| 独立报告格式 | | |

| Option | Description | Selected |
|--------|-------------|----------|
| 本 phase 做库与状态模型，服务接入留 Phase 7 | | ✓ |
| 本 phase 做完后台定时任务 | | |

---

## Claude's Discretion

Trino 协议细节、printer API、Engine trait 形态（流式 / 整批）、文件与字段命名、CI 容器版本与灌数方式。

## Deferred Ideas

Kerberos；drift 定时执行与 `/health`（Phase 7）；L2 启用与自检（Phase 6）；按主体透传 Trino 身份。
