# Phase 7 Source And Edge Coverage

日期：2026-10-29。来源：07-PLANNING-PREFLIGHT.md 的确定性46项edge输出。此处 resolved 表示已写入可执行验收合同，所有运行状态仍为 pending；不代表测试已通过。

## Edge Coverage

applicable=46，resolved=46，unresolved=0，unclassified=0，explicit=46，backstop=0。每行验收已在对应PLAN的 must_haves.truths 与任务behavior中承接。

| Requirement | Category | Status | Verification | Plan | Acceptance Criterion |
|---|---|---|---|---|---|
| API-01 | adjacency | resolved | explicit | 07-03 | API-01 adjacency/ordering/empty：相等排序值由现有 pk/keyset 稳定破同，空/单项/nullable 和重复对象按原 Query 语义，has_more 决定结束。 |
| API-01 | empty | resolved | explicit | 07-03 | API-01 adjacency/ordering/empty：相等排序值由现有 pk/keyset 稳定破同，空/单项/nullable 和重复对象按原 Query 语义，has_more 决定结束。 |
| API-01 | ordering | resolved | explicit | 07-03 | API-01 adjacency/ordering/empty：相等排序值由现有 pk/keyset 稳定破同，空/单项/nullable 和重复对象按原 Query 语义，has_more 决定结束。 |
| API-01 | concurrency | resolved | explicit | 07-03 | API-01 concurrency：在途请求固定快照，新请求反映已提交撤权；慢查询不持 metadata 锁。 |
| API-02 | empty | resolved | explicit | 07-05 | API-02 empty：空metadata/数组、单项、required/nullable、任意JSON与未知字段的schema和serde正反例一致。 |
| API-02 | encoding | resolved | explicit | 07-05 | API-02 encoding：tagged union真实字段名、递归ObjectSet/WhereClause、additionalProperties、Unicode描述、decimal字符串与Int number一致。 |
| API-02 | concurrency | resolved | explicit | 07-05 | API-02 concurrency：OpenAPI导出由静态平台DTO生成，多个并发导出字节稳定，不依赖可变租户/定义或开启服务。 |
| API-03 | empty | resolved | explicit | 07-03 | API-03 empty/encoding/concurrency：空错误描述也有稳定 code/apiName/request_id，UTF-8 正确保留且按字节限额，并发请求 ID 隔离。 |
| API-03 | encoding | resolved | explicit | 07-03 | API-03 empty/encoding/concurrency：空错误描述也有稳定 code/apiName/request_id，UTF-8 正确保留且按字节限额，并发请求 ID 隔离。 |
| API-03 | concurrency | resolved | explicit | 07-03 | API-03 empty/encoding/concurrency：空错误描述也有稳定 code/apiName/request_id，UTF-8 正确保留且按字节限额，并发请求 ID 隔离。 |
| API-04 | idempotency | resolved | explicit | 07-10 | API-04 idempotency：同command_id显式重试返回原receipt，client不自动重试写入；异参/过期/丢响应语义明确。 |
| API-04 | concurrency | resolved | explicit | 07-10 | API-04 concurrency：每实例bearer隔离、并发请求request_id/取消独立，跨principal、撤权、超时和结构化错误不被吞掉。 |
| API-05 | boundary | resolved | explicit | 07-11 | API-05 boundary/precision：±(2^53−1)、2^53±1、i64 min/max、u64 seq/version上界、decimal长小数原样往返；wire仍用既有整数JSON token与decimal字符串。 |
| API-05 | precision | resolved | explicit | 07-11 | API-05 boundary/precision：±(2^53−1)、2^53±1、i64 min/max、u64 seq/version上界、decimal长小数原样往返；wire仍用既有整数JSON token与decimal字符串。 |
| API-05 | idempotency | resolved | explicit | 07-11 | API-05 idempotency：相同command_id显式重放原receipt/time，写client不自动重试或换session重发。 |
| API-05 | concurrency | resolved | explicit | 07-11 | API-05 concurrency：每实例bearer、AbortSignal与并发响应隔离；缺lossless运行时能力明确失败，不能静默舍入。 |
| MCP-01 | concurrency | resolved | explicit | 07-08 | MCP-01 concurrency：取消、expiry、跨主体、disconnect、SSE和重连不改变身份或自动重发写入；管理工具不可达。 |
| MCP-02 | adjacency | resolved | explicit | 07-07 | MCP-02/03 adjacency/empty/ordering：先授权再分页，空/单项结果与同apiName/排序值稳定；跨页没有重复/遗漏，has_more决定结束。 |
| MCP-02 | empty | resolved | explicit | 07-07 | MCP-02/03 adjacency/empty/ordering：先授权再分页，空/单项结果与同apiName/排序值稳定；跨页没有重复/遗漏，has_more决定结束。 |
| MCP-02 | ordering | resolved | explicit | 07-07 | MCP-02/03 adjacency/empty/ordering：先授权再分页，空/单项结果与同apiName/排序值稳定；跨页没有重复/遗漏，has_more决定结束。 |
| MCP-02 | concurrency | resolved | explicit | 07-07 | MCP-02/03 concurrency：reload/revoke后新工具请求采用新snapshot，在途请求固定旧definition；权限不按session缓存。 |
| MCP-03 | adjacency | resolved | explicit | 07-07 | MCP-02/03 adjacency/empty/ordering：先授权再分页，空/单项结果与同apiName/排序值稳定；跨页没有重复/遗漏，has_more决定结束。 |
| MCP-03 | empty | resolved | explicit | 07-07 | MCP-02/03 adjacency/empty/ordering：先授权再分页，空/单项结果与同apiName/排序值稳定；跨页没有重复/遗漏，has_more决定结束。 |
| MCP-03 | ordering | resolved | explicit | 07-07 | MCP-02/03 adjacency/empty/ordering：先授权再分页，空/单项结果与同apiName/排序值稳定；跨页没有重复/遗漏，has_more决定结束。 |
| MCP-03 | concurrency | resolved | explicit | 07-07 | MCP-02/03 concurrency：reload/revoke后新工具请求采用新snapshot，在途请求固定旧definition；权限不按session缓存。 |
| MCP-04 | empty | resolved | explicit | 07-07 | MCP-04 empty/encoding/concurrency：无参/required/nullable/Unicode描述的schema与真实validator一致；definition切换不混用旧schema和新执行。 |
| MCP-04 | encoding | resolved | explicit | 07-07 | MCP-04 empty/encoding/concurrency：无参/required/nullable/Unicode描述的schema与真实validator一致；definition切换不混用旧schema和新执行。 |
| MCP-04 | concurrency | resolved | explicit | 07-07 | MCP-04 empty/encoding/concurrency：无参/required/nullable/Unicode描述的schema与真实validator一致；definition切换不混用旧schema和新执行。 |
| MCP-05 | idempotency | resolved | explicit | 07-07 | MCP-05/06 idempotency/concurrency：dryrun重复调用零业务写且走fence/CAS/guard；提交重放同command_id保留receipt，跨transport并发恰好一份提交。 |
| MCP-05 | concurrency | resolved | explicit | 07-07 | MCP-05/06 idempotency/concurrency：dryrun重复调用零业务写且走fence/CAS/guard；提交重放同command_id保留receipt，跨transport并发恰好一份提交。 |
| MCP-06 | idempotency | resolved | explicit | 07-07 | MCP-05/06 idempotency/concurrency：dryrun重复调用零业务写且走fence/CAS/guard；提交重放同command_id保留receipt，跨transport并发恰好一份提交。 |
| MCP-06 | concurrency | resolved | explicit | 07-07 | MCP-05/06 idempotency/concurrency：dryrun重复调用零业务写且走fence/CAS/guard；提交重放同command_id保留receipt，跨transport并发恰好一份提交。 |
| MCP-07 | adjacency | resolved | explicit | 07-08 | MCP-07 adjacency/empty/ordering：授权过滤后的空/单项catalog/schema/table正确，稳定keyset页无重复遗漏，相等边界不跨allow-list。 |
| MCP-07 | empty | resolved | explicit | 07-08 | MCP-07 adjacency/empty/ordering：授权过滤后的空/单项catalog/schema/table正确，稳定keyset页无重复遗漏，相等边界不跨allow-list。 |
| MCP-07 | ordering | resolved | explicit | 07-08 | MCP-07 adjacency/empty/ordering：授权过滤后的空/单项catalog/schema/table正确，稳定keyset页无重复遗漏，相等边界不跨allow-list。 |
| MCP-07 | concurrency | resolved | explicit | 07-08 | MCP-07 concurrency：撤销sql:raw或L2证明失败/过期立即阻断下一请求，不能使用旧session授权。 |
| MCP-08 | concurrency | resolved | explicit | 07-07 | MCP-08 concurrency：无论reload/toolset/session交错，L3和定义/授权变更工具从不列出或接受调用。 |
| DX-01 | idempotency | resolved | explicit | 07-12 | DX-01/02 idempotency：重复启动/seed不覆盖数据；相同command显式重放不重复修改，重复reset行为明确且仅在dev目录执行。 |
| DX-01 | concurrency | resolved | explicit | 07-12 | DX-01/02 concurrency：活跃实例持独占dev锁，第二实例与并发reset拒绝；异常退出后可恢复且不会删任意路径。 |
| DX-02 | idempotency | resolved | explicit | 07-12 | DX-01/02 idempotency：重复启动/seed不覆盖数据；相同command显式重放不重复修改，重复reset行为明确且仅在dev目录执行。 |
| DX-02 | concurrency | resolved | explicit | 07-12 | DX-01/02 concurrency：活跃实例持独占dev锁，第二实例与并发reset拒绝；异常退出后可恢复且不会删任意路径。 |
| DX-04 | concurrency | resolved | explicit | 07-14 | DX-04 concurrency：并发build使用唯一平台artifact名和隔离目录，聚合要求三平台齐全且校验匹配；失败/取消不发布。 |
| L2-04 | idempotency | resolved | explicit | 07-06 | L2-04 idempotency：重复启动/关闭不留monitor lease或活跃probe，失败/过期/恢复转换不产生重复审计，monitor不写storage read_audit。 |
| L2-04 | concurrency | resolved | explicit | 07-06 | L2-04 concurrency：证明过期与发送侧同时拒绝L2；在途probe/recovery/停机交错仍关闭准入且清理失败可见。 |
| SCOPE-07 | idempotency | resolved | explicit | 07-13 | SCOPE-07 idempotency/concurrency：重复管理操作与并发reload/revoke/请求/重放不产生混代或绕过当前授权，MCP无管理写入口。 |
| SCOPE-07 | concurrency | resolved | explicit | 07-13 | SCOPE-07 idempotency/concurrency：重复管理操作与并发reload/revoke/请求/重放不产生混代或绕过当前授权，MCP无管理写入口。 |

## Multi-Source Coverage Audit

| Source | ID | Item | Plans | Status |
|---|---|---|---|---|
| GOAL | SC-1 | 全REST端点组、统一错误、OpenAPI3.1、Rust/TS可调用client | 01–05,10,11,13 | COVERED |
| GOAL | SC-2 | stdio与HTTP、主体toolset、管理工具不可达 | 07–09,13 | COVERED |
| GOAL | SC-3 | 动态Action/validate-only、六L2名称参数与allow-list | 07,08,15,16,13 | COVERED |
| GOAL | SC-4 | REST/MCP同授权与edit log | 02,04,07–09,13 | COVERED |
| GOAL | SC-5 | Windows无外部依赖完整持久demo及三平台发行workflow | 12,14 | COVERED |
| GOAL | SC-6 | L2证明失败/过期/恢复与健康一致 | 06,08,13 | COVERED |
| REQ | API-01 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-01,07-02,07-03,07-04,07-05,07-06,07-13 | COVERED |
| REQ | API-03 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-01,07-03,07-04,07-05,07-06,07-13 | COVERED |
| REQ | SCOPE-07 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-01,07-02,07-04,07-07,07-08,07-09,07-13,07-16 | COVERED |
| REQ | MCP-06 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-02,07-04,07-07,07-08,07-09,07-13 | COVERED |
| REQ | API-02 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-05 | COVERED |
| REQ | L2-04 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-06,07-08,07-13 | COVERED |
| REQ | MCP-01 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-07,07-08,07-09,07-13 | COVERED |
| REQ | MCP-02 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-07 | COVERED |
| REQ | MCP-03 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-07 | COVERED |
| REQ | MCP-04 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-07 | COVERED |
| REQ | MCP-05 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-07 | COVERED |
| REQ | MCP-07 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-07,07-08,07-09,07-13,07-15,07-16 | COVERED |
| REQ | MCP-08 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-07,07-08,07-13 | COVERED |
| REQ | API-04 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-10,07-14 | COVERED |
| REQ | API-05 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-11,07-14 | COVERED |
| REQ | DX-01 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-12,07-14 | COVERED |
| REQ | DX-02 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-12,07-14 | COVERED |
| REQ | DX-04 | 以 REQUIREMENTS.md 的原编号验收为准 | 07-14 | COVERED |
| CONTEXT | D-01 | stdio只认证桥接已有serve，无第二数据宿主 | 09 | COVERED |
| CONTEXT | D-02 | dev持久、空状态seed、显式安全reset、活跃锁 | 12 | COVERED |
| CONTEXT | D-03 | L2故障200 degraded/core503/未配置正常/fail-closed | 06,08,13 | COVERED |
| RESEARCH | R-01 | 已核验rmcp/axum、Tokio能力与包legitimacy | 01,07–09 | COVERED |
| RESEARCH | R-02 | 共享短锁原子definition/roles/bindings，锁外compile/network | 01,02,13 | COVERED |
| RESEARCH | R-03 | Action/history snapshot-aware窄入口、wrapper、fence/CAS/guard、receipt语义 | 02,04,07,13 | COVERED |
| RESEARCH | R-04 | 平台DTO递归/nullable/serde union与OpenAPI3.1一致 | 03–05 | COVERED |
| RESEARCH | R-05 | Rust手写DTO client与TS生成类型+可调用完整wrapper | 10,11 | COVERED |
| RESEARCH | R-06 | TS整数token无损/decimal字符串、native能力不足明确拒绝 | 11 | COVERED |
| RESEARCH | R-07 | HTTP Parts可信身份、逐请求授权、session容量/TTL/owner | 07,08 | COVERED |
| RESEARCH | R-08 | bridge禁redirect与expired-session重发、有界取消/EOF | 09 | COVERED |
| RESEARCH | R-09 | toolset与动态Action schema共用当前snapshot | 07,08 | COVERED |
| RESEARCH | R-10 | 六L2工具完整参数、四typed EXPLAIN、限定表名与可信默认 | 08,15,16,13 | COVERED |
| RESEARCH | R-11 | monitor/drift宿主所有权、健康脱敏与显式清理 | 06,13 | COVERED |
| RESEARCH | R-12 | Action依赖先完成、独立读/SSE取消、audit排空、异常非零 | 06,09,13 | COVERED |
| RESEARCH | R-13 | Windows持久dev、嵌入source/ontology/scope、脱离checkout | 12,14 | COVERED |
| RESEARCH | R-14 | SQLite/PG race与真实Trino失效/恢复；缺环境不得绿色 | 02,13 | COVERED |
| RESEARCH | R-15 | 三native平台实际archive/checksum/安装smoke | 14 | COVERED |
| RESEARCH | R-16 | ASVS L1、high gate、rustls/TLS/依赖与发布权限 | 全部threat_model,13,14 | COVERED |

无 MISSING 项。CONTEXT 的 Deferred Ideas 排除；本阶段没有产品前端或LLM推理，不生成UI-SPEC/AI-SPEC。ADR限定范围见 COVERAGE.md，不因detector命中增加产品能力。

## Planner Validation Evidence

revision1：16份PLAN共32个task，12个topological waves。07-08保留六L2工具与HTTP身份隔离，原08-3全部行为由07-16的两个task承接。07-16依赖08与15，stdio09和真实服务13依赖16；10/11依赖06的真实health生产端点。MVP用户故事源于本轮获授权重述的ROADMAP Phase7 Goal，16份PLAN均引用同一故事，六项SC和18项需求未改。05与12的one-way决策仍由既有ADR及D-02覆盖。

当前wave结构：1=01，2=02，3=03，4=04，5=05，6=06/15，7=07/10/11，8=08，9=16，10=09，11=12/13，12=14。所有生产task、PG/Trino/client/release运行状态仍为pending。结构、路径、coverage和同wave文件冲突由revision1落盘后的实际gate核验，不以拆分本身宣称通过。

revision1实际gate结果：16份frontmatter.validate与verify.plan-structure全部valid=true、exit0；05/12仅保留已接受one-way决策的通用warning。waveErrors=[]、conflicts=[]、missing @ refs=[]、badArtifacts=[]；全部闭合且无sentinel。46条edge验收均仍出现于PLAN，18REQ保留，16份PLAN均有完整MVP故事。08范围为7文件，16为5文件，均2task。API detector重新返回detected=true，coverage matrix valid=true（surface25/integrate21/optout4）。git diff --check退出0。未运行cargo或任何业务验收，未修改根lock，未commit/push。
