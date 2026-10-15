---
status: testing
phase: 01-workspace-schema-lint
source: [01-VERIFICATION.md]
started: 2026-10-15T17:44:12Z
updated: 2026-10-15T17:44:12Z
---

## Current Test

number: 1
name: miette 彩色诊断在 Windows Terminal 中可读
expected: |
  在 Windows Terminal 运行 `cargo run -q -p ontoforge -- validate examples/ontologies/invalid/v04-unknown-field`，
  看到带颜色的错误头 `ONT-V04`、源码片段、指向 `primaryKy` 的下划线与 "did you mean `primaryKey`?" 提示，排版无乱码。
awaiting: user response

## Tests

### 1. miette 彩色诊断在 Windows Terminal 中可读
expected: 运行上面的命令，颜色、源码片段、下划线位置与 help 提示都正确显示，无乱码或错位
result: [pending]

### 2. VS Code 基于导出 schema 的 YAML 补全
expected: 安装 Red Hat YAML 扩展后打开 `examples/ontologies/aviation/objects.yaml`（首行有 yaml-language-server 的 $schema 注释），输入 `kind: ` 时提示 ObjectType / LinkType / ActionType，按 kind 收窄后续字段补全，未知字段被标红
result: [pending]

### 3. GitHub Actions CI 在 windows-latest 与 ubuntu-latest 上全绿
expected: push 分支后 `.github/workflows/ci.yml` 两个平台的 fmt / clippy / test 全部通过，包括 Unix 下的 symlink 与非 UTF-8 文件名测试
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
