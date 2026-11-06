# 核验边界与治理判断

## 原始证据身份

- 当前HEAD: 060019fe52005c6b85ae5c5f59da9868f464e579；ci-41994479295-final.json的headSha一致，五个job completed/success。
- 生产源码工作树相对HEAD没有差异。storage/auth/expr/query四个核心crate相对旧签署f8851ca也没有差异；共享engine/CLI等moved文件仍重新核对，不用该结论跳过共享边界。
- 旧报告原始字节快照为03-VERIFICATION.before.md与04-VERIFICATION.before.md；与主会话report-resign-baseline-03/04.json原covered集合逐项对账。旧报告全文不作为行为证据。
- 不写STATE/ROADMAP/task_plan/共享SUMMARY/矩阵/索引，不执行Cargo、容器、CI触发、commit/push/发布/归档。不声称调用ralph-loop或Warp。

## changed files 的合同影响

| 文件 | 影响判断与当前证据 |
|---|---|
| CONTRIBUTING.md | 只更新后继MinIO/JDBC Iceberg运行与权限文档；不改变03-20的PG schema/minimum-privilege/append-only部署合同。 |
| Cargo.toml | native moved，但f8851ca..HEAD无文本diff。核对当前锁定版本/feature和TLS helper；不能从native moved推断依赖版本回归或臆造原始byte差异。 |
| ci/pg/seed.sql | 事务末尾新增独立cardinality_records及alias views，不改原aviation/rowpred/query corpus；本次真实原语/读语义具名测试均经过该seed。 |
| ci/trino/iceberg/iceberg.properties | 后继Phase07.2由local warehouse换S3/MinIO；04测试使用query-services.sh的独立runtime catalog，并非直接消费此空密码模板；本次不以Phase04结论替代ENGINE-05。 |
| crates/ontoforge-engine/src/lib.rs | 新增cardinality_check模块导出；旧Engine/typed printer接口保留。 |
| crates/ontoforge-engine/src/printer/ast.rs | 新增CountDistinct typed节点；03既有Pred节点及04 source Select没有变成raw SQL或多表。 |
| crates/ontoforge-engine/src/printer/dialect.rs | 仅打印count(DISTINCT typed_expr)，递归走同一安全printer；当前string/identifier roundtrip、source shape和真实EXPLAIN通过。 |
| crates/ontoforge-engine/src/printer/tests.rs | 新增CountDistinct针对别名fk/n/N的typed AST断言，旧断言保留；当前workspace具名tests实际执行。 |
| crates/ontoforge-engine/tests/doc_type_table.rs | 增加ONT-W18文档码允许集合，未移除原V码/required码；当前doc_diagnostic_codes_exist通过。 |
| crates/ontoforge-engine/tests/l2_monitor.rs | 更强的取消测试：等真实probe登记后abort，检查具体query被清理，允许重新启动；未改生产monitor；当前双平台具名测试通过。仅记录共享依赖影响，不扩大本phase L2验收。 |
| crates/ontoforge-schema/src/diag.rs | 只增加新W18，不替换V21..V27或放宽验证。 |
| crates/ontoforge/src/cli.rs | 增加需online的--check-cardinality显式flag；reload/migrate/scope/import/dev-token契约不变，原CLI负测仍执行。 |
| crates/ontoforge/src/lib.rs | 在线warning摘要分支更准确，未改变reload/migrate等入口或已有诊断退出码；source基数归Phase07.1合同。 |

完整diff、exit和命令在changed-source-inspection.json，native moved分别10/8项，不以合并后13项掩盖任一phase覆盖。

## 真实环境而非早退

- integration I:567已注入ONTOFORGE_PG_URL，PG conformance实际45/0/0，reload_pg 11、migrate_pg 4、scope_store 3、history_masking 1、import 2；同名workspace ignored均不计PASS。
- I:1105-1106注入PG与Trino，rowpred三个actual ok在1116-1118。rowpred_trino_it.rs:29对缺env panic，不return成功。
- I:1135-1136注入PG与Trino，reload_trino_it三个actual ok含cached_column_requires_flush_before_definition_switch:1144；测试先NoFlush断言NotVisible/active v1，再真实flush断言v2。
- I:1162为query_read_pg注入PG，actual 2 tests通过；I:1187-1188为trino_query_it注入两URL，1199-1203为5个actual ok。required()与support::pg_store()均缺env报错。
- phase7-real-services由ci/server-services.sh require_env、启动服务、生成dev token、等待health后run_ignored；S:586输出server ready，S:603为pg_http_mixed_snapshot_reload_revoke_and_history，S:631为snapshot_pg_never_mixes_definition_roles_bindings，S:643为trino_http_long_read_releases_snapshot_lock。脚本run_ignored按精确test名先discovery再执行，支持函数required()缺env失败，不是网络测试静默跳过。
- isolated job B:601是精确cargo命令，B:613-616为PG/Trino/isolated模式/artifact目录。该job有专用runner和自己的services，无同host并行测试负载。test源码measure在isolated分支确实执行5warmup+40measured，并逐样本error与严格p95阈值检查。
- B:1174-1177四路p95依次107.648791/472.588841/407.732135/1.127974ms，均严格小于500/1000/1500/50。B:1228上传artifact ID13400156425，zip SHA256 bb4104a29e00af3a078f8718eb4e123a17657ee262951341efb341f720b66499。
- 本worker没有重算当前raw JSON所有sample/fragment；不沿用旧41759457559的数值冒充当前测量。真实EXPLAIN parser拒绝0、>2或截断plan，当前aggregate_corpus_and_distributed_explain通过。

## 需求与延期边界

03全部16个ID、04全部10个ID均由PLAN声明，无orphan。REQUIREMENTS是合同，checkbox不是证据。STORE-05的edit写入unique/并发子合同由03证明；source报告明确映射Phase07.1，不在此报告重验或重新宣称全需求完成。原03-VALIDATION关于source报告未完成的描述已是历史快照，和当前REQUIREMENTS分阶段Complete存在时间边界；本worker无权修改该治理文件，保留且不因签署新指纹洗掉。SCOPE-07 MCP无变更工具/transport仍由06/07验收，03证据仅证明scope store管理写与元数据snapshot原语。

## 禁止项

四项均为verification:test而非judgment。TLS当前wired helper+6项hermetic负测(U:453-461)+真实helper执行；rowpred SQLite不更宽用SQLite/Rust property+当前Trino同语料差分；dev配置不能单独启用用config_alone_never_enables_dev_tokens；import source-backed/默认覆盖用source_backed_type_is_rejected与existing_objects_block_create_only。禁止项并非仅按声明status:resolved放行。无无enforcement却绿色的test-tier项。

## 人工项和advisory

- 03-20 PLAN两个human-check内容是CI步骤和非零计数，当前原始日志已逐目标满足并有实际执行，不需要再要求人打开同一页面；关闭的是可机器验证的检查，不伪称视觉UAT。
- 唯一未决WARNING是03-01.1安装前人工legitimacy批准只有SUMMARY声称。当前构建与tests不证明历史批准顺序。没有override，不能计VERIFIED。
- 未发现缺失/空实现/断链/动态数据静态替身；所有407旧covered文件存在，受核Rust源码没有未引用TBD/FIXME/XXX。新关切不凭推测升级BLOCKER。
- 保留旧advisory：Windows dev-key ACL/已有文件权限及symlink加固、公共apply_migration调用者主动提供RolesCheck的集成责任、历史SQLite race flaky。当前单次通过不等于flaky已根治。
- 不把Nyquist compliant声明或全局CI绿等同全部验收。Phase03文档人工证据未闭合，native应返回human_needed；Phase04只对其36项合同签passed。
