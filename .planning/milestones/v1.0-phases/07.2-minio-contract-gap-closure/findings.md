# Findings

- 既有合同和供应链核对复用 `../../milestones/v1.0-audit/MINIO-FEASIBILITY.md` 与 `target/minio-phase-preparation.json`，不重复调查。
- office-hours/CEO跳过：既有ENGINE-05合同缺口。主会话brainstorming处理可靠性/并发清理设计；用户已授权独立第二意见同向即自动推进。
- 当前nightly使用local warehouse且report-failure会发issue；必须保证手动验收不发送外部消息。
- 核心runner的生命周期/环境guard/cleanup必须TDD；纯Dockerfile配置和文档不适合逐行测试，以真实构建运行及配置检查验证，记录此TDD跳过范围。
- `.codegraph/`不存在，使用rg及直接文件读取。
- 首次执行预条件失败：WSL的docker是Desktop占位提示且cargo不存在。此前真实服务脚本通过Git Bash/Windows Cargo运行；现ci/query-services.sh有cygpath和MSYS_NO_PATHCONV模式，ci/test_tls_deps.py有显式Git Bash选择。不得以启用WSL integration为由重启用户Docker daemon。
- Windows私有性定义经真实ACL与独立第二意见锁定：Owner=current SID、protected DACL、可信SID为current/SYSTEM/Administrators/OWNER RIGHTS（后者以Owner正确为前提），拒绝其他授权ACE。修正原仅允许current SID的过严校验；测试必须覆盖实际子文件与异常ACE，避免Set-Acl触发SACL权限。
- 首轮483真实source/copied=23，原因是runner只运行seed.sql。历史29来自ci/query-services.sh seed中的query_corpus.sql固定Q-0001..Q-0006，并非Action副作用；应复用seed.sql→rowpred_corpus.sql→query_corpus.sql原顺序，不改29断言。
- 首轮trino.log命中临时秘密，扫描已拒绝公开证据；cleanup Docker filter漏label=为明确实现bug，executor修复并核对owner资源归零，不能将首轮判pass。
- 日志第二意见采用源头修复：Trino通过私有env的AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY获取默认SDK凭据，不写catalog静态S3凭据键；483/484官方S3FileSystemLoader已核对provider分支，实际鉴权/全量日志扫描仍待运行。保留严格扫描和真实原始日志边界。
- 新增测试必须进入持续验证：nightly显式RunnerContract/NightlyContract和helper-test，避免仅本地测试后无人运行。Linux唯一平台例外为Windows ACE拒绝测试（已有Windows真GREEN），跨平台权限正例及其他目标仍必须执行；不默认发现LiveFault。
