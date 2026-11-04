# Phase 07 增量复验

日期：2026-11-04。目标源码 `f8851ca073fed34e80e605f0ac91880259df7e12`。本文件由独立 verifier 维护，属于既有 GSD milestone 审计的独立证据账本。

## 范围和状态

- [x] 读取原 canonical、终审报告、INT-01/TLS 修复账本及独立安全结论。
- [x] 亲核 `adc0227..f8851ca` 的生产/测试/CI diff、真实 HTTP 新测试断言及最终 CI 原始日志。
- [x] 独立执行 TLS 六场景隔离测试：6 passed，exit 0；没有执行真实 Cargo。
- [x] 更新所属 SUMMARY、UAT、SECURITY、VALIDATION 与 canonical。
- [x] 最后生成 fingerprint 并查询 GSD status：passed，exit 0。

旧 80 条 truth 与 48 项 threat 是历史独立审计集合。本轮保留其原始证据，检查源码差异与最终回归，只对 INT-01/TLS 及其接线作完整增量核验，不宣称重新执行全部旧测试或重新审计全部威胁。STORE-05 source 缺口与 ENGINE-05 历史 MinIO 差异属于 milestone 其他范围，不由 Phase 07 通过结论消除。

已读取并应用 `gsd-verify-work` / `gsd-progress`，实际运行 `query init.verify-work 7`、`query init.progress` 与 decision coverage（3/3 honored）。自动 UAT 依据用户既有授权，均记录 `source: automated`，不冒称人工见证。未运行 Cargo/Docker、未提交、未发布或发送外部消息。文档审计无业务实现改动，TDD 不适用；Warp、ralph-loop 不可用，未声称调用。当前工具无 Write，使用结构化 `apply_patch` 写文档，不使用 shell heredoc。

## 独立源码与行为核对

| 增量 | 源码与断言 | 实际证据 | 结论 |
|---|---|---|---|
| INT-01 | `TrinoEngine::l2_edit_catalog()` 从同一不可变 ACL config 取值；`call_l2` 先检查 Enabled，再取可信 catalog，缺失即 SOURCE_UNAVAILABLE；六工具共享 `gate()` 的 L2Policy | `54a47dd` 生产 diff；`mcp_l2_defaults.rs` 新5个 HTTP tests；原 RED 0/2、同名 GREEN 2/2、相关回归83/83；最终 Windows/Ubuntu 新5项均 ok | VERIFIED |
| edit catalog hard exclusion | SQL validate 与 metadata 最终 target 都使用 deployment exclusion，大小写忽略；defaults/qualified override 不绕过 | 九次拒绝逐次核对全部上游与业务请求零增量、source fallback不增、audit为空；两套 catalog 六工具正向值与6条audit；无Trino六工具不可用 | VERIFIED |
| TLS guard | 单次完整 `cargo tree`，`set -euo pipefail`，精确 package 首字段；两 OS CI 接线 hermetic 与真实 helper | verifier 实跑 `python -B -m unittest discover -s ci -p test_tls_deps.py -v`：6 tests in 0.399s，OK，exit 0；最终两 OS 两步骤 success；真实 helper日志 `tls_real_exit=0` | VERIFIED |

测试质量：新5个 HTTP 测试无 ignore；TLS 六测试执行 workflow 的实际 command/helper，仅替换 Cargo 边界。断言为确定响应值、请求次数、audit 与 fallback，而非仅 status。mock Trino 允许业务读取敏感 fixture，专门隔离应用 fence，不证明真实环境曾泄漏数据。schema-only grant 与每个 metadata direct 大写请求未单独枚举；共享 policy 与大小写检查代码成立，未发现可复现缺陷。债务/disabled扫描无命中。

## 最终证据身份

- CI `41759804733`：headSha=f8851ca，completed/success；Windows、Ubuntu、integration、phase7-real-services 四必需 job success。原始 Windows/Ubuntu 日志分别在3245–3257、3265–3277记录新5测试 ok；TLS六测试与真实helper步骤成功。
- 动态服务输入：亲核 `git diff 868643c f8851ca`，唯一改变为 `cli_color.rs` 的 ColorCase 类型别名，实际服务/SDK/fixture/lock 输入不变；verify commit.txt=f8851ca。51 passed 为 SQLite5 + PG1 + fresh43 + Iceberg2。原 runner exit 1 必须保留；独立 `final-check.log` 证明 provenance、凭证扫描、清理 exit 0。Windows owner 正常退出由执行 owner 的原始账本与清理日志记录，本 verifier 没有再次启动服务。
- release `41759804768`：三 native + aggregate success，publish skipped。source f8851ca，实际 packaged `22913c8e79ae98b6604109d6e05a15f27cd50292`，两个完整 tree 均 `0ecc59382aaf12e782c508ba64858e2689299fd7`；独立 archive/hash/manifest 证据见 `RELEASE-ARTIFACT-AUDIT.md` 与 `target/milestone-release-41759804768/`。不把 source 与 PR merge checkout 说成同一个 commit。

## 归属与历史边界

API-02 累计完成归07-05；MCP-02/03/04/05归07-07；L2-03 transport归实际六工具adapter的07-08，并注明本次54a47dd与Phase06共同贡献。原07-08 PLAN未列L2-03，保持原PLAN与旧完成日期，新增dated累计说明，不倒写原始计划。中央 REQUIREMENTS/ROADMAP/STATE 由主会话维护。

## 最终复核与交还

- 当前canonical `status: passed`、80/80；原80条与48项威胁保留历史证据边界，增量INT-01/TLS CLOSED；SECURITY为SECURED、open=0。VALIDATION改为规范`validated`，通过GSD `verification.append-audit`追加本轮2 found / 2 resolved / 0 escalated；新测试不重记为旧32 task。
- UAT14/14，均automated；`query phase.uat-passed 7 --uat-only`与`--require-verification --pick passed`均true。三个SUMMARY通过`summary-extract`确认API-02、MCP-02/03/04/05、L2-03及原有IDs；07-14新增当前release/final CI/fresh证据，旧历史保留。
- verifier按最终CI metadata的workspace步骤时间窗口独立计数：Windows150组1496/0/117、Ubuntu150组1503/0/117。没有重复累计其他定向job或将两OS合计成唯一测试数。
- verifier额外只读执行archive核验：三个archive SHA、binary SHA、内外manifest、source/package完整tree一致，exit0；没有写回他人verification.json或运行二进制。首次自写读取误假定外部manifest固定名，遇FileNotFoundError；核查真实`*.manifest.json`命名后以原审计程序去掉写入行的只读版本重核通过。另一次rg的Windows路径通配报错，已改为目录加`-g '*.log'`读取成功。这两次工具错误不是产品失败，也未隐藏。
- GSD最终fingerprint覆盖237文件：`v4:sha256:2653bcf9d8651491b658036144b9685eb0e82adbf83b235e75f0d74464cd2f68`。`query verification.status .planning/phases/07-rest-mcp-shell --pick status`实际输出passed、exit0。`git diff --check`通过，仅显示既有CRLF→LF提示。
- 已完成受分配文档范围。没有修改全局STATE/ROADMAP/REQUIREMENTS、他人文件或实现，没有commit。全局STORE-05/ENGINE-05及Phase7.1方案选择仍由主会话处理；不据此宣称milestone完成。
