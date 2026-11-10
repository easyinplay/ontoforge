# Phase01 有界完整重新核验结果

STATUS: COMPLETE
NATIVE_GATE: human_needed

COMPLETE 仅代表本次有界复核和报告交付完成，不代表 Phase01、v1.0 milestone、人工判断或发布获准。没有新人工授权，没有新增 override，没有发现新产品缺陷或需自行接受的新 gray area。

## 原合同复核范围

- 最终 HEAD：060019fe52005c6b85ae5c5f59da9868f464e579。
- 14份 PLAN、14份 SUMMARY 全文读取；97条 PLAN truth逐项映射为96 VERIFIED +1既有 editor PASSED (override)。SUMMARY仅用于定位，不作通过证据。
- ROADMAP MVP goal经 owner 验证valid；5项 SC全量纳入，全部 VERIFIED。97+5=102合同索引（101 VERIFIED +1原override），存在语义重叠，不声称102个独立用户行为。
- 原11项 must-haves全部保留并回映射，仍为10 VERIFIED +1原override的11/11。这个历史汇总口径与97条PLAN细粒度计数不同，均不包括3项待人工判断。
- 34个 task acceptance block、132条验收条款，以及14份PLAN的verification/success_criteria均收取；58/58声明artifact、32/32声明key link核实；01-13没有声明块，exit66按N/A，另作源码/历史/测试核实。
- 12个requirement ID无orphan；18/18可追踪CONTEXT decisions honored；315个原covered输入全部存在并检查，5个变化源码有独立assertion及历史/最终执行映射。

## 行为证据及新鲜性

复用真实 CI41994479295（headSha完全一致，5个job成功）和本地最终HEAD日志，不重跑Rust/CI。164个选定具名测试逐一核实源码实质assert/expect及WindowsCI、UbuntuCI、本地workspace三路成功记录；额外3个cardinality兼容测试单列。4887是跨job成功记录条数，不是独立测试数。

本地原始命令： cargo test --workspace --locked -- --test-threads=1

2026-11-05T08:43:50.3003373Z至08:51:34.9365322Z，exit_code=0，原始summary合计1528 passed /0 failed /119 ignored；ignored不算通过。未声称本轮执行、未声称私有服务或容器重新验收。schema/snapshot与HEAD无diff，既有runner禁止更新expected。

5个移动输入：CONTRIBUTING.md、Cargo.toml、crates/ontoforge-schema/src/diag.rs、crates/ontoforge/src/cli.rs、crates/ontoforge/src/lib.rs。分别核对editor章节不变、原10crate与accepted后续ADR、Diagnostic线格式/定位及W18增量、online flag约束、warning计数与offline分派不变。当前产品相对HEAD无diff。

历史实现约束未被静默改写：原7个CONTRIBUTING章节现8个；原10crate现11个有ADR0021，方向修订有ADR0023；01-03当前canonicalize确实存在，是Phase02 CR-S01/82e6dde内部containment安全加固，不虚称grep零匹配；01-13快照和锁文件按实际历史区间核实。详见CONTRACT-RESULTS.md。

## covered 前后差异

| 项目 | 结果 |
|---|---|
| 原covered | 315 |
| 现covered | 346 |
| 保留 / 删除 / 新增 | 315 /0 /31 |
| 原digest | v4:sha256:ab5ef91bd70deeaba8abb80fd7352d53165111c05d99b47edda07a0ccb904a6c |
| 新digest | v4:sha256:5752098c8f87c1f3524d3b7901cfd1f5049eb49ada764b21e5219eaa5cbef893 |
| 原报告SHA256 | bc964c6652a1e22c91a0abfb3cdaacf009b651c35fdd97d3f3796f4d4c5455ae |
| 新报告SHA256 | 98c80776c3170489cf0d72007fcaf942e88dbd605207cb713a5b300257adc310 |
| 输入hash前后差异 | 329份输入逐hash，0差异 |
| native routing | stale -> human_needed；owner exit0 |
| git diff --check | exit0 |

原报告原字节冻结为01-VERIFICATION.before.md。fingerprint只在完整合同/源码/执行证据核验后用官方owner生成，315个旧covered零删除；新增31个为独立冻结证据与最终HEAD原始执行输入。不把main-*.json、MAIN-REMAINING-GATES.md、matrix、T1或PWF加为新依赖。后验RESULT、covered-set-diff、native-gate-results、acceptance-check不进入covered，避免自依赖。

## 集成 RED 与格式修复

主会话真实运行 `requirements-final-recompute.cjs` 在第131行失败：`12 !== 0`，出现12个orphan。这是本次报告把Requirements Coverage兼容表格改成bullet导致的机器接口回归，不是一次通过。

本轮仅恢复该节12行表格：第一列PLAN，第二列Requirement ID，列名Status，12行状态均为SATISFIED_TECHNICAL，并逐行保留人工边界。未修改派生脚本，也未接受任何人工项。

复用原脚本未修改的解析逻辑做单节定向复现，修复前解析0行、12个orphan；修复后解析12行、0个orphan，全部Status为SATISFIED_TECHNICAL。此结果不是完整矩阵重算通过，完整集成重算由主会话执行。RED、GREEN、脚本hash、前后报告hash及命令回执保存在integration-requirements-format-regression.json。

native gate重查exit0、human_needed；git diff --check exit0。acceptance-check.json的current_report_hash已更新。报告不属于covered，原346项covered及digest未变，没有修改任何covered输入；本段和RED记录是后验集成回执，不加入covered。

## 必须保留的人工边界

1. 01-01原创出处禁止项：UNCERTAIN / WARNING。限定禁名扫描零命中不是完整原创出处证明。
2. 01-01路径/用户名禁止项：UNCERTAIN / WARNING。相对定位链、固定坐标和产品snapshot扫描有证据，但原judgment仍需显式resolution。
3. 01-08 ONT-V不可关闭/降级：UNCERTAIN / WARNING。先结构校验后配置、ID拒绝与negative tests真实通过不等于原judgment已由人工接受。

已有editor体验沿01-UAT Test2的既有override，不重开、不扩大；终端human-check沿Test1既有pass。3项judgment不受这些历史记录或“继续”影响。01-13 backstop有独立历史/具名执行证据，不混作新增人工授权。最终native gate必须是human_needed，不能是passed。

## 可核查交付文件

- 报告：.planning/phases/01-workspace-schema-lint/01-VERIFICATION.md。
- 完整合同及源码/测试映射：CONTRACT-RESULTS.md、contract-index.json、truth-results.json、source-assertions.json、acceptance-contracts.json、requirements-coverage.json、summary-review.json。
- 真实具名执行：executed-tests.json、selected-test-source.json、supplemental-inspection.json（CI身份、本地原始头尾/合计、额外3测试）。
- 全旧输入与专门5源码核实：covered-file-inspection.json、changed-source-inspection.json、anti-patterns.json；原文/owner artifact/link/decision回执在contracts.json。
- 冻结基线与差异：01-VERIFICATION.before.md、covered-baseline.json、evidence-inputs.json、fingerprint-results.json、covered-set-diff.json。
- 写后判定：native-gate-results.json、acceptance-check.json、mvp-story-validation.json。
- 采集方法：collect.cjs、build-evidence.cjs、inspect-details.cjs。参考report-resign-03-04的收集方法，不复制其判定。

## 写入与范围限制

仅写本报告和report-resign-01目录；没有改生产、其他phase报告、matrix/T1/PWF，没有回滚他人变更，没有操作3容器、5用户untracked或清target，没有commit/push/publish/archive。Phase02只读确认旧MinIO FAILED已由07.2闭合，02原判不改；不解除02-05.13、02-10.8/私有SQLServer与公司数据边界。未沿用撤回审计。

ralph-loop/Warp不可用，没有伪称；使用完整合同到实现/真实运行/人工门/覆盖/natively routed gate的有界验收循环。纯报告和既有证据核验，不引入实现代码或新测试，TDD不适用；状态落在授权T2证据和报告，不写用户禁止改动的PWF。
