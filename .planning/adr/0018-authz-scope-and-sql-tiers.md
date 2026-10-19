# ADR-0018 授权：用户 / 组 / 角色、scope 模型与 SQL 级别

- status: accepted（「缺失属性求值为 false」一句 superseded by ADR-0025，2026-10-20；其余不变）
- date: 2026-10-16
- supersedes: ADR-0008 的 AuthZ 部分（AuthN 部分保留：OIDC JWT + 自写 JWKS 缓存、dev 本地 token）
- 依据：`docs/designs/trino-engine-sql-tiers.md` §3、§4；Eng Review D5 / D12 / D13 / D21

## Context
头号验收标准：L0 的任何查询路径和任何 Action 都无法逃逸调用者的 scope。隐藏物理表名不等于授权，行 / 列级安全因此从 v2 提前。

## Decision
- **主体**：user、group（可同步自 OIDC `groups` claim）、role；role 绑定 user 或 group。`principal.attrs` 由 JWT claim 按配置映射，缺失按 null，相关谓词求值为 false（fail-closed）。
- **role 授权项**：可见 object type；每个 type 的属性 allow-list；行谓词（CEL，**只能引用同行属性与 `principal.attrs`**，validate 强制）；action 执行权；SQL 级别；L2 可访问的 catalog / schema；MCP toolset（ADR-0019）。
- **多 role 合并**：行可见 = `R_1 OR … OR R_n`；列 x 输出为 `CASE WHEN (OR 授予 x 的 R_i) THEN x END`。所有表达式只能绑定到这个 scoped 基关系。
- **同源两处执行**：同一份 CEL 行谓词既编译成 SQL（Trino / SQLite），也在 Rust 内对单行求值（PG 直读快速路径）；差分测试强制两者等价。
- **版本令牌**：对外 `_version` = `HMAC(server_key, principal_id ‖ type ‖ pk ‖ 规范化(edit 行 + source 行))`，不透明、不可枚举；密钥轮换使旧令牌一律冲突。
- **Action 鉴权管线**：① 认证 ② action 执行权 ③ 前置读原始行（PG 事务外读 source，再开事务按 (type, pk) 顺序锁 edit 读集）④ 目标行可见（拒绝与不存在返回同一错误）⑤ 写字段 ⊆ 调用者对该行的可见列且不含系统字段 ⑥ submission criteria 只在调用者的遮蔽视图上求值，引用当前行被遮蔽的列即 `ScopeDenied` ⑦ 合成新状态 ⑧ 写后授权：新状态对调用者可见，link 两端都可见，release 后的 source 行可见 ⑨ 单事务提交。比对客户端 expected 令牌在 ③ 完成；不对 source 二次读取，前置读到提交之间的 source 变更不被检测（写入保证边界文档）。
- **配置期 lint**：定义 reload 与 role / scope 变更时，被授予某 action 的 role 列集必须覆盖其 criteria 引用的列，否则 E 级错误、不生效。
- **`command_id`**：作用域 = 主体，唯一键 `(principal_id, command_id)` + `params_hash`；同键异参报 `CommandIdReused`；回放缓存结果前重新鉴权；`get_command_result` 供响应丢失后查询；默认保留 7 天。
- **SQL 级别**：L0 ontology 工具（默认）；L1 Ontology SQL（推迟到下一 milestone）；L2 `sql:raw` 原始 Trino DQL；L3 `sql:admin` 仅 CLI / 管理端，不进 agent MCP。
- **L2 边界**：scope 只到 catalog / schema allow-list，对象级行列安全不作用于 L2（有意设计，界面须明示）。AST 白名单（`sqlparser`，只允许 SELECT / WITH，拒绝 `TABLE()` 表函数、`CALL`、`EXECUTE`、会话 / 事务语句，解析不了即拒绝，执行发送重序列化后的 SQL）；allow-list 硬排除 edit store catalog 与 `system` 元数据表。L2 硬依赖 Trino 侧权限：启动自检只读探测「allow-list 外 catalog、edit store catalog 被拒绝」且「L2 身份读 `system.runtime.queries` 只见自己的查询」，任一无法证明即禁用 L2 并告警。
- **定义与授权的变更权**：ontology 定义以文件为准，仅部署方 reload 生效；role / scope 只能经管理端 / L3 修改并审计；agent MCP 没有任何变更工具；每个请求固定使用开始时的定义版本。

## Rejected
- criteria 在原始值上求值：dry run 可二分探测被遮蔽列。
- PG 快速路径改走 PG 方言 scoped SQL：需第三个 renderer，引用 link 的谓词还得在 PG 内 join 不在 PG 的 source。
- 内容指纹作为 `_version`：低基数被遮蔽列可被枚举反推。
- 行谓词引用 link / 其他对象：推迟到下一 milestone。

## Consequences
越权 conformance 套件覆盖：换 ID、伪造主体属性、多 role 列 × 行交叉、谓词 / 聚合反推、令牌反推、dry run 探测、写后不可见、L2 越界与 system 表；替身与 Trino 两端都跑。
