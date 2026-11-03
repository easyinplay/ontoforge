# T08 / N3 MCP 实际业务并发修复

日期2026-11-03，基线8913f92及当前typed测试；状态真实RED确认、设计接受待实施。

## 事实与第二意见

target/phase7-explain-server-tests-r2.log：真实ServerHost max_concurrent_requests=1，首typed EXPLAIN已进入上游且被barrier阻塞，第二HTTP200!=429。context::admit在next.run得到SSE response头后强制take AdmissionPermit，业务handler仍在rmcp独立worker执行。call_tool只有Action分支take，且与middleware亦有抢夺窗口。query层没有数量permit，不能在测试fake Semaphore掩盖问题。

Claude Code REFINE支持将permit改为业务执行生命周期（target/phase7-mcp-permit-claude-review.json）。独立源码核查rmcp3.8.0 tower2066/service1574-1596，Parts仅每请求context，不随session保存；initialize只存不含extensions的params，modern/list同样无长存。故Arc生命期可安全转移。随后CC AGREE有界解析reservation细化（target/phase7-mcp-buffer-reservation-claude-review.json），用户授权同方向自动推进。

## 接受方案

- REST/MCP继续共用ServerLimits.max_concurrent_requests的同一business Semaphore，超限保持HTTP429/CONCURRENCY_LIMIT、零上游和业务audit。
- AdmissionPermit仍Arc<Mutex<Option<OwnedSemaphorePermit>>>。middleware只drop自己Arc，不再take。call_tool首次await前唯一take为局部RAII，缺失failclosed；L2/core guard覆盖实际执行和收尾；Action将同一guard move给host-owned task，取消/断连不释放仍执行Action的槽。
- 精确MCP endpoint（与真实router匹配的/mcp及合法尾斜杠形式）由outer admission传入共享AdmissionContext及ServerLimits；outer保留既有read tracking/同deadline/shutdown。认证和owner校验依旧先于控制内容分类及任何业务。
- 为避免auth/body缓冲无界，MCP POST在outer先try business作短暂reservation；满则try固定1独立control semaphore，均满429。这仅约束资源，不授予执行。持reservation才进入auth/owner及bounded to_bytes(max_body)，整个过程受outer deadline。
- POST分类：单JSON对象、jsonrpc=2.0、exact notifications/cancelled、完全无id成员（含null拒绝）、params对象且有效Some RequestId；交叉用原bytes反序列化rmcp ClientJsonRpcMessage/CancelledNotificationParam以防重复字段及typed含义不一致。不信任header自称control，原始字节重建给rmcp。其余notification保持普通业务准入，不扩豁免。
- 合法cancel必须持独立control槽；若原持business，先try control再释放business，失败429且RAII全回收。普通RPC仅business reservation可进入handler，control fallback里的普通消息原429，不能发上游。
- 精确MCP DELETE仅带session走独立control，仍auth/owner。GET SSE仅原auth/owner/deadline，不持business/长期control；不新增SSE产品配置，已有session cap保持。其他路径不因header/path前缀误绕过。
- 解析失败/超body/timeout/初始化失败/未分派handler最后Arc释放；不新增无界queue或依赖。限额从实际ServerLimits传递，必要将admit state的Duration替换ServerLimits并同步已知测试callsite，不随意读取默认limit绕过用户设置。

## 验收与ownership

### 实施期生命周期补强

Staff与真实REST→MCP测试共同发现：若REST分支只把Arc放extensions，Axum Body extractor会在handler执行前drop Parts。Claude Code AGREE恢复outer REST clone直到next.run/timeout结束，只drop不take，见target/phase7-rest-permit-lifetime-claude-review.json。MCP分支由rmcp Parts保活，不额外保留初始business clone，避免合法cancel转换control后旧business reservation仍被outer保留。属于落实原RAII合同，无额外配置/接口策略。

已有N3真实RED保留。GREEN必须四typedformat cap1：首请求实际阻塞、第二429且全上游/业务audit无增量、释放后第三成功。补REST→MCP和MCP→REST竞争、满槽cancel真正取消目标且不触发额外业务、非法/伪造control不绕过、GET/SSE与DELETE不会堵control、Action在断连/显式cancel后仍持槽直至task完成。失败解析/初始化/超限body回收，原token/owner/T24/sse/shutdown回归。

t24_session_fix拥有context.rs、mcp.rs、lib.rs及control存放所需state.rs，mcp_transport.rs及unit/mcp_sessions.test.rs最小callsite变更、独立07-MCP-EXECUTION-ADMISSION-IMPLEMENTATION.md。不得改T24 manager；新独立test文件可创建。typed现有mcp_l2_defaults.rs由explain lane已冻结，worker可运行但如需修改须报告。主会话维护共享计划与提交。Cargo唯一owner/jobs2，先完成批准设计再实施；真实RED已存在，不以编译失败当RED。所有重大gray area回主会话CC，不自改合同。
