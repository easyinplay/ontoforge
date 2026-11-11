# Second Opinion — Admin Console 边界与形态

你是独立 reviewer。只基于下列需求、现状、约束与已验证事实独立提出方案，不要猜我的偏好、不要联网。

## 背景需求

为 Rust 自托管 ontology ops layer（ontoforge，类 Foundry Ontology 核心，v1.1 已发布）补一个**全功能 admin console**，两块核心：ontology 管理 + 数据源配置（PG/Trino 等）。用户同时在问两个具体问题：

**Q1**: 前端管理页面应不应该实现后端配置管理功能？如果要，边界画在哪里（哪些配置可 UI 管、哪些必须留 TOML/ops）？
**Q2**: 前端形态应该像 bkn-foundry 那样拆独立 repo（其 `bkn-studio` 仓），还是同仓（monorepo）？若同仓，"不耦合"的正确边界是什么？

## 已验证事实（勿重复调研）

**现状（量化）**：
- API 面仅 8 条路由（POST /api/actions/{action}、POST /api/aggregate、GET /api/commands/{id}、GET /api/objects/{type}/{pk}[+/history]、GET /api/ontology、POST /api/query、GET /health）——**零 admin 端点**。
- 数据源配置今天**只在 TOML 文件**（`[store]`/`[webhook]`/source bindings），`url_env`/`url_file` 凭据引用模式——server 读配置，无运行时管理接口。
- 登录只有 dev-token（--dev 模式）；无真实 principal 登录、无会话。
- `clients/typescript` 已发布 `ontoforge@1.1.0`（openapi-fetch + codec，纯 API client 先例）。
- workspace 是多 crate Rust 项目，Apache-2.0，self-hosted/open-source 定位。

**铁律**：
- 所有写必经 Action 校验授权（不能 UI 直连 DB）。
- 凭据永不明文回传（现有 url_env/url_file 语义保持）。
- self-hosted——离线可建、不依赖 SaaS。

**对标事实**：
- Palantir Foundry 有独立前端应用层（Ontology Manager/Data Connection/Developer Console）——但被其 v1.0 Out-of-Scope 明确排除过前端（"不是 ontology 核心"）。
- bkn-foundry 后端独立、UI 在独立 repo `bkn-studio`。

## 请回答

1. **Q1 边界**：后端配置分哪些层，各层 UI 权限应如何（哪些必须 TOML-only、哪些可受管、哪些永不进 UI）？TOML↔DB 双真相源问题怎么解（若可管）？desired/applied 运行时状态如何呈现？webhook target 这类受信配置算不算受管？凭据的 UI 形态（掩码框 vs 引用选择器 vs 其他）？
2. **Q2 形态**：拆 repo vs 同仓的判断依据是什么（发布粒度/契约变更频率/团队/多后端）？若同仓 `apps/console`，"不耦合"的硬性约束有哪些（前端只消费 API 类型？契约测试怎么挂？）？部署形态（静态资产同源 vs 独立前端服务）对认证与安全面各意味着什么？
3. **风险**：这两个决策方向上各自的前两位风险。

给出定稿判断 + 理由，不要折中糊弄。