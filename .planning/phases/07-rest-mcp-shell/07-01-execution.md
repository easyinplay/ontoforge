# 07-01 执行记录

## 计划与边界

- 当前阶段计划固定为 `.planning/phases/07-rest-mcp-shell/task_plan.md`。共享计划与 T1 状态由 orchestrator 管理。
- 本记录是 executor 的 planning-with-files 工作记录，不创建根目录竞争计划。
- 按 01-1 tracer、01-2 CLI 顺序执行，每项先真实行为 RED、机器判定、test commit，再 GREEN、feat commit。
- 01-1 验收：三个 `tracer_` 测试通过，真实 TCP、文件 SQLite、真实 dev token、QueryService、重开后的 durable audit、source Pending 期间 snapshot/reload 不阻塞。
- 01-2 验收：两个 `serve_` 测试通过，参数/配置错误拒绝、真实 listener 与受控关闭，生产接 Ctrl+C。
- 最后执行 workspace regression、fmt、clippy、TLS gate，记录实际结果后提交 SUMMARY。

## 方法与前置检查

- 已加载 GSD execute-plan、TDD、checkpoints、summary、worktree-path-safety、executor-examples，以及本阶段 CONTEXT/RESEARCH/ENG-REVIEW/PATTERNS/TEST-MATRIX/EDGE-COVERAGE。
- 显式应用 planning-with-files 与 superpowers:test-driven-development。现有批准合同没有新增灰区，跳过重复 brainstorming。
- ralph-loop、Warp 未安装；使用实际 PowerShell，采用明确验收、修复、重验循环，不声称使用不可用工具。
- 共享文档存在 orchestrator 未提交修改；不纳入 executor commit。用户无关 untracked 路径保持原状。
- 已真实执行 project_root_pin，root 和分支匹配。
- Context7 MCP 与 ctx7 CLI 未暴露，库文档需要查阅批准 research 的 primary-source 链接或已安装源文档。

## 进度

- [x] 01-1 RED、机器判定、test commit `7897e39`
- [x] 01-1 GREEN（tracer 3/3、query audit 12/12）；feat commit 后执行 feedback gate
- [x] 01-1 feedback 再跑指定 tracer：3 passed，commit `5df9f2a`
- [x] 01-2 RED：Cargo 2 executed / 2 failed，nextest 原生 JUnit 同样 2 failed；`RED_EVIDENCE_OK`
- [x] 01-2 test commit `1df4a16`
- [x] 01-2 GREEN：serve_ 4/4；配置回归 35/35；等待原子 feat commit
- [ ] workspace/fmt/clippy/TLS
- [ ] SUMMARY 自检与提交

## 发现

- `request_snapshot` 必须复用 `reload::begin_definition_tx`，锁内读取 active definition 与 scope，显式事务结束后再 compile_scope 或 source 网络。
- GSD Rust inline test pathspec 缺口已知；不扩大 pathspec，不伪造 Cargo evidence。机器门若拒绝真实报告则回流明确阻塞。

## 错误与未决事项

- `C:/Users/easyi/.Codex/rules/web-search.md` 不存在，阶段 findings 记录了 `.claude/rules/web-search.md` 备用路径。
- 2026-10-29：Task 01-1 在 RED 代码建立之前暂停。`QueryService::execute` 在 `crates/ontoforge-query/src/lib.rs:244` 内部调用 `ReadAudit::reserve`，后者在 `crates/ontoforge-storage/src/read_audit.rs:224` 生成 request_id；`QueryPage` 仅返回 rows/truncated，未暴露审计 request_id。本 PLAN 的九个路径中没有 query/lib.rs 或 read_audit.rs，不能合法实现 HTTP request_id 与 durable audit 相等的合同。
- 已向 orchestrator 返回 `STATUS: NEEDS_CLARIFICATION`。最小建议：批准 QueryService 的兼容窄入口，使适配层能够取得同一个 ReadAuditGuard 的 request_id；保留 execute wrapper 与现有审计管线。准确返回类型及错误关联方式由主会话核对并锁定，不在 executor 内自决。
- orchestrator 明确指令暂停此能力并以 CHECKPOINT 返回；未写业务代码、未执行 Cargo 测试、无 RED/GREEN commit，不声称任何验收已通过。
- TDD 证据格式风险尚未实际触发：`references/tdd.md` 列出的 adapter 包含 TAP、JUnit XML、swift-testing、Python unittest 与 go test，未列 Cargo/libtest console。需要真实 Cargo target failure 后运行机器门；若不支持，应增加识别真实 libtest 输出和目标断言的 adapter，或采用合法 runner 生成的支持格式。不得手工转换或伪造报告，不修改工具安装目录。Rust inline RED commit pathspec 是另一项独立的已知缺口。

## 交接状态

- 当前节点：01-1 read_first / 接口核查完成，RED 尚未建立。
- 本 executor 仅新增本 `07-01-execution.md`，未修改九个实现路径。
- 未创建 SUMMARY；没有生产提交，不构成 production commit 后缺 SUMMARY 的中间状态。
- 下一步：主会话核对并批准 request_id 窄入口及所有权后，重新 dispatch 01-1。

## 恢复 (2026-10-29)

- 已读取更新 PLAN，主会话批准 `execute_audited -> QueryExecution` 窄接口与额外四个文件所有权，原阻塞解除。当前 base 为 `bdd9ad1`。
- 主会话已完成 bounded brainstorming；不重复审批接口。取消沿用 guard Drop，reserve 失败 ID 为 None，finish 错误保留 guard ID。
- cargo-nextest 0.9.150 独立工具提供真实 JUnit，先读取帮助并核验配置，RED 报告与 Cargo 日志分开保存，原样输入 GSD classifier。
- 当前执行恢复至 01-1：先最小可编译宿主接口和真实 listener 测试；空 Router 是暂未实现的 production skeleton，不伪造成完整功能。
- 首次编译用 `cargo test -p ontoforge-server --lib tracer_ --no-run` 完成，2m53s，无编译/fixture错误。
- 指定 Cargo tracer RED：3 executed / 3 failed，退出101；nextest同目标JUnit退出100。GSD verdict `RED_EVIDENCE_OK`，report_errors=[]，目标断言为实际HTTP404而预期200。另两个目标在404/401和source barrier断言失败，均由尚未接线路由解释。
- Query RED：3 executed / 1 passed / 2 failed，成功与finish错误未返回guard ID；reserve失败None本来已满足，不将此GREEN宣称为RED。对应真实JUnit经机器门验证；保留完整原始报告、绝对路径及起止时间。
- nextest官方配置页 `https://nexte.st/docs/machine-readable/junit/` 实际HTTP200，核验配置键。最初误用 `/docs/reporting/junit/` 返回404，已改正，没有改变测试报告。
- 01-1 GREEN：共享定义锁中读取definition/scope，显式commit后编译scope；Query保留原execute wrapper，execute_audited复制guard ID。HTTP仅投影ObjectRow并返回同ID，未认证/注入在读取前拒绝。
- 指定tracer命令3 passed/0 failed，query audit全文件12 passed/0 failed。取消、reserve、finish、durable重开、source Pending/reload合同均有实际测试覆盖。
- 01-2 RED 的失败分别为有效 `serve` 参数未注册、真实 listener 未启动，均命中目标行为断言。没有编译或 fixture 错误。报告 `task2-nextest-red.json` 包含原样 JUnit；下一步提交 RED 后实现。

## 01-2 合同检查点 (2026-10-29)

- 主会话要求暂停在查询版本 signing key 生命周期决策处。已核对 CONTEXT、RESEARCH、ENG-REVIEW、ServerConfig，当前只有 `auth.dev.key_file`，没有查询版本 token 的可信持久 key 配置。
- `TokenCodec` 需要 key，HTTP 返回的 `ObjectRow` 包含版本 token。进程随机 key 会使重启前 token 失效，不能自行引入这个生产默认。向主会话回流 `STATUS: NEEDS_CLARIFICATION`。
- Task 01-2 的 RED 已提交；无关装配保留在工作区：`Command::Serve`、dispatch、runtime、配置/认证/显式 dev/engine/listener/store/受控关闭，server 增加接收 signing_key 的轻量构造入口。
- 当前 `serve.rs` 的 signing_key 是待决策的空值，构造会明确失败并 drain audit，因此不是可用 GREEN。此缺口只用于当前未提交检查点，不得带入完成提交。
- 主会话原文要求：只完成与该决策无关的装配后终态 CHECKPOINT，保留 Task01-2 RED 和当前变更，不提交不完整 GREEN 或 SUMMARY。遵照此指令，不修改共享状态文件。
- 待主会话锁定可信 key 配置和生命周期后继续 Task 01-2 GREEN，再执行完整 workspace/fmt/clippy/TLS 验收。
- 装配编译检查 `cargo check -p ontoforge --locked` 退出 0；证据 `07-01-evidence/task2-assembly-check.json`。这仅证明可编译，不证明 Task 01-2 行为已 GREEN。所有 Cargo session 已结束。

## 01-2 持久 key 合同恢复 (2026-10-29)

- 主会话已锁定方案 B 并更新 PLAN，当前恢复 base `74db66c`。新增精确所有权 engine/config.rs、engine/tests/server_config.rs、示例配置和 docs/api.md，不再重复询问此决策。
- `[query] signing_key_file` 可选以兼容旧命令；serve 必须存在可信引用并在监听前读取至少 32 bytes 的原始 key，沿既有 64 KiB secret 文件上限。独立 auth key、不自动创建、不改变 token wire。
- 新增真实重启与 key 独立行为测试。Cargo RED 2 executed / 1 passed / 1 failed，nextest 同样；目标失败为现有 parser 拒绝 query 配置导致真实宿主不启动。不是编译/fixture 错误，原生 JUnit 判定 `RED_EVIDENCE_OK`。
- 负向 key 测试在 RED 已通过，但当前仅说明现有宿主 fail closed，不将它宣称为缺失 key 读取实现的单独 RED；GREEN 后连同正向启动一起验证。
- engine query config 的解析回归正在执行，随后生成 reporter 证据并提交追加 RED，再补最小 key loader 和 CLI 接线。
- engine 配置 RED 1 executed / 1 failed，`RED_EVIDENCE_OK`；追加 test commit `8aa4d46`。
- GREEN 使用 `QueryConfig::load_signing_key`，沿既有 `read_limited` 的 64 KiB 上限读取原始 bytes，至少 32 bytes；错误不含内容或路径。CLI 在 auth/engine/listener 前读取，不隐式创建/旋转，不复用 dev key。doc 与示例只含 key reference。
- 指定 serve_ 4 passed / 0 failed；server_config 全 35 passed；最终 tracer 3 passed；fmt check exit 0。真实同 key 重启后 `VersionTokenCodec::verify` 通过，错误 key 拒绝，替换 auth key 不改变 query token。query audit 最终回归正在编译。
- query audit 最终 12 passed / 0 failed，Task 01-2 目标验收已满足。GREEN 原子提交后执行整体 gates。证据脚本补建零输出命令的空日志，使 fmt 的 logPath 指向真实文件；重复 fmt check 仍 exit 0。
