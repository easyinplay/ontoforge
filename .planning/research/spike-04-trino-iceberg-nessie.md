# Spike 4：Trino 对 Iceberg + Nessie 湖仓的支持度（方向性，不阻塞）

**日期：** 2026-10-16
**上游：** `docs/designs/trino-engine-sql-tiers.md`「Gates before /plan-eng-review」第 4 项
**范围：** 只做文档 / 源码调研，未起任何服务、未跑任何 SQL。所有「运行时行为」结论若无官方原文支撑，一律标 `[unverified]`，并在文末列出下一 milestone 需要的证伪探针。
**版本基线：** Trino 484（2026-10-16 发布）、Nessie 0.109.0（2026-10-08）、Iceberg 1.12.0（2026-09-30）、Doris 4.1.5（2026-10-15）、Kyuubi v1.12.0（2026-07-28）`[verified: gh api repos/*/releases]`

## 引用约定

为控制篇幅，`[verified: Tn]` 中的 `Tn` 指下表 URL；凡标 verified 的条目都在本次会话读过原文或源码。

| Key | URL |
|---|---|
| T1 | https://trino.io/docs/current/object-storage/metastores.html （源：trinodb/trino `docs/src/main/sphinx/object-storage/metastores.md`） |
| T2 | https://trino.io/docs/current/connector/iceberg.html （源：`docs/src/main/sphinx/connector/iceberg.md`） |
| T3 | https://github.com/trinodb/trino/blob/master/core/trino-spi/src/main/java/io/trino/spi/connector/Connector.java |
| T4 | https://github.com/trinodb/trino/blob/master/core/trino-main/src/main/java/io/trino/transaction/InMemoryTransactionManager.java |
| T5 | https://github.com/trinodb/trino/blob/master/plugin/trino-iceberg/src/main/java/io/trino/plugin/iceberg/IcebergMetadata.java |
| T6 | https://trino.io/docs/current/connector/postgresql.html （含 include 的 `jdbc-common-configurations.fragment`、`procedures-execute.fragment`、`query-passthrough-warning.fragment`） |
| T7 | https://trino.io/docs/current/security/file-system-access-control.html |
| T8 | https://trino.io/docs/current/sql/create-catalog.html |
| N1 | https://projectnessie.org/guides/iceberg-rest/ |
| N2 | https://projectnessie.org/guides/transactions/ |
| N3 | https://projectnessie.org/guides/sql/ |
| N4 | https://projectnessie.org/nessie-latest/trino/ （源：projectnessie/nessie `site/in-dev/trino.md`） |
| N5 | https://github.com/projectnessie/nessie/blob/main/api/model/src/main/java/org/projectnessie/api/v2/http/HttpTreeApi.java |
| N6 | https://github.com/projectnessie/nessie/blob/main/catalog/service/rest/src/main/java/org/projectnessie/catalog/service/rest/IcebergApiV1GenericResource.java |
| I1 | https://iceberg.apache.org/docs/latest/nessie/ （源：apache/iceberg `docs/docs/nessie.md`） |
| I2 | https://github.com/apache/iceberg/blob/main/core/src/main/java/org/apache/iceberg/TableProperties.java |
| D1 | https://doris.apache.org/docs/lakehouse/catalogs/iceberg-catalog |
| D2 | https://doris.apache.org/docs/lakehouse/best-practices/doris-nessie |
| K1 | https://github.com/apache/kyuubi/blob/master/docs/connector/spark/iceberg.md |

## 结论先行

1. Trino 484 的 Iceberg connector 原生支持 `nessie` 和 `rest` 两种 catalog，Nessie 都能接；但 **Nessie ref（分支）只能按 catalog 固定，不能按查询切换**，且 **Trino 只支持单语句自动提交写入，没有多语句、多表原子事务**。
2. 多表原子性只能靠 **Nessie 分支 + merge**：ontoforge 自己调 Nessie REST API v2 建分支、合并，Trino 只负责在「指向某个分支的 catalog」里执行单表写入。
3. 推荐 catalog 方案：**生产默认 `iceberg.catalog.type=rest` 指向 Nessie 的 `/iceberg/{ref}` 端点**（Doris 只能这样接 Nessie，Spark / Kyuubi 也通用）；但 Nessie 的 Iceberg REST 仍标 experimental，且「只返回单个 snapshot」会削弱 `FOR VERSION AS OF <snapshot_id>` 分页，需要证伪探针后再锁定。原生 `nessie` 类型保留为备选。
4. 快照一致分页可用，但要按「每张 Iceberg 表各记一个 snapshot_id」设计，并接受 snapshot 过期 / 被 Nessie REST 隐藏后游标失效的降级路径。
5. PostgreSQL connector：`metadata.cache-ttl` 默认 `0s`（关闭），读后写可见性在元数据层面没有障碍；`system.query` 表函数和 `system.execute` 存储过程都存在，后者能写库，必须在 Trino 访问控制和 DB 账号两层同时封死。

## Q1 Catalog 类型、Nessie 原生 vs REST、分支选择

- `iceberg.catalog.type` 合法值：「Valid values are `hive_metastore`, `glue`, `jdbc`, `rest`, `nessie`, and `snowflake`.」`[verified: T1]`
- 原生 Nessie 配置键：`iceberg.nessie-catalog.uri`（必填，例 `https://localhost:19120/api/v2`）、`iceberg.nessie-catalog.ref`（「The branch/tag to use for Nessie. Defaults to `main`.」）、`default-warehouse-dir`、`read-timeout`（默认 `25s`）、`connection-timeout`（默认 `5s`）、`enable-compression`、`authentication.type`（仅 `BEARER`）、`authentication.token`、`client-api-version`（`V1`/`V2`）`[verified: T1]`。限制：「The Nessie catalog does not support view management or materialized view management」`[verified: T1]`。
- REST 配置键（节选）：`iceberg.rest-catalog.uri / prefix / warehouse / security`（`NONE`/`SIGV4`/`GOOGLE`/`OAUTH2`）`/ session`（`NONE`/`USER`）`/ snapshot-loading-mode`（`ALL`/`REFS`）`/ vended-credentials-enabled / nested-namespace-enabled / oauth2.token / oauth2.credential / oauth2.server-uri / http-headers` `[verified: T1]`。
- 弃用情况：Trino 484 文档对 `nessie` 类型没有弃用声明（只对 JDBC catalog 说「Consider the REST catalog as an alternative solution」）`[verified: T1]`。「没有弃用」是文档缺省，不构成「长期支持」的证据 `[unverified]`。
- Nessie 官方立场：Nessie 文档给出了从 `type=nessie` 迁移到 `type=rest` 的对照表（`uri` 从 `.../api/v2` 改为 `.../iceberg`，`ref` 改为在 uri 末尾追加分支名），并说两者可以同时使用：「You can safely use your current Nessie applications, those that use `type=nessie` ... concurrently with applications using Nessie via Iceberg REST」`[verified: N1]`。但页首警告：「Support for Iceberg REST is currently considered experimental in Nessie!」`[verified: N1]`。
- REST 选分支的方式：`http://127.0.0.1:19120/iceberg/experiments|sales`（分支 `experiments`、warehouse `sales`）；`prefix` 选项「Don't set this for Nessie」，改为把分支名追加到 `uri` `[verified: N1]`。
- **按查询选分支：不支持。** 原生类型只有 catalog 级 `iceberg.nessie-catalog.ref`，Trino Iceberg 的 session property 里没有 Nessie / ref 相关项 `[verified: trinodb/trino plugin/trino-iceberg IcebergNessieCatalogConfig.java、IcebergSessionProperties.java]`。REST 类型的分支写死在 `uri` 里 `[verified: T1, N1]`。
- 可能的绕行（均未验证）：
  - Nessie REST 支持在表名里写 `` `my_table@my_other_branch` `` / `` `my_table#<commit-hash>` ``，文档示例是 Spark 语法 `[verified: N1]`；Trino 用双引号标识符 `"my_table@branch"` 是否能原样传给 Nessie、是否被小写化 `[unverified]`。
  - Trino `CREATE CATALOG` 可运行时建 catalog（需 catalog management 为 `dynamic`），但「The complete `CREATE CATALOG` query is logged, and visible in the Web UI」，会泄露凭据 `[verified: T8]`。
- Nessie 的 Trino 集成说明：「Trino does not support S3 request signing, so it is mandatory to configure the assume role/credentials vending functionality」，且要求 `fs.native-s3.enabled=true` `[verified: N4]`。该页还说 Trino 无法配置 OAuth 端点；但 Trino 484 已有 `iceberg.rest-catalog.oauth2.server-uri` `[verified: T1]`，Nessie 这段说明可能已过时 `[unverified]`。

## Q2 Trino 对 Iceberg 的写入能力与事务

- 支持 `INSERT`、`UPDATE`、`DELETE`、`TRUNCATE`、`MERGE` `[verified: T2]`；row-level 写入需 format version 2（v3 实验性）`[verified: T2]`。
- **没有多语句写事务。** SPI 默认 `isSingleStatementWritesOnly()` 返回 `true`（「the engine will enforce that auto-commit mode is used for writes」）`[verified: T3]`；`IcebergConnector` 未覆盖该方法 `[verified: trinodb/trino plugin/trino-iceberg IcebergConnector.java，rg 无匹配]`；引擎在显式事务里写这类 catalog 会抛 `"Catalog only supports writes using autocommit: "` `[verified: T4:464-465]`。
- **没有跨 catalog 写。** 同一事务写第二个 catalog 抛 `"Multi-catalog writes not supported in a single transaction. Already wrote to catalog "` `[verified: T4:460-462]`。
- 结论：一条 Trino 语句 = 一张 Iceberg 表的一个 commit。多表原子性 Trino 自身给不了 `[verified: T3, T4，推论]`。
- 并发与隔离（乐观并发）：
  - 提交时 `rowDelta.validateFromSnapshot(<读取时的 snapshot>)`，再按表属性 `write.delete.isolation-level` 决定是否 `validateNoConflictingDataFiles()`；始终 `validateDeletedFiles()`；UPDATE / MERGE 额外 `validateNoConflictingDeleteFiles()`；有谓词时设置 `conflictDetectionFilter` 缩小冲突检测范围 `[verified: T5:3461-3475, 3550-3576]`。
  - Iceberg 默认 `write.delete/update/merge.isolation-level` 均为 `"serializable"` `[verified: I2:432-449]`；注意 Trino 只读取 `write.delete.isolation-level` 来决定 UPDATE / MERGE 的级别 `[verified: T5:3469]`。
  - 元数据层提交重试：表属性 `max_commit_retry`，默认取 `iceberg.max-commit-retry`（默认 `4`）`[verified: T2]`。重试只解决元数据指针竞争；数据冲突校验失败时语句失败，需要 ontoforge 重试 `[unverified：具体错误码未查]`。
  - 容错执行：「Read and write operations are both supported with any retry policy.」`[verified: T2]`

## Q3 Time travel 与快照一致分页

- 语法：`FOR VERSION AS OF 8954597067493422955`、`FOR TIMESTAMP AS OF TIMESTAMP '...'`，以及 Iceberg 表级分支 / 标签 `FOR VERSION AS OF 'test-branch'` `[verified: T2]`。
- `$snapshots` 列：`committed_at, snapshot_id, parent_id, operation, manifest_list, summary`；官方示例取最新 snapshot：`SELECT snapshot_id FROM example.testdb."customer_orders$snapshots" ORDER BY committed_at DESC LIMIT 1;` `[verified: T2]`。另有 `$refs`（`BRANCH`/`TAG`）、`$history`、`table_changes(start_snapshot_id, end_snapshot_id)` 表函数 `[verified: T2]`。
- Trino 文档没有 Iceberg 表级分支 / 标签的创建 DDL，也没有写入指定分支的语法（只有读）`[verified: T2，全文 rg "branch" 只命中 $refs、$history 与 time travel]`。
- **用于分页：可行，但有三个限制。**
  1. snapshot id 是表级的。多表（叠加读、link join）分页需要游标里存 `{table → snapshot_id}`，每张表各自 `FOR VERSION AS OF` `[推论]`。
  2. PG edit store 没有 snapshot，叠加读的结果不可能做到全快照一致；设计稿已把快照一致分页限定为「全 Iceberg 来源」，与此一致。
  3. **Nessie REST 只暴露单个 snapshot：**「we have deliberately chosen to only return the state of a table or view as a _single_ snapshot in Iceberg」`[verified: N1]`。由此推断：经 REST 接 Nessie 时，表一旦有新提交，旧 `snapshot_id` 对 Trino 不可见，`FOR VERSION AS OF <旧 id>` 很可能失败 `[unverified，需探针 P1]`。Nessie 提供的替代是表名后缀 `#<commit-hash>` / `#<timestamp>` `[verified: N1]`，Trino 能否用见 Q1 `[unverified]`。
- `expire_snapshots` 会让旧 snapshot 消失 `[verified: T2]`，游标 TTL 必须短于 snapshot 保留期。

## Q4 Nessie 多表提交、分支 / 合并，以及 Trino 能否参与

- Nessie REST API v2（`v2/trees`）：`POST` 建分支 / 标签；`PUT {ref}` 把引用指向某个 hash；`DELETE {ref}`；`POST {branch}/history/commit`（「Commit one or more operations against the given 'branch'.」）；`POST {branch}/history/merge`（「Merge commits from another reference onto 'branch'.」）；`POST {branch}/history/transplant`；`GET {ref}/history` 与 diff `[verified: N5]`。
- 跨表事务的两种方式：「Via Branches ... a sequence of commits on one branch can be exposed as a single consistent view to other users through the use of a merge operation」；「Single Commits: Nessie allows a single commit operation to include many object changes simultaneously ... tools will need to be enhanced」`[verified: N2]`。
- Iceberg 文档对分支法的定性：「you can approximate a multi-table transaction ... merged back into the main branch atomically ... this isn't a true multi-table transaction」；单个 Iceberg 客户端仍然「limited to the set of single table transactions」`[verified: I1]`。
- Nessie 实现了 Iceberg REST 的多表提交端点 `POST /v1/{prefix}/transactions/commit`（`commitTransaction`）`[verified: N6:80, 235-249]`。Trino 只做单语句写入（Q2），所以不会用到它 `[推论]`。
- 冲突粒度：「Nessie maintains state and locks at table granularity」，冲突时拒绝或交给表格式进一步解决 `[verified: N2]`。隔离级别全部是乐观锁，多数工具实际处于 Read Committed 或 Repeated Read `[verified: N2]`。
- 提交元数据：Iceberg REST 请求可带 `Nessie-Commit-Authors`、`Nessie-Commit-Message` 等头 `[verified: N1]`。ontoforge 的 action log 可以据此把主体写进 Nessie commit；Trino 能否透传这些头（`iceberg.rest-catalog.http-headers` 是 catalog 级静态值 `[verified: T1]`）→ 按请求透传 `[unverified]`。
- **Trino 参与分支事务的可行模式**（推论，需探针 P3/P5）：ontoforge 调 Nessie API 建 / 重置分支 → 在一个**固定指向该分支**的 Trino catalog 里逐表执行 `INSERT` / `MERGE` → ontoforge 调 `history/merge` 合并到 `main`，把返回的 commit hash 写入 action log。分支名固定在 catalog 配置里，因此并发 batch 需要「预置 N 个 staging catalog / 分支」的池，或串行。
- Spark 的做法：原生 `NessieCatalog`（`spark.sql.catalog.nessie.type=nessie`、`ref=main`）`[verified: I1]` + Nessie Spark SQL 扩展，语法包括 `CREATE BRANCH ... FROM`、`USE REFERENCE ... AT`、`MERGE BRANCH ... INTO`、`ASSIGN BRANCH` `[verified: N3]`，即会话内切分支，比 Trino 灵活。扩展是否兼容 `type=rest` `[unverified]`。

## Q5 Doris、Spark、Kyuubi 对 Iceberg REST / Nessie 的支持

- **Doris**：`iceberg.catalog.type` 支持 `hms / rest / hadoop / glue / dlf / jdbc / s3tables`，**没有原生 nessie 类型** `[verified: D1]`。官方 best practice 用 REST 接 Nessie：`'iceberg.catalog.type' = 'rest'`、`'iceberg.rest.uri' = 'http://YOUR_NESSIE_HOST:19120/iceberg/main'`、`'iceberg.rest.vended-credentials-enabled' = 'true'`，并说明「The Nessie REST Catalog URI format is `http://HOST:PORT/iceberg/{branch}`」`[verified: D2]`。写入：`INSERT` / `INSERT OVERWRITE`；`DELETE` / `UPDATE` / `MERGE INTO`「experimental ... since version 4.1.0」；Iceberg 表级分支读写（`iceberg_tbl@branch(b1)`、`INSERT INTO iceberg_table@branch(b1)`）since 3.1.0 `[verified: D1]`。注意这是 Iceberg 表级分支，不是 Nessie catalog 级分支。REST catalog 的 DDL / 写回能力「Depends on the service provider」`[verified: D1]`。
- **Spark**：原生 Nessie 与 Iceberg REST 都是标准 Iceberg 客户端配置，Nessie 迁移表就是针对这类客户端写的 `[verified: N1, I1]`；分支工作流见 Q4。
- **Kyuubi**：只是 Spark（等引擎）的 SQL 网关，官方 Iceberg 文档示例是 `spark.sql.catalog.spark_catalog.type=hive` 加 Iceberg 扩展 `[verified: K1]`。接 Nessie / REST 靠透传 Spark catalog 配置，文档没有 Nessie 示例 `[unverified]`。

## Q6 Trino PostgreSQL connector：透传与缓存

- 表函数 `query(varchar) -> table` 存在，调用方式 `TABLE(example.system.query(query => '...'))`；官方警告：「Only the data source performs validation or security checks for these queries ... Trino does not perform these tasks. Only use passthrough queries to read data.」`[verified: T6]`
- **还有写透传：** 存储过程 `system.execute`，「Queries can also invoke statements that insert, update, or delete data」，「only subject to any security or access control of the underlying data source」`[verified: T6]`。设计稿 L2 白名单已禁止 `CALL`，Trino 侧也要封住。
- 如何禁用（Trino 侧）：connector 没有关闭开关的配置项 `[unverified：文档中未见，属缺省证据]`。官方手段是 file-based access control：有 function 规则时按首条匹配，无匹配即拒绝；**没有 function 规则时只能执行 `system.builtin` 内的函数**；文档点名「Care should be taken when granting permission to the `system` schema of a catalog, as this is the schema Trino uses for table function such as `query`.」`[verified: T7]`。procedure 规则同理：没有规则时只能执行 `system.builtin` 内的存储过程 `[verified: T7]`。因此只要启用 file-based access control，并且不给 ontology 身份和 L2 身份授予 `system` schema 的 `EXECUTE`，就能封死这两条路；Trino 默认的 allow-all 访问控制下它们可执行 `[推论]`。PG 账号只读是第二道防线（文档原意就是「只有数据源做校验」）。
- 缓存：`metadata.cache-ttl` 「Defaults to `0s` (caching disabled)」；`metadata.schemas/tables/statistics.cache-ttl` 默认继承它；`metadata.cache-missing` 默认 `false`；可用 `CALL system.flush_metadata_cache()` 手动清 `[verified: T6]`。`case-insensitive-name-matching.cache-ttl` 默认 `1m`，只影响名称映射 `[verified: T6]`。
- 读后写可见性：connector 文档没有行级 / 结果级数据缓存配置 `[unverified：缺省证据，需 spike 3 实测]`。只要保持 `metadata.cache-ttl=0s`，新建 edit store 表 / 列能立刻被看到；行数据每次查询都下推到 PG `[unverified]`。

## 对 ontoforge 下一 milestone 的含义

1. **事务边界放在 Nessie，不放在 Trino。** `batch` action = ontoforge 编排「Nessie 建 / 重置分支 → Trino 在 staging catalog 逐表单语句写 → Nessie merge」。action log 记录的 commit id 用 merge 返回的 `main` 上的 hash。Trino 执行到一半失败时，直接丢弃 / 重置分支即可回滚，这正是分支法的价值。
2. **staging catalog 池**：Trino 不能按查询切 ref，所以在 Trino 侧预配置 `lake`（指向 `main`，只读身份）和若干 `lake_stage_N`（指向 `ontoforge-stage-N`，写身份），ontoforge 用租约分配。不推荐运行时 `CREATE CATALOG`（日志泄露凭据，且要求 dynamic catalog 模式）。
3. **merge 冲突是常态**：Nessie 按表粒度判冲突，`main` 上同表有并发提交（含 materialize）时 merge 会失败。需要「重置分支到 main 新 head 后重放」的重试逻辑，或按表串行化 batch 与 materialize。
4. **materialize edit → Iceberg**：一条 `MERGE INTO lake_stage_N.x.t USING pg_edit.x.edits` 只写一个 catalog，Trino 允许 `[推论，探针 P4]`。PG 与 Iceberg 之间没有共同事务，materialize 必须幂等（按 `(pk, _version)` 去重），水位（已 materialize 到的 edit log 序号）随 Nessie commit 一起记录。
5. **分页游标**：`snapshot_id` 字段扩展为「表 → snapshot_id」映射（或 Nessie commit hash，取决于 P1/P2 结果）；游标失效返回明确错误码（如 `CursorExpired`），客户端从头翻页。
6. **L2 防线补充**：AST 白名单之外，部署文档要求启用 Trino file-based access control，L2 身份与 ontology 身份都不授予任何 catalog 的 `system` schema 函数 / 存储过程执行权；启动自检增加「执行 `system.query` 必须被拒绝」探测。

## 推荐 catalog 方案

- **默认：`iceberg.catalog.type=rest`，`iceberg.rest-catalog.uri=http://<nessie>:19120/iceberg/<branch>`，`iceberg.rest-catalog.vended-credentials-enabled=true`，`fs.native-s3.enabled=true`。** 理由：Doris 只能这样接 Nessie `[verified: D1, D2]`；Spark / Kyuubi 通用；Nessie 的迁移方向是 REST `[verified: N1]`；凭据集中在 Nessie，Trino 不持有对象存储长期密钥 `[verified: N1, N4]`。
- **备选：`iceberg.catalog.type=nessie` + `iceberg.nessie-catalog.ref`。** 保留以防 P1 证明 REST 下 snapshot 分页不可用，或 Nessie REST 的 experimental 状态在目标版本出问题。两者可同时指向同一个 Nessie `[verified: N1]`，Trino 侧切换只改 catalog 配置文件。
- 分支管理一律由 ontoforge 直接调 Nessie REST API v2（Rust HTTP client），不依赖任何引擎的 SQL 扩展。

## 风险

| # | 风险 | 依据 | 缓解 |
|---|---|---|---|
| R1 | Nessie Iceberg REST 仍是 experimental | `[verified: N1]` | 保留原生 `nessie` 类型备选；锁定 Nessie 版本做 CI |
| R2 | REST 只暴露单 snapshot，snapshot id 分页可能失效 | `[verified: N1]`，影响 `[unverified]` | 探针 P1/P2；失效即 `CursorExpired` |
| R3 | Trino 不能按查询选 ref，batch 并发受限于 staging catalog 数 | `[verified: T1, N1]` | catalog 池 + 租约 |
| R4 | Trino 无多表事务，分支法只是「近似」多表事务 | `[verified: T3, T4, I1]` | 原子性由 Nessie merge 提供；中途失败丢弃分支 |
| R5 | merge 表级冲突，与 materialize / 外部 ETL 抢同一张表 | `[verified: N2]` | 重置分支后重放，或按表串行 |
| R6 | Trino + Nessie 需要 STS / vended credentials；MinIO 可用性 `[unverified]`；RustFS 仓库有 `rustfs/src/admin/handlers/sts.rs`，与 Nessie assume-role 的兼容性 `[unverified]` | `[verified: N4]` | 下一 milestone 环境 spike 实测两种对象存储 |
| R7 | `system.query` / `system.execute` 透传，默认 allow-all 下可执行 | `[verified: T6, T7]` | file-based access control + 只读 DB 账号 + 自检探测 |
| R8 | snapshot 过期使游标 / time travel 失效 | `[verified: T2]` | 游标 TTL < snapshot 保留期 |
| R9 | Nessie 非 Apache 项目，治理依赖单一社区；目前活跃（0.109.0 发布于 2026-10-08，仓库未归档）`[verified: gh api repos/projectnessie/nessie]` | — | 方案只依赖 Iceberg REST 标准端点 + Nessie v2 树 API，替换 catalog 时影响面可控 |

## 下一 milestone 证伪探针（Linux CI：Trino 484 + Nessie 0.109 + MinIO）

- P1：REST 类型下对同一表提交两次，用第一次的 `snapshot_id` 跑 `FOR VERSION AS OF`，记录成功或报错原文；原生 `nessie` 类型同测。
- P2：REST 类型下 `SELECT * FROM lake.ns."t@branch"` 与 `"t#<hash>"` 是否可用（含大写分支名）。
- P3：固定 staging catalog 写两张表 → Nessie merge → `main` catalog 一次查询同时看到两表变更。
- P4：`MERGE INTO <iceberg catalog> USING <postgresql catalog>` 单语句可执行；两个并发 `MERGE` 改同一分区时的错误码与重试表现。
- P5：`main` 上同表被并发修改后 merge 的冲突响应格式（供 ontoforge 映射错误码）。
- P6：启用 file-based access control 且无 function / procedure 规则时，`TABLE(pg.system.query(...))` 与 `CALL pg.system.execute(...)` 必须被拒绝，记录报错原文。
