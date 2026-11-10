# Phase 07.2 MinIO Contract Gap Closure

## Goal

补齐 ENGINE-05 / ADR-0017 的真实 MinIO + PostgreSQL JDBC Iceberg 合同。原需求见 `../../REQUIREMENTS.md`，已有研究见 `../../milestones/v1.0-audit/MINIO-FEASIBILITY.md`。

## Tasks

- [x] 主会话收敛构建、bucket客户端、隔离和cleanup决策；取得只给事实的Claude Code独立意见。
- [x] GSD planner/checker形成可执行PLAN与验证矩阵；2计划、2串行wave，checker零问题，decision coverage 7/7。
- [x] subagent按TDD实现固定源码artifact和runner，483最终run通过；PLAN01定点review三项全部关闭，详07.2-01-SUMMARY和07.2-RUNNER-REVIEW。
- [x] 验证484、重启持久性、失败/超时清理、凭据扫描，并恢复安全的nightly矩阵入口；真实 GitHub nightly 41833307824 两版通过。
- [x] 独立代码/安全/目标验收，PR与CI通过后合并main；详07.2-MERGE-RESULT.md。

## Ownership and Constraints

主会话拥有本计划、progress、findings及T1；实施subagent独占Cargo/Docker，`CARGO_BUILD_JOBS=2`。不修改既有Phase07/07.1的指纹输入，不触发未经授权的issue/comment，不发布Release/npm。保留用户untracked及既有容器。Warp/ralph-loop不可用，采用真实PowerShell/WSL验收闭环并保留退出码，不伪称调用。

## Acceptance

固定官方MinIO源码及Go builder，真实artifact版本/SHA256/provenance；Trino483与484各source=29/copy=29、精确Rust目标1 passed/0 failed/0 ignored；PG metadata_location为s3://warehouse/且bucket有真实对象；保留数据重启后不重做CTAS仍读29行；正常/失败/超时清理且凭据不入证据；实际nightly矩阵通过。完成证据归本phase的SUMMARY/VERIFICATION。

## Next Step

本phase已完成并合并main。后续按 ../../milestones/v1.0-audit/task_plan.md 对账剩余milestone证据；ENGINE-05私有SQL Server历史judgment保留。
