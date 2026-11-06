# Phase05/06 报告门独立复核记录

## 范围和判断

- PLAN_ID=v1.0-audit；读取主计划追加报告门单元。仅修改两份 VERIFICATION 与本证据目录。
- 本轮执行 PowerShell、Node 只读源码/日志核验、GSD native queries 和 apply_patch；没有 Cargo、容器、服务器、新 CI、commit/push。ralph-loop/Warp 不可用，未声称调用。
- 初始 native status：05 stale（4 个 moved_files），06 stale（1 个 moved_file）；原始报告按字节保存为 05/06-VERIFICATION.before.md。
- 当前 HEAD/CI head 均为 060019fe52005c6b85ae5c5f59da9868f464e579，run 41994479295 五 job completed/success。五 job 绿仅作来源身份，不代替合同验收。
- 58 条合同逐条记录在 contracts.json；每个 behavior-dependent truth 必须有当前 CI 的具名 ok 和对应源码断言。未将 workspace ignored、0 tests 或 discovery 当作通过。
- 05 36 条是 5 ROADMAP SC + 32 PLAN truths 去重 1 条；06 22 条覆盖 5 ROADMAP SC 和 24 PLAN truths 的合并，未减少合同。

## 服务真实性

- I=ci-41994479295-integration.log。I:694-719 明确 PG URL、9 ignored tests discovered、running 9 tests、9 passed / 0 failed / 0 ignored。
- I:721-817 实际执行 PG fault 5、race 4、idempotency 6、outbox 2；I:819-848 实际非 ignored source_guard 1。
- I:950-1020 提供专属受限/宽松 Trino URL 58081/58082，6 个真实 ACL tests 全通过；trino_l2_acl_it.rs 中 assert_version 严格断言 483，测试创建并前后观察 RUNNING 查询，拒绝精确 PERMISSION_DENIED。32 条 Display SQL 使用大小写严格的 FINISHED marker。
- I:1234-1270 提供 ONTOFORGE_PG_URL 与 ONTOFORGE_TRINO_URL，先发现 4 个 Action source tests 再实际执行 4/0/0。trino_action_it.rs required() 对缺失 env panic，fixture 真实连接 editstore/demo；没有缺 env 的 Ok/return 早退。新增 schema counter 仅隔离 fixtures，当前 CI 的 source_schema_names_are_unique_at_same_timestamp 也实际 ok。
- SQLite fault 的 test-support 显式执行由 U:4173-4207、W:4196-4230 与本地 reverify-sqlite-atomic.log 共同证明；普通 workspace 未开 feature 的 0 项不算证据。

## 五项变化输入的实质复核

- nightly.yml：迁移到 Phase07.2 MinIO/Iceberg lifecycle runner，不再沿用旧 nightly 内嵌 Action 命令；Phase05 原合同要求的 Action PG/Trino 非零发现和真实执行仍在 ci.yml，且本 HEAD I 日志逐项成立。本次未宣称真实 scheduled failure issue/持续治理观察通过。
- trino_action_it.rs：schema 名增加进程 ID、时间与 AtomicUsize，新增并行唯一性断言；原四条真实服务行为及无 env fail-closed 保持。
- cli.rs/lib.rs：仅 validate --check-cardinality 和 warning summary，未改变 webhook-worker、Action、授权或幂等调用链。
- l2_monitor.rs 测试：startup 取消从固定 80ms 改为等 probe 注册后 abort，保存 query ID、确认 task 未结束、等待 DELETE 及移除该 ID，再验证新 monitor Enabled。没有删除取消、cleanup 或恢复断言；当前 U:2143/W 对应具名测试真实 ok。

## Native artifact/link 启发式纠偏

- 原样输出在 collection.json；05 artifact 一项 Missing export 为 Rust impl 方法识别限制，commit.rs:567 的 pub async fn commit_action 及 :586 guarded 实际存在。
- 05 三个 link pattern 漂移为 request_hash、SeaQuery .do_nothing()、source::prepare/required_properties；非断链。claim loser rollback 后 action_command 新读取，与真实 PG winner-abort/replay 测试相符。
- 06 十二项 Target not referenced 是目标文件路径不作为源码字符串出现，不能冒充12项缺实现：Rust mod/import/trait/方法链、store.rs:213/327 migration 宏、read_audit.rs READ_AUDIT_INSERT_* 参数绑定、真实 conformance/audit/ACL 测试逐项证实接线。报告 Key Link 表逐项给出实质依据。

## 测试质量和边界

- 读取具名测试函数与关键 helper：PG race 用独立连接和 Barrier，assert_single_winner 精确 (1,1) 并核对 durable rows/log/command/outbox。fault 对比多个真实表快照；audit shutdown 明确先 poll Pending 再释放 guard、等待并查询 durable row，非仅 await 返回值。
- 发现 dev_distributed_binary_query_action_history_without_checkout 名称过宽，其函数只启动进程并检查配置；明确不采用它证明 Action 后 Query 可见。SC5 改以 trino_action_unique_source_executes 的 QueryService 后读 properties[name] == New 与真实 CLI webhook 接收测试证明。不是 BLOCKER，另有直接覆盖。
- 未发现未引用 TBD/FIXME/XXX；没有 PLAN prohibitions/backstop/human-check 或 shell probe 合同。Fake Engine 与 MockServer 只证明分支/调用次数及开发 HTTP，不冒充生产。
- 正式治理、生产 webhook/接收端联调、时间敏感人工观察均未新增证据，不改绿；具体边界见 boundaries.json。05 #8 的 VERIFIED 仅表示历史 BLOCKED 与一次性替代治理记录准确，不表示正式 gate 执行通过。
- 没有新实现 gap，没有新测试请求。旧 H1/H2/H3/R1 的历史失败与 RED/GREEN 仍保留在原 covered 集合及旧报告快照，不删除既有治理文件。

## 闭环

先完成逐项源码/真实日志判断，再调用 native verification.fingerprint，严格 old covered_files ⊆ new covered_files，保留全部 .planning 项；最后 native verification.status 和独立内容/覆盖 gate。最终 machine receipt 为 final-gate.json。
