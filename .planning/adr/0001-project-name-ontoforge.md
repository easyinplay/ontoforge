# ADR-0001 项目名定为 ontoforge

- status: accepted
- date: 2026-10-15
- decider: 用户

## Context
原工作名 FoundryOntology 中的 "Foundry" 是 Palantir 产品名，对外发布有商标风险，也易被误认为官方项目；crates.io 名称发布后不可回收。

## Decision
- 品牌名 / CLI 名：`ontoforge`
- crate 前缀：`ontoforge-`（crates.io 于 2026-10-15 核查未占用）
- GitHub 仓库：`easyinplay/ontoforge`（已由 `FoundryOntology` 改名）
- 本地目录：`D:\DataSpellProjects\ontoforge`

## Consequences
新代码、文档中不再出现 FoundryOntology、`fo-` 占位前缀；`docs/foundry/` 下的 Palantir 原文镜像保持原样。
