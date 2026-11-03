# 07-15 typed EXPLAIN 动态证据补齐

2026-11-03，Nyquist对原07-15-2发现命名为timeout_cancel_concurrency的测试只执行四format成功路径与audit总4，没有三类故障刺激。旧wrapper取消/execute timeout不能直接替代typed四format合同。主会话亲读l2_execute.rs:656确认，Claude Code AGREE（target/phase7-explain-evidence-claude-review.json）。

仅修改query/tests/l2_execute.rs及必要专用helper，复用真实L2Service/TrinoEngine与受控HTTP upstream：四ExplainType参数化timeout、确定请求进入后cancel，以及真实可信并发permit耗尽行为。先读实际实现，按既有拒绝/等待合同断言，不新增语义。每请求audit增量恰好1、状态/取消释放正确，独立请求不被误取消；barrier/Notify及有界超时，无sleep伪并发。成功补证据可直接GREEN；生产bug须真实RED后回主会话CC决策，不自行改生产。

Cargo当前CLI独占，本lane先准备test，不启动Cargo；后由主会话串行调度。文档独立ledger，不能修改共享planning。已提供的Warp/ralph-loop不可用，不声称调用。

## 并发层级更正

实际L2Service/TrinoEngine没有数量permit，max_concurrent_requests归ServerLimits/context::admit，满则429。原PLAN明确写concurrency上限，故不能用query层并行成功冒充上限。Claude Code AGREE边界修正并要求typed路由证据，见target/phase7-explain-concurrency-claude-review.json。query lane保留四types timeout/cancel/独立并发；同worker扩ownership到server/tests/mcp_l2_defaults.rs的最小fixture释放通知及新typedExplain limiter测试：真实ServerHost上限1、首请求上游barrier占槽、第二HTTP429且上游增量0、释放后第三成功，拒绝不reserve audit，成功各1条。不在query fixture造假Semaphore，不改生产配置或实现。
