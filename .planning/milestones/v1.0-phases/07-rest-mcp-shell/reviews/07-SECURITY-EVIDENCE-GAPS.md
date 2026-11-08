# Phase 7 动态证据缺口闭合

2026-11-04，冻结源码 `adc02278b5cc55111def9a30a1c3b778485e8279`。E1–E7 与 N1/N2/N3 均已动态 GREEN；T24、E6、T08 的生产缺陷和修复期回归已独立复签。48/48 threats CLOSED，详 `07-T08-SECURITY-RECHECK.md`。本表记录证据缺口，不将缺口数量等同生产漏洞数量。

| ID / 威胁 | 原证据缺口 | 闭合证据与边界 |
|---|---|---|
| E1 / T10 | HTTP未跨definition fence，PG仅旧对象版本 | HTTP safety3/3；最终fresh PG4/4含definition stale barrier。第二store reload后submit/validate拒绝，完整持久行零变更。 |
| E2 / T20,T21,T37 | 静态catalog/schema未证明跨transport race或在途snapshot | Actions10/10、MCP contract16/16；最终fresh PG4/4含REST/MCP race。同command一次提交、原receipt/event/time一致、当前授权及定义fence拒绝。 |
| E3 / T23,T26 | 缺token到期、session关闭及丢响应发送计数 | transport11/11；stdio11/11覆盖实际Initialize/session关闭、执行前断连、提交后丢响应不隐式重放，显式replay原receipt。首轮5/3协议/fixture错误由后继GREEN闭合。 |
| E4 / T28,T32 | 缺双有效主体撤权及在途取消隔离 | 最终fresh Rust9/9、TS17/17，真实grant/revoke与双请求隔离。TS只证明响应在途AbortSignal隔离，不扩称服务端执行取消。 |
| E5 / T31 | 缺u64 max公开端点 | 最终fresh TS17/17含18446744073709551615n、unsafe number发送前拒绝及decimal字符串；范围为WireJson。 |
| E6 / T16,T22,T38,T46 | 同session撤权/TTL/health/零发送贯通不足 | L2 core2/defaults9、六工具矩阵1/1；最终admission defaults10/10、fresh Trino2/health2/ACL6。授权预检先于availability的真实RED已修复，独立复签CLOSED。 |
| E7 / T29,T33 | 缺写丢响应次数与完整持久增量 | 最终fresh Rust9/9、TS17/17；故障proxy只发送一次，显式replay保留原receipt/time，业务行与五张事务表不重复提交。 |
| N1 / 07-09-2 | stdio成功链及不同主体直接调用不足 | stdio11/11 + Mia定向1/1（8次拒绝）；Alice完整读取/validate/Action/history/receipt与REST等价，Mia拒绝后持久行及Alice receipt不变；Spec CLOSED。 |
| N2 / 07-09-1,2 | 无实际302与在线坏token | 在线host 401/零业务请求；302 origin收到bearer但trap零连接；有界退出/无secret；Spec CLOSED。 |
| N3 / T08 | typed EXPLAIN仅顺序成功，cap1第二请求200≠429 | query15/15覆盖四格式timeout/cancel/独立请求；permit修复后typed limit及相邻测试50/50，REST↔MCP、slow body/control、Action持槽GREEN；Spec/Staff/security CLOSED。 |

## 最终证据与集成门关闭

- `target/phase7-final-adc0227-verify.log`：服务17（PG4、drift2、snapshot1、Trino2、health2、ACL6）+ Rust9 + TS17 = 43，全部实际执行，`fresh_verify_exit=0`。生成、strict typecheck、build、credential scan与cleanup通过；副本 `target/phase7-services/evidence/final-adc0227-verify.log`。
- `target/phase7-mcp-admission-green-r2.log`：lib25、admission4、L2defaults10、transport11，共50/50，零失败/ignored；Clippy/fmt通过。该集合与上表重叠，不相加为唯一用例总数。
- 复签：`07-FINAL-SPEC-REVIEW.md`、`07-FINAL-STAFF-REVIEW.md`、`07-T24-SECURITY-RECHECK.md`、`07-E6-SECURITY-RECHECK.md`、`07-T08-SECURITY-RECHECK.md`；末尾 `07-FINAL-SIMPLIFICATION.md` 建议0。
- release `41669982362` 三native+aggregate成功，publish skipped，三个archive独立验hash，详 `../07-14-SUMMARY.md`。
- CI `41669982375` completed/success，Windows test、Ubuntu test、integration、phase7-real-services四必需job全部成功。元数据与Windows/Ubuntu完整日志均在target保存，G5已关闭；32task动态门齐全后nyquist=true，UAT已complete、13/13（975f78e），Phase goal由独立VERIFICATION归并。

没有创建/上传Release、发布npm或合并PR。原始RED、fixture失败与后继GREEN保留在各lane ledger。历史07-02 D4与07-09 D3的unknown/human源于当时未跑，现由PG/stdio自动化实测覆盖，原SUMMARY不改；最终UAT记录裁决，CC AGREE见 `target/phase7-final-uat-review.json`。

最终发布状态：PR #7/#8均OPEN、Draft、mergedAt=null，见`target/phase7-pr{7,8}-final-status.json`；release publish skipped。等待期间Windows pending记录已由最终CI结果替代。
