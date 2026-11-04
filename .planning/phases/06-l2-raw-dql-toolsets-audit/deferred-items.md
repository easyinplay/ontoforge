# Deferred Items

- `security::cap_counts_final_masked_row_and_version` 使用 16 MiB 行触发固定最终字节上限，与 read audit 的 1 秒 pending 期限共用墙钟。Windows 并行 workspace 曾出现 `AuditUnavailable`，单独及单线程全量通过。后续应将最终行字节上限的纯计算断言与审计时间敏感集成验证拆开；当前 CI 使用 `--test-threads=1`，不能声称慢机器上完全消除波动。
