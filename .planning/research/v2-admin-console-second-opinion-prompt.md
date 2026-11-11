# v2 Admin Console — Second Opinion Request

你是独立 reviewer。只基于下列需求、现状、约束与已验证事实独立提出方案，不要猜我的偏好、不要联网。

## 需求（用户原意）

为 ontoforge 补齐前台管理页面，属「重」档全功能 admin console，两块核心：
1. **Ontology 管理**：object/link/property/action/role 定义的浏览与（受控）编辑管理
2. **数据源配置管理**：PostgreSQL 数据库连接、Trino 集群/catalog 等 source binding 的配置界面
3. 顺带运维面：edit log / audit 审计浏览、webhook outbox 状态、action 监控

ontology 是 AI 在用，但必须有管理页面——用户明确这是必须补齐的方向，非可选加分。

## 已验证事实（勿重复调研大面）

**项目**：Rust workspace `ontoforge`（自托管 ontology ops layer，类 Foundry Ontology 核心），v1.1 已发布（tags/Release/npm ontoforge@1.1.0）。crate：ontoforge(binary/server/CLI)、action、engine(Trino)、storage(SQLite/PG edit store)、query、auth、api、schema、expr、server、client。

**API 面现状（量化缺口）**：
- `docs/openapi.json` 仅 **8 条路由**（7 条 /api/* + /health）：覆盖 object query/list/get、action submit/validate/history、定义读取。
- **零 admin 端点**：无 ontology 定义 CRUD API、无 source config API、无 audit/outbox 查询 API、无 user/role 管理 API。
- 数据源配置今天**只在 TOML 文件**（`[store]`/`[webhook]`/source bindings），`url_env`/`url_file` 凭据引用模式——server 进程读配置，无运行时管理接口。
- 认证：scope model + token codec（dev token 于 --dev）；授权：角色+scope，Action 控制写路径。
- `clients/typescript`：已发布 `ontoforge@1.1.0`（ESM, openapi-fetch 生成型 client + codec）。
- OpenAPI 由 utoipa 生成——新端点自动入 spec/SDK codegen 链路。

**参考产品形态（立项时调研）**：
- Palantir Foundry：Ontology Manager（本体定义管理）、Data Connection（数据源/credential 管理）、Developer Console（应用注册/SDK）、Object Explorer/Workshop（对象数据浏览）——前端应用层。
- bkn-foundry（openbkn-ai）：后端独立，UI 在独立 repo `bkn-studio`——studio 形态（ontology admin + datasource config）。
- open-foundry（syzygyhack）：TS monorepo 全栈，api-gateway+governance+domain-packs，spec 重实现轻。
- v1.0 REQUIREMENTS Out-of-Scope 明确排除过"前端产品"，当时定位是 ontology 核心先立住——现在用户明确要补管理面，进入 v2 范畴。

**约束**：
- 写入纪律：所有写必经 Action 校验授权——admin UI 对 ontology/数据源配置的任何修改也必须走受控写路径（不能 UI 直连 DB 写）。
- 凭据纪律：数据源密码不落前端/不落 API 响应明文（现有 url_env/url_file 引用模式的语义要保持）。
- self-hosted/open-source 定位——front-end 选型需离线可建、不依赖重 SaaS。
- 用户环境主力 Windows + WSL/Linux；CI ubuntu。

## 请回答

1. **形态**：独立前端 repo 模式（bkn-studio）vs monorepo 内新增前端包（如 `clients/web` 或 `apps/console`）？给出选择 + 理由（发布粒度、API 版本协调、贡献者门槛）。
2. **技术选型**：前端框架与构建（React+Vite / SvelteKit / Vue+Nuxt / 或更轻的 htmx+服务端模板）、组件库（自带 vs 第三方）、状态与数据层（基于已发布的 TS client SDK 还是直接 openapi-fetch）、路由与认证形态（SPA+token / BFF / 服务端会话）。
3. **后端扩展边界**：admin API 域怎么切（ontology-def / source-config / audit / ops / user-role 五域？）、数据源配置落 DB 还是继续 TOML+reloader、凭据永远不来回传的 UI 形态（掩码回显？写后即焚?）、auth 端点要不要扩（dev-token → real principal 登录）。
4. **分期**：这个「重」档 console 合理的 phase 切分（最小可用面 → 完整面），每期验收标准建议。
5. **主要风险**：你能想到的前三位（API 未冻结改 UI 返工 / auth 模型撑不起 UI 登录 / 数据源凭据 UI 安全 / 前端技术债）。

输出定稿方案 + 风险清单。