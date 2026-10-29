# 07-04 生命周期预查与待决项

日期：2026-10-29。状态：proposed，尚未实施，07-02 修复不依赖本项。

## 已查事实

- `crates/ontoforge-server/src/context.rs:87` 的 outer timeout drop 整个 handler，`:116` 的 ScopedEngine 收紧到 HTTP deadline。独立 REST 查询已明确允许 drop 并记录 cancelled。
- `07-ENG-REVIEW.md:74` 与 `07-06-PLAN.md` 要求已接受 Action 和依赖读取拥有生命周期，先排空 Action，再取消独立读和关闭审计。
- Action source prepare 调 `read_pk_source(..., None)`，不产生独立 Query read audit。Action 失败沿领域错误，不能用 Query trace fault 覆盖。
- Action `command_id` 类型虽为 Option，正常提交由 `validate_request` 强制非空；validate-only 允许省略。保留既有 receipt、异参拒绝、TTL 和零写入合同。
- 现有 Action test gate 不是 snapshot-aware；真实 HTTP 测试须使用可控 source Engine 或经核准的最小 test-support，不得用旧 wrapper 冒充固定 snapshot。

## 推荐方案 A

认证、body 解析、固定请求 snapshot 和基本请求检查完成后，正式提交转为宿主持有的受跟踪任务，任务接管当前请求的 owned semaphore permit。此处“接受”只表示开始领域执行，不承诺写入一定成功。领域授权、fence、CAS、guard 和事务继续决定结果。

HTTP deadline 或断连仅结束等待响应，不取消任务。任务按既有领域 source bounds 完成并释放许可，防止 HTTP 超时后并发上限失效；不能机械套用独立 Query 的 HTTP deadline adapter。领域错误和成功 receipt 保留原语义，不自动重试。超时响应仍为 504，结果未知时使用同 command_id 显式 replay 或 GET receipt。

validate-only 保持零写入，可在请求 deadline 内执行并取消，无需在响应消失后继续。正式 Action 已进入任务后，即使最终是参数/授权/冲突拒绝，也只返回原领域结果，不扩展审计 schema。

07-04 最小接入任务所有权及正常 serve 退出时的 Action 等待；07-06 在此基础上完成完整停止准入、有界排空、独立读/SSE、monitor/DB 关闭与失败退出。不能在 07-04 完成后仍有脱离宿主、审计已关闭的后台写任务。

## 可选方案 B

正式 Action 同样由宿主持有并保持并发许可，但 HTTP 不施加响应 deadline，保持等待领域结果；断连后仍完成任务。优点是正常连接不会收到“504 但随后可能已提交”的结果；代价是该端点不再遵循统一 HTTP deadline，等待时间由领域 bounds 控制。独立读取与 validate-only 仍使用 HTTP deadline。

## 第二意见及取舍

Grok session 25404 返回实质正文，支持宿主持有任务、owned permit 与丢响应后恢复；55 秒 wrapper 随后 exit 1，记为部分第二意见。其“直接复用 Query Engine adapter”建议与代码中 HTTP deadline 收紧冲突，不采纳；duplicate command_id 的行为已由既有领域测试锁定，不新增产品选择。以上以本地源码和已批准合同为依据。

## 验收补充草案

真实 HTTP 下阻塞 Action source，触发响应超时和 TCP 断开，释放后重新打开 store 验证恰好一份 receipt/edit/log/outbox；相同 command_id 显式 replay 返回原 receipt/time。受跟踪任务完成前保持并发许可，结束后释放；正常 shutdown 等待它，不能先 shutdown read audit。validate-only 超时及缺失 command_id 不产生业务写入。不得以独立 Query cancelled/0 代替 Action 持久化证据。
