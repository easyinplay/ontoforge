# Phase 3 缺口规划发现

- `03-VERIFICATION.md`: `status: human_needed`、`gaps_remaining: []`。03-21..03-23 已关闭前一轮三个 gap。
- `03-UAT.md`: `status: partial`；CI 项通过，Trino 缓存 flush 隔离验证因 `PERMISSION_DENIED` 阻塞。
- GSD `planner-gap-closure.md` 仅将验证报告的 `gaps:` 与 `status: diagnosed` 的 UAT 缺口作为新 `gap_closure` PLAN 的输入；当前均无可规划条目。
- `gsd-planner` 独立复核返回 `PLANNING INCONCLUSIVE`；不生成 `03-24-PLAN.md`。
- 本次仅同步规划状态，未修改业务代码；TDD 不适用。
- V1 独立补证：`reload_trino_it.rs` 已在加列前查询表；CI catalog 默认无缓存，现有绿测不能证明 flush 生效。独立 Docker daemon 已启动。
- `flush_metadata_cache` 清空整个 catalog；负向与正向测试必须串行。V1 涉及高可靠性验证，按 `superpowers:test-driven-development` 先见证无缓存时的红测，再开启缓存取得绿测。
- V1 红绿验证完成：原 CI catalog 无缓存时 `cached_column_requires_flush_before_definition_switch` 按预期失败（NoFlush 已使 v2 active）；启用 `metadata.cache-ttl=10m` 与 `metadata.cache-missing=true` 后通过，完整 `reload_trino_it` 串行 3/3 通过；PR CI run 39712341919 三 job 通过。
- V2 诊断起点：Windows CI runs 39582728729、39710470956 首跑均在 `reload.rs:1119` 的 v2 `reload` 返回 SQLite code `5` / `database is locked`，重跑通过。测试用两个 `EditStore` pool 指向同一临时 SQLite 文件；写入任务紧循环提交，`reload` 使用 `BEGIN IMMEDIATE`，连接设置 WAL 和 5 秒 `busy_timeout`。是否达到等待上限仍需执行单元复核，不能归因于终端异常关闭。
- V2 涉及并发回归，执行 `superpowers:test-driven-development`：现有 CI 首跑失败作为 RED 证据；修复后重复运行与当前 HEAD CI 作为 GREEN 证据。修复应保留原测试对旧写拒绝和 edit log 版本顺序的断言。
- V2 执行证据：旧代码本地连续 20 次通过，确认本机不易复现；既有 Windows CI RED 日志明确落在 v2 `reload` 的 code `5`。测试局部增加 30 秒截止时间与 10ms 间隔，仅重试 `ReloadError::Store(StoreError::Query { sqlstate: Some("5") })`，其他错误立即失败；修改后目标测试 20/20、`reload` 测试集 32/32、fmt、Clippy、diff 检查通过。
- V2 PR CI 证据：commit `4eebd0a` 的 run 39715149664 attempt 1 为 `success`；Windows、Ubuntu、integration 三 job 均通过。Windows 日志明确记录 `reload_racing_commits_never_lets_an_old_write_through ... ok`。单次 CI 通过不能证明偶发锁竞争已绝迹，有界重试仍以 30 秒截止，超时或非 code `5` 错误会使测试失败。
