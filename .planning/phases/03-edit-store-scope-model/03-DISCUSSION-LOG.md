# Phase 3: Edit Store & Scope Model - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-19
**Phase:** 03-edit-store-scope-model
**Areas discussed:** 角色 / scope 的管理通道、行谓词的 null 与 CEL 子集、edit log 记录内容、reload / migrate 落地形态

---

## 角色 / scope 的管理通道

| Option | Description | Selected |
|--------|-------------|----------|
| roles-as-code | 独立目录 YAML（Role / RoleBinding），CLI plan / apply 写系统库并记管理审计 | ✓ |
| 命令式 CLI 直改系统库 | role create / grant / bind 逐条写库并审计 | |
| 文件为唯一真相 | role 只经文件 + reload 生效，REST 只读 | |

| Option | Description | Selected |
|--------|-------------|----------|
| 绑定入文件，成员来自 IdP | 绑定随 apply 全量同步，user ↔ group 取自 JWT groups claim | ✓ |
| 绑定与本地组都走 CLI / 库 | 文件只定义 role 本身 | |
| 两者都入文件，含本地组 | OIDC groups 与本地组并存 | |

**User's choice:** roles-as-code；绑定入文件，成员来自 IdP。
**Notes:** 接受自决项：principal_id = iss + sub；首次认证自动建 user 记录；claim 映射在 `ontoforge-server.toml`；apply 是 role 唯一写入口，Phase 7 REST 对 role 只读。

---

## 行谓词的 null 与 CEL 子集

| Option | Description | Selected |
|--------|-------------|----------|
| SQL 三值逻辑 | Rust 实现 Kleene 逻辑，unknown → 不可见 | ✓ |
| CEL 语义为准 | SQL 侧用 IS DISTINCT FROM / COALESCE 模拟 | |
| 引用属性为 null 即整体 false | 最保守，但写不出 null 分支规则 | |

| Option | Description | Selected |
|--------|-------------|----------|
| 可编译的中等子集 | 比较、逻辑、== null、in、startsWith / endsWith / contains | ✓ |
| 最小子集 | == != in && \|\| ! 与 == null | |
| 较宽子集 | 再加算术、size、matches | |

**User's choice:** SQL 三值逻辑；中等子集。
**Notes:** 接受自决项：attrs 在映射中声明类型；引用大小写不敏感 collation 列 → E；完整 proptest 只跑 Rust vs SQLite。

---

## edit log 记录内容

| Option | Description | Selected |
|--------|-------------|----------|
| 只存新值 + 操作类型 | 与按属性 patch 同构，可重放重建 edit store | ✓ |
| 旧值 + 新值 | 审计直观，但复制 source 值且旧值可能过时 | |
| 整行叠加快照 | 与历史表重复，重放语义含混 | |

| Option | Description | Selected |
|--------|-------------|----------|
| 按当前 scope 遮蔽 | 只返回当前可见列值，原始参数仅管理通道 | ✓ |
| 仅管理通道可查历史 | 业务应用与 agent 无法展示变更历史 | |
| 按写入时 scope 返回 | 泄露被遮蔽列历史值与参数 | |

**User's choice:** 只存新值 + 操作类型；按当前 scope 遮蔽。
**Notes:** 接受自决项：一条 log = 一次提交，全局 seq + ULID event_id，记 def_version；import 以 system 主体记入；参数原样存储；触发器拒绝 UPDATE / DELETE。

---

## reload / migrate 落地形态

| Option | Description | Selected |
|--------|-------------|----------|
| migrate 时显式声明 rename | `--rename <Type>.<old>=<new>` 搬数据，未声明则归档 | ✓ |
| 定义中加稳定 id | 改名零成本，但改定义格式 | |
| 改名即删除 + 新增 | 已编辑值静默回落 source | |

| Option | Description | Selected |
|--------|-------------|----------|
| 库函数 + 一次性 CLI | `ontoforge reload`（含 `--dry-run`），Phase 7 复用 | ✓ |
| 并入 migrate 命令 | 模糊新增类 / 破坏性边界 | |
| 只交付库函数 | SC1 只能靠测试验收 | |

**User's choice:** 显式 rename；库函数 + 一次性 CLI。
**Notes:** 接受自决项：同表改名归档列（带 def_version 后缀），清理走 migrate 子命令；物理表名 = 固定前缀 + apiName 派生；reload 时重跑 criteria 列覆盖 lint，失败整体不切换。

---

## Claude's Discretion

系统库表结构、Role / RoleBinding 字段与目录命名、dev token 签名与密钥存放、JWKS 刷新策略、历史表快照写入时机、SQLite edit store 与替身库的关系、`postgresql_embedded` 门控方式。

## Deferred Ideas

- Action 参数脱敏（随 Markings 立项）
- 本地自建组
- 定义中的稳定 id
