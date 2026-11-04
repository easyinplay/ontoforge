# Phase 03–07 合并记录

2026-11-04用户授权：已完成并通过验收的开发任务应合并main，替代此前PR7/8保持Draft的限制。不包含Release/npm发布，也不声明整个v1.0 milestone完成。

## 交付范围

复用PR8，base改为main，整体纳入Phase03–07。严格祖先链为main 948ed0f → Phase03 c57165d → Phase04 9c4ba02 → Phase05 bb9aeb6 → Phase06 2c9eee6 → Phase07 f8851ca → 文档复签a6efc11。采用merge commit保留身份，PR6/7关联提交均被包含。Phase07.1实现不在本次提交范围。

f885至a6仅.planning文档变更。INT-01修复54a47dd和TLS守卫修复ef7ac92均为f885祖先，避免将已知修复前的中间快照作为main的交付结果。

## 验收与证据

- 源码基线f885：CI 41759804733的Windows、Ubuntu、integration、phase7-real-services四项成功；release 41759804768的三个平台和aggregate成功，publish skipped。
- 独立历史复签：Phase03 8/8、04 36/36、05 36/36、06 22/22、07 80/80 passed。原始分阶段审查与复签边界见本目录PHASE-*-REVERIFY.md。
- 在a6独立worktree中，原生verification.status返回passed，phase uat-passed 07 --require-verification返回passed=true、14/14、blockers=[]。SECURITY threats_open=0。
- 首次独立checkout缺失58份target日志，原生返回stale。独立只读审查确认237项覆盖输入的原始digest可用原件恢复；逐一SHA256校验复制后原生通过。未修改VERIFICATION或其fingerprint。mtime的moved_files提示不是内容差异证明。
- 58份原件现归档为evidence/phase07-original-evidence.tar.gz；索引phase07-original-evidence.json保存相对路径、长度和SHA256，归档后逐项读取核对全部一致。需要重放原生门时，在仓库根执行以下命令恢复同路径证据：

```powershell
tar -xzf .planning/milestones/v1.0-audit/evidence/phase07-original-evidence.tar.gz
node C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs query verification.status .planning/phases/07-rest-mcp-shell
```

## 保留边界

STORE-05 source报告由未完成Phase07.1负责；ENGINE-05精确MinIO合同仍未闭环；milestone仍gaps_found。旧配置错误回显为03-VERIFICATION中的historical advisory，未声称修复；旧TLS critical已由后继实现和CI关闭。新提交CI通过前不执行merge，实际merge SHA与PR状态由交付后记录补充。
