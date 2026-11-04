---
status: SECURED
scope: INT-01 and TLS-GUARD only
threats_open: 0
reviewer: independent gsd-security-auditor
date: 2026-11-04
source_commits: [54a47dd, ef7ac92]
---

# 两项增量修复安全复签

独立 auditor 只读源码、调用链与既有原始日志，返回 SECURED，局部2/2 CLOSED。本文件由主会话按其最终结构化结果落盘；auditor未运行Cargo/服务，不代表整个milestone通过。

| 项 | 分类/级别 | 结论与源码证据 |
|---|---|---|
| INT-01 / L2-03 / 关联T-07-22 | Elevation of Privilege / Information Disclosure，high | CLOSED。TrinoEngine accessor从实际ACL配置取catalog；MCP要求Enabled后注入，缺失即拒绝。SQL、discovery与最终table target共享同一L2Policy。 |
| TLS-GUARD / dependency gate | Tampering，high（局部分类） | CLOSED。helper单次完整图查询失败则中止，读取package首字段精确拒绝openssl/native-tls；CI调用hermetic测试和真实helper。此结论不重审整个T-07-SC。 |

## 亲核行为与信任边界

- config.rs:572校验ACL edit与store catalog一致，:827/:1035校验非空单段名。未声明store.trino时，l2_health.rs:274经acl_table_probe→probe_identifier拒绝非法值，无法取得Enabled。未添加从role或MCP请求读取可信catalog的路径。
- execute/explain先validate_l2；metadata先解析最终target再排除。printer与adapter均ASCII忽略大小写；defaults与qualified override不能绕过最终检查。
- HTTP拒绝用例同时断言全部上游statement、业务statement、source fallback与audit均无增量。list_catalogs在合法枚举后过滤敏感名；正常六工具保留六条成功audit。
- 无Trino时六工具SOURCE_UNAVAILABLE、source调用不增且audit为空。
- TLS测试运行workflow的实际command/helper，只替换Cargo边界；覆盖单独/同时禁包、无禁包、查询失败、相似名称，断言只查询一次完整图。
- 新SQLite gate启用test-support并先发现至少5条；PGsource gate启用embedded-pg且精确执行非ignored测试，fixture优先ONTOFORGE_PG_URL。

## 亲读日志

| 日志 | 结果 |
|---|---|
| target/l2-catalog-red.log | 0 passed / 2 failed，预期应用fence失败 |
| target/l2-catalog-green.log | 2 passed / 0 failed |
| target/l2-catalog-server-regression.log | 83 passed / 0 failed / 0 ignored |
| target/l2-catalog-clippy.log | 完成无错误 |
| target/milestone-tls-real-tree.log | tls_real_exit=0 |

## 精确限制

1. 新fixture同时授予catalog/schema grants，未分别跑schema-only；metadata uppercase覆盖列表输出，未单独覆盖每种direct请求。共享policy与大小写代码检查成立，未发现必须追加才可关闭的已知缺口。
2. Windows/Linux新workflow实际结果仍由主会话收集，不能用静态接线替代。
3. auditor核对CI action默认CARGO_TERM_COLOR=always，并读Cargo官方源码提交3c747ea3ead23951c1b43a5edfcbdb449135ddeb：prefix none不为package首字段加颜色，颜色可在尾部(*)；未执行pinned1.98强制颜色实验，保留CI实际helper验证。
4. HTTP fixture返回合成数据，证明应用层fence，不证明真实环境曾有数据泄露。

没有发现本次两项修复新增的未映射攻击面；不据此推断整个milestone不存在其他威胁。
