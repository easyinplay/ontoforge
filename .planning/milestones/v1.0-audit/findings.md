# Findings

- MinIO后续独立咨询（target/minio-restoration-independent-opinion.json）自行推荐保留原标准、固定官方源码自建测试artifact再隔离验证。原02-10 PLAN要求MinIO但未要求官方签名预编译image，主会话采用此工程路径，不改ADR验收语义。当前仍仅研究，builder pin/构建/运行尚未完成，ENGINE-05保持Partial；详MINIO-FEASIBILITY.md。

- 新独立Iceberg意见（target/iceberg-fixture-independent-opinion.json）：local warehouse路径483/484证据充分，但accepted ADR0017与02-10 PLAN明确MinIO，SUMMARY披露替代不等于正式决策修订。当前是规范偏差及对应覆盖缺口，非已证产品故障。主会话保留未闭环，不在缺少最终处置时新增accepted ADR或宣称MinIO/S3通过；ENGINE-05当前追踪降为Partial。SQLServer私有fixture与nightly外部发帖分支证据边界继续保留。

- 2026-11-04用户修正第二意见协议：只提供需求、现状、约束和已验证事实；不得附带主会话方案，不要求AGREE/REFINE/DISAGREE。对方独立给方案后主会话再比较；同方向自动推进，实质分歧才问用户。旧定向审查作为历史保留，当前Phase7.1与Iceberg差异已重做独立咨询，不能继续把旧AGREE当本次独立结论。

- Phase7.1 planning gates：codebase/context drift分别no-structure-md/no-upstream-artifacts，非阻塞SKIP；UI gate正确入口是`check ui.plan-gate`，先前`query ui.plan-gate`/`ui plan-gate`为命令路由错误，已按check-command-router.cjs纠正。研究不引入库升级或外部新API。

- 2026-11-04 source合同收敛：Claude Code第二次AGREE（target/source-cardinality-scope-followup.json，实际MiniMax-M3.1-Flash[1m]）。现IR仅单列PK/FK；新增opt-in --check-cardinality，source-backed link.to OneToOne按非NULL FK分组COUNT(DISTINCT mapped PK)>1告警。不评估referential integrity，诊断明确此边界；不引入复合键或跨backend JOIN。完整合同将归Phase7.1 CONTEXT。
- 2026-11-04 恢复核对：f885 CI41759804733的Ubuntu、integration、phase7-real-services已success，Windows仍运行；release41759804768三平台成功，独立artifact重算进行中。动态owner已报SQLite5、PG sourceguard1、服务17 GREEN，客户端与Iceberg尚未完成。不得将runner整体提前标绿。

- 初稿fa81ca0的79/79满足、integration7/7、flows6/6缺乏完整独立核查，已撤回。三源初筛79项均出现在VERIFICATION表，无orphan；15项SUMMARY completion metadata缺失，待逐项核验，不能据此认定功能缺失。
- 独立integration发现L2-03断链：McpL2Tools::gate与allowed_catalog写死editstore；Phase6使用config.l2_acl.edit.catalog。自定义store.trino.catalog与acl.edit.catalog合法，但MCP应用层不会硬排除它。底层完整ACL仍可拒绝，未证明实际数据泄露。
- 原始第二意见target/milestone-l2-catalog-second-opinion.json：Claude Code（实际MiniMax-M3.1-Flash[1m]）AGREE。六工具需语义明确的拒绝或metadata过滤、业务上游零发送；不能以未知tool或Trino拒绝冒充应用层验证。
- Phase1 Nyquist 35 task、12requirement映射，34自动GREEN与1历史显式SKIP；新增cli_color真实子进程六环境场景，451定向测试通过。未重复冻结源码的全workspace。
- Phase2/3/5独立矩阵97tasks：87COVERED/4PARTIAL/6有理由SKIP。现有CI未执行SQLite action_atomic_*（缺test-support）和PG source_guard_absent_target_insert_race（embedded-pg且非ignored）。已补CI入口，待实跑。
- TLS gate真实RED：旧两行`! cargo tree`遇openssl存在/native-tls缺失仍exit0；Cargo查询失败也被反转为成功。CC AGREE改为单次完整图+精确包名，错误fail-closed；当前没有发现实际禁用依赖。
- nightly失败会自动创建/comment issue，本轮不直接触发；Iceberg普通integration早退不算通过，需本地隔离fixture或等价有效证据。
- SOURCE-CARDINALITY：独立explorer确认STORE-05 source侧未实现；pk_check.rs:124只ObjectType并只取primary_key，binding diagnostics只表列类型，读取link OneToOne/OneToMany共用分支不报source重复。03-13 PLAN/SUMMARY原明确不交付source报告；没有取消需求的ADR。现中央STORE05取消完成勾选，edit部分仍有证据，不能用对象PK重复测试替代FK基数。
- 最小静态复现：aviation副本maintenance_records.csv将R-002 tail_number改为R-001的D-AAAA，PK仍唯一却违反aircraftCurrentMaintenance oneToOne。当前validate --online --check-pk按源码不会报告此FK重复；尚未执行此复现，待新缺口TDD。
# 2026-11-04交付补记

Phase07.1独立verifier通过18/18，native verification.status与UAT10/10通过。phase.complete因历史01/02未完成将下一指针选回Phase01，主会话按实际当前交付工作将digest保留07.1 completed、下一步ship/MinIO，不伪称旧judgment已处理。requirements.mark-complete对现有STORE-05的自定义Partial说明返回not_found且未写；实际ID存在，依据独立SATISFIED结论由T1 owner直接同步checkbox与traceability为Complete，不修改全局GSD工具。

用户新增合并授权已落实：PR8整体Phase03–07合入main ef0ab9c，精确head CI与打包验证通过；PR6自动merged、PR7祖先包含后关闭重复入口。详MERGE-RESULT.md。此前“不合并”均为历史授权边界，已被用户当前指令覆盖。
