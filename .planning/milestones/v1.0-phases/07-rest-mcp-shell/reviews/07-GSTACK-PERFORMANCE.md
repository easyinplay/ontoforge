---
specialist: performance
base: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
head: 972c7be9cc3e91460d300b6f45fd814df023a539
mode: read-only-static-review
result: 1 INFORMATIONAL
independence: 审查者参与过 MCP 与 dev 实现，存在上下文偏差；本报告不替代 fresh reviewer 交叉核对。
---

采用 `C:/Users/easyi/.agents/skills/gstack/review/specialists/performance.md`。证据固定为 `git diff fd24beb..972c7be`，通过对应提交的源码核对调用链。未运行 Cargo、benchmark 或数据库探针，以下成本来自静态控制流与可构造数据规模，不宣称已测得延迟或峰值内存。

## Finding

{"severity":"INFORMATIONAL","confidence":97,"path":"crates/ontoforge-server/src/history_page.rs","line":20,"category":"performance","summary":"history 的下一页存在性检查会额外物化整个旧 commit 组；小页请求的额外 I/O 与解码成本取决于未返回的旧组 payload 大小。","fix":"作为后续非阻塞优化，可增加复用相同 object/PK/beforeSeq 与授权前置条件的有界存在性查询，只读取一个索引键或 SELECT 1，不通过完整 history_with_snapshot 获取 payload/header；保持完整组分页与规范化页字节预算合同。","fingerprint":"crates/ontoforge-server/src/history_page.rs:20:performance","specialist":"performance","evidence":"新增 helper 将 limit 设为 1 后调用 history_with_snapshot，仅使用 is_empty。该方法在 storage/history.rs:147 执行 locate，:156..165 补齐最后 commit 组，:173 与 :183 读取 payload/header，:207 构造全部 HistoryEntry。因此有旧组时新增四次串行 history SQL 及 O(B_older) payload 解析，而非一次键存在性查询。可构造最新一条小修改、前一 commit 对同对象含 256 条各约 4 KiB 的编辑，limit=1 当前页只返回小记录，但 lookahead 额外读取并解码约 1 MiB payload 后丢弃。server REST/MCP 使用 max_edits=256，action/lib.rs:459..463 允许 items×rules<=256，故该规模不要求绕过普通 Action 编辑数上限。max_bytes 检查在 lookahead 之后，约束的是返回页，不能限制这部分额外读取。本项是已批准 lookahead 设计的性能成本提醒，不是正确性 blocker。","test_stub":"创建一个 256-entry 旧 commit 组和一条较新的小修改；请求 limit=1，在数据库层记录 statement 数和读取 payload 字节；比较同样当前页但旧组 payload 大小不同的请求。优化后保留完全相同的 entries/truncated/nextBeforeSeq，同时存在性检查不读取旧组 payload。"}

## 其余检查

- history 字节计算逐 entry 序列化并累计，只在 commit 组边界序列化空 envelope；未发现对已扫描前缀重复序列化造成的 O(n²)。完整组扩张属于原 storage 合同，不作为本轮新增缺陷。
- 未新增逐 entry 的数据库调用循环。上述 lookahead 的查询次数为常数增加，问题在未返回组的物化成本，不标为 N+1。
- TypeScript 错误路径在 fetch 层一次读取 body 并执行 lossless 解码；诊断校验按数组长度线性遍历。没有新增生产依赖或 bundle 扩张，也不把原有整包响应解析列为新回归。
- MCP catalog 原本已按请求构建；新增参数校验与 observer 未引入每元素数据库操作。没有规模证据支持将局部 clone/映射成本单列为 finding。
- dev 文件操作属于既有启动准备路径；新增 OS try_lock 非阻塞，并移除了 PID 检测子进程。本增量未引入运行中请求循环内的同步磁盘工作。

RF-01、RF-05、07-14 明确未实施，不计入发现。报告仅含上述一项非阻塞性能观察，不要求当前增量扩展 storage 接口。
