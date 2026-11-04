# Phase 7 Prohibition Recall And Precision

日期：2026-10-29。SPEC缺失，按spec-less两阶段协议执行。逐REQ先问“功能可能静默变成作者不接受的什么”，再筛routine与canon，保留bespoke产品/安全意图。不虚构check descriptor。

## Recall → Precision

| Requirement | Recall Candidates | Precision Decision | Kept Projection |
|---|---|---|---|
| API-01 | 全部端点可能悄然丢能力；越权/空集/错误未处理 | 能力完整性已入GOAL/REQ和edge；安全canon转security | 无bespoke保留项 |
| API-02 | schema与serde漂移；错误物理名外泄 | 前者routine入schema测试；后者canon转security | 无bespoke保留项 |
| API-03 | 错误吞掉request_id；SQL/secret外泄 | 前者routine入error tests；后者canon转security | 无bespoke保留项 |
| API-04 | client隐式重试写；跨实例token串用 | 错误/幂等入edge；身份隔离canon转security | 无bespoke保留项 |
| API-05 | 静默舍入/为JS改wire；只交types；token泄露 | 保留用户数值合同；完整client入GOAL；secret为canon | P-03 → 07-11 |
| MCP-01 | stdio另启数据宿主；session绕认证；EOF悬挂 | 保留产品单宿主意图；认证canon；EOF为edge | P-01 → 07-09 |
| MCP-02 | describe泄露越权metadata；空列表崩溃 | 前者canon转security；后者edge | 无bespoke保留项 |
| MCP-03 | 查询放宽scope；分页重复 | 前者canon转security；后者edge | 无bespoke保留项 |
| MCP-04 | schema接收任意字段；reload混代 | routine/edge，已有精确schema与snapshot验收 | 无bespoke保留项 |
| MCP-05 | dryrun悄然写入；fence失效 | 业务正确性与stateful edge，入Action tests | 无bespoke保留项 |
| MCP-06 | 第二套权限算法；重复edit log | 合同完整性与edge；权限canon转security | 无bespoke保留项 |
| MCP-07 | 只过滤list而不限制call；参数少格式 | 前者canon；后者明确需求能力，入08/15 | 无bespoke保留项 |
| MCP-08 | agent获得管理能力 | 已有显式需求与security gate，不重复mint | 无bespoke保留项 |
| DX-01 | 重启静默丢数据；reset删任意目录 | 保留D-02持久意图；路径canon转security | P-02 → 07-12 |
| DX-02 | 示例依赖checkout；重启清空demo修改 | 前者交付正确性入artifact；后者共用D-02 | P-02去重 → 07-12 |
| DX-04 | 未经授权发布/合并；archive缺binary | 保留授权边界；artifact正确性入脚本验收 | P-05 → 07-14 |
| L2-04 | core200掩盖L2失效；宽松身份仍放行 | 保留透明健康意图；授权canon转security | P-04 → 07-06,13 |
| SCOPE-07 | 混代授权；MCP管理写 | 现有显式需求/edge与security覆盖 | 无bespoke保留项 |

## Kept Items

- P-01：stdio不得启动独立数据宿主或打开第二套edit store，只能认证桥接已有serve（D-01）。 lifecycle=resolved，verification=test，投影到07-09。
- P-02：dev正常重启不得清空用户修改、history或receipt；重建只能来自显式安全reset（D-02）。 lifecycle=resolved，verification=test，投影到07-12。
- P-03：TypeScript client不得静默舍入整数，也不得为方便client改变既有整数JSON token与decimal字符串wire合同。 lifecycle=resolved，verification=test，投影到07-11。
- P-04：core健康时不得把已配置L2的故障或证明过期伪装成正常健康；必须暴露脱敏degraded原因并阻断L2（D-03）。 lifecycle=resolved，verification=test，投影到07-06,07-13。
- P-05：本轮执行不得合并PR #7、创建Release或发布npm；PR #7必须保持Draft OPEN。 lifecycle=resolved，verification=judgment，投影到07-14。

已实际调用 probe-core.cjs::projectProhibitions，并将返回的statement/status/verification逐字投影到PLAN must_haves.prohibitions。保留5条，其中test=4、judgment=1；全部descriptor-less，无check_*，故执行前及verify时均flagged-unverified，不能因resolved或未来task通过而静默判绿。test表示可测试意图，尚无工具可识别的wired enforcement证据；judgment需主会话核对真实PR/Release/npm状态。普通测试日志只证明其具体验收，不伪造descriptor定位或fail-first机器证明。

Canon breadcrumb：OWASP授权、secret泄露、注入、path traversal与依赖tampering由各PLAN STRIDE测试及 $gsd-secure-phase ASVS L1 high gate处理，不mint重复prohibition。routine的空集、排序、幂等、取消、生命周期归07-EDGE-COVERAGE与07-VALIDATION。
