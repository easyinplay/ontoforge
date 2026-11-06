# ENGINE-05 子范围决议

状态：accepted-for-audit-routing

## 决议

将 Phase07.2 已完成的 MinIO/S3 与 JDBC Iceberg 双版本合同作为 ENGINE-05 的已闭合子范围；ENGINE-05 整体继续保持 Partial。Phase02 的私有 SQL Server fixture、公开 fixture 数据来源和公开 CI 隔离判断继续等待对应证据或明确人工 resolution。

## 理由

Phase07.2 已提供 483/484 的真实运行、29 行 source/copied、restart 保持、S3 对象、cleanup 和归档重放证据。GitHub 官方方案也把私有网络服务验证放在 self-hosted/private network runner 中，并要求保留独立测试输入和结果。当前没有私有 SQL Server 执行记录，也没有把公开 fixture 的来源声明升级为人工事实。

## 验收条件

1. MinIO/S3 子范围保持 Phase07.2 `9/9 passed`，不覆盖历史失败记录。
2. ENGINE-05 仍在需求矩阵中保持未勾选或 Partial，直到私有 SQL Server 与来源/隔离边界各有可审计证据或逐项人工 resolution。
3. 不把公开 CI、历史报告、fingerprint 或本次决议写成私有环境已测试。
4. 该决议不授权发布、归档、commit/push 或 `gsd-complete-milestone`。

## 来源

`COMPARABLE-SOLUTIONS-RESEARCH.md`、Phase02 `02-VERIFICATION.md`、Phase07.2 `07.2-VERIFICATION.md`。
