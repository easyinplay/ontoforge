# 同类方案调研

## 结论

同类工程通常把私有数据源验证放在受控的 self-hosted runner 或 private network runner 中，同时保留测试输入、环境、结果和清理证据。公开 CI 的成功只能证明公开环境路径，不能证明私有 SQL Server 或企业数据来源。

## 依据

- GitHub 官方文档说明 self-hosted runner 可使用公司维护的硬件和本地网络服务，责任方需要维护运行环境：[Self-hosted runners](https://docs.github.com/en/actions/concepts/runners/self-hosted-runners)。
- GitHub 官方文档说明 hosted runner 也可以通过 private networking 访问 on-premises 服务，但需要明确的网络与身份配置：[Private networking with GitHub-hosted runners](https://docs.github.com/en/actions/concepts/runners/private-networking)。
- Google Cloud 官方测试样例建议：外部资源通过环境变量注入，测试自行创建所需数据并清理，而不是把外部数据放进公开 fixture：[Testing with external resources](https://github.com/GoogleCloudPlatform/python-docs-samples/blob/main/AUTHORING_GUIDE.md)。
- 软件验证指导要求保留测试步骤、输入数据和测试结果，外部现场测试属于独立验证边界：[FDA General Principles of Software Validation](https://www.fda.gov/media/73141/download?attachment=)。

## 对当前审计的适用结论

Phase07.2 的 MinIO/S3 证据满足公开可复验的独立子范围闭合条件。ENGINE-05 的私有 SQL Server fixture、数据来源和公开 CI 隔离仍需要私有环境测试记录或需求负责人明确的人工 resolution。继续刷新公开 CI 或报告 fingerprint 不能替代这两类证据。

本调研没有改变产品代码、需求定义或报告评分。
