# Phase 12 Second Opinion Request — First Public Release shape

你是独立 reviewer。请只基于下列需求、现状、约束与已验证事实，独立给出发布方案；不要猜我的偏好。不要联网。

## 需求（REL-07 逐字验收）
1. v1.0 与 v1.1 两个 tag 推送到远端且在远端仓库可见
2. 存在 v1.1 的 GitHub Release 且附带 release notes
3. npm registry 上该包已发布，v1.1 版本可被安装
4. `.planning/MILESTONES.md` 记录 tag、Release 链接与 npm 包版本，且不再声称存在开放缺口

## 现状（已验证事实）
- Rust workspace `ontoforge`（自托管 ontology ops layer，Foundry-like），main HEAD `12e5e4f`。
- workspace `[workspace.package] version = "0.1.0"`，rust-version 1.98。
- `v1.0` tag 已存在于本地，指向 `1bfa237`（v1.0 milestone merge commit）；远端目前**没有任何 tag**。
- 可发布 npm 候选：`clients/typescript/package.json` —
  `"name": "@ontoforge/typescript-client"`、`"private": true`、**无 version/license/main/types/exports/files 字段**；src 为 openapi-fetch 生成型 client（index.ts 导出 codec + generated types），scripts 有 build（tsc → dist/）与 test:contract；devDeps typescript 7.0.2，deps openapi-fetch 0.17.0。
- 另一 package.json `tools/openapi-gen`（内部 codegen，`private: true`）不应发布。
- 仓库根**没有 LICENSE 文件**、没有 README.md（仅 CONTRIBUTING.md）；Cargo.toml 无 license 字段。
- `.github/workflows/release.yml`：在 `release: published` 事件上为 3 个平台构建 native archive（ubuntu x86_64 / macos aarch64 / windows x86_64-msvc）+ smoke 测试 + 上传产物到 release assets；PR 上同样跑构建但不传 assets。
- 用户当前 `npm whoami` = 未登录；`@ontoforge` npm scope 是否已被用户注册未知。
- 项目宣称 "open-source self-hosted"。

## 请回答
1. **tag 语义**：annotated vs lightweight？v1.1 应打在 main 哪个 commit 上（含发布准备的 commit 之后还是先打 tag 再补包元数据）？v1.0 远端缺失是补推还是重打？
2. **npm 包形状**：包名取舍（scope `@ontoforge/typescript-client` 需 org 存在 + `--access public`；未注册 scope 会 404 — 备选 `ontoforge-client` unscoped？），version 取 1.1.0 还是与 crate 0.1.0 对齐，入口字段（main/types/exports/files/prepublishOnly）最小正确集，dist-only vs src+dist。
3. **LICENSE**：缺 LICENSE 对 "open-source" 发布是不是必须先补？选 MIT / Apache-2.0 / 双许可的理由；Cargo.toml `[workspace.package] license` 是否同步。
4. **Release notes**：v1.1 notes 应包含什么结构（milestone 摘要 / 从 v1.0 起变更 / 安装方式 / 证据指针）？
5. **顺序**：tag → release → npm → MILESTONES 的正确次序与每步的可回滚性。

请给出定稿方案 + 主要风险（尤其是 npm 首次发布的不可逆坑）。
