# Phase 7 API Coverage

日期：2026-10-29。detector输入为ROADMAP Phase 7全文加16份PLAN，revision1落盘后重新执行。初次detected=true的signals为wire/api、wire/rest、wire/endpoint，包含第一方wire文字的保守命中；以下按ADR-0019与既有Phase6/Phase7范围登记，不扩大MCP产品面。INTEGRATE为规划承接，运行验收全部pending。

| capability | decision | reason |
|---|---|---|
| REST ontology metadata | INTEGRATE | 07-03/05/10/11 |
| REST object PK read | INTEGRATE | 07-01/03/05/10/11 |
| REST object-set filter/search/searchAround/set operations | INTEGRATE | 07-03/05/10/11 |
| REST aggregate | INTEGRATE | 07-03/05/10/11 |
| REST Action submit/batch/validate-only | INTEGRATE | 07-02/04/05/10/11 |
| REST command result/replay/current authorization | INTEGRATE | 07-02/04/10/11 |
| REST object history/current masking/keyset | INTEGRATE | 07-02/04/10/11 |
| REST health | INTEGRATE | D-03，07-06/10/11/13 |
| OpenAPI 3.1 export/recursive schemas/errors | INTEGRATE | 07-05 |
| Rust full callable client | INTEGRATE | ADR-0009，07-10 |
| TypeScript generated callable lossless client | INTEGRATE | ADR-0009，07-11 |
| MCP stdio authenticated bridge | INTEGRATE | D-01，07-09 |
| MCP streamable HTTP/session ownership/cancel | INTEGRATE | 07-07/08 |
| MCP core describe/PK/query/aggregate | INTEGRATE | ADR-0019，07-07 |
| MCP full history/dynamic Actions/validate-only | INTEGRATE | ADR-0019，07-07 |
| MCP sql:raw execute_query(query) | INTEGRATE | ADR-0019，07-08/13 |
| MCP sql:raw list_catalogs() | INTEGRATE | ADR-0019，07-08/13 |
| MCP sql:raw list_schemas(catalog?) | INTEGRATE | 六工具08、可信部署默认值16、真实服务13 |
| MCP sql:raw list_tables(catalog?,schema?) | INTEGRATE | 六工具08、可信部署默认值16、真实服务13 |
| MCP sql:raw get_table_schema(table,catalog?,schema?) | INTEGRATE | s.t/c.s.t override，07-08/15/16/13 |
| MCP sql:raw explain_query(query,format?) | INTEGRATE | LOGICAL/DISTRIBUTED/VALIDATE/IO，07-08/15/13 |
| MCP L3/definition reload/roles/scope mutation | OPT-OUT | ADR-0019与MCP-08明确禁止agent管理能力，仅保留已有管理通道 |
| MCP additional mcp-trino tools outside approved six | OPT-OUT | ADR-0019限定六工具，外部参考不自动扩展产品范围 |
| stdio independent data host | OPT-OUT | D-01锁定已有serve认证桥接 |
| npm public publish/new Release/PR7 merge | OPT-OUT | 当前授权只实现与验证，PR7保持Draft OPEN；07-14配置仅响应将来明确授权release事件 |

证据入口：各PLAN的automated verify、07-VALIDATION.md、最终SUMMARY/VERIFICATION；不能用此矩阵取代真实PG/Trino、clients或三平台artifact执行。
