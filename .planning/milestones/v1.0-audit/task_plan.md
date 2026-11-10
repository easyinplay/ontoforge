# v1 milestone 审计与已知合同缺口收口

## Goal

独立核对79项v1需求的三源证据、跨阶段接线与当前验证状态；修复已确认的既有合同缺口，不新增产品范围。v1.0仅审计标识，不是版本发布授权。

## Tasks

- [x] 读取GSD audit/validate与历史phase状态，识别stale/Nyquist draft。
- [x] 撤回fa81ca0过早通过结论，恢复独立integration与三源审计。
- [x] Phase1 Nyquist：颜色环境行为补测，451定向GREEN；保留用户UAT观察边界。
- [x] L2-03/MCP-07自定义edit catalog硬排除：实际RED、可信配置接线、六工具拒绝/过滤GREEN、独立review及最终Phase7复验通过。
- [x] TLS依赖门：修复openssl-only与Cargo查询失败的fail-open，hermetic反例与真实依赖图/双平台CI通过。
- [x] 补执行现有feature测试（SQLite action_atomic 5例、PG source_guard 1例）和实际local Iceberg 483/484证据，保留非零发现。
- [x] ENGINE-05的MinIO精确合同缺口由Phase07.2独立验收9/9闭合；私有SQL Server历史judgment仍保留，详 ../../phases/07.2-minio-contract-gap-closure/07.2-VERIFICATION.md。
- [x] 汇总INTEGRATION.md、REQUIREMENTS-AUDIT.md与逐项矩阵，替换不充分的初稿评分。
- [x] STORE-05 source oneToOne报告缺口由Phase07.1独立验收18/18闭合，UAT10/10、最终workspace1525/0/119；不以PK/edit代替source报告。
- [x] 按实际缺口推进Phase2/3/5 Nyquist与Phase1–6历史验证重新核对；保留真实partial/human_needed，不强行全绿。
- [x] 对最终源码完成必需CI/受影响服务验证、恢复Phase7报告新鲜性，再生成最终milestone结论与状态。
- [x] 按用户新增授权，Phase03–07由PR8、Phase07.1由PR9合入main；Phase04/05堆叠链路已包含。

## 当前工作单元

**Status:** complete（Phase02原合同与后续闭合证据对账交付）；Phase01人工门仍human_needed，Phase02仍gaps_found，milestone仍为gaps_found。
验收真相源为VERIFICATION.md；最终结论与剩余合同/报告门见SUMMARY.md。无生产源码变更，证据计数器缺口已真实TDD并独立复检CLOSED。

## Next Step

03-01.1明确授权已按单项PASSED (override)落地，Phase03 native passed139/139。剩余Phase01/02报告门及ENGINE-05私有SQL Server/来源judgment见REQUIREMENTS-AUDIT.md；不自动豁免、发布、归档或进入新phase。

## 追加报告门收口

- [x] Phase03–04原合同逐项复核及报告门重新验收：03为human_needed138/139（缺03-01.1独立历史批准记录），04passed36/36；复核交付不等于03门闭合。
- [x] Phase05–06原合同逐项复核及报告门重新验收：05passed36/36、06passed22/22，外部环境边界保留。
- [x] Phase07.1原合同逐项复核及报告门重新验收：passed18/18，真实source OneToOne目标已核。
- [x] 独立核对覆盖集合、证据完整性，重算矩阵并更新当前结论：43 satisfied / 28 partial / 8 unsatisfied；freshness22/22与双轴review通过，Phase03人工门未闭合。

本追加单元仅处理已实现合同与报告门。跳过office-hours、CEO与brainstorming：无新产品范围、API设计或待选实现方案。纯验证与文档阶段不新增业务逻辑，TDD不适用；若发现需要业务修复，先返回证据与灰色决策，不自行实现。ralph-loop与Warp工具未提供，使用明确验收闭环和真实终端日志替代，不伪称调用。

## 03-01.1授权落地

- [x] 记录本轮明确授权并由独立verifier处理唯一历史证据项：138 VERIFIED + 1 PASSED (override)，345→350原覆盖全保留，旧human_needed报告及回执保持历史时点。
- [x] 独立查询native gate并真实重算矩阵：59 satisfied / 12 partial / 8 unsatisfied，--check PASS、freshness22/22；当前摘要及main-final-check.json记录本轮，不追改旧授权时点。

当前是明确决策的文档/验收处置，无新实现方案或生产修改，跳过brainstorming与业务TDD；沿用真实终端与逐项验收闭环，不伪称ralph-loop/Warp。


## Phase01 当前合同复核

- [x] 独立核对全部14份PLAN合同、原11项must-haves、最终源码与既有最终HEAD真实执行证据，保留原covered集合。
- [x] 仅在逐项证据复核完成后恢复报告新鲜性；三项judgment与已接受editor边界不自动豁免。
- [x] 查询canonical gate、重算需求矩阵并记录新时点验收，不覆盖先前106项历史索引。

这是既有合同复核，无产品范围或实现设计变更，跳过office-hours、CEO、brainstorming与业务TDD。ralph-loop/Warp未提供，使用真实终端与明确验收闭环，不声称调用。

## Phase02 当前合同复核

- [x] 按原10份PLAN、103条合同与当前源码复核Phase02，保留308项原covered；独立记录07.2对MinIO缺口的闭合证据，不将旧FAILED继续描述为当前缺陷。
- [x] 保留两项来源/CI judgment与ENGINE-05私有SQL Server未执行边界，不以“继续”作为override授权，不触达公司环境。
- [x] 独立查询报告门，真实重算矩阵与回归，保存新时点回执，不覆盖此前142项历史索引。

Phase02复核后的决议已落盘：MinIO/S3作为ENGINE-05已闭合子范围，整体ENGINE-05仍Partial。下一步只处理私有SQL Server、来源与公开CI隔离的人工证据门；不把继续推进视为override。

2026-11-06：用户撤回误发的default/composite API key请求，不进行81个API key的读取、重分组或测试。这是既有合同复核，无新产品或架构决策，跳过office-hours、CEO及brainstorming；纯文档验证不新增业务逻辑，TDD不适用。ralph-loop/Warp未提供，沿用真实终端与明确验收闭环，不伪称调用。

## 延续约束

- 用户2026-11-04已授权完成的开发任务合并main，覆盖旧PR7/8不合并限制。合并前核对scope、验收、CI与依赖顺序；不创建Release、不发布npm或自动归档。
- 不改旧报告指纹来冒充重新验证，不将unknown/未运行改列manual-only取得绿色。
- 自动推进依据用户授权：向Claude Code只给事实、需求和代码，取得其独立方案再比较；同向或方向一致的组合自动推进，实质分歧才请用户裁决。
- Warp/ralph-loop未提供；使用明确验收闭环但不声称调用它们。

## Decisions

- L2修复为既有合同内的bounded修复；可信deployment config注入McpL2Tools并统一SQL与metadata排除。全面迁移到另一service会扩大改动，暂不采用。
- 用户授权覆盖brainstorming的重复确认要求；主会话已明确方案且CC AGREE，继续TDD。office-hours/CEO跳过：这是已定义合同的bug修复。
