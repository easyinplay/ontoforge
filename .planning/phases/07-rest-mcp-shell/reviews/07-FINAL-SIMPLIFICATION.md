# Phase 07 最后增量审查与简化复核

日期：2026-11-04。基线 `552556e`，审查 HEAD `adc02278b5cc55111def9a30a1c3b778485e8279`。

Critical：0；Informational：0；简化建议：0。不建议改动冻结实现。

先完成 Standards/Maintainability review，再按 verify-simplify 尾部规则静态复核。依据 `.claude/CLAUDE.md:14`、`:15`、`:39` 的平台、fixture 和现有模式要求，以及 `Cargo.toml:56` 的 lint 规范；排除 lint 已覆盖项。仅审 T08 指定生产文件与 admission 测试；typed/query/CLI 增量只检查新增抽象是否必要。

| 证据 | 保留理由 |
|---|---|
| `crates/ontoforge-server/src/context.rs:101` | 分层反序列化分别拒绝重复键、非法 requestId、非通知及 rmcp 不接受的 params，不是可直接合并的重复逻辑。 |
| `crates/ontoforge-server/src/context.rs:296` | `_http_permit` 在 extractor 丢弃 Parts 后保持 REST 执行额度。 |
| `crates/ontoforge-server/src/mcp.rs:542`、`:624` | permit 随读取调用持有，Action 则转交独立任务；删除会改变并发限制。 |
| `crates/ontoforge-query/tests/l2_execute.rs:670` | `ExplainGate` 服务四种 ExplainType 的可控超时/取消验证，未增加通用框架。 |
| `crates/ontoforge/tests/stdio_security_support/mod.rs:210`、`:383` | redirect trap 与有界 HTTP 解析均有测试消费者；未为跨 crate fixture 隔离重复追加抽象。 |

仅静态复核，未运行 Cargo、服务、测试或 commit，未改实现代码。主会话提供的 50/50 与 Clippy 结果未在此重新执行，fresh/CI 状态由最终验证报告负责。

## 冻结指纹核对

下列七个实际工作树 Git blobs 与 `target/phase7-mcp-admission-fingerprints.txt` 一致；同时逐项核对该文件记录的 SHA256，全部一致。

| 文件 | 审查 Git blob |
|---|---|
| crates/ontoforge-server/src/context.rs | 12de800b01837b3b8ec8e26b819c82d382e0ac82 |
| crates/ontoforge-server/src/mcp.rs | 7b154b0100f8e9433ab95061369d075cd8d1539e |
| crates/ontoforge-server/src/lib.rs | d14d825c8e60199fe983bd472fa4c0e66837c5e1 |
| crates/ontoforge-server/src/state.rs | c6a2287eb87f6b06759a8c9a493e7cd8ca4faad0 |
| crates/ontoforge-server/tests/mcp_execution_admission.rs | 10090eb81b28be113519b5cfdbfd1393ce2fe64c |
| crates/ontoforge-server/tests/unit/mcp_admission.test.rs | d4204f34d83518505c639ae4b0421a20327dddda |
| crates/ontoforge-server/tests/unit/mcp_sessions.test.rs | f98955789069fed1e2a27f3d101f45033c666692 |

## 辅助抽象复核指纹

| 文件 | Git blob |
|---|---|
| crates/ontoforge-query/tests/l2_execute.rs | 4813a568f6f54aa9dc4d16c676499217b99a0c21 |
| crates/ontoforge-server/tests/mcp_l2_defaults.rs | 8d951961ce704772a9135937e6d380e26dd4ecd3 |
| crates/ontoforge/tests/cli_stdio.rs | 4c3bf392652e43032f04d4828c51c6f597e780a2 |
| crates/ontoforge/tests/stdio_security_support/mod.rs | 7b91517336080e3c339f0656c93a2eff95ca827b |
