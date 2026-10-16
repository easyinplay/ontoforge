# ADR-0021 数据源绑定文件、必填 backing 与引擎决策

- status: accepted
- date: 2026-10-16
- decider: Claude（按用户「决策自主」偏好落地 discuss-phase 已锁定的决策与 research 推荐）
- 取代：ADR-0015 中「object type 增加可选 source」一条
- 修订：ADR-0011（新增 crate `ontoforge-engine`）、ADR-0012（新增字段 `backing`、新增 kind `SourceBinding`、manifest 新增 `defaultProfile`）
- 依据：`.planning/phases/02-source-binding-engine/02-CONTEXT.md` D-01..D-15、`02-RESEARCH.md` Open Questions 1-8、`02-REVIEWS.md` E1 / E3 / E12 / E20

## Context
ADR-0015 把物理绑定写成 object type 上的可选内联 `source`。这使定义文件携带 catalog / schema / 表名，开源示例会带上公司命名，定义与部署环境耦合；同一份定义在 dev（SQLite 替身）与 prod（Trino）之间无法切换绑定。Phase 2 的 discuss-phase 锁定了独立绑定文件（D-01）、显式 `backing`（D-02）与引擎相关决策（D-03..D-15），research 对 8 个 open question 给出推荐，orchestrator 已全部采纳。本 ADR 把这些决定一次落盘，供 Phase 2 的 10 个 plan 与后续 phase 引用。

## Decision
1. **绑定文件。** 绑定写在独立的 `kind: SourceBinding` 文档中，只能放在 `bindings/<profile>/` 下，profile 由目录推出、名字限制为 `[a-z][a-z0-9_-]{0,31}`，只作已加载 map 的 key、绝不拼入文件系统路径；一个绑定只指向单表或 view，不接受 SQL 片段（D-01、D-04）。文档字段：`target`（`{ objectType: X }` 或 `{ linkType: y }`，只允许 manyToMany link）、`source`（`catalog`、`schema`、`table`）、`columns`（按属性 apiName 或 link 的 `from` / `to` 为 key，值为 `{ column?, timezone?, collation? }`，未列出的属性按同名列匹配，D-03）、`fixture`（`{ file }`，相对绑定文件所在目录，仅 `.csv` / `.json`）。
2. **backing。** `backing: source | editOnly` 在 ObjectType 上必填，在 manyToMany LinkType 上必填、其他 cardinality 禁止。缺失一律是 ONT-V05（link 上的非法出现是 ONT-V11），绝不默认成任何一个值。这是对 ADR-0012「后续字段为向后兼容扩展」惯例的有意例外：可选默认 `editOnly` 无法拦住「同时漏写 backing 与绑定」，部署方会以为在读企业数据，实际读到空集（D-02、open question 5）。`source` 但选定 profile 无绑定、`editOnly` 却有绑定，由后续 plan 的绑定完备性检查报 ONT-V16。
3. **profile 选定。** manifest 新字段 `defaultProfile`；选定顺序 `--profile` > 环境变量 `ONTOFORGE_PROFILE` > `defaultProfile`（D-05）。示例缺省 profile `dev` 指向 SQLite 替身。
4. **decimal。** `precision` / `scale` 缺省 38 / 10，两者各自独立取缺省；合法范围 1 ≤ precision ≤ 38、0 ≤ scale ≤ precision（ONT-E13）。源列 decimal(p,s) 与声明 decimal(P,S) 兼容当且仅当 `s ≤ S` 且 `p - s ≤ P - S`；兼容且 `p < P` 或 `s < S` 时报 W；否则 E。该规则取代 D-13「精度与小数位不超过声明值」的字面读法，理由：decimal(10,2) 放进 decimal(10,5) 会溢出整数位（open question 6）。
5. **timezone。** source-backed 对象的每个 timestamp 属性必须在绑定 `columns` 条目中声明 `timezone`（IANA 名或 `aware`），缺失为 ONT-E14；在线复核时声明 IANA 而源列带时区报 W，声明 `aware` 而源列无时区报 E（open question 1）。
6. **fixture 语义。** CSV 空单元格对所有类型（含 String）都是 NULL；JSON fixture 用真 `null`；CSV 中的 array 单元格写 JSON 数组文本（open question 8）。
7. **`validate --online` 的引擎选择。** 未给 `--server-config` 时，若选定 profile 的绑定全部带 `fixture` 则用内存 SQLite 替身，否则报错并提示 `--server-config`；给了服务配置时以配置为准（`[profiles.<name>] engine = "trino" | "sqlite"`，`sqlite` 即 fixture 驱动的内存替身，不提供指向现成 SQLite 库文件的目录配置项，eng review E1），配置映射到 Trino 而绑定带 `fixture` 时给 W（open question 2）。本 phase 不做服务配置的隐式查找。
8. **整批返回。** `Engine` 整批返回，client 内强制 `max_rows` / `max_bytes`，命中时 `DELETE nextUri` 并置 `truncated`；流式接口不在本 phase（open question 3）。
9. **engine crate。** 新 crate `ontoforge-engine`（修订 ADR-0011）：依赖 `ontoforge-schema`，不依赖 `ontoforge-query`；SQLite 替身用 `sqlx`（`rusqlite` 与 `sqlx` 的 `libsqlite3-sys` links 冲突，RESEARCH Pitfall 1）。
10. **服务配置。** `ontoforge-server.toml` 只经 `--server-config` 指定；密钥只以 `*_env` / `*_file` 引用；不提供跳过证书校验的开关；时长字段以毫秒整数表示（D-06..D-10）。
11. **诊断码分配**（码永不复用，D-14）：
    - ONT-V16 绑定定义与完备性（含 profile 名、fixture 引用与 fixture 内容）；
    - ONT-V17 在线绑定不匹配（E 或 W）；
    - ONT-V18 `--check-pk` 发现的重复或 NULL 主键；
    - ONT-V19 因引擎侧原因无法完成在线校验（不可用、认证失败、超时、协议或查询错误）；
    - ONT-V20 服务配置错误；
    - lint 新规则：ONT-E13（decimal 精度）、ONT-E14（timestamp timezone）、ONT-E15（collation 只用于 string）、ONT-E16（geopoint 不支持 source 绑定）、ONT-W17（fixture 缺少绑定列，D-03 的离线部分；fixture 表头无法读取时同一规则给出「无法离线检查」的定位提示，不静默跳过，E20）。E10-E12、W12-W16 已被 foundry spec 规则表占用，故从 E13 / W17 起编号。
12. **退出码。** 0 无 error；1 有 error（含绑定不匹配、重复主键、配置错误）；2 命令行用法错误（clap）；3 引擎侧原因导致在线校验无法完成（不可用、认证失败、超时、协议或查询错误，ONT-V19，与绑定错误区分，D-14）；101 内部错误。不可用与认证失败中止整轮；单个绑定或单个 catalog 的查询类错误只让对应绑定得到 ONT-V19、其余绑定照常比对，只要出现任一 ONT-V19 退出码即为 3（E3）。
13. **ENGINE-02 的完成口径。** 本 phase 交付 `EngineError::Unavailable` 分类、`Engine::status()` 与测试；`/health` 与 describe 暴露属 Phase 7（API-01）（open question 7）。drift 在本 phase 交付库函数与 healthy / degraded 状态模型（降级原因 `SOURCE_SCHEMA_DRIFT` 与单个目标元数据不可读的 `SOURCE_UNREADABLE`，E3；healthy 保留精度 / 时区 warning，E12），启动 / 定时执行在 Phase 7（D-15）。

## Rejected
- **内联 `source`**（ADR-0015 原文）：定义与部署配置耦合，开源示例会带公司表名，dev / prod 无法切换绑定。
- **backing 可选、缺省 editOnly**：漏配静默变成没有数据的 edit-only 对象。
- **`rusqlite`**：与 `sqlx` 的 `libsqlite3-sys` links 冲突（RESEARCH Pitfall 1）。
- **D-13 字面 decimal 读法**（精度与小数位各自不超过声明值）：decimal(10,2) 放进 decimal(10,5) 会溢出整数位。
- **服务配置中指向现成 SQLite 库目录的 `sqlite_dir`**：无成功标准或锁定决策需要它，且引入只读打开与路径拼接风险；Phase 7 `serve --dev` 时再评估（E1）。

## Consequences
- Phase 1 fixture 全量迁移（02-01）：所有 ObjectType 与 manyToMany LinkType 文档补 `backing`，aviation 示例中 Airport 为 editOnly，其余 object type 与 flightCrew 为 source。
- 新增 invalid 样例 `v05-missing-backing`、`v11-backing-on-one-to-many` 与快照；`schemas/ontoforge-definition.schema.json` 重新导出。
- 后续 phase 的读路径按 `backing` 与选定 profile 选表；ADR-0015 的类型契约、主键契约与 drift 条款仍有效，只有「内联 source」一条被本 ADR 取代。
