# T24 真实 session 生命周期修复

日期：2026-11-03。基线：b068ff5。状态：设计接受，实施与动态验收 pending。

## 决策与第二意见

采用宿主所有的 bounded SessionManager 薄封装，内部继续使用锁定 rmcp 3.8.0 LocalSessionManager。StreamableHttpService 必须拿到封装本身，确保其 worker 自然退出的 close_session 回调也进入统一回收路径。owner 状态与真实 session 同属一个宿主，移除进程全局 owner 生命周期。保留每 HTTP 与每 tool 重新认证、权限 snapshot、旧协议 owner 校验和新无 session 协议。

Claude Code 返回 REFINE，同方向补强（原文：target/phase7-t24-claude-review.json；实际模型 MiniMax-M3.1-Flash[1m]）。按用户授权自动推进。拒绝仅 middleware semaphore 方案，因为不能可靠覆盖自然退出；不升级/fork rmcp。

## 不变量

- create/evict 原子串行化，真实资源连同初始化中的 session 总数始终不超过 256。创建前回收过期/最老项，沿用现有按最后合法访问时间的 oldest 语义；并发初始化也占用容量。
- 初始化登记 pending entry；响应后 owner bind 只能更新仍存在的同一 session，绝不能重新插入被驱逐 entry。rmcp session ID 唯一；若不重用 ID且不存在重插路径，不额外建立无限 tombstone 集合。跨主体/未知/过期请求不得刷新 TTL。
- close 路径统一删除真实 session 和 owner。不得持有 std Mutex 跨 await，不串行化慢 initialize/业务请求；必须分析自然退出回调重入和close等待的死锁风险。
- 后台周期回收无人访问的过期 session；任务由宿主取消并 join，不能 detached 泄漏。默认 owner TTL 30分钟，rmcp本身300秒idle优先自然回收也是合法结果。
- initialize/bind失败及时回收。DELETE、自然退出、容量驱逐、TTL与shutdown幂等且不复活。
- shutdown复用既定10秒总预算，已接受Action仍由现有宿主owned task机制排空/记录超时，禁止新增transport取消Action行为。测试短TTL/小cap只作为测试注入，不加产品配置。

## 验收

真实 HTTP连续/并发 initialize检查真实manager峰值及最终容量；被驱逐session真实移除且不可用；无人请求TTL释放；DELETE释放；两个host隔离；错误owner不延长寿命；初始化失败无残留；shutdown清零、回收任务完成。保留现有Action/cancel/SSE回归。先可归因行为RED，再最小GREEN和Clippy/fmt。不能用编译失败冒充RED。

## E1–E7第二意见取舍

接受真实barrier、实际发送次数、持久表增量、提交后丢响应及显式replay的补强。测试证据补齐不必人为改坏生产代码制造RED；若发现生产缺陷则另做真实RED/GREEN。

不采纳两项范围扩展：撤权只影响后续授权snapshot，不自动abort已接收的在途请求；请求取消隔离单独验证。通用JSON codec并未承诺u64以上BigInt全部拒绝，只在具体typed API边界依既有合同验证。E6统计被测调用的上游请求增量，不能把合法后台monitor流量误报为绕过。

## Ownership

T24 worker拥有server session及其装配/生命周期与mcp_transport测试。SDK worker拥有Rust/TS client测试及独立fixture。server evidence worker拥有HTTP/MCP Action/contract/L2测试及专用support。共享STATE/task_plan/progress/findings由主会话单写，Cargo仅T24 worker当前持有，jobs2。Warp/ralph-loop未提供，不声称已调用。
