# Phase 07.2 本地证据归档

`local-evidence.tar.gz` 保存已冻结的本地公开证据，`local-evidence-index.json` 记录每文件路径、字节数、SHA256、归档SHA256与来源边界。已实际逐成员重算并核对全部索引，拒绝绝对路径、路径穿越、重复成员及symlink。

保留最终49/49 nightly、此前3次nightly失败、483/484 restart、3次真实故障、helper强杀恢复、TDD/SDK与固定构建日志。`scan_passed=false`或缺少明确扫描证明的目录仅收安全metadata。早期指定成功run的扫描依据见`07.2-RUNTIME-EVIDENCE.md`，索引保留具体来源策略。

恢复前先核对归档SHA256与索引。仅在新的空目录解压，成员路径保持仓库根目录相对路径：

```powershell
Get-FileHash .\local-evidence.tar.gz -Algorithm SHA256
New-Item -ItemType Directory -Path .\restored-evidence
tar -xzf .\local-evidence.tar.gz -C .\restored-evidence
```

解压后按索引逐文件核对SHA256及大小，再用于证据查阅。归档不含私有env/state、binary/image或运行中的`ci/`；CI另行归档。本归档只证明证据完整性，不代替独立验收，也不关闭ENGINE-05。

## CI 证据归档

`ci-evidence.tar.gz` 与 `ci-evidence-index.json` 单独保存同一提交 `b7e44fa934b56c9d68620a4ea103d68ee2fb3a92` 的三个成功终态：nightly `41833307824`、CI `41833241053`、release `41833240915`。归档包含3份run JSON、10份成功job日志、nightly核验JSON与核验脚本，共15个文件；skipped job以run JSON原始记录保留。未收录auth状态、watch日志或私有材料。

全部成员已逐项核对SHA256和大小，并实际恢复到临时空目录后再次核对；已拒绝绝对路径、路径穿越、symlink及重复成员。恢复时先核对CI归档SHA256，再在新的空目录执行：

```powershell
Get-FileHash .\ci-evidence.tar.gz -Algorithm SHA256
New-Item -ItemType Directory -Path .\restored-ci-evidence
tar -xzf .\ci-evidence.tar.gz -C .\restored-ci-evidence
```

成员保留仓库根目录相对路径，恢复后按CI索引再次核对。此归档不替代独立phase验收；release workflow成功不表示执行了发布，上传release资产job的原始状态为skipped。
