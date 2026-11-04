---
status: accepted
date: 2026-11-03
---

# Source drift 宿主准入与健康状态

## 决策

宿主执行启动及周期source drift检查，复用raw ontology engine和已有check_drift。部分逻辑target漂移/不可读时，仅拒绝该target实际source访问；其他target及无需source的完整patch/editOnly路径保留。核心engine可用且只有target降级时，匿名health返回HTTP200、status degraded；engine-wide失败、当前版本Pending或整轮CheckFailed时返回HTTP503。成功SELECT1不能清除drift检查失败。

source准入严格绑定请求固定的`(def_version, effective_profile)`及逻辑BindingTarget。无精确匹配报告为Pending并合并唤醒唯一monitor，不借用其他版本结果。保留`SOURCE_SCHEMA_DRIFT`、`SOURCE_UNREADABLE`，新增`SOURCE_CHECK_PENDING`和`SOURCE_CHECK_UNAVAILABLE`稳定错误码，拒绝source操作均映射HTTP503。内部可区分engine-wide/check失败，对外不泄露底层detail。

首次runtime engine/probe失败仍启动HTTP，以health503和周期检查支持恢复；结构配置或初始活动定义读取失败非零退出并清理。启动首次检查先于请求准入。单个task、30s周期、20s整轮预算；不新增公开配置、TTL、去抖、自动DDL/reset/绑定纠偏或dev锁。无source为notApplicable，不发metadata探测。

匿名health保留既有status/engineStatus/可选l2字段，不增加target名、物理名、版本或计数。失败操作通过已有安全错误信封给出稳定原因。复用现有10s关闭总预算和engine cancellation drain。

## 理由与授权

延续ADR-0015按目标降级“其余照常”、ADR-0021的drift分类与Phase7 D-03的核心故障503。用户2026-11-03授权同方向第二意见自动推进；Claude Code返回AGREE/AGREE_WITH_REFINEMENT，主会话接受上述合同及减少新增wire/配置面的补强，无实质方向分歧。

## 实施边界

技术方案及完整测试矩阵见`../phases/07-rest-mcp-shell/reviews/07-DRIFT-FIX-DESIGN.md`。只保留一个结果slot，exact-key与在途不可变view保证隔离，必要的短锁观察标记不构成多版本缓存。Action可在owned任务内部构造借用其captured host的ScopedEngine，不强制新增第二种GatedEngine。完整实现及真实Trino/PG验收仍待完成。

第二意见原始证据：`target/phase7-claude-contract-review.json`（exit0；Claude Code配置模型MiniMax-M3.1-Flash[1m]）。
