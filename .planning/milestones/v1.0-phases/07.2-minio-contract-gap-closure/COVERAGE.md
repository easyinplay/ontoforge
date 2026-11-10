# Phase 07.2 Coverage

## API Coverage

覆盖对象是测试fixture对MinIO S3 SDK的集成，非产品API扩展。D-02已锁定小client只创建bucket与枚举对象；D-05的Iceberg数据读写由既有Trino connector完成。下表明确其余能力不属于此phase，不生成第二套S3产品功能。

| capability | decision | reason |
|---|---|---|
| MakeBucket | INTEGRATE | 小client创建本run的warehouse，01-1 |
| ListObjects/ListObjectsV2及分页 | INTEGRATE | 小client枚举metadata/data，01-1/01-2 |
| S3请求签名、region、deadline和错误传播 | INTEGRATE | 复用固定minio-go/v7 SDK，01-1/01-2 |
| Iceberg object PUT/GET/HEAD及metadata读写 | INTEGRATE | 由既有Trino connector在真实CTAS/读取/restart执行，01-1/02-1 |
| BucketExists/ListBuckets/GetBucketLocation独立管理入口 | OPT-OUT | D-02限定make/list，显式region与全新bucket足够，本phase不提供管理CLI |
| RemoveBucket及对象删除CLI | OPT-OUT | D-03/D-04使用owner-scoped volume清理，客户端不增加删除API |
| FPutObject/GetObject/StatObject独立应用API | OPT-OUT | D-05由Trino执行数据路径，无产品S3 API扩展授权 |
| CopyObject/ComposeObject/SelectObjectContent | OPT-OUT | 原Iceberg fixture合同不含应用侧对象加工 |
| Presigned URLs/PostPolicy | OPT-OUT | 无对外上传下载产品入口 |
| Bucket policy/ACL/IAM/STS及用户管理 | OPT-OUT | 独立短生命周期fixture使用临时身份，无多租户管理scope |
| Versioning/lifecycle/retention/object lock/legal hold | OPT-OUT | 不改变生产保留策略，持久性仅限本run restart |
| Notifications/replication/encryption/KMS/transfer acceleration | OPT-OUT | 原合同不含这些部署能力 |
| MinIO admin API/集群管理/站点修复 | OPT-OUT | D-01固定单fixture server，不建设管理平台 |
| 其他minio-go SDK业务能力 | OPT-OUT | D-02显式仅make/list；未列方法不隐式扩展产品scope |

## Multi-Source Coverage Audit

| SOURCE | ID | Feature/Requirement | Plan | Status | Notes |
|---|---|---|---|---|---|
| GOAL | 07.2 | 恢复MinIO/S3与JDBC合同、483/484及nightly | 01/02 | COVERED | 真实运行是关闭条件 |
| REQ | ENGINE-05 | Linux PG/Trino及nightly Iceberg既有MinIO缺口 | 01/02 | COVERED | SQL Server私有fixture沿既有合同，不扩本phase |
| CONTEXT | D-01 | 固定commit/builder、官方recipe、readonly modules、DEVELOPMENT、provenance | 01-1 | COVERED | scratch需实跑 |
| CONTEXT | D-02 | 同源码minio-go make/list、deadline、无秘密输出 | 01-1/01-2 | COVERED | 不新建依赖图 |
| CONTEXT | D-03 | 独立bridge、owner、动态loopback、临时catalog、两volume | 01-1/01-2/02-1 | COVERED | 既有容器和PR catalog不改 |
| CONTEXT | D-04 | 私有秘密、quiet、真实scan、所有退出路径安全清理 | 01-1/01-2/02-1/02-2 | COVERED | SIGKILL由CI独立补偿，非trap保证 |
| CONTEXT | D-05 | 两版29/29、guard、精确1/0/0、PG S3和对象、restart无CTAS | 01-1/02-1 | COVERED | 全为live门 |
| CONTEXT | D-06 | 全nightly corpus、同runner、schedule通知门、实际两矩阵 | 02-2/02-3 | COVERED | 不发布registry/Release/npm |
| CONTEXT | D-07 | 独立review/security/goal后改ENGINE-05，按授权合并 | 02-3 | COVERED | 主会话执行独立gate和T1更新，不改旧fingerprint |
| RESEARCH | R-01 | runner RED/GREEN含partial-create、timeout、guard、隔离、泄漏 | 01-1/01-2/02-1 | COVERED | mock与live分别留证 |
| RESEARCH | R-02 | builder layers/modules取得及scratch可运行尚未验证 | 01-1 | COVERED | 构建失败不能宣称完成 |
| RESEARCH | R-03 | WSL/CI mount、权限、随机端口及named volumes | 01-1/02-1/02-3 | COVERED | 两环境实际证据 |
| RESEARCH | R-04 | nightly deadline与资源限制 | 02-2/02-3 | COVERED | 同机唯一owner、jobs2 |
| RESEARCH | R-05 | 复用已有Rust目标及SQL，不修改测试语义 | 01-1/02-2 | COVERED | 外部S3和29行断言补强 |

无MISSING项。排除项：CONTEXT deferred中的Phase01/02历史judgment、生产webhook联调、发布和milestone归档。API opt-out均来自已锁定fixture范围，不新增产品决定。

## Dependencies与规划证据

| Plan | Needs | Creates | Wave | Checkpoint |
|---|---|---|---|---|
| 07.2-01 | 已有PG SQL、Trino目标、固定源码研究 | 483真实fixture及runner/client测试 | 1 | 无 |
| 07.2-02 | 01的runner与真实483证据 | 双版本restart/fault、完整nightly、CI索引 | 2 | 无 |

两个plan共享runner/tests，因此明确串行。校准查询factor=1、sample_count=0、confidence=low；估计分别36000和30000 tokens。Graphify未启用，不当作新证据；assumption-delta scan返回detected=false。未检测到本phase修改ORM schema，复用现有SQL实际初始化并查询其数据。未新增npm/pip/cargo package install任务。

当前树路径已由tracked查询核实；新Dockerfile/bucket.go/runner/tests尚不存在，不能用文件缺失充当RED。已有Iceberg模板使用local warehouse，nightly只有failure通知条件且无always cleanup，均待实施。旧验证命令中与本phase无关的cardinality测试不机械复制；Cargo nightly命令直接继承当前workflow。
