---
status: accepted
date: 2026-11-08
---

# 私有数据源不按数据库类型单列

`sqlserver` 只是 Trino 暴露的一类 catalog，与 `mysql`、`hive`、`iceberg`、`iceberg_rest` 并列。ontoforge 只与 Trino 交互，代码里没有任何按数据库类型分叉的路径：`crates/ontoforge/tests/private_fixtures.rs` 的门控条件是「是否私有环境」（`ONTOFORGE_PRIVATE_FIXTURES` + `#[ignore]`），不是「是否 SQL Server」。

因此 ENGINE-05 不再点名 SQL Server。私有环境的数据源一律表述为「Trino 的某一类 catalog」，验证入口统一是仓库外私有 fixture。后续出现 mysql 或其他 catalog 类型时，不需要新增能力类别，也不需要新增判断项。

后果：Phase 02 的 G-02-D 判断项、`02-11-HUMAN-ACCEPTANCE.md` 与 `02-VERIFICATION.md` 中 ENGINE-05 的措辞按通用表述重述。

用户 2026-11-08 在 `/gsd-verify-work 02` 的 G-02-D 判断环节提出并拍板。
