# Phase03–04 逐项复签证据

本文件为当前核验结果，旧 SUMMARY 不作行为证据。当前源码 HEAD `060019fe52005c6b85ae5c5f59da9868f464e579`，CI run `41994479295`。仅消费已有真实执行日志，本 worker 没有执行 Cargo、测试、容器或服务。

Phase03 的 134 条 PLAN truths 中 133 条 VERIFIED、1 条 UNCERTAIN/WARNING；另5条 ROADMAP SC均 VERIFIED。Phase04 的31条 PLAN truths与5条SC均 VERIFIED。未按旧Phase03 8条汇总缩小合同。

## 03-edit-store-scope-model

| ID | 原合同 truth | 判断 | 当前证据 |
|---|---|---|---|
| 03-01.1 | sea-query-sqlx 与 getrandom（名称来源 [ASSUMED]）只在人工核验 crates.io / docs.rs 页面通过后才写入根 `Cargo.toml`；未核验前本 plan 不执行任何 install | UNCERTAIN (WARNING) | 仅03-01-SUMMARY.md:132声称approved；当前源码与CI不能重建安装前人工审查/授权发生顺序。需要独立批准记录或开发者明确接受该历史证据边界。；源/链路详 03-01 |
| 03-01.2 | 根 `Cargo.toml` 一次性声明本 phase 全部新依赖与 feature（sqlx 增 postgres / migrate / macros / json / chrono / tls-rustls-aws-lc-rs，sea-query 1.0.2，sea-query-sqlx 0.9.1，jsonwebtoken 11.1.0 只开 aws_lc_rs，getrandom 0.4.3，chrono 增 clock，postgresql_embedded 0.21.0 关闭默认 feature）；03-02..03-20 不再修改任何 Cargo.toml 与 Cargo.lock（它们的 cargo 命令一律带 `--locked`） | VERIFIED | Cargo.toml/Cargo.lock 锁定既定版本与 feature；changed-source-inspection.json 的 phase03ManifestHistory exit=0 且 stdout 为空，e6d4823..1df4a7f 的受管 manifests 未改变；当前 CI cargo 命令含 --locked。不能据此补出 01.1 的人工批准。；源/链路详 03-01 |
| 03-01.3 | crate 依赖方向为 storage → {schema, expr, auth, engine}、auth → {schema, expr, engine}，`ontoforge-storage` 不再依赖 `ontoforge-query`，依赖图无环；修正记录在 ADR-0023 并在设计文档的依赖方向行指向它 | VERIFIED | Cargo 依赖为 storage → schema/expr/auth/engine；auth → schema/expr/engine；ADR-0023 accepted，docs/designs/trino-engine-sql-tiers.md 指向 ADR；workspace 编译成功排除依赖环。；源/链路详 03-01 |
| 03-01.4 | `ontoforge_storage::store::sqlite_connect_options` 打开的每个连接都 `PRAGMA recursive_triggers = 1`，并注册与 SQLite 替身同一实现的 `ontoforge_decimal` / `ontoforge_timestamp` collation（`'1.5'` 与 `'1.50'` 在该 collation 下相等） | VERIFIED | windows:3745 engine_collations_are_registered; windows:3746 every_pooled_connection_has_recursive_triggers；源/链路详 03-01 |
| 03-01.5 | jsonwebtoken 选定 aws-lc-rs 后端：HS256 签发再校验的往返不 panic；不引入第二个 crypto 栈 | VERIFIED | windows:1795 hs256_round_trip_does_not_panic；源/链路详 03-01 |
| 03-01.6 | `cargo tree --workspace --features ontoforge-storage/embedded-pg -i openssl` 与 `-i native-tls` 都报「did not match any packages」；CI `test` 作业有同名守卫步骤 | VERIFIED | ci/check-tls-deps.sh 从 cargo tree 全图精确匹配包名并对查询失败 fail-closed；ci/test_tls_deps.py 六个 hermetic 反例与真实 CI helper step 均成功，见 service/tls 证据。旧 -i 报错字符串命令是已替换的守卫机制，不作为当前行为证据。；源/链路详 03-01 |
| 03-02.1 | 开发者可以在属性上写 `indexed: true`（缺省 false）；IR `PropertyDef.indexed` 进入 `schemas/ontoforge-definition.schema.json`；`indexed: "yes"` 报 ONT-V06 并带 line:col（ADR-0016 索引规则的定义层输入） | VERIFIED | windows:2032 count_alias_cannot_change_indexed_fk_sample_order; windows:3036 indexed_defaults_to_false; windows:3037 indexed_must_be_boolean; windows:3038 indexed_true_is_kept; windows:3668 removing_indexed_keeps_index；源/链路详 03-02 |
| 03-02.2 | aviation 示例的 `Flight.status` 声明 `indexed: true`，`ontoforge validate examples/ontologies/aviation` 仍然通过 | VERIFIED | examples/ontologies/aviation/objects.yaml 的 Flight.status.indexed=true；当前示例 CLI reload 与 schema drift/示例测试通过。；源/链路详 03-02 |
| 03-02.3 | 新增诊断码 ONT-V21..V27（scope 文档、行谓词、criteria 列覆盖、物理名、破坏性变更、edit store 操作、import 数据），均为不可经 `ontoforge.toml` 关闭的 V 级；码永不复用 | VERIFIED | schema/src/diag.rs 定义 V21..V27，validate 诊断不可通过 lint 配置静默；新增 W18 不覆盖任何 V 码；config/scope/reload 的具名负测覆盖不可生效路径。；源/链路详 03-02 |
| 03-02.4 | `ontoforge-server.toml` 支持 `[store]`（kind sqlite \| postgres、path 或 url_env / url_file、schema_edit / schema_sys、lock_timeout_ms、probe_timeout_ms、`[store.trino] catalog`）与 `[auth]`（`[[auth.oidc]]` 多个 issuer、`[auth.principal_attrs.<name>]` 的 claim 与类型、`[auth.dev] key_file`），未知键与非法值报 ONT-V20 并带 line:col（per D-05） | VERIFIED | windows:2216 store_postgres_with_trino_catalog; windows:2218 store_sqlite_defaults; windows:2221 unknown_key_in_store_or_auth_is_v20；源/链路详 03-02 |
| 03-02.5 | PG 连接串只能经 `url_env` / `url_file` 引用，不能明文写入配置（沿用 Phase 2 D-06 / D-07） | VERIFIED | windows:2214 store_plaintext_url_is_rejected；源/链路详 03-02 |
| 03-02.6 | OIDC 配置只接受非对称算法白名单，`HS*` 与 `none` 报 ONT-V20；issuer 必须是 https（回环地址可 http）、不含 `\|`、互不重复 | VERIFIED | windows:2197 oidc_issuer_rules; windows:2199 oidc_rejects_symmetric_and_none_algorithms；源/链路详 03-02 |
| 03-02.7 | 主体属性类型 `AttrType`（string / integer / boolean / string_list / integer_list）定义在 `ontoforge_schema::ir::attrs`；运行期取值 `Scalar` / `AttrValue` / `Attrs` 定义在 `ontoforge_expr::rowpred::value`，`Attrs::get` 对未声明或缺失的属性返回 `AttrValue::Null`（per D-05），list 取值不得含 Null 元素 | VERIFIED | windows:2203 principal_attrs_types; windows:2434 missing_attr_is_unknown_and_invisible；源/链路详 03-02 |
| 03-03.1 | `ontoforge_storage::reload::reload` 在 SQLite edit store 中为每个 object type 建一张 `of_obj_<snake>` 表、为每个 M:N link type 建一张 `of_lnk_<snake>` 表（per D-17，前缀遵守 ADR-0002 的 `of_`），并写入 `of_def_version` 行与 `of_def_state.active_def_version` | VERIFIED | windows:3650 first_reload_creates_tables; windows:3657 new_many_to_many_link_creates_a_table；源/链路详 03-03 |
| 03-03.2 | edit 表的 set_mask 物理编码按 Task 1 checkpoint 的结论落地（推荐：每个非 pk 属性一个 `<col>__set` 布尔列）；系统列一律 `of_` 前缀：`of_version`、`of_deleted`、`of_released`、`of_def_version`、`of_log_seq`；link 表为 `(a_pk, b_pk, of_deleted, of_version, of_def_version, of_log_seq)`，主键 `(a_pk, b_pk)` | VERIFIED | windows:3377 ddl::tests::create_table_declares_collation_flags_and_system_columns; integration:605 every_value_type_round_trips；源/链路详 03-03 |
| 03-03.3 | reload 是 `ontoforge_storage::reload::reload` 库函数（per D-15）：CLI（03-04）与 Phase 7 服务进程都只调用它；`ReloadOptions { dry_run: true }` 只返回将执行的 DDL，不建表、不写 `of_def_version` | VERIFIED | windows:3649 dry_run_changes_nothing；源/链路详 03-03 |
| 03-03.4 | 同一定义连续 reload 两次，第二次不产生任何 DDL、`def_version` 不变（STORE-02 idempotency）；SQLite 上 DDL 执行可重入：先内省 `table_info` 再 ADD COLUMN，中途失败后重跑能完成 | VERIFIED | windows:3667 reload_resumes_after_partial_ddl; windows:3669 second_reload_is_noop；源/链路详 03-03 |
| 03-03.5 | 新增 object type / M:N link / 可空属性在 reload 时自动生成 DDL；删属性、删 type、改属性类型（含 decimal 精度）、改 pk、新增 required 属性、link 端点变化一律判为破坏性，报 ONT-V25（指向 `ontoforge migrate --plan`）且不执行任何 DDL、不切换定义（STORE-02、orchestrator 决定 2） | VERIFIED | windows:3648 destructive_changes_are_rejected；源/链路详 03-03 |
| 03-03.6 | store 层按 `Backend` / `Db` 枚举区分 SQLite 与 PostgreSQL（STORE-01 的同一份 store 代码）；本 plan 的 PG 分支返回明确的 `StoreError::Config`、不 panic，PG 实现在 03-04 | VERIFIED | integration:655 first_reload_on_pg_puts_edit_tables_in_edit_schema；源/链路详 03-03 |
| 03-03.7 | storage 发往数据库的 SQL 文本只经 `sql_exit.rs`：其他 src 文件不得出现 SQL 关键字开头的字符串字面量与 `AssertSqlSafe`，也不得出现 SQLite 的替换式冲突写法（源码守卫测试） | VERIFIED | windows:3738 sql_text_only_from_sql_exit；源/链路详 03-03 |
| 03-03.8 | edit store 物理格式（set_mask 编码、命名与长度上限、系统列、两种方言的取值编码与列类型、PG 两个 schema、SQLite 独立文件）经 Task 1 checkpoint 确认并记录于 ADR-0024，03-04 的 PG 方言按此实现 | VERIFIED | ADR-0024 accepted 记录 orchestrator 的 checkpoint、__set/命名/63-byte/方言/schema 决定；physical/ddl/store 当前编码对应。这里核对治理记录与实现，不把 SUMMARY 当批准。；源/链路详 03-03 |
| 03-04.1 | `ontoforge reload <dir> --server-config <file>` 对 aviation 示例在 SQLite edit store 中建出 edit 表并报告 `def_version 1`（STORE-02）；CLI 只做加载 → validate + lint → 打开 store → 调用 `ontoforge_storage::reload::reload` → 输出（per D-15），validate 或 E 级 lint 错误时不打开 store、不执行 DDL | VERIFIED | windows:1485 invalid_definitions_never_open_store; windows:1488 reload_creates_tables; windows:3650 first_reload_creates_tables；源/链路详 03-04 |
| 03-04.2 | `ontoforge reload --dry-run` 只打印将执行的 DDL，不建表、不写 `of_def_version`（per D-15） | VERIFIED | windows:1483 dry_run_prints_ddl_only；源/链路详 03-04 |
| 03-04.3 | 退出码与诊断：成功 0；定义诊断（含库函数返回的 ONT-V25）与缺 `[store]` 的 ONT-V20 为 1；store 不可用（ONT-V26）为 3；`--format json` 输出 `{"diagnostics": [...], "result": {"defVersion", "changed", "dryRun", "ddl"}}` | VERIFIED | windows:1303 json_output_has_exactly_the_documented_fields_and_no_ansi; windows:1379 json_output_reports_the_result; windows:1436 plan_json_output; windows:1486 json_output; windows:1487 missing_store_section；源/链路详 03-04 |
| 03-04.4 | PostgreSQL 后端：edit 表建在 `schema_edit`（缺省 `ontoforge_edit`），系统表与 sqlx 迁移记录在 `schema_sys`（缺省 `ontoforge_sys`）；string 列 `COLLATE "C"`；DDL 用 `IF NOT EXISTS`，中途失败后重跑能完成；同一 reload 测试在 PG 上经 `ONTOFORGE_PG_URL` 或 `--features embedded-pg` 通过（STORE-01） | VERIFIED | integration:655 first_reload_on_pg_puts_edit_tables_in_edit_schema; integration:656 additive_reload_on_pg_adds_columns；源/链路详 03-04 |
| 03-04.5 | 两种方言的 DDL 由 insta 快照固定，PG 列类型与 ADR-0024 一致 | VERIFIED | windows:3377 ddl::tests::create_table_declares_collation_flags_and_system_columns；源/链路详 03-04 |
| 03-04.6 | `support::pg_store()` 在两种 PG 来源都缺失时 panic，ignored 的 PG 测试不得静默通过；03-08 起的 `conformance_pg` 复用它 | VERIFIED | storage/tests/support/mod.rs 的 pg_store 对 ONTOFORGE_PG_URL 或 embedded-pg 两路均缺失时 panic；当前 integration 已设置 URL，具名 PG 测试非早退。；源/链路详 03-04 |
| 03-05.1 | 行谓词只用 cel 0.15 解析（经 `ontoforge_expr::analyze` 的大栈线程），随即降为自有 `Pred` IR；求值不使用 cel 运行时（其 `null == 'a'` 为 false、`null != 'a'` 为 true，与 D-06 相反） | VERIFIED | windows:2457 rejects_everything_outside_the_subset；源/链路详 03-05 |
| 03-05.2 | Rust 单行求值器实现 SQL 三值（Kleene）逻辑（per D-06）：比较任一侧为 Null → Unknown；`== null` / `!= null` 恒为二值；`&&` / `\|\|` / `!` 按 Kleene 真值表；`x in [...]`：有相等元素 → True，列表为空 → False（x 为 Null 时也为 False），x 为 Null 且列表非空 → Unknown，否则 False；`x in principal.attrs.<list>` 而该属性缺失（Null）→ Unknown；字符串匹配任一侧为 Null → Unknown；最终 Unknown 视为不可见 | VERIFIED | windows:2430 in_empty_list_is_false_even_for_null; windows:2432 in_literal_list_is_three_valued; windows:2433 kleene_laws；源/链路详 03-05 |
| 03-05.3 | 因此 `object.x != 'a'` 在 x 为 null 时不可见；`object.region == null \|\| object.region == principal.attrs.region` 让未分配区域的对象可见——这一与标准 CEL 不同的语义写入 `docs/scope.md` 与 ADR-0025（per D-06） | VERIFIED | windows:2428 explicit_null_branch_visible; windows:2435 null_column_not_equal_is_unknown；源/链路详 03-05 |
| 03-05.4 | 允许的子集（per D-07）：`==` `!=` `<` `<=` `>` `>=`、`&&` `\|\|` `!`、`== null` / `!= null`、`in`（右侧为列表字面量或 list 型 `principal.attrs`）、`startsWith` / `endsWith` / `contains`（目标必须是 string 属性）；拒绝宏（all / exists / exists_one / map / filter）、`has()`、`matches`、算术、`size`、三元、bytes / uint 字面量与其他函数，各给出带表达式内 line:col 的 `PredIssue` | VERIFIED | windows:2450 accepts_string_matching; windows:2457 rejects_everything_outside_the_subset；源/链路详 03-05 |
| 03-05.5 | 同行限制（ADR-0018 / D12）：自由变量根只允许 `object` 与 `principal.attrs`；`object.<name>` 必须是该 type 的现存属性；`principal.attrs.<name>` 必须在 server config 声明（D-05）；`principal.id` / `principal.groups` 本 phase 不开放 | VERIFIED | windows:2440 unknown_root_rejected；源/链路详 03-05 |
| 03-05.6 | 类型检查比 `infer.rs` 更严：比较两侧必须同族（bool / 整数族 / double 族 / string），int 与 double 不得混比；`<` `<=` `>` `>=` 只用于 string 与数值；date / timestamp / decimal / array / geopoint 属性不得出现在比较、`in` 与字符串匹配中（`== null` / `!= null` 除 geopoint 外均可）；列表字面量元素同族且不含 null | VERIFIED | windows:2436 ordering_follows_value_families; windows:2457 rejects_everything_outside_the_subset；源/链路详 03-05 |
| 03-05.7 | 缺失的主体属性在求值时为 Null（per D-05），fail-closed 来自「Unknown 不可见」而不是对缺失属性特判（SCOPE-05） | VERIFIED | windows:2434 missing_attr_is_unknown_and_invisible；源/链路详 03-05 |
| 03-05.8 | `Pred` 的 Display 输出可再次解析的规范 CEL 文本，`compile(display(p)) == p` 由 proptest 覆盖 | VERIFIED | windows:2427 display_round_trip；源/链路详 03-05 |
| 03-06.1 | 管理员可以在独立的 scope 目录（不与 ontology 定义或 `SourceBinding` 混放）用 YAML / JSON 写 `kind: Role` 与 `kind: RoleBinding`（per D-01），`ontoforge_schema::loader::load_scope_dir` 把目录合并为 `ScopeSet` IR，错误带文件名与 line:col | VERIFIED | windows:3080 loads_aviation_scope_example; windows:3085 scope_diagnostics；源/链路详 03-06 |
| 03-06.2 | `Role` 的授权项字段：`objectTypes.<Type>.properties`（属性 allow-list）与可选 `rowFilter`（CEL 文本）、`actions`、`sqlLevels`（raw / admin，L0 隐含）、`l2.catalogs` / `l2.schemas`（`catalog.schema`）、`toolsets`（core / full / sql:raw，ADR-0019）；全部 camelCase + `deny_unknown_fields`（ADR-0012，SCOPE-02） | VERIFIED | windows:3089 toolset_value_is_checked; windows:3090 unknown_field_is_v04_with_location；源/链路详 03-06 |
| 03-06.3 | `RoleBinding` 把一个 role 绑定到 subjects：`user: "<iss>\|<sub>"`（principal_id，per D-04）或 `group: <name>`（per D-02，组成员关系不落库，来自 JWT `groups` claim）；subjects 为空、同一 binding 内 subject 重复、user 不是恰好一个 `\|` 分隔的非空两段、binding 指向未定义的 role → ONT-V21；同名 role 或同一 role 的两个 RoleBinding 文档 → ONT-V08 | VERIFIED | windows:3077 binding_to_unknown_role_is_v21; windows:3078 duplicate_role_is_v08; windows:3086 second_binding_for_role_is_v08; windows:3088 subject_rules；源/链路详 03-06 |
| 03-06.4 | ontology 目录中出现 Role / RoleBinding、scope 目录中出现 ObjectType / LinkType / ActionType / SourceBinding → ONT-V03，help 指出应放的目录 | VERIFIED | windows:3082 object_type_in_scope_dir_is_v03; windows:3084 role_in_ontology_dir_is_v03; windows:3087 source_binding_in_scope_dir_is_v03；源/链路详 03-06 |
| 03-06.5 | `ontoforge schema export` 导出第三个文件 `ontoforge-scope.schema.json`；示例 scope 文档通过该 schema 校验；`schemas/` 与代码一致由 drift 测试守护 | VERIFIED | windows:3060 committed_schemas_are_up_to_date; windows:3068 scope_schema_is_the_third_exported_file；源/链路详 03-06 |
| 03-06.6 | 示例 `examples/scopes/aviation/` 含三个虚构 role（dispatcher、crewScheduler、maintenanceLead）与它们的 RoleBinding；dispatcher 与 crewScheduler 对 Flight 授予不同的列集与不同的行谓词，构成「A 的列 × B 的行」交叉用例 | VERIFIED | windows:1910 a_columns_times_b_rows_do_not_leak; windows:3080 loads_aviation_scope_example；源/链路详 03-06 |
| 03-07.1 | 客户端持 OIDC IdP 签发的 JWT（RS256 / ES256 / EdDSA 等非对称算法）经 `Authenticator::authenticate` 认证为 `Principal`；JWKS 从 `ontoforge-server.toml` 配置的 `jwks_uri` 自动拉取并缓存（AUTH-01，ADR-0008 自写缓存） | VERIFIED | windows:1829 es256_token_becomes_principal; windows:1836 jwks_is_fetched_once_for_repeated_tokens; windows:1846 rs256_token_becomes_principal；源/链路详 03-07 |
| 03-07.2 | `Principal.id` = `"{iss}\|{sub}"`（per D-04），同时保留 issuer 与 subject；`Principal.groups` 来自配置的 `groups_claim`（per D-02，组成员关系不落库）；多个 issuer 受信时组名规范化为 `"{iss}\|{group}"`，单个 issuer 时为裸组名（orchestrator 决定 8） | VERIFIED | windows:1846 rs256_token_becomes_principal; windows:1851 two_issuers_namespace_groups；源/链路详 03-07 |
| 03-07.3 | `Principal.attrs` 按 `[auth.principal_attrs]` 的 claim → 属性映射与类型生成（per D-05）；claim 缺失 → 属性缺失（求值为 Null）；claim 类型与声明不符、list claim 含 null 或异类元素 → 拒绝该令牌 | VERIFIED | windows:1840 list_claim_with_null_or_foreign_element_is_rejected; windows:1841 missing_attr_claim_is_absent; windows:1847 scalar_claim_of_the_wrong_type_is_rejected；源/链路详 03-07 |
| 03-07.4 | 验证器按令牌头的 `alg` 路由，不看未验证 payload 决定算法：只接受该 issuer 配置的非对称算法白名单；`none`、`HS*` 在 OIDC 路径一律拒绝；永不跟随令牌头的 `jku` / `x5u` / `jwk`，JWKS 地址只来自配置 | VERIFIED | windows:1824 alg_none_is_rejected; windows:1825 algorithm_outside_the_issuer_allow_list_is_rejected; windows:1833 header_key_urls_are_never_followed；源/链路详 03-07 |
| 03-07.5 | `iss`、`aud`、`exp`、`sub` 必填并校验，`nbf` 若存在则校验，leeway 取配置（缺省 30 s）；过期、错 issuer、错 audience、签名错、未知 issuer 都返回不含令牌内容的错误 | VERIFIED | windows:1830 expired_token_is_rejected; windows:1842 missing_required_claims_are_rejected; windows:1843 not_yet_valid_token_is_rejected; windows:1853 wrong_audience_is_rejected; windows:1854 wrong_issuer_is_rejected；源/链路详 03-07 |
| 03-07.6 | JWKS 缓存：TTL 取响应 `Cache-Control: max-age` 并夹在 [60 s, 24 h]、缺省 1 h；`kid` 未命中时单飞重拉，两次重拉至少间隔 `jwks_min_refresh_secs`，未知 kid 负缓存；拉取失败时继续使用旧密钥集直到 `jwks_max_stale_secs` 后 fail-closed；响应体上限 1 MiB、请求超时 10 s | VERIFIED | windows:1827 cache_control_max_age_is_clamped; windows:1828 concurrent_kid_miss_is_single_flight; windows:1838 kid_miss_refetches_once_then_throttles; windows:1845 oversized_jwks_rejected; windows:1848 stale_keys_survive_idp_outage；源/链路详 03-07 |
| 03-08.1 | 一次 `EditStore::commit` 在单个数据库事务内完成：锁定或创建 edit 行 → 计算新状态 → 写 `of_edit_log` 头行取得 `seq` → 写 `of_edit_log_entry` → 写 `of_edit_history` 快照；任一步失败整体回滚，不留下 edit 行或日志（STORE-03） | VERIFIED | integration:596 commit_appends_one_log_record; integration:597 commit_set_creates_edit_row; integration:599 commit_writes_history_snapshot; integration:607 invalid_values_are_rejected_without_writes；源/链路详 03-08 |
| 03-08.2 | 一条 log 记录 = 一次成功提交（per D-11）：全局单调 `seq`、ULID `event_id`、actor（principal_id 或保留的 `system:import`）、action、提交时间（Rust 侧生成）、`def_version`、Action 参数原样 JSON；`seq` 在全部行锁取得之后才分配，因此同一 pk 的 seq 顺序与版本顺序一致 | VERIFIED | integration:596 commit_appends_one_log_record; integration:600 concurrent_commits_on_same_pk_serialize；源/链路详 03-08 |
| 03-08.3 | 每条 entry 只存新值与操作类型（per D-10）：`(kind, type_name, pk, op ∈ {set, unset, tombstone, release, link_add, link_remove}, payload, version_after)`；set 的 payload 是「设置的属性 → 新值」，unset 的 payload 是属性名列表，其余为 null；不存旧值、不复制 source 值 | VERIFIED | integration:611 log_records_every_op; integration:618 second_set_patches_only_given_properties；源/链路详 03-08 |
| 03-08.4 | edit 行的值按属性类型校验后写入（byte / short / integer 的范围、decimal 精度与小数位、date / timestamp 规范文本、array 元素类型、geopoint 经纬度范围），pk 不可经 set 修改；不符返回 `EditError`，不写入 | VERIFIED | integration:605 every_value_type_round_trips; integration:607 invalid_values_are_rejected_without_writes；源/链路详 03-08 |
| 03-08.5 | conformance 框架：`tests/conformance/mod.rs` 的每个用例经 `all_cases!` 同时生成 SQLite 实跑目标与 PG ignored 目标（STORE-01），后续 plan 只在 `conformance/mod.rs` 中追加用例 | VERIFIED | integration:605 every_value_type_round_trips；源/链路详 03-08 |
| 03-08.6 | edit log 的具体物理格式（三张表的列、取值 JSON 编码、unset payload、保留主体 `system:import`、历史快照结构）经 Task 1 checkpoint 确认后才首次写入（per D-10 / D-11，one-way）；ADR-0026 由 03-09 记录 | VERIFIED | ADR-0026 accepted 的 decider/Context 记录 03-08 checkpoint 与 S1-S4；postgres/sqlite 0002_edit_log.sql、log/history 当前列与约束对应，三张表由真实 shared conformance 验证。；源/链路详 03-08 |
| 03-09.1 | edit log、entry 与历史表只能追加（per D-12）：PG 与 SQLite 都以触发器拒绝 UPDATE / DELETE，PG 另以语句级触发器拒绝 TRUNCATE，SQLite 的替换式写入在 `recursive_triggers = ON` 下同样被拒；同一 conformance 用例在两个后端通过 | VERIFIED | integration:585 append_only_rejects_sqlite_replace_writes; integration:587 append_only_rejects_truncate_on_pg; integration:588 append_only_rejects_update_and_delete；源/链路详 03-09 |
| 03-09.2 | 按 seq 顺序把 edit log 重放到一个全新 store，重建出的 edit 表与原 store 逐列相等（per D-10），重放时计算出的版本与 entry 记录的 `version_after` 不一致即报错 | VERIFIED | integration:614 replay_detects_version_divergence; integration:615 replay_rebuilds_edit_store；源/链路详 03-09 |
| 03-09.3 | 同一个 `Commit` 提交两次产生两条 log 记录、版本各 +1，store 层不去重（幂等键是 Phase 5 的 `command_id`）；SQLite 与 PG 结果相同（STORE-01 idempotency） | VERIFIED | integration:616 same_commit_twice_creates_two_records；源/链路详 03-09 |
| 03-09.4 | ADR-0026 记录 edit log 格式（依据 03-08 Task 1 checkpoint 结论与实际落地的三张表）、seq 在行锁之后分配的理由、保留主体 `system:import`、重放语义与 append-only 的两道防线 | VERIFIED | ADR-0026 记录三张表、行锁后分配 seq、system:import、version_after 重放校验和 append-only/部署权限双边界，当前 commit/log/migration 与其一致。；源/链路详 03-09 |
| 03-10.1 | reload 按 ADR-0016 的索引规则建索引：pk 为主键；声明 `indexed: true` 的属性建 `of_ix_<table>_<col>`；oneToOne link 的外键属性在 `to` 端 edit 表上建部分唯一索引 `of_ux_<table>_<col>`（条件：该列已设置且行未删除），使 1:1 基数在 edit store 写入时由数据库强制（STORE-05） | VERIFIED | windows:3646 aviation_reload_creates_indexes; windows:3658 one_to_one_unique_index_rejects_second_row；源/链路详 03-10 |
| 03-10.2 | 索引对账是加法式的：新增的索引在 reload 时 `IF NOT EXISTS` 创建；普通索引从不删除（可能被 role 行谓词依赖，03-16 / 03-18）；link 从 oneToOne 放宽为 oneToMany 时删除对应唯一索引（只删索引、不丢数据）；收紧为 oneToOne 时若已有 edit 数据违反唯一性，reload 失败、保留旧定义并报 ONT-V26 | VERIFIED | windows:3662 relaxing_one_to_one_drops_unique_index; windows:3668 removing_indexed_keeps_index; windows:3674 tightening_to_one_to_one_with_conflicts_keeps_old_definition；源/链路详 03-10 |
| 03-10.3 | 物理名超过 PG 63 字节限制时按 ADR-0024 截断并附 FNV-1a 哈希；截断后两个定义映射到同一表 / 列 / 索引名，或属性列与系统列（`of_*`）同名（如属性 `ofVersion`），reload 报 ONT-V24 且不执行 DDL | VERIFIED | windows:3653 long_names_round_trip; windows:3670 snake_collision_is_v24; windows:3672 system_column_collision_is_v24；源/链路详 03-10 |
| 03-10.4 | diff 覆盖 link 级变化：新增 M:N link → 建表；删除 link、改端点 type → ONT-V25；decimal 精度 / 小数位或 array 元素类型变化 → ONT-V25；已归档列（`archived_columns`）不参与「删除」判断 | VERIFIED | windows:3652 link_changes_are_destructive; windows:3657 new_many_to_many_link_creates_a_table; windows:3659 precision_and_items_changes_are_destructive；源/链路详 03-10 |
| 03-11.1 | 同一 `Pred` 经 `ontoforge_auth::scope::lower` 降为 printer AST、打印为 SQLite SQL 执行的三值结果，与 `ontoforge_expr::rowpred::eval` 对同一行同一主体属性的结果逐一相等；完整 proptest 只跑 Rust vs SQLite（per D-09 / D23，SCOPE-04） | VERIFIED | windows:1861 corpus_cases_match_on_sqlite; windows:1865 rust_eval_equals_sqlite；源/链路详 03-11 |
| 03-11.2 | Trino 端跑固定种子语料 + 历史反例（`tests/data/rowpred_corpus.json`）：`rowpred_trino_it` 在 Linux CI 对 `demo.rowpred.rows` 逐行比较 Trino 与 Rust 的三值结果（per D-09，03-20 接入 CI） | VERIFIED | integration:1117 trino_matches_rust_on_the_corpus_cases; integration:1118 trino_matches_rust_on_seeded_predicates；源/链路详 03-11 |
| 03-11.3 | 主体属性在编译期代入为字面量：缺失 → `NULL`；空列表 → 常量 FALSE（printer 拒绝空 `IN ()`，Trino 不接受）；任何未知值的比较降为 `<列> = NULL`，三值结果为 Unknown，与 Rust 一致 | VERIFIED | windows:1862 empty_list_attr_is_false_on_both_sides; windows:1864 null_column_comparison_is_unknown_on_both_sides；源/链路详 03-11 |
| 03-11.4 | 字符串匹配只经 printer 的新节点 `Expr::StringMatch` 输出：Trino 方言为 `LIKE '<模式>' ESCAPE '\'`（转义 `\`、`%`、`_`），SQLite 方言为 `GLOB '<模式>'`（`[`→`[[]`、`*`→`[*]`、`?`→`[?]`）；模式文本最后仍经 `quote_string` 出口；两方言都区分大小写、通配符按字面 | VERIFIED | windows:2181 string_match_matches_rust_on_sqlite; integration:1117 trino_matches_rust_on_the_corpus_cases；源/链路详 03-11 |
| 03-11.5 | 多 role 合并（ADR-0018，SCOPE-03）：`TypeScope.row = OR(R_i)`，`TypeScope.columns[x] = OR(R_i where x ∈ C_i)`；没有任何 role 授予该 type 时不构造 TypeScope（fail-closed，SCOPE-05）；行可见 = row 为 True，列 x 可见 = 行可见且 columns[x] 为 True；同一结构既可 Rust 求值也可降为 SQL，供 Phase 4 拼装「role 谓词 AND 原列过滤、遮蔽只在投影层」 | VERIFIED | windows:1914 lowered_scope_matches_evaluate_on_sqlite; windows:1915 no_grant_is_invisible；源/链路详 03-11 |
| 03-11.6 | 「A 的列 × B 的行」不泄露：只满足 role A 谓词的行上，只属于 role B 列集的列不可见（专项用例 + proptest：可见列集恰为 ∪{C_i \| R_i(row) 为 True}） | VERIFIED | windows:1910 a_columns_times_b_rows_do_not_leak; windows:1918 visible_columns_are_union_of_true_grants；源/链路详 03-11 |
| 03-11.7 | 同一属性被两个 role 授予时，列谓词是两 role 谓词的 OR；两 role 谓词结构相同时只出现一次（SCOPE-02 adjacency） | VERIFIED | windows:1913 identical_role_predicates_appear_once; windows:1917 same_property_from_two_roles_is_or；源/链路详 03-11 |
| 03-12.1 | 开发者无需外部 IdP，用 `ontoforge dev-token --server-config <file> --sub alice --groups dispatchers --attr homeAirports=FRA,MUC` 生成本地令牌（AUTH-02）；dev token 的 groups 由 CLI 指定（per D-02） | VERIFIED | windows:1360 issues_token_verifiable_by_dev_verifier；源/链路详 03-12 |
| 03-12.2 | dev token 为 HS256，`iss` 与 `aud` 固定为 `ontoforge-dev`，principal_id 为 `ontoforge-dev\|<sub>`（per D-04）；属性按 `[auth.principal_attrs]` 的 claim 名与类型写入，验证时复用与 OIDC 相同的 claims → Principal 映射（per D-05） | VERIFIED | windows:1365 scalar_attrs_are_converted_by_declared_type; windows:1803 dev_token_round_trip；源/链路详 03-12 |
| 03-12.3 | 签名密钥是 32 字节 OS 随机数（getrandom），首次使用时以独占创建方式写入 `--key-file` 或 `[auth.dev] key_file`；Unix 上权限 0600，Windows 依赖用户目录 ACL（文档注明）；已有密钥文件被复用 | VERIFIED | windows:1809 key_file_is_created_once_and_reused; ubuntu:1826 key_file_permissions_unix；源/链路详 03-12 |
| 03-12.4 | dev 验证器只能由调用方显式持有 `DevMode` 凭证构造；`Authenticator::from_config` 即使读到 `[auth.dev]` 段也不启用 dev 验证，此时 HS256 令牌一律被拒（Phase 7 的 `serve --dev` 是唯一预期的持有者） | VERIFIED | windows:1802 config_alone_never_enables_dev_tokens；源/链路详 03-12 |
| 03-12.5 | 验证器按令牌头 `alg` 路由：HS256 → dev 路径（仅在已启用时），非对称 → OIDC 路径；HS256 令牌的 `iss` 不是 `ontoforge-dev`、签名错、过期 → 拒绝 | VERIFIED | windows:1804 expired_dev_token_rejected; windows:1806 hs256_with_other_issuer_rejected; windows:1817 wrong_key_rejected；源/链路详 03-12 |
| 03-13.1 | 叠加按属性 patch（STORE-07）：edit 只存设置过的属性，未设置的属性跟随 source；unset 清除对应 `__set` 与取值；tombstone 压制 source 行且不会被 set「复活」（返回 `Tombstoned`）；release 是显式状态（清除全部 `__set`、回到纯 source），只对 source-backed type 合法；版本每次成功写入严格 +1、永不重置或减小 | VERIFIED | integration:619 source_object_lifecycle; windows:3612 model_matches_store；源/链路详 03-13 |
| 03-13.2 | 状态转移表（实现与参考模型共同的规格）：不存在 + set → 创建（editOnly 须含全部 required 属性）；不存在 + tombstone → source-backed 允许（压制 source 行）、editOnly 为 `NotFound`；live + set / unset / tombstone → 允许；live + release → source-backed 允许、editOnly 为 `ReleaseOnEditOnly`；tombstoned + release → 回到纯 source（source-backed）；tombstoned + set / unset / tombstone → 拒绝；released + set → 重新 live；released + tombstone → 允许；released + unset / release → 拒绝；editOnly 的 unset 不得清除 required 属性 | VERIFIED | integration:604 edit_only_rules; integration:619 source_object_lifecycle; windows:3612 model_matches_store；源/链路详 03-13 |
| 03-13.3 | M:N link 边与对象同一叠加规则（D19）：add → `of_deleted = false`，remove → `of_deleted = true`（保留行以压制 source join 行），两者可反复切换且每次版本 +1；同一边连续 add 两次只有一行 live 行、版本 2（STORE-05 idempotency） | VERIFIED | integration:601 duplicate_link_add_keeps_one_live_row; integration:609 link_add_remove_toggle；源/链路详 03-13 |
| 03-13.4 | oneToOne 基数在 edit store 写入时强制（STORE-05）：两个对象把同一外键值设为已设置时，第二次提交返回 `EditError::CardinalityViolation { link }` 且整个提交回滚；oneToMany 的外键天然单值；source 侧基数不在本 phase | VERIFIED | integration:603 concurrent_fk_claims_one_wins; integration:612 one_to_one_violation_is_reported；源/链路详 03-13 |
| 03-13.5 | 内存参考模型 proptest（SC3）：随机提交序列同时作用于朴素参考模型与真实 SQLite store，每步的成功 / 失败类别、全部 edit 行与 link 行逐一相等；序列结束后把日志重放到新 store 与原 store 逐列相等 | VERIFIED | windows:3612 model_matches_store；源/链路详 03-13 |
| 03-13.6 | 并发（STORE-01 concurrency）：两个并发提交修改同一 pk 在 SQLite（`BEGIN IMMEDIATE`）与 PG（行锁）上都串行化，最终版本为 2，两条日志的 seq 顺序与 `version_after` 顺序一致；两个并发提交争用同一 oneToOne 外键值时恰好一个成功，失败者不留任何 edit 行或日志 | VERIFIED | integration:600 concurrent_commits_on_same_pk_serialize; integration:603 concurrent_fk_claims_one_wins；源/链路详 03-13 |
| 03-14.1 | `compile_scope` 是 scope 校验与编译的同一个函数（per D-01：apply 前离线执行，不通过则整体不生效）：输入 ontology、选定 profile、`[auth.principal_attrs]` 的类型、`ScopeSet` 与可选 `SourceMap`，输出 `CompiledScope` 或全部诊断（不在第一个错误处停止） | VERIFIED | windows:1883 all_errors_reported_together; windows:1885 aviation_scope_compiles；源/链路详 03-14 |
| 03-14.2 | ONT-V21：role 引用不存在的 object type / 属性 / action、属性 allow-list 不含该 type 的主键（orchestrator 决定 4）、allow-list 重复、`toolsets` 含 `sql:raw` 或 `l2` 非空而 `sqlLevels` 不含 `raw`、`l2.schemas` 不是 `catalog.schema` 形式 | VERIFIED | windows:1884 allow_list_without_primary_key_is_v21; windows:1892 l2_without_raw_level_is_v21; windows:1901 unknown_object_type_is_v21；源/链路详 03-14 |
| 03-14.3 | ONT-V22：`rowFilter` 超出 D-07 子集、类型错误、引用非同行属性或未在 `ontoforge-server.toml` 声明的 `principal.attrs`（per D-05 / D-07）；或引用了在选定 profile 绑定中声明 `collation: caseInsensitive` 的列（per D-08）；诊断定位到 `rowFilter` 值并在消息中给出表达式内行列 | VERIFIED | windows:1886 bad_row_filter_is_v22_located; windows:1887 collation_case_insensitive_is_v22; windows:1899 undeclared_attr_is_v22；源/链路详 03-14 |
| 03-14.4 | ONT-V23（criteria 列覆盖，SCOPE-06）：role 被授予某 action 时，该 action 每条 submission criteria 中 `objects.<param>.<prop>` 引用的属性必须在该 role 对参数所指 type 的 allow-list 内，`objects.<param>` 必须指向该 role 可见的 type；无法静态确定的动态路径一律报错（fail-closed）；诊断定位到 role 的 `actions` 条目 | VERIFIED | windows:1785 dynamic_object_path_is_v23_fail_closed; windows:1786 hidden_criteria_column_is_v23_at_the_action_entry；源/链路详 03-14 |
| 03-14.5 | fail-closed（SCOPE-05）：主体没有任何 role 授予某 type 时，`PrincipalScope.types` 中没有该 type（不可见）；role 的 `objectTypes` 为空 = 该 role 不授予任何 type；省略 `rowFilter` = 该 role 对该 type 的全部行 | VERIFIED | windows:1890 empty_object_types_grants_nothing; windows:1894 principal_without_roles_sees_nothing；源/链路详 03-14 |
| 03-14.6 | `CompiledScope::for_principal` 按 `RoleBinding` 解析主体的 role：`user` subject 与 `principal.id` 相等或 `group` subject 属于 `principal.groups`（D-02，多 issuer 时组名已带 issuer 前缀）；同一 role 被多条 subject 命中只计一次；types 经 03-11 的 `merge` 合并，actions / sqlLevels / l2 / toolsets 取并集 | VERIFIED | windows:1893 principal_with_two_roles_gets_merged_flight_scope; windows:1903 user_subject_binding；源/链路详 03-14 |
| 03-14.7 | `CompiledScope::row_filter_properties` 给出每个 type 被行谓词引用的属性集合，供 03-16 / 03-18 的索引对账（ADR-0016「行谓词引用的属性」） | VERIFIED | windows:3664 reload_creates_row_filter_indexes；源/链路详 03-14 |
| 03-15.1 | 被 reload 以 ONT-V25 拦下的破坏性变更可经 `ontoforge migrate --plan` 查看确定的步骤清单与 plan hash，再经 `ontoforge migrate --apply --plan-hash <hash>` 执行；apply 在同一事务内重算 plan，hash 不一致即拒绝且不执行任何 DDL（防 TOCTOU） | VERIFIED | windows:1433 migrate_plan_then_apply; windows:3581 apply_with_stale_hash_is_rejected；源/链路详 03-15 |
| 03-15.2 | 旧列归档 = 同表改名为带 `def_version` 后缀的归档列（per D-16）：`<col>` → `<col>__arch_v<N>`、`<col>__set` → `<col>__set__arch_v<N>`，保留类型、不搬数据、读路径忽略（记录在 `PhysicalSchema.archived_columns`）；删除的 type 整表改名为 `of_obj_<x>__arch_v<N>`；改属性类型 = 归档旧列 + 新建同名空列；改 pk = 归档整表 + 新建表 | VERIFIED | windows:3582 archive_removed_property; windows:3587 primary_key_change_archives_table; windows:3589 removed_type_archived; windows:3594 type_change_archives_and_adds; integration:646 archive_removed_property_on_pg；源/链路详 03-15 |
| 03-15.3 | 改名在 diff 上表现为删 + 增（per D-14）；`--rename <Type>.<old>=<new>` 与 `--rename <OldType>=<NewType>` 显式声明时把 edit 数据随列 / 表改名搬到新名下（类型必须相同，否则报错）；未声明时旧列归档、新列为空 | VERIFIED | windows:3590 rename_object_type_moves_table; windows:3591 rename_property_moves_data; windows:3595 undeclared_rename_archives_and_adds_an_empty_column；源/链路详 03-15 |
| 03-15.4 | 清理归档列只经显式子命令 `ontoforge migrate --drop-archived --plan` / `--apply`（per D-16）；SQLite 先删除归档列上的索引再 `DROP COLUMN` | VERIFIED | windows:1431 drop_archived_plan_and_apply; windows:3583 drop_archived_cleans_up；源/链路详 03-15 |
| 03-15.5 | migrate 与 reload 共用同一个 DDL 执行与定义切换路径（03-18 的 `lock_timeout`、advisory 锁、栅栏与 Trino 可见探测因此同时作用于 migrate）；新的 `of_def_version` 行在 `migration_json` 中记录本次执行的步骤与 rename 声明，供跨版本重放使用 | VERIFIED | windows:3656 migrate_waits_for_trino_after_the_switch；源/链路详 03-15 |
| 03-15.6 | migrate 成功后对同一定义 reload 不再报 ONT-V25；对已执行的 plan 再次 `--apply --plan-hash <旧 hash>` 因 plan 已变（为空）而被拒绝，不会重复归档（STORE-02 idempotency） | VERIFIED | windows:3588 reapply_after_success_is_rejected；源/链路详 03-15 |
| 03-16.1 | 管理员用 `ontoforge scope plan <scope-dir> --ontology <dir> --server-config <file>` 查看将要发生的 role 增 / 改 / 删与绑定增 / 删（含将删除的绑定，per D-02 全量同步）及 plan hash，再用 `ontoforge scope apply ... --plan-hash <hash>` 写入系统库（per D-01，SCOPE-01） | VERIFIED | windows:1512 plan_then_apply_aviation_scope; windows:3722 full_sync_removes_bindings_and_roles；源/链路详 03-16 |
| 03-16.2 | apply 之前离线执行 03-14 的 `compile_scope`（行谓词校验与 criteria 列覆盖，per D-01 / SCOPE-06）；任一诊断 → 退出码 1，系统库不发生任何写入；ontology 与 edit store 当前生效定义不一致时同样拒绝（提示先 reload） | VERIFIED | windows:1507 invalid_scope_writes_nothing; windows:1510 ontology_differing_from_the_active_definition_is_refused；源/链路详 03-16 |
| 03-16.3 | apply 在单个事务内：重算 plan 并比对 hash → 写 `of_role`（规范化 IR JSON + 内容 hash）、`of_role_binding` → 为行谓词引用的属性建 `of_ix_<table>_<col>` 索引（只增不删，orchestrator 决定 8）→ 追加一条 `of_admin_audit`（actor = OS 用户 @ 主机、plan JSON、前后 hash）；`of_admin_audit` 与 edit log 一样只能追加（触发器） | VERIFIED | windows:3718 apply_creates_row_filter_indexes; windows:3720 apply_writes_roles_bindings_and_audit; windows:3721 audit_insert_failure_rolls_back_scope_and_indexes；源/链路详 03-16 |
| 03-16.4 | apply 是系统库中 role / 绑定的唯一写入口（per D-03，SCOPE-07）：storage 中只有 `scope_store.rs` 写 `of_role` / `of_role_binding`，源码守卫测试强制；Phase 7 的 REST 管理端对 role 只读 | VERIFIED | windows:3730 scope_tables_written_only_by_scope_store；源/链路详 03-16 |
| 03-16.5 | 首次认证的主体经 `EditStore::record_login(&Principal)` 自动建 user 记录，只存 principal_id、issuer、subject、创建时间与最近登录时间，不存任何权限或 claim（per D-04）；同一主体的最近登录时间在进程内至多 60 秒写一次 | VERIFIED | windows:3724 login_recorder_writes_once_per_interval; windows:3729 record_login_creates_user_once；源/链路详 03-16 |
| 03-16.6 | plan 输出与内容 hash 与 YAML 中的书写顺序无关：role 按 apiName、绑定按 (role, kind, subject) 排序；IR 中的列表字段（properties、actions、sqlLevels、toolsets、l2）在存储与哈希前排序去重（SCOPE-02 ordering） | VERIFIED | windows:3725 order_independent_hash；源/链路详 03-16 |
| 03-16.7 | `docs/scope.md` 增「Role 与 RoleBinding」一节：目录约定、字段、plan / apply、全量同步、审计、组来自 JWT `groups` claim（多 issuer 时带 issuer 前缀）、dev token 的组、REST 只读 | VERIFIED | docs/scope.md 的 Role/RoleBinding、目录、全量同步、审计、issuer groups、dev token、REST只读均对应 scope_store/CLI 和 auth claims 实现。；源/链路详 03-16 |
| 03-17.1 | 客户端（经 Phase 7 外壳）可以查询某个对象的变更历史：`EditStore::history` 按 seq 从新到旧返回该对象的每次变更（seq、event_id、actor、action、提交时间、操作类型、变更后版本），支持 `limit`（≤ 500）与 `before_seq` 游标（STORE-04；本 phase 交付库级 API） | VERIFIED | windows:3543 pagination_keeps_entries_of_one_commit_together; windows:3544 pagination_with_before_seq；源/链路详 03-17 |
| 03-17.2 | 历史按当前 scope 遮蔽（per D-13）：调用者须对该对象**当前**可见；不可见、不存在、调用者对该 type 无任何授权三种情况返回同一个 `HistoryError::NotFound`（Display 相同） | VERIFIED | windows:3538 invisible_check_precedes_store_access; windows:3539 invisible_nonexistent_and_ungranted_are_identical；源/链路详 03-17 |
| 03-17.3 | 每条历史只返回当前可见列的值，可见列由当前行经同一 `TypeScope::evaluate` 求出（不是历史行）；其余被改动的列只出现在 `masked_changes` 中、不给值；序列化后的响应不含任何被遮蔽列的值 | VERIFIED | windows:3547 serialized_history_never_contains_masked_values; windows:3549 visibility_uses_current_row；源/链路详 03-17 |
| 03-17.4 | Action 原始参数只对管理通道（`CallerKind::Admin`）返回，普通调用者的每条历史 `params` 为 None（per D-13） | VERIFIED | windows:3545 params_hidden_from_ordinary_caller；源/链路详 03-17 |
| 03-18.1 | `def_version` 栅栏（ADR-0016 / D20）：每次提交在写事务开始时读取 `of_def_state.active_def_version`（PG `FOR SHARE`），与 `Commit.def_version` 不等即 `DefinitionStale` 并整体回滚；reload / migrate 的切换 `UPDATE of_def_state` 等待在途写事务结束，之后旧定义版本的写入一律被拒（SCOPE-07「每个请求固定使用开始时的定义版本」的写侧） | VERIFIED | windows:3671 stale_writer_rejected_after_switch; integration:659 in_flight_commit_blocks_switch_on_pg; integration:665 reload_racing_commits_never_lets_an_old_write_through_on_pg；源/链路详 03-18 |
| 03-18.2 | reload 原子生效（STORE-02）：DDL 事务前置 `SET LOCAL lock_timeout`（取 `[store] lock_timeout_ms`）与 `pg_advisory_xact_lock`（与 migrate、scope apply 共用同一把锁）；锁等待超时（SQLSTATE 55P03）→ `ReloadError::LockTimeout`、保留旧定义（CLI 输出 ONT-V26、退出码 3） | VERIFIED | integration:658 definition_lock_is_shared_and_bounded_by_lock_timeout; integration:663 lock_timeout_keeps_old_definition；源/链路详 03-18 |
| 03-18.3 | 两个 reload 并发时单飞：第二个等待第一个完成后看到新的生效定义，diff 为空、不重复建表、`def_version` 只增加一次；SQLite 上由 `BEGIN IMMEDIATE` 保证（STORE-02 concurrency） | VERIFIED | windows:3647 concurrent_reloads_single_flight; integration:661 concurrent_reloads_single_flight_on_pg；源/链路详 03-18 |
| 03-18.4 | reload 时对系统库中已 apply 的 role 重跑 `compile_scope`（行谓词校验与 criteria 列覆盖，per D-18 / SCOPE-06）；任一失败则整体不切换、不执行任何 DDL，保留旧定义，诊断指明 role 与 type；通过时为行谓词引用的属性对账索引（只增不删） | VERIFIED | windows:3664 reload_creates_row_filter_indexes; windows:3666 reload_rechecks_applied_roles；源/链路详 03-18 |
| 03-18.5 | 配置了 Trino 的 PG edit store（profile 引擎为 trino 且 `[store.trino] catalog` 存在）：DDL 提交后定义版本先处于 `pending`，reload 执行 `CALL <catalog>.system.flush_metadata_cache()`，再用 `Engine::describe_tables` 轮询直到新表与新列（数组列除外）对 Trino 可见才切换为 `active`；在 `probe_timeout_ms` 内不可见 → `ReloadError::NotVisible`，不切换（已提交的 DDL 是加法、对旧定义无害，重跑幂等） | VERIFIED | integration:664 pending_switch_on_pg_serves_the_old_definition_until_visible; integration:1144 cached_column_requires_flush_before_definition_switch；源/链路详 03-18 |
| 03-18.6 | Trino 对 `CALL` 返回的无列结果被 client 正确处理为空的 `QueryOutput`（wiremock 测试）；`CALL` 文本只由 printer 生成 | VERIFIED | windows:1987 printer::tests::flush_metadata_cache_is_a_call_of_the_catalog_procedure; windows:1988 printer::tests::flush_metadata_cache_quotes_the_catalog; windows:1989 printer::tests::flush_metadata_cache_rejects_an_empty_catalog; windows:2272 call_without_columns_returns_empty_output；源/链路详 03-18 |
| 03-18.7 | migrate 走与 reload 相同的锁、栅栏、探测与切换路径（同一组测试断言 migrate 也受 `lock_timeout` 与探测约束） | VERIFIED | windows:3656 migrate_waits_for_trino_after_the_switch；源/链路详 03-18 |
| 03-19.1 | 开发者可以用 `ontoforge import <ontology-dir> --server-config <file> --type <ObjectType> --file <csv\|json>` 导入 editOnly object type 的对象，或用 `--link <linkType>` 导入 editOnly M:N link 的边（STORE-06，orchestrator 决定 3）；替身引擎数据仍由 Phase 2 的绑定 fixture 提供 | VERIFIED | windows:1377 imports_airports_csv; windows:3560 editonly_link_import；源/链路详 03-19 |
| 03-19.2 | 导入作为系统 edit 记入 edit log（per D-11）：actor 与 action 都是保留主体 `system:import`，params 记录文件名、批次序号与行数；每批（缺省 1 000 行、`--batch-size` 1..=10 000）一次提交，`def_version` 取当前生效版本 | VERIFIED | windows:3557 airports_csv_is_imported_as_system_edits; windows:3559 batches_follow_batch_size；源/链路详 03-19 |
| 03-19.3 | 整个文件先完整解析与校验，再写入：类型转换（与 edit store 取值规则一致，decimal / date 复用 engine fixture 的公开转换函数）、required 属性、文件内主键重复、未知列、行数上限（缺省 1 000 000，`--max-rows`）任一失败 → ONT-V27（带文件名与行号）且不写入任何行 | VERIFIED | windows:3563 file_level_validation; windows:3566 json_decimals_are_never_rounded；源/链路详 03-19 |
| 03-19.4 | 已存在对象的处理显式选择：`--on-existing error`（缺省，create-only：任何已存在的 edit 对象 → ONT-V27 列出主键、不写入）、`skip`（跳过已存在者，可用于中断后续跑）、`update`（对已存在者执行 set）；tombstoned 对象在任何模式下都不被复活 | VERIFIED | windows:3562 existing_objects_block_create_only; windows:3574 tombstoned_object_is_never_revived；源/链路详 03-19 |
| 03-19.5 | 跨批次不是单事务：某一批失败时之前的批次已提交，CLI 报告已提交的批次与失败位置；以 `--on-existing skip` 重跑可续完（导入二次执行的结果确定） | VERIFIED | windows:1380 link_import_and_partial_failure; windows:3559 batches_follow_batch_size；源/链路详 03-19 |
| 03-20.1 | PR 的 Linux `integration` 作业在真实 PostgreSQL 17 上运行 edit store 的全部 PG 用例（`conformance_pg`、`reload_pg`、`scope_store` 的 ignored 用例），与 Windows / Ubuntu `test` 作业的 SQLite 用例是同一套 conformance（STORE-01、ROADMAP SC2） | VERIFIED | integration:605 every_value_type_round_trips; integration:663 lock_timeout_keeps_old_definition; integration:672 pg_scope_apply；源/链路详 03-20 |
| 03-20.2 | 同一作业对 Trino 483 运行行谓词固定语料（`rowpred_trino_it`，per D-09）与 reload 可见探测（`reload_trino_it`，STORE-02）；`ci/pg/rowpred_corpus.sql` 在 `demo` 库中建立语料表 | VERIFIED | integration:1117 trino_matches_rust_on_the_corpus_cases; integration:1144 cached_column_requires_flush_before_definition_switch；源/链路详 03-20 |
| 03-20.3 | 「ignored 测试确实存在」守卫覆盖全部新增目标（每个目标 `--ignored --list` 计数 > 0），绿色结果不可能意味着什么都没跑 | VERIFIED | integration 原始日志 I:1324/1327/1330/1333/1336/1339/1342/1345/1348/1351 给出非零 discovery；同一 target 另有 actual test ok 与 summary，不把 --list 当执行。；源/链路详 03-20 |
| 03-20.4 | `ci/trino/catalog/editstore.properties` 设置 `postgresql.array-mapping=AS_ARRAY`，使 edit 表的数组列对 Trino 可见（Pitfall 7） | VERIFIED | ci/trino/catalog/editstore.properties 保留 postgresql.array-mapping=AS_ARRAY 和 metadata.cache-ttl=10m/metadata.cache-missing=true；当前缓存隔离具名测试 I:1144 通过。；源/链路详 03-20 |
| 03-20.5 | `test` 作业额外以 `--features embedded-pg` 跑一次 storage 的 clippy，防止本机 PG 测试路径失修；默认 `cargo test --workspace` 仍只依赖 SQLite（Windows 本机零外部依赖） | VERIFIED | ci.yml test 双平台的 clippy --features embedded-pg step 在最终 CI 均成功；workspace 默认运行的 PG/Trino cases 仍为 ignored，不计入这些后端通过数。；源/链路详 03-20 |
| 03-20.6 | `docs/edit-store.md` 写明部署要求：PG 两个 schema 与账号最小权限（服务进程对日志 / 历史 / 审计表只有 INSERT 与 SELECT，REVOKE UPDATE / DELETE / TRUNCATE；触发器对表所有者不是安全边界）、Trino catalog 只读 edit schema 且不开数据 / 结果缓存、数组映射、reload / migrate / scope / import / dev-token 的用法与退出码、`lock_timeout_ms` 与 `probe_timeout_ms` | VERIFIED | docs/edit-store.md 保留 PG schema/minimum privileges/REVOKE 与 owner trigger 非安全边界、Trino 只读与缓存、数组、各 CLI 用法/退出码和 timeout。当前 Iceberg/MinIO 文档变化属后继合同，不改 Phase03 表格式。；源/链路详 03-20 |
| 03-21.1 | decimal 属性收到带小数的 JSON 数字时以 ValueError 拒绝，提示把 decimal 写成字符串（如 "12.50"），不再经 f64 舍入；整数 JSON 数字（serde_json 的 is_i64 / is_u64）与十进制字符串按原文进入 edit 行、edit log 与历史快照（gap 1，CR-A01，STORE-03 / STORE-06，D-10） | VERIFIED | windows:3414 value::tests::decimal_json_numbers_must_be_exact_integers; windows:3566 json_decimals_are_never_rounded；源/链路详 03-21 |
| 03-21.2 | 17 位以上有效数字的 decimal（12345678.1234567891，decimal(38, 10)）经 commit 写入后，在 SQLite 与 PostgreSQL 上 edit 行、日志 payload、下一次提交的历史快照三处逐字相同（共享 conformance 用例 every_value_type_round_trips） | VERIFIED | integration:605 every_value_type_round_trips；源/链路详 03-21 |
| 03-21.3 | `ontoforge import` 的 JSON 文件里带小数的 decimal 数字报 ONT-V27、整个文件不写入；同一个值写成字符串时逐字保存；20 位整数 JSON 数字原样保存（import.rs 第 763 行的属性路径） | VERIFIED | windows:3566 json_decimals_are_never_rounded；源/链路详 03-21 |
| 03-21.4 | 含 U+0000 的字符串（属性值、字符串主键、数组元素）、年份 0000 的 date、换算到 UTC 后不在 0001..9999 年内的 timestamp、含 U+0000 的 Action 参数字符串或键、action 名与 actor，都在校验阶段以 EditError::Value 拒绝且不写入；SQLite 与 PostgreSQL 结论相同，PostgreSQL 不再在校验通过之后以 StoreError 失败（WR-A01） | VERIFIED | integration:623 values_postgres_cannot_store_are_refused_on_every_backend；源/链路详 03-21 |
| 03-21.5 | `open_postgres` 使用的连接选项（`pg_connect_options`）在 search_path 旁固定 DateStyle、TimeZone 与 extra_float_digits；即使数据库级缺省为 DateStyle 'SQL, DMY'、extra_float_digits 0、TimeZone 'America/New_York'，会话上 `SHOW` 读到的也是 `ISO, MDY`、`3`、`UTC`（WR-A03） | VERIFIED | integration:622 pg_sessions_use_the_pinned_settings；源/链路详 03-21 |
| 03-22.1 | `ontoforge migrate --apply` 在定义锁内、执行任何迁移步骤之前，按新定义重新编译 `scope apply` 存下的 role；任一 role 不再成立（如 allow-list 引用了被删除的属性）时以 ONT-V21 / V22 / V23 拒绝、退出码 1，定义不切换、edit 表不变（gap 3a，WR-B07，D-18 / SCOPE-06） | VERIFIED | windows:1434 migrate_refuses_a_change_that_breaks_an_applied_role; integration:648 pg_migrate_rechecks_applied_roles；源/链路详 03-22 |
| 03-22.2 | 同一迁移在不做 role 复核时可以应用，说明拒绝来自 role 复核；与 role 无关的破坏性变更在带 role 复核时照常应用 | VERIFIED | windows:3586 migrate_with_roles_applies_when_the_roles_still_compile；源/链路详 03-22 |
| 03-22.3 | SQLite 与嵌入式 PostgreSQL 上行为一致：被拒绝的迁移不留下归档列，active def_version 不变 | VERIFIED | integration:648 pg_migrate_rechecks_applied_roles；源/链路详 03-22 |
| 03-22.4 | 提交条件整体读取对象参数（`objects.<param>`，不带属性，如 `objects.flight != null`、`objects.flight == params.x`）时，视为读取该类型的全部属性：role 的 allow-list 不含该类型的全部属性就报 ONT-V23；scope plan / apply、reload 与 migrate 的复核都经同一个 compile_scope 生效（gap 3b，WR-C05，D-01） | VERIFIED | windows:1788 whole_object_read_needs_every_property；源/链路详 03-22 |
| 03-23.1 | 在没有 `target/ontoforge-dev/` 的全新检出中，按 `examples/server/ontoforge-server.dev.toml` 运行 `ontoforge reload examples/ontologies/aviation` 退出码 0，并创建 `target/ontoforge-dev/editstore.db`；随后 `ontoforge scope plan` 也成功（gap 2，CR-D01，SC1 第一步） | VERIFIED | windows:1484 example_config_reload_works_from_a_fresh_tree；源/链路详 03-23 |
| 03-23.2 | 在同样的全新检出中，`ontoforge dev-token --server-config examples/server/ontoforge-server.dev.toml --sub mia` 退出码 0，stdout 只有一行 token，并创建 32 字节的 `target/ontoforge-dev/dev-token.key`（gap 2，AUTH-02） | VERIFIED | windows:1358 example_config_dev_token_works_from_a_fresh_tree；源/链路详 03-23 |
| 03-23.3 | SQLite edit store 打开时创建 `[store] path` 缺少的上级目录；dev 密钥首次创建时创建缺少的上级目录，Unix 上新建目录的权限为 0700。参数被拒绝时仍不留下任何文件 | VERIFIED | windows:1364 rejected_arguments_leave_no_key_file_behind; windows:1812 missing_key_directory_is_created; windows:3748 open_creates_the_parent_directories_of_the_sqlite_file；源/链路详 03-23 |
| 03-23.4 | 示例配置头注释与 docs/edit-store.md 如实描述首次使用时的行为；代码注释与文档都不再声称 Windows 上的密钥文件只有本人可读，并把这一限制记为已知缺口（WR-C01 / WR-D01，本 plan 不改 ACL） | VERIFIED | examples/server/ontoforge-server.dev.toml 与 docs/edit-store.md 说明自动创建父目录及 Windows 用户目录 ACL 限制；不声称本轮修复 Windows ACL。；源/链路详 03-23 |

## 04-scoped-l0-read

| ID | 原合同 truth | 判断 | 当前证据 |
|---|---|---|---|
| 04-01.1 | 客户端按 pk 读取时先走 edit store 的无副作用只读 API；只有 source-backed 对象仍需 source 列时才发一条 typed `LIMIT 2` source 查询（D-02，QUERY-01） | VERIFIED | windows:2586 edited_pk_uses_store_only; windows:2596 unedited_pk_uses_single_source_query；源/链路详 04-01 |
| 04-01.2 | tombstone 与越权对象都表现为 `NotFound`，release 回到 source，重复 source pk 表现为 `AmbiguousObject`；错误只含 apiName（D-03，QUERY-01/09） | VERIFIED | windows:2595 tombstone_is_not_found_and_release_reads_source; windows:2642 pk_probe_is_indistinguishable；源/链路详 04-01 |
| 04-01.3 | pk 结果只投影调用者在该合并行上可见且请求的属性，并携带绑定 principal/type/pk/definition/source/edit 状态的 HMAC `_version`（D-03/D-06，QUERY-09） | VERIFIED | windows:2644 version_changes_with_principal_scope_definition_and_row; windows:2646 version_query_uses_format_tag_and_changes_with_scope；源/链路详 04-01 |
| 04-01.4 | storage 读取不存在的 key 不插入 version 0 行、不加写锁、不改变 edit log 或定义状态（QUERY-08） | VERIFIED | windows:2589 missing_read_has_no_side_effect; windows:3623 get_batch_and_scan_preserve_rows_log_and_definition；源/链路详 04-01 |
| 04-01.5 | `ExecutionLimits` 由可信 host 构造并在请求开始冻结；Phase 4 初始默认是32 backend calls、10,000 raw candidates、2,048累计backfill keys、64 MiB decoded bytes，wire不能提供或调高，零值/溢出拒绝构造（QUERY-09） | VERIFIED | windows:2587 execution_limits_phase4_defaults_are_exact; windows:2588 execution_limits_reject_zero_and_overflow；源/链路详 04-01 |
| 04-02.1 | ObjectSet/WhereClause 是 serde wire contract，完整覆盖 eq/gt/gte/lt/lte/isNull/in/startsWith/containsAnyTerm/containsAllTerms/and/or/not，并在编译前按 ontology 类型检查（D-01，QUERY-02） | VERIFIED | windows:2678 every_where_operator_validates_and_evaluates；源/链路详 04-02 |
| 04-02.2 | 用户谓词只能读取该行上被某个适用 role 授予的逻辑列；source 下推把 column grant 与 predicate 绑定，合并后仍用同一 Rust 语义复核，filter/sort 不产生 role 行×列交叉（D-03，QUERY-09） | VERIFIED | windows:2569 hidden_columns_never_reach_sort_filter_or_projection; windows:2667 scope_row_and_filter_are_anded_without_role_crossing；源/链路详 04-02 |
| 04-02.3 | String token search 采用空白 term 的 any/all 匹配、无 relevance order；ASCII corpus 在 Rust/SQLite/Trino 上一致，无法安全下推的形态留给 bounded Rust recheck（D-05，QUERY-03/08） | VERIFIED | integration:1203 where_search_set_and_relationship_corpus；源/链路详 04-02 |
| 04-02.4 | L0 source compiler 只能产生一个 `TableRef`、没有 JOIN/CTE/UNION/raw SQL；typed printer 支持 outer CASE mask 与 typed distributed EXPLAIN（D-01，QUERY-10） | VERIFIED | windows:2665 all_source_variants_have_one_table_and_logical_aliases; windows:2668 source_select_has_exactly_one_table；源/链路详 04-02 |
| 04-03.1 | query crate 可按 logical type 与 canonical pk 批量读取 edit patches，单批硬上限 256，且 scan 使用 stable keyset、显式 limit，不做 offset/full-table materialization（D-02，QUERY-04） | VERIFIED | windows:3619 batch_accepts_256_keys_and_all_value_types; windows:3628 scan_uses_keyset_and_preserves_flags；源/链路详 04-03 |
| 04-03.2 | M:N link edit 可按任一 endpoint 有界读取，返回 typed endpoint keys、deleted/version；FK 不在 storage 建第二份关系表（D-05，QUERY-05） | VERIFIED | windows:3624 link_fk_request_is_rejected_with_logical_name; windows:3625 link_from_or_to_endpoint_is_bounded_and_preserves_removed_edges；源/链路详 04-03 |
| 04-03.3 | 所有 read APIs 在 SQLite/PG 都只执行 SELECT，不插 version 0 skeleton、不锁 FOR UPDATE、不改变 log/definition/table row count（QUERY-08） | VERIFIED | windows:3623 get_batch_and_scan_preserve_rows_log_and_definition; integration:1170 sqlite_and_postgres_object_reads_match；源/链路详 04-03 |
| 04-03.4 | storage error/public structs 使用 logical names；physical schema/table/column 只在 crate-private query builder 与日志 source 中存在（QUERY-09） | VERIFIED | windows:3622 corrupt_property_error_uses_logical_name；源/链路详 04-03 |
| 04-04.1 | source 与 edit candidates 并行取得后，由唯一 Rust production overlay 合并、去重、scope/recheck、排序，并以最多 256 keys 的 backfill/refill 填页（D-01/D-02，QUERY-04/08） | VERIFIED | windows:2573 overlay_matches_independent_reference; windows:2613 edge_overlay_matches_independent_reference; windows:2630 pagination_backfill_batches_never_exceed_256; windows:2638 pagination_refills_after_tombstone_and_patch_moves_row_into_filter；源/链路详 04-04 |
| 04-04.2 | keyset cursor 按规范化排序值与 pk 恢复，HMAC 绑定 principal、scope fingerprint、definition、object type 与 ordering；上下文不一致或被篡改时 fail closed（D-06，QUERY-04/09） | VERIFIED | windows:2626 cursor_multikey_null_pages_have_no_duplicates_or_omissions; windows:2628 cursor_rejects_tamper_and_context_replay；源/链路详 04-04 |
| 04-04.3 | 分页继续补取到 page 满或所有 candidate lane 耗尽；work budget 耗尽返回 typed explicit error，不把不完整结果伪装成正常页（D-02，QUERY-09） | VERIFIED | windows:2632 pagination_budget_exhaustion_returns_no_partial_page; windows:2636 pagination_refills_across_three_false_positive_windows；源/链路详 04-04 |
| 04-04.4 | `has_more` 由经 overlay/scope/recheck 后实际存在下一行决定；`truncated` 只表示最终 row/byte cap，不表示 refill 或 budget 状态（D-06，QUERY-04/09） | VERIFIED | windows:2629 final_row_and_byte_caps_truncate; windows:2640 pagination_stops_after_eligible_lookahead；源/链路详 04-04 |
| 04-04.5 | 同一请求共享冻结的`ExecutionLimits`：32 backend calls、10,000 raw candidates、2,048累计backfill keys、64 MiB decoded bytes；逐维B可成功、B+1无partial失败，256仍是单批keys上限（QUERY-09） | VERIFIED | windows:2577 pagination_execution_limit_boundaries_are_inclusive; windows:2639 pagination_source_and_edit_share_all_work_budgets；源/链路详 04-04 |
| 04-05.1 | FK searchAround 读取合并后对象的logical FK；M:N 将source join-table edges与PG link edits在Rust合并，Add/Remove与deleted语义遵守ADR-0024（D-05，QUERY-05） | VERIFIED | windows:2615 many_to_many_merges_source_and_link_edits_fixed; windows:2616 outbound_fk_uses_scoped_merged_object_value；源/链路详 04-05 |
| 04-05.2 | 任何related target都通过正常的scoped pk/list read hydrate；隐藏、tombstone或目标row scope拒绝时不返回且不可由错误区分（D-03/D-05，QUERY-05/09） | VERIFIED | windows:2614 many_to_many_hydrates_257_targets_in_bounded_batches；源/链路详 04-05 |
| 04-05.3 | union/intersect/subtract只对相同logical object type的已授权pk集合运算，随后复用正常overlay/projection/order/caps；集合代数不能成为scope或definition旁路（QUERY-06/09） | VERIFIED | windows:2656 set_result_reuses_normal_hydration_order_and_cursor; windows:2657 valid_same_type_set_and_cross_type_rejection；源/链路详 04-05 |
| 04-05.4 | FK、M:N与set operations各有独立reference corpus，candidate重复、edit移动、multi-role与空集合保持确定、去重且有界（D-07，QUERY-08） | VERIFIED | windows:2613 edge_overlay_matches_independent_reference; windows:2654 set_ops_match_independent_btreeset_model; windows:2655 set_ops_match_independent_btreeset_model_fixed；源/链路详 04-05 |
| 04-06.1 | count/sum/avg 用source partial加edited-key old/new contribution correction得到exact merged result；avg由sum+count组成，不能平均两个avg（D-04，QUERY-07） | VERIFIED | windows:2480 count_sum_avg_match_full_materialization_reference；源/链路详 04-06 |
| 04-06.2 | min/max以typed single-table TopN source windows与全部edit candidates合并，持续补取直到证明winner或返回显式work/cost error；不能因首个candidate失效返回错误值（D-04，QUERY-07/10） | VERIFIED | windows:2482 min_max_edited_values_can_win_or_remove_winner; windows:2483 min_max_refill_past_invalid_top_rows；源/链路详 04-06 |
| 04-06.3 | distinct与groupBy只使用scope后可见值；connector能力或预算不足以证明exact结果时返回typed explicit error，永不以`truncated`或partial buckets代替完成（D-03/D-04，QUERY-07/09） | VERIFIED | windows:2481 group_distinct_matches_visible_merged_reference；源/链路详 04-06 |
| 04-06.4 | aggregate fixed/property corpus覆盖SQLite reference、multi-role row×column、null、edited membership/value/group移动与connector能力选择（D-07，QUERY-08） | VERIFIED | windows:2478 count_sum_avg_connector_capability_selects_supported_plan; windows:2480 count_sum_avg_match_full_materialization_reference；源/链路详 04-06 |
| 04-06.5 | 可信host冻结的Phase 4初始`AggregateLimits`为1,024 groups、4,096 buckets、请求总计10,000 distinct values、16 MiB distinct memory；与`ExecutionLimits`共享同一请求budget，wire不可调高（QUERY-09） | VERIFIED | windows:2477 aggregate_wire_request_rejects_capabilities; windows:2479 count_sum_avg_limits_phase4_defaults_are_exact；源/链路详 04-06 |
| 04-07.1 | 真实Linux CI在PostgreSQL 17 + Trino 483上实际执行storage `query_read_pg`与query `trino_query_it` ignored tests，且独立discovery guard证明每个target至少有一个ignored test（D-07，QUERY-08） | VERIFIED | integration:1169 sqlite_and_postgres_link_reads_match; integration:1170 sqlite_and_postgres_object_reads_match; integration:1200 fixed_semantics_and_cursor_refill；源/链路详 04-07 |
| 04-07.2 | 每种source-producing query shape都对真实Trino执行`EXPLAIN (TYPE DISTRIBUTED)`，parser必须识别至少1个且不超过2个fragment；无法解析也失败（D-01，QUERY-10） | VERIFIED | windows:2605 truncated_distributed_explain_is_rejected_before_fragment_count; integration:1199 aggregate_corpus_and_distributed_explain；源/链路详 04-07 |
| 04-07.3 | 真实backend corpus覆盖multi-role、overlay、cursor、traversal、set、aggregate与physical-error消毒，SQLite expected与Trino/PG结果逐项一致（QUERY-08/09） | VERIFIED | integration:1200 fixed_semantics_and_cursor_refill; integration:1201 real_physical_errors_are_sanitized; integration:1202 real_security_roles_and_cursor_replay; integration:1203 where_search_set_and_relationship_corpus；源/链路详 04-07 |
| 04-07.4 | 专用无并行负载的Linux CI run以固定dataset、至少5次warmup和40次measured samples证明pk/list/aggregate/edited-PG-direct p95预算，并保存raw timings、plan、环境与run URL（D-07，QUERY-10） | VERIFIED | integration:1230 phase4_real_backend_p95；源/链路详 04-07 |

## 证据组与接线

### 03-01

依赖、SQLite连接选项与供应链守卫；历史批准单独判 UNCERTAIN。

- PLAN: .planning/phases/03-edit-store-scope-model/03-01-PLAN.md
- 实现: Cargo.toml、crates/ontoforge-storage/src/store.rs、.planning/adr/0023-phase3-crate-dependency-direction.md
- 接线: crates/ontoforge-storage/src/store.rs → crates/ontoforge-engine/src/sqlite/mod.rs；crates/ontoforge-storage/Cargo.toml → crates/ontoforge-auth/Cargo.toml

### 03-02

Object IR/schema 与 server config 校验；ONTV20/21..27 未被新 W18 替换。

- PLAN: .planning/phases/03-edit-store-scope-model/03-02-PLAN.md
- 实现: crates/ontoforge-schema/src/ir/object.rs、crates/ontoforge-schema/src/ir/attrs.rs、crates/ontoforge-expr/src/rowpred/value.rs、crates/ontoforge-engine/src/config.rs、crates/ontoforge-schema/src/diag.rs
- 接线: crates/ontoforge-engine/src/config.rs → crates/ontoforge-schema/src/ir/attrs.rs；crates/ontoforge-engine/src/config.rs → crates/ontoforge-schema/src/diag.rs

### 03-03

physical→diff→reload.execute_steps→sql_exit；dry-run/破坏性拒绝/幂等保持旧定义。

- PLAN: .planning/phases/03-edit-store-scope-model/03-03-PLAN.md
- 实现: crates/ontoforge-storage/src/physical.rs、crates/ontoforge-storage/src/diff.rs、crates/ontoforge-storage/src/reload.rs、crates/ontoforge-storage/src/sql_exit.rs、crates/ontoforge-storage/tests/sql_exit_guard.rs、.planning/adr/0024-edit-store-physical-format.md
- 接线: crates/ontoforge-storage/src/reload.rs → crates/ontoforge-storage/src/diff.rs；crates/ontoforge-storage/src/ddl.rs → crates/ontoforge-storage/src/sql_exit.rs；crates/ontoforge-storage/src/store.rs → crates/ontoforge-storage/migrations/sqlite/0001_definitions.sql

### 03-04

CLI加载校验后打开store并调用reload；PG双schema、DDL快照与缺env失败路径。

- PLAN: .planning/phases/03-edit-store-scope-model/03-04-PLAN.md
- 实现: crates/ontoforge/src/reload.rs、crates/ontoforge-storage/migrations/postgres/0001_definitions.sql、crates/ontoforge-storage/tests/support/mod.rs、crates/ontoforge-storage/tests/reload_pg.rs
- 接线: crates/ontoforge/src/reload.rs → crates/ontoforge-storage/src/reload.rs；crates/ontoforge-storage/src/store.rs → crates/ontoforge-storage/migrations/postgres/0001_definitions.sql；crates/ontoforge-storage/tests/reload_pg.rs → crates/ontoforge-storage/tests/support/mod.rs

### 03-05

CEL只作解析，自有Pred求值，SQL三值语义与受限引用；Display roundtrip property。

- PLAN: .planning/phases/03-edit-store-scope-model/03-05-PLAN.md
- 实现: crates/ontoforge-expr/src/rowpred/ir.rs、crates/ontoforge-expr/src/rowpred/parse.rs、crates/ontoforge-expr/src/rowpred/eval.rs、docs/scope.md、.planning/adr/0025-row-predicate-semantics.md
- 接线: crates/ontoforge-expr/src/rowpred/parse.rs → crates/ontoforge-expr/src/analyze.rs；crates/ontoforge-expr/src/rowpred/eval.rs → crates/ontoforge-expr/src/rowpred/value.rs

### 03-06

独立scope loader→ScopeSet/Role/RoleBinding；schema第三导出与示例授权交叉。

- PLAN: .planning/phases/03-edit-store-scope-model/03-06-PLAN.md
- 实现: crates/ontoforge-schema/src/ir/scope.rs、crates/ontoforge-schema/src/loader/scope.rs、schemas/ontoforge-scope.schema.json、examples/scopes/aviation/roles.yaml
- 接线: crates/ontoforge-schema/src/loader/scope.rs → crates/ontoforge-schema/src/loader/dispatch.rs；crates/ontoforge-schema/src/schema_export.rs → schemas/ontoforge-scope.schema.json

### 03-07

OIDC header算法白名单、claim验证、JWKS限额/单飞/缓存与issuer组命名。

- PLAN: .planning/phases/03-edit-store-scope-model/03-07-PLAN.md
- 实现: crates/ontoforge-auth/src/authn/mod.rs、crates/ontoforge-auth/src/authn/principal.rs、crates/ontoforge-auth/src/authn/jwks.rs、crates/ontoforge-auth/tests/oidc.rs
- 接线: crates/ontoforge-auth/src/authn/oidc.rs → crates/ontoforge-auth/src/authn/jwks.rs；crates/ontoforge-auth/src/authn/principal.rs → crates/ontoforge-expr/src/rowpred/value.rs

### 03-08

commit校验→单事务锁行→seq→行/entry/history→commit；任一步错误rollback。

- PLAN: .planning/phases/03-edit-store-scope-model/03-08-PLAN.md
- 实现: crates/ontoforge-storage/src/commit.rs、crates/ontoforge-storage/src/log.rs、crates/ontoforge-storage/src/value.rs、crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql、crates/ontoforge-storage/tests/conformance/mod.rs
- 接线: crates/ontoforge-storage/src/commit.rs → crates/ontoforge-storage/src/state.rs；crates/ontoforge-storage/src/commit.rs → crates/ontoforge-storage/src/sql_exit.rs；crates/ontoforge-storage/tests/conformance_pg.rs → crates/ontoforge-storage/tests/support/mod.rs

### 03-09

0002迁移触发器→append-only；read_log→replay，version_after拒绝不一致。

- PLAN: .planning/phases/03-edit-store-scope-model/03-09-PLAN.md
- 实现: crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql、crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql、crates/ontoforge-storage/src/log.rs、.planning/adr/0026-edit-log-format.md
- 接线: crates/ontoforge-storage/src/log.rs → crates/ontoforge-storage/src/state.rs；crates/ontoforge-storage/tests/conformance/mod.rs → crates/ontoforge-storage/src/log.rs；crates/ontoforge-storage/tests/conformance/mod.rs → crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql

### 03-10

physical/naming/diff/index→reload；部分唯一索引、只增索引和放宽基数删除unique。

- PLAN: .planning/phases/03-edit-store-scope-model/03-10-PLAN.md
- 实现: crates/ontoforge-storage/src/naming.rs、crates/ontoforge-storage/src/physical.rs、crates/ontoforge-storage/src/ddl.rs
- 接线: crates/ontoforge-storage/src/physical.rs → crates/ontoforge-schema/src/ir/link.rs；crates/ontoforge-storage/src/diff.rs → crates/ontoforge-storage/src/ddl.rs

### 03-11

Pred→scope.lower→typed printer→SQLite/Trino，与Rust eval逐行比；per-column role OR。

- PLAN: .planning/phases/03-edit-store-scope-model/03-11-PLAN.md
- 实现: crates/ontoforge-engine/src/printer/ast.rs、crates/ontoforge-engine/src/printer/escape.rs、crates/ontoforge-auth/src/scope/lower.rs、crates/ontoforge-auth/src/scope/merge.rs、crates/ontoforge-auth/tests/rowpred_diff.rs、ci/pg/rowpred_corpus.sql
- 接线: crates/ontoforge-auth/src/scope/lower.rs → crates/ontoforge-engine/src/printer/ast.rs；crates/ontoforge-engine/src/printer/dialect.rs → crates/ontoforge-engine/src/printer/escape.rs；crates/ontoforge-auth/src/scope/merge.rs → crates/ontoforge-expr/src/rowpred/eval.rs

### 03-12

CLI DevKey→HS256→显式DevMode；config本身不启用；Unix权限与Windows限制区分。

- PLAN: .planning/phases/03-edit-store-scope-model/03-12-PLAN.md
- 实现: crates/ontoforge-auth/src/authn/dev.rs、crates/ontoforge/src/dev_token.rs
- 接线: crates/ontoforge-auth/src/authn/mod.rs → crates/ontoforge-auth/src/authn/dev.rs；crates/ontoforge-auth/src/authn/dev.rs → crates/ontoforge-auth/src/authn/principal.rs

### 03-13

state→commit overlay转换，与独立Model.apply逐步值/错误/receipt/log replay比较；双后端并发。

- PLAN: .planning/phases/03-edit-store-scope-model/03-13-PLAN.md
- 实现: crates/ontoforge-storage/src/state.rs、crates/ontoforge-storage/tests/model/mod.rs、crates/ontoforge-storage/tests/model_prop.rs
- 接线: crates/ontoforge-storage/src/commit.rs → crates/ontoforge-storage/src/state.rs；crates/ontoforge-storage/src/commit.rs → crates/ontoforge-storage/src/physical.rs

### 03-14

compile_scope共用完整诊断；whole-object criteria覆盖、fail-closed与selected profile约束。

- PLAN: .planning/phases/03-edit-store-scope-model/03-14-PLAN.md
- 实现: crates/ontoforge-auth/src/scope/compile.rs、crates/ontoforge-auth/src/scope/coverage.rs、crates/ontoforge-auth/tests/criteria_coverage.rs
- 接线: crates/ontoforge-auth/src/scope/compile.rs → crates/ontoforge-expr/src/rowpred/parse.rs；crates/ontoforge-auth/src/scope/coverage.rs → crates/ontoforge-expr/src/walk.rs；crates/ontoforge-auth/src/scope/compile.rs → crates/ontoforge-auth/src/scope/merge.rs

### 03-15

CLI plan hash→锁内重算/角色复核→归档/rename→共用reload探测发布；拒绝旧hash。

- PLAN: .planning/phases/03-edit-store-scope-model/03-15-PLAN.md
- 实现: crates/ontoforge-storage/src/migrate.rs、crates/ontoforge/src/migrate.rs、crates/ontoforge-storage/tests/migrate.rs
- 接线: crates/ontoforge-storage/src/migrate.rs → crates/ontoforge-storage/src/reload.rs；crates/ontoforge/src/migrate.rs → crates/ontoforge-storage/src/migrate.rs

### 03-16

scope plan/apply→scope_store单事务/元数据锁/索引/append audit；login去节流。

- PLAN: .planning/phases/03-edit-store-scope-model/03-16-PLAN.md
- 实现: crates/ontoforge-storage/src/scope_store.rs、crates/ontoforge-storage/src/index.rs、crates/ontoforge-storage/migrations/postgres/0003_scope.sql、crates/ontoforge/src/scope.rs、examples/server/ontoforge-server.dev.toml
- 接线: crates/ontoforge/src/scope.rs → crates/ontoforge-auth/src/scope/compile.rs；crates/ontoforge/src/scope.rs → crates/ontoforge-storage/src/scope_store.rs；crates/ontoforge-storage/src/scope_store.rs → crates/ontoforge-storage/src/index.rs

### 03-17

history_with_snapshot先visible_columns，再真实entry/history查询，按当前可见列mask；Admin才取params。

- PLAN: .planning/phases/03-edit-store-scope-model/03-17-PLAN.md
- 实现: crates/ontoforge-storage/src/history.rs、crates/ontoforge-storage/tests/history_masking.rs
- 接线: crates/ontoforge-storage/src/history.rs → crates/ontoforge-auth/src/scope/merge.rs

### 03-18

commit def fence；reload/migrate/scope同定义锁；pending/flush/describe/publish与旧版本服务。

- PLAN: .planning/phases/03-edit-store-scope-model/03-18-PLAN.md
- 实现: crates/ontoforge-storage/src/reload.rs、crates/ontoforge-engine/src/printer/call.rs、crates/ontoforge-storage/tests/reload_trino_it.rs
- 接线: crates/ontoforge-storage/src/commit.rs → crates/ontoforge-storage/src/reload.rs；crates/ontoforge-storage/src/reload.rs → crates/ontoforge-auth/src/scope/compile.rs；crates/ontoforge-storage/src/reload.rs → crates/ontoforge-engine/src/printer/call.rs

### 03-19

import完整文件校验→editOnly/create-only→分批commit/system:import，跨批非原子明确报告。

- PLAN: .planning/phases/03-edit-store-scope-model/03-19-PLAN.md
- 实现: crates/ontoforge-storage/src/import.rs、crates/ontoforge/src/import.rs、examples/data/aviation/airports.csv
- 接线: crates/ontoforge-storage/src/import.rs → crates/ontoforge-storage/src/commit.rs；crates/ontoforge-storage/src/value.rs → crates/ontoforge-engine/src/sqlite/fixture.rs

### 03-20

ci.yml→query-services PG/Trino→实际ignored执行与discovery→deployment docs；不把全局绿作为证据。

- PLAN: .planning/phases/03-edit-store-scope-model/03-20-PLAN.md
- 实现: .github/workflows/ci.yml、docs/edit-store.md、ci/trino/catalog/editstore.properties
- 接线: .github/workflows/ci.yml → crates/ontoforge-storage/tests/conformance_pg.rs；.github/workflows/ci.yml → ci/pg/rowpred_corpus.sql

### 03-21

decimal保真与PG可表示域/固定会话设置，否定JSON浮点舍入。

- PLAN: .planning/phases/03-edit-store-scope-model/03-21-PLAN.md
- 实现: crates/ontoforge-storage/src/value.rs、crates/ontoforge-storage/src/commit.rs、crates/ontoforge-storage/src/store.rs、crates/ontoforge-storage/tests/conformance/mod.rs、crates/ontoforge-storage/tests/conformance_pg.rs
- 接线: crates/ontoforge-storage/src/import.rs → crates/ontoforge-storage/src/value.rs；crates/ontoforge-storage/src/commit.rs → crates/ontoforge-storage/src/value.rs；crates/ontoforge-storage/tests/conformance_pg.rs → crates/ontoforge-storage/src/store.rs

### 03-22

CLI传RolesCheck；migrate.apply_in锁内compile_scope后才DDL；全对象criteria读取需全列。

- PLAN: .planning/phases/03-edit-store-scope-model/03-22-PLAN.md
- 实现: crates/ontoforge-storage/src/migrate.rs、crates/ontoforge/src/migrate.rs、crates/ontoforge-auth/src/scope/coverage.rs、crates/ontoforge-storage/tests/migrate_pg.rs
- 接线: crates/ontoforge/src/migrate.rs → crates/ontoforge-storage/src/migrate.rs；crates/ontoforge-storage/src/migrate.rs → crates/ontoforge-storage/src/scope_store.rs；crates/ontoforge-auth/src/scope/compile.rs → crates/ontoforge-auth/src/scope/coverage.rs

### 03-23

fresh-tree reload/scope plan/dev-token与父目录权限，输入拒绝无文件副作用。

- PLAN: .planning/phases/03-edit-store-scope-model/03-23-PLAN.md
- 实现: crates/ontoforge-storage/src/store.rs、crates/ontoforge-auth/src/authn/dev.rs、examples/server/ontoforge-server.dev.toml、docs/edit-store.md
- 接线: crates/ontoforge/src/dev_token.rs → crates/ontoforge-auth/src/authn/dev.rs；crates/ontoforge/src/reload.rs → crates/ontoforge-storage/src/store.rs

### 04-01

QueryService::execute→read_object_edit→按需LIMIT 2 source→overlay→TypeScope.evaluate→投影/TokenCodec；只读store API。

- PLAN: .planning/phases/04-scoped-l0-read/04-01-PLAN.md
- 实现: crates/ontoforge-storage/src/query_read.rs、crates/ontoforge-query/src/lib.rs、crates/ontoforge-query/tests/pk_read.rs
- 接线: crates/ontoforge-query/src/lib.rs → crates/ontoforge-storage/src/query_read.rs；crates/ontoforge-query/src/lib.rs → crates/ontoforge-auth/src/scope/merge.rs；crates/ontoforge-query/src/lib.rs → crates/ontoforge-engine/src/printer/mod.rs

### 04-02

wire ast→compile类型与grant校验→typed single TableRef；Rust merged-row复核，token any/all不做相关度。

- PLAN: .planning/phases/04-scoped-l0-read/04-02-PLAN.md
- 实现: crates/ontoforge-query/src/ast.rs、crates/ontoforge-query/src/compile.rs、crates/ontoforge-engine/src/printer/ast.rs、crates/ontoforge-query/tests/sql_shape.rs
- 接线: crates/ontoforge-query/src/compile.rs → crates/ontoforge-auth/src/scope/merge.rs；crates/ontoforge-query/src/compile.rs → crates/ontoforge-engine/src/printer/ast.rs

### 04-03

query_read typed SELECT，≤256 batch与stable keyset scan；FK无额外关系表，PG/SQLite逐值比。

- PLAN: .planning/phases/04-scoped-l0-read/04-03-PLAN.md
- 实现: crates/ontoforge-storage/src/query_read.rs、crates/ontoforge-storage/tests/query_read.rs、crates/ontoforge-storage/tests/query_read_pg.rs
- 接线: crates/ontoforge-storage/src/query_read.rs → crates/ontoforge-storage/src/physical.rs；crates/ontoforge-storage/src/query_read.rs → crates/ontoforge-storage/src/commit.rs

### 04-04

execute并行候选→overlay去重→backfill/refill→scope/recheck→lookahead/cap；cursor HMAC和共享work budget。

- PLAN: .planning/phases/04-scoped-l0-read/04-04-PLAN.md
- 实现: crates/ontoforge-query/src/overlay.rs、crates/ontoforge-query/src/execute.rs、crates/ontoforge-query/src/cursor.rs、crates/ontoforge-query/tests/overlay_prop.rs
- 接线: crates/ontoforge-query/src/execute.rs → crates/ontoforge-storage/src/query_read.rs；crates/ontoforge-query/src/execute.rs → crates/ontoforge-query/src/overlay.rs；crates/ontoforge-query/src/cursor.rs → crates/ontoforge-query/src/execute.rs

### 04-05

link source edges+edits→authorized target keys→normal execute_list_internal hydration；set同一路径。

- PLAN: .planning/phases/04-scoped-l0-read/04-05-PLAN.md
- 实现: crates/ontoforge-query/src/link.rs、crates/ontoforge-query/src/set.rs、crates/ontoforge-query/tests/search_around.rs、crates/ontoforge-query/tests/set_ops.rs
- 接线: crates/ontoforge-query/src/link.rs → crates/ontoforge-query/src/execute.rs；crates/ontoforge-query/src/link.rs → crates/ontoforge-storage/src/query_read.rs；crates/ontoforge-query/src/set.rs → crates/ontoforge-query/src/execute.rs

### 04-06

partial sum/count old/new correction；TopN补取；exact distinct/group与typed error；独立CSV/BTreeSet/reference模型。

- PLAN: .planning/phases/04-scoped-l0-read/04-06-PLAN.md
- 实现: crates/ontoforge-query/src/aggregate.rs、crates/ontoforge-query/tests/aggregate.rs、crates/ontoforge-query/tests/sql_shape.rs、crates/ontoforge-query/tests/security.rs
- 接线: crates/ontoforge-query/src/aggregate.rs → crates/ontoforge-query/src/execute.rs；crates/ontoforge-query/src/aggregate.rs → crates/ontoforge-query/src/compile.rs；crates/ontoforge-query/src/aggregate.rs → crates/ontoforge-storage/src/query_read.rs

### 04-07

当前integration环境/非零discovery/具名真实语义+EXPLAIN；独立isolated job执行四路5+40样本并上传raw artifact。

- PLAN: .planning/phases/04-scoped-l0-read/04-07-PLAN.md
- 实现: crates/ontoforge-query/tests/trino_query_it.rs、crates/ontoforge-query/tests/query_bench_it.rs、.github/workflows/ci.yml、.planning/phases/04-scoped-l0-read/04-PERFORMANCE.md
- 接线: .github/workflows/ci.yml → crates/ontoforge-query/tests/trino_query_it.rs；crates/ontoforge-query/tests/trino_query_it.rs → crates/ontoforge-engine/src/printer/mod.rs；crates/ontoforge-query/tests/query_bench_it.rs → .planning/phases/04-scoped-l0-read/04-PERFORMANCE.md

## 精确证据位置

- truth-results.json 保存每条truth的源码路径/行号与具名测试日志job/行号。
- selected-test-source.json 保存对应测试函数原文片段；预期模型与production状态函数分离，不能仅按函数名判断。
- 03/04-contracts.json 保存全部PLAN frontmatter、native artifact/link检查与原生stale详情。
- service-environment-evidence.json 保存PG/Trino环境注入、非零discovery和真实后端执行片段。
- executed-tests.json仅记录actual `test <name> ... ok`，不把ignored或discovery当PASS。总记录数含不同job/step重复，不能用3362作workspace通过总数。
- changed-source-inspection.json保存13个不同moved文件的完整git diff、核心crate无差异和manifest历史。
- evidence-inputs.json固定原始CI文件SHA256；covered-file-inspection.json固定407个旧覆盖文件的存在性/字节SHA与marker扫描。

## 未验证边界

03-01.1安装前人工legitimacy批准仅存在SUMMARY叙述，不计通过；请求开发者提供独立批准记录或明确override。没有源码失败证据，不将其写成FAILED。原native stale是证据新鲜性失效，不等同业务失败。没有新增测试需求；额外测试需先由主会话授权。当前性能仅限最终isolated runner；本worker未下载最新raw artifact或独立重算其每个样本，而是读取具名测试的实际断言、模式/环境和最终打印阈值。raw artifact上传成功由同一job的ID/zip SHA记录证明，不把旧run样本伪装为当前样本。
