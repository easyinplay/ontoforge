# edit store 部署与运维

本文档面向部署和运维 ontoforge 的人，说明 edit store 是什么、怎么配置、PostgreSQL 与 Trino 侧需要哪些权限和设置，以及 `reload`、`migrate`、`scope`、`import`、`dev-token` 的用法和退出码。设计依据见 `.planning/adr/0016-storage-system-data-and-typed-edit-store.md`（形态）、`0024-edit-store-physical-format.md`（物理格式与 schema 划分）、`0026-edit-log-format.md`（edit log 与 append-only）。行谓词与 role 的写法见 [docs/scope.md](scope.md)。

## edit store 是什么

edit store 保存 ontology 对象的写入结果和审计痕迹：

- 每个 object type 一张强类型 edit 表（`of_obj_<type>`），每个多对多 link type 一张 link 表（`of_lnk_<link>`）。表由 `ontoforge reload` 按 ontology 定义生成，不手写 DDL。
- 每次提交在 `of_edit_log`、`of_edit_log_entry`、`of_edit_history` 追加不可变记录；管理操作（reload、migrate、scope apply）写 `of_admin_audit`。
- 定义快照（`of_def_version`）、user、role、role binding 等系统数据也在这里。

两种后端：

| 后端 | 用途 | 说明 |
|---|---|---|
| SQLite | 本机开发、单机试用 | 一个独立文件，不依赖外部服务。Trino 看不到它 |
| PostgreSQL | 生产 | 两个 schema；Trino 通过 PostgreSQL connector 读取 edit 表 |

## 配置

edit store 写在服务配置（`ontoforge-server.toml`，命令行的 `--server-config`）的 `[store]` 段：

```toml
[store]
kind = "postgres"          # 或 "sqlite"
url_env = "EDIT_STORE_URL" # PostgreSQL 连接串所在的环境变量；也可用 url_file
# path = "edit.db"         # kind = "sqlite" 时的文件，相对配置文件
schema_edit = "ontoforge_edit"  # 默认值
schema_sys = "ontoforge_sys"    # 默认值
lock_timeout_ms = 5000          # 默认值
probe_timeout_ms = 60000        # 默认值

[store.trino]
catalog = "editstore"      # Trino 中映射 schema_edit 的 catalog；不写则不做 Trino 可见探测
```

连接串不写进配置文件，只通过 `url_env` 或 `url_file` 引用。

- `lock_timeout_ms`：等待锁的上限，同时约束 reload、migrate、scope apply 取得 advisory lock 的等待。超时报错，已有定义保持不变，可重试或调大。
- `probe_timeout_ms`：reload 等待 Trino 看到新表与新列的上限，见下文「reload」。
- `[store.trino]`：只对 PostgreSQL 后端有意义。

## PostgreSQL

### 两个 schema

| schema | 内容 | Trino |
|---|---|---|
| `ontoforge_edit` | `of_obj_*`、`of_lnk_*` | 可见：只授 `USAGE` 与 `SELECT` |
| `ontoforge_sys` | 定义、`_sqlx_migrations`、日志、历史、user、role、审计 | 不可见：Trino 账号无任何权限 |

user、role、日志含主体标识与授权规则，Trino 账号读到它们等于绕过 scope，所以必须留在 `ontoforge_sys`，且不授权给 Trino 账号。

### 账号划分与最小权限

建议两类账号：

- 维护账号：执行 `reload`、`migrate`、`scope apply`。它拥有（或有 `CREATE` 权限的）两个 schema 和其中的表，负责建表、改表、跑 sqlx 迁移。
- 服务进程账号：应用和 `import` 运行时使用。它不是日志表的所有者。

服务进程账号对 edit 表需要 `SELECT`、`INSERT`、`UPDATE`；对日志、历史、审计表只给 `INSERT` 与 `SELECT`：

```sql
GRANT USAGE ON SCHEMA ontoforge_edit, ontoforge_sys TO ontoforge_svc;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA ontoforge_edit TO ontoforge_svc;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA ontoforge_sys TO ontoforge_svc;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA ontoforge_sys TO ontoforge_svc;

-- append-only：日志、历史、审计表不允许改写、删除或清空
REVOKE UPDATE, DELETE, TRUNCATE ON
  ontoforge_sys.of_edit_log,
  ontoforge_sys.of_edit_log_entry,
  ontoforge_sys.of_edit_history,
  ontoforge_sys.of_admin_audit
FROM ontoforge_svc;
```

注意：

- 表上的触发器（`of_reject_mutation()`）拒绝 UPDATE、DELETE、TRUNCATE，但**对表的所有者不是安全边界**：所有者可以 `ALTER TABLE ... DISABLE TRIGGER` 或 `DROP TRIGGER`（ADR-0026）。所以服务进程账号不能是这些表的所有者，`REVOKE` 才是真正的防线。
- `ALL TABLES` 只覆盖已存在的表。新表由维护账号创建，需要用 `ALTER DEFAULT PRIVILEGES` 补授权，或在每次 `reload` 之后重新执行上面的 `GRANT`，然后重新执行 `REVOKE`。
- 对 `ontoforge_sys` 的 `GRANT ... ON ALL TABLES` 之后必须重新 `REVOKE`，顺序不能反。
- 触发器和 `REVOKE` 防的是误操作和被攻破的服务进程，不防拥有 `CREATE ROLE`、超级用户或表所有者身份的人。

### Trino 账号

```sql
GRANT USAGE ON SCHEMA ontoforge_edit TO ontoforge_trino;
GRANT SELECT ON ALL TABLES IN SCHEMA ontoforge_edit TO ontoforge_trino;
```

不授予 `ontoforge_sys` 的任何权限。同样需要对后来新增的表补授权。

## Trino

edit store 的 catalog 使用 PostgreSQL connector，指向存放 `ontoforge_edit` 的数据库，示例见 `ci/trino/catalog/editstore.properties`：

```properties
connector.name=postgresql
connection-url=jdbc:postgresql://<host>:5432/<database>
connection-user=ontoforge_trino
connection-password=<从 secret 注入>
postgresql.array-mapping=AS_ARRAY
```

要点：

- **`postgresql.array-mapping=AS_ARRAY`**：edit 表的数组列（array 类型属性）在默认设置下被 connector 跳过，Trino 里看不到这些列。reload 的可见探测和任何读取这些列的查询都依赖这个设置，生产部署同样需要。
- **不开启数据或结果缓存**：edit 表是被持续写入的，metadata 缓存之外的任何缓存都会让 Trino 读到旧值。reload 通过 `CALL system.flush_metadata_cache()` 刷新 metadata 缓存，其他缓存没有对应的失效手段。
- catalog 名要和 `[store.trino] catalog` 一致。

### reload 的 Trino 可见探测

对 PostgreSQL 后端且配置了 `[store.trino]`、且这次变更新增了 Trino 需要看到的东西时，`reload` 分两步：

1. 在定义锁内执行 DDL，并把新定义版本以 `pending` 状态提交。旧定义仍是 active，继续服务。
2. 调用 `CALL <catalog>.system.flush_metadata_cache()`，反复探测直到 Trino 能看到新表与新列，上限 `probe_timeout_ms`。探测成功后，用第二个短事务把 pending 版本变成 active；从这一刻起，按旧版本校验的写入被拒绝。

没有探测，或这次变更不需要 Trino 看到任何新东西时，DDL、新版本与激活在同一个事务里完成。

探测超时则 reload 失败（退出码 3），旧定义保持 active。DDL 只做加法，已经提交，pending 版本留在库里（见「已知缺口」）。

## 命令

所有命令都读 `--server-config`；`--format json` 输出机器可读结果；`--profile` 选择绑定 profile。

### 退出码

| 退出码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 定义、配置或输入有问题（诊断码 `ONT-V*`）；包括破坏性变更被 `reload` 拒绝（`ONT-V25`）、`--plan-hash` 不匹配 |
| 3 | edit store 不可用或操作失败（连不上、锁超时、Trino 探测超时）；诊断码 `ONT-V26` |

`validate --online` 也用 3 表示引擎无法回答，含义同样是「环境问题，不是内容问题」。

### `ontoforge reload <DIR> --server-config <FILE>`

让 edit 表与定义一致。

- 加法（新 object type、新多对多 link、可空属性）自动执行。
- 破坏性变更（删属性、删 type、改属性类型、改主键、新增 required 属性、link 端点变化）报 `ONT-V25`，不执行任何 DDL，改走 `migrate`。
- `--dry-run`：只打印会执行的 DDL。
- 定义没有变化时 `changed` 为 `false`，不产生新版本。

### `ontoforge migrate <DIR> --server-config <FILE> (--plan | --apply --plan-hash <HASH>)`

先 `--plan` 看步骤和 hash，再 `--apply --plan-hash <HASH>`。apply 会重新生成计划，hash 不一致就拒绝，避免执行的不是审阅过的内容。

- `migrate` 在 PostgreSQL 且配置了 `[store.trino]` 时同样做 Trino 可见探测；探测超时时迁移已经应用且 active，只是 Trino 还没看到新名字，命令以退出码 3 结束，稍后再运行探测相关步骤或确认 Trino 设置即可。
- `--rename 'Type.old=new'` 声明属性改名；`--rename 'OldType=NewType'` 声明 object type 改名。不声明则旧列被归档，新列为空。
- `--drop-archived`：计划或执行删除所有归档列与归档表。**这会删除数据。**

### `ontoforge scope plan|apply <SCOPE_DIR> --ontology <DIR> --server-config <FILE>`

roles-as-code：`plan` 打印将写入的 role 与 role binding 和计划 hash；`apply --plan-hash <HASH>` 写入。写入前要求 ontology 已经被 edit store 加载（先 `reload`），否则拒绝。`--server-config` 里需要 `[auth.principal_attrs]`。

### `ontoforge import <DIR> --server-config <FILE> (--type <T> | --link <L>) --file <FILE>`

把 CSV 或 JSON 导入 editOnly 的 object type 或多对多 link。

- `--on-existing error|skip|update`：默认 `error`，只允许创建，遇到已有对象整体拒绝；`skip` 允许中断后重跑；`update` 用文件里的值更新已有对象。
- `--batch-size`（1 到 10000，默认 1000）：每个 commit 的行数。`--max-rows`（默认 1000000）：行数上限。
- 每个批次是一次 commit，actor 与 action 记为 `system:import`。
- 空字符串单元格表示「不设置」，import 不会取消设置某个属性。

### `ontoforge dev-token --server-config <FILE> --sub <SUB>`

签发开发用 token，只被以 `--dev` 启动的服务接受，不可用于生产。

- 主体 id 为 `ontoforge-dev|<SUB>`；`--groups A,B`、`--attr name=value`（名称须在 `[auth.principal_attrs]` 中声明）、`--ttl`（`m`、`h`、`d`，最长 30d，默认 8h）。
- 签名密钥在 `--key-file`（默认 `[auth.dev] key_file`）首次使用时创建。

## 已知缺口

以下问题已知且未修复，部署前请知悉：

1. **import 的 create-only 有竞态（03-19）。** 已有的 key 在第一个批次提交之前一次性读出。如果别的写入方在这次读取和本批次提交之间创建了同一个 key，import 会对它打补丁，而不是拒绝。`Commit` 目前没有「必须不存在」的前置条件。单个操作员运行 CLI 时可能性小；之后由 Action 创建对象的写入方才是现实的并发来源。
2. **SQLite 的 `database is locked` 没有映射为 `LockTimeout`。** SQLite 后端遇到写锁冲突时报通用的存储错误（退出码 3），而不是带 `lock_timeout_ms` 提示的锁超时。
3. **Trino 探测失败后会留下 pending 版本（03-18）。** 探测超时时，DDL 与 `pending` 版本已经提交，没有成为 active。旧定义继续服务，再次 `reload` 可以重试，但在此之前库里有一个未激活的版本记录，没有清理命令。
4. **同一个 commit 内交换两个 oneToOne 的 key 会被拒绝（03-13）。** 立即检查的部分唯一索引在 PostgreSQL 上不能推迟，所以两个对象在一次提交里互换 key 报 `CardinalityViolation`。把 key 从一个对象移到另一个对象，只有在放弃 key 的对象排在前面（按锁顺序）时才成功。
5. **dev-token 密钥文件的并发首次创建（03-12）。** 密钥以独占方式创建，已存在的文件不会被覆盖；两个进程同时首次使用同一个路径时，失败的一方读取胜者写的文件，此时文件可能尚未写完。先单独运行一次 `dev-token` 创建密钥，再并发使用。
6. **CI 里 Trino 相关步骤尚未在 GitHub 上运行过。** 行谓词语料和 reload 可见探测（含 `flush_metadata_cache()` 经客户端是否可用，RESEARCH A5）在写本文档时只在本机编译与列出，没有对真实 Trino 执行过。
