# 独立方案咨询：Phase 10 Audit Timing Test Split

你是资深 Rust 测试架构师。请独立提出方案，不要受任何既有倾向影响。只输出方案与关键风险，不要改代码。

## 需求（验收标准，逐字）

Phase 10「Audit Timing Test Split」（REL-04），Success Criteria：

1. Given 16 MiB 行触发的最终字节上限断言, When 该断言在并行测试中运行, Then 它不再依赖墙钟, 单独运行与并行运行均稳定通过
2. Given read audit 的 1 秒 pending 期限集成验证, When 它运行, Then 它是与纯计算断言分离的独立测试, 期限被超时时给出明确的失败原因
3. Given CI 的 workspace 测试任务, When 移除 `--test-threads=1` 规避项后连续多次运行, Then 任务稳定为绿
4. Given 一台较慢的机器, When 工作区测试以默认并行度运行, Then 不再出现由该断言拆分问题导致的 `AuditUnavailable` 波动

另有一条关联缺陷登记（WINDOWS#18）：`crates/ontoforge/tests/cli_check_cardinality.rs` 的两个 300ms deadline E2E 用例在全工作区并行负载下偶发超时失败（隔离重跑全绿，判定为时序 flake）。

## 现状（已验证事实，文件:行号）

### A. `crates/ontoforge-query/tests/security.rs:1766-1870` `cap_counts_final_masked_row_and_version`

单测试混三段：
1. 16MiB 行 → `execute_list` → 断言 rows 空/truncated/has_more/cursor 空（纯计算）
2. `tokio::time::timeout(3s)` 轮询 `f.store.read_audit().health().pending != 0`，注释「the first audit row reaches the sink before the second large read」；测试对 audit 行内容零断言
3. 改 scope 掩码列 → 第二个 `execute_list` → 断言 masked rows/version（纯计算）

fixture() 用 `EditStore::open(StoreSettings{backend: Sqlite{path}, ...})`，默认 `ReadAuditSettings`。

### B. read_audit 机制（`crates/ontoforge-storage/src/read_audit.rs`）

- `ReadAuditSettings{capacity:1024, batch_size:128, flush_interval:250ms, max_pending_age:1000ms}`；validate 约束 flush_interval∈[1ms,1000ms]、max_pending_age∈[flush_interval,1000ms]
- `reserve()` → `health().available` 为假时返回 `Err(AuditUnavailable)`；available = accepting && worker 活着 && `oldest_pending ≤ 1s`；permit 经 capacity semaphore
- `enqueue` 先把 id 插入 `pending: Mutex<BTreeMap<u64,Instant>>` 再 try_send 到 channel
- worker 每 `flush_interval` tick 或 `batch.len()==batch_size` 时 `flush`，**flush 一次性写完整个 batch vec**（batch_size 是触发器不是限速器）；成功则逐条 `state.forget(id)` 清 pending
- flush 失败时 `available=false`，每 50ms 重试；持续失败 ≥max_pending_age → drop batch、eprintln、worker 返回 Err（永久不可用）
- `EditStore::open_with_read_audit(settings, audit_settings)` 是 pub 的注入缝

### C. `crates/ontoforge/tests/cli_check_cardinality.rs`（E2E，真子进程 `Command::new(CARGO_BIN_EXE_ontoforge)` + wiremock）

- `cardinality_total_deadline_is_shared`（:411）：`query_timeout_ms=500`，快查 80ms 延迟 + 慢查 `set_delay(2s)`；断言第三目标不被查询（`cardinality_requests==2`）+ DELETE cancel 送达
- `cardinality_budget_starts_after_precheck_and_pk`（:617）：`query_timeout_ms=300`，4 个 mock 各 `set_delay(180ms)`；断言 exit 0 + ONT-W18（无 V19）+ 恰好 4 请求。语义：预算从 cardinality 轮开始计，precheck+PK 共 540ms 已超 300ms 预算，但单 scan 180ms 在预算内
- deadline 编排在 `crates/ontoforge/src/online.rs:309-336`：`Instant::now()` + `ctx.timeout.saturating_sub(started.elapsed())` 逐目标递减；engine 侧 `trino/mod.rs:285` `tokio::time::timeout(ctx.timeout, ...)`
- 同文件已有确定性覆盖：`online.rs:858/900` 用 `#[tokio::test(start_paused=true)]` + BudgetEngine 断言 remaining 递减、第三 scan 被阻止、过期预算→exit 3
- 子进程无法注入时钟（除非给 prod 代码加测试钩，倾向不这么做）

### D. CI

`.github/workflows/ci.yml:59` `cargo test --workspace --locked -- --test-threads=1`（commit ea695ed 随 06-08 PG 审计接入引入）。另有 4 处 `--test-threads=1` 用于 `--ignored` 的 PG/Trino 集成 job（共享外部服务，可保留）。

## 约束

- Windows 本机 `cargo build/test` 必须通过，不依赖 Docker；Postgres 测试需 `ONTOFORGE_PG_URL`，默认只跑 SQLite
- 倾向只改测试与注入既有缝，不改 prod 语义；若需 prod 改动须论证
- 证据纪律：任何「稳定」断言要有实跑输出；移除 --test-threads=1 需本机全工作区多轮跑绿
- workspace 约 1500 个测试

## 请独立回答

1. security.rs 混用测试怎么拆？纯计算部分如何做到确定性（不依赖墙钟）？我注意到一个候选方向：第二个 `execute_list` 换全新 fixture（新 store 的 pending 必空，reserve 不可能因 age 失败）。你是否有更优方案？
2. SC2 的「1 秒 pending 期限」独立集成测试怎么写才既真实触发期限又不靠运气？注意 flush 一次性写全 batch，pending 只有在 sink 写失败时才可能驻留超过 1s——你能想到哪些确定性让 sink 失败或让 pending 老化的手段？
3. 两个 cardinality E2E 用例：放宽到秒级预算 vs 移入单独 ignored 车道 vs 其他方案？各自的取舍？
4. 摘掉 `--test-threads=1` 的风险面：除了这两个已知 flake，还需要排查哪些类别（固定端口、环境变量、共享临时目录、单例）？建议的验证顺序？
5. 你看到的其它坑。
