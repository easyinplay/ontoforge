# E3 bridge 动态证据方案

2026-11-03，主会话设计，Claude Code REFINE同方向接受，按用户授权自动推进。原文target/phase7-bridge-claude-review.json，实际模型MiniMax-M3.1-Flash[1m]。只补既有07-09合同测试，不改生产默认协议。

## 实码更正（优先于下方初稿）

真实首轮测试与rmcp3.8.0 service/client.rs:755,767确认production `serve_client`显式选择Initialize，默认上游会话不是modern discover；现代方法名为server/discover。首轮3失败分别为错误discover计数及receipt包装提取，并非重试实现bug。Claude Code对更正方案AGREE，见target/phase7-bridge-actual-protocol-claude-review.json。最终测试直接透传当前真实Initialize/session，不再使用虚设legacy gateway改写；断言initialize1/真实session/真实DELETE→host404/no reinit。故障与显式回放均在实际session协议验证；2026-07-28无session由server transport已测。MCP receipt从structuredContent.receipt取完整对象，不改生产wire。

Nyquist补齐原07-09-2：新增真实stdio成功链（双有效主体scope/list/describe/query、authorized validateOnly零持久写、Action/history/result与REST一致）和在线badtoken/真实302→loopback trap零连接或请求的否定场景，stderr/stdout凭据不泄露。Claude Code分别AGREE/REFINE同向支持，见target/phase7-stdio-success-claude-review.json与phase7-stdio-deny-claude-review.json。复用当前fixture，不改生产。

## 初稿历史（默认modern假设已被上节更正）

真实CLI子进程连接受控fault proxy，再转发真实ServerHost。默认modern分支透传discover，断言无session；统计protocol请求与业务POST分开。

legacy expiry分支显式模拟legacy-only gateway：只对discover返回correlated method-not-found，让rmcp走现有fallback；initialize限定旧版本并转发真实host，真实session ID及响应透传。后续notification/tool/DELETE按真实session转发。建立成功后由host manager关闭实际session，下一tool的404必须来自host。精确断言原bridge只有1次initialize、1次失败tool，无自动重初始化/重放。文档不得把测试gateway说成默认生产协议。

no-retry分支使用默认modern协议：proxy完整接收业务POST但不转发后断连，1POST/0commit；另一case转发真实Action并收到已提交receipt后丢弃响应，1POST/1commit。CLI应返回error或transport终止。用barrier确认注入位置，并在有界settle窗口后检查没有第二次业务POST。显式调用者同command_id在新连接回放原receipt/time，完整持久表无新增；新连接自身协议请求单独计数。

复用现有tokio/reqwest/serde和test fixture。每个等待有timeout，代理后台失败可观察，子进程/连接/任务清理有界。server合法token到期POST/GET/DELETE由T24 mcp_transport lane同时补齐。实现ownership限CLI tests/cli_stdio.rs及其专用support，禁止覆盖server或SDK lane文件。
