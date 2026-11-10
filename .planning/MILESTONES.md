# Milestones — ontoforge

## v1.1 — Gap Closure（2026-11-10）

**状态**：RELEASED。Phases 8-12 全部验收闭合，v1.1 公开发行完成。

### 发布记录

| 项 | 值 |
|---|---|
| v1.0 tag | `1bfa237`（annotated `df9281a`，历史 milestone 补推） |
| v1.1 tag | annotated `5936117` → 发布 commit；另有 `v1.1.0` 同指此 commit（release.yml 门禁要求 tag==v{semver}，v1.1 为 milestone 名、v1.1.0 为发布 machinery tag） |
| GitHub Release | https://github.com/easyinplay/ontoforge/releases/tag/v1.1.0 |
| npm 包 | `ontoforge@1.1.0`（unscoped，registry.npmjs.org；已验证 `npm install ontoforge@1.1.0` + import 正常） |
| Native 资产 | release.yml run 42833422911：3 构建 + verify + upload 全绿，9 资产（3 平台 × archive+sha256+manifest）已在 Release 可见 |

### 验收闭合

- REL-01/02（Phase 8 webhook E2E）：CI 真 PG 投递 + 人工签字 ACCEPTED
- REL-03（Phase 9 supervision）：CI harness+systemd 双证 + WSL2 真机 reboot/kill-9 人工签字 ACCEPTED
- REL-04（Phase 10 audit timing split）：`--test-threads=1` 移除，CI 并行全绿
- REL-05/06（Phase 11 wording/annotation）：私有 Trino 数据源措辞统一 + SUMMARY superseded 注解
- REL-07（Phase 12 release）：双 tag 远端可见、Release + notes、npm 1.1.0 可安装、本记录

**缺口状态**：无开放缺口。REL-01 至 REL-07 全部 Complete。

### 历史

- v1.0 MVP（Phases 1-7.x）：2026-11-08 shipped。`.planning/milestones/` 下有审计与 phase 归档。
