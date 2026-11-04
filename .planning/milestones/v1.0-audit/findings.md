# Findings

- 初稿fa81ca0的79/79满足、integration7/7、flows6/6缺乏完整独立核查，已撤回。三源初筛79项均出现在VERIFICATION表，无orphan；15项SUMMARY completion metadata缺失，待逐项核验，不能据此认定功能缺失。
- 独立integration发现L2-03断链：McpL2Tools::gate与allowed_catalog写死editstore；Phase6使用config.l2_acl.edit.catalog。自定义store.trino.catalog与acl.edit.catalog合法，但MCP应用层不会硬排除它。底层完整ACL仍可拒绝，未证明实际数据泄露。
- 原始第二意见target/milestone-l2-catalog-second-opinion.json：Claude Code（实际MiniMax-M3.1-Flash[1m]）AGREE。六工具需语义明确的拒绝或metadata过滤、业务上游零发送；不能以未知tool或Trino拒绝冒充应用层验证。
- Phase1 Nyquist 35 task、12requirement映射，34自动GREEN与1历史显式SKIP；新增cli_color真实子进程六环境场景，451定向测试通过。未重复冻结源码的全workspace。
- Phase2/3/5独立矩阵97tasks：87COVERED/4PARTIAL/6有理由SKIP。现有CI未执行SQLite action_atomic_*（缺test-support）和PG source_guard_absent_target_insert_race（embedded-pg且非ignored）。已补CI入口，待实跑。
- TLS gate真实RED：旧两行`! cargo tree`遇openssl存在/native-tls缺失仍exit0；Cargo查询失败也被反转为成功。CC AGREE改为单次完整图+精确包名，错误fail-closed；当前没有发现实际禁用依赖。
- nightly失败会自动创建/comment issue，本轮不直接触发；Iceberg普通integration早退不算通过，需本地隔离fixture或等价有效证据。
