# Phase 07: REST & MCP Shell - Pattern Map

**Mapped:** 2026-10-29
**范围:** CONTEXT、RESEARCH、ENG-REVIEW 的完整 Phase 07；下表 52 个建议文件。
**状态:** 仅模式映射，业务尚未实施。新增模块拆分与 TS package 名为 planner 可调整的文件组织建议，不是新增产品决策。

## File Classification

`modify` 表示现有 tracked 文件；`new` 表示待创建。A-E 是下文已读取并经 `git ls-files -- <path>` 验证的主要 analog。`partial` 仅借鉴数据流或边界，不能当现成功能；`none` 须使用 RESEARCH 的已核验库接口。表中每行一个文件，测试文件名同样是建议。

| New/Modified File | 状态 | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|---|
| `Cargo.toml` | modify | config | batch | 自身 workspace dependencies/lints | exact |
| `Cargo.lock` | modify | config | batch | Cargo 解析生成 | none |
| `crates/ontoforge-storage/src/lib.rs` | modify | config | transform | 自身模块注册 | exact |
| `crates/ontoforge-storage/src/request_snapshot.rs` | new | service | request-response | A | exact |
| `crates/ontoforge-storage/src/reload.rs` | modify if needed | service | CRUD | A | exact |
| `crates/ontoforge-action/src/lib.rs` | modify | service | CRUD | 自身 execute/result wrapper；A 的定义固定边界 | partial |
| `crates/ontoforge-storage/src/history.rs` | modify | service | request-response | 自身 history wrapper；A 的定义固定边界 | partial |
| `crates/ontoforge-api/Cargo.toml` | modify | config | batch | 自身 workspace 继承 | exact |
| `crates/ontoforge-api/src/lib.rs` | modify | model | transform | D | role-match |
| `crates/ontoforge-api/src/dto.rs` | new | model | transform | D | exact |
| `crates/ontoforge-api/src/error.rs` | new | model | transform | B 的类型化错误 | partial |
| `crates/ontoforge-api/src/openapi.rs` | new | utility | transform | 无 utoipa analog | none |
| `crates/ontoforge-server/Cargo.toml` | modify | config | batch | 自身 workspace 继承 | exact |
| `crates/ontoforge-server/src/lib.rs` | modify | provider | request-response | C 的显式宿主所有权 | partial |
| `crates/ontoforge-server/src/state.rs` | new | provider | request-response | C | partial |
| `crates/ontoforge-server/src/context.rs` | new | middleware | request-response | B + A | partial |
| `crates/ontoforge-server/src/rest.rs` | new | controller | request-response | 无 axum handler analog | none |
| `crates/ontoforge-server/src/error.rs` | new | utility | transform | B 的错误分类 | partial |
| `crates/ontoforge-server/src/mcp.rs` | new | controller | streaming | 无 tracked rmcp analog | none |
| `crates/ontoforge-server/src/lifecycle.rs` | new | service | event-driven | C | role-match |
| `crates/ontoforge-server/src/health.rs` | new | controller | request-response | C 的 monitor 状态生命周期 | partial |
| `crates/ontoforge-client/Cargo.toml` | modify | config | batch | 自身 workspace 继承 | exact |
| `crates/ontoforge-client/src/lib.rs` | modify | service | request-response | 无平台 REST client analog | none |
| `crates/ontoforge/Cargo.toml` | modify | config | batch | 自身 workspace 继承 | exact |
| `crates/ontoforge/src/lib.rs` | modify | route | request-response | 自身 Command dispatch | exact |
| `crates/ontoforge/src/cli.rs` | modify | config | request-response | 自身 clap Command | exact |
| `crates/ontoforge/src/serve.rs` | new | service | event-driven | C | partial |
| `crates/ontoforge/src/mcp.rs` | new | service | streaming | 无 tracked stdio bridge analog | none |
| `crates/ontoforge/src/dev.rs` | new | service | file-I/O | E 的持久临时目录/进程生命周期 | partial |
| `crates/ontoforge-storage/tests/request_snapshot.rs` | new | test | CRUD | E 的真实 store fixture | role-match |
| `crates/ontoforge-action/tests/frozen_definition.rs` | new | test | CRUD | E 的真实 Action fixture | role-match |
| `crates/ontoforge-api/tests/openapi_snapshot.rs` | new | test | transform | 无 OpenAPI snapshot analog | none |
| `crates/ontoforge-api/tests/wire_roundtrip.rs` | new | test | transform | D 的 serde 合同 | partial |
| `crates/ontoforge-server/tests/support/mod.rs` | new | test | request-response | E | role-match |
| `crates/ontoforge-server/tests/http_contract.rs` | new | test | request-response | E | role-match |
| `crates/ontoforge-server/tests/mcp_contract.rs` | new | test | streaming | E 的进程结构 | partial |
| `crates/ontoforge-server/tests/health_lifecycle_it.rs` | new | test | event-driven | E + C | partial |
| `crates/ontoforge/tests/cli_stdio.rs` | new | test | streaming | E | role-match |
| `crates/ontoforge/tests/dev_lifecycle.rs` | new | test | file-I/O | E | exact |
| `crates/ontoforge-client/tests/contract.rs` | new | test | request-response | E | role-match |
| `clients/typescript/package.json` | new | config | batch | 无 TS package analog | none |
| `clients/typescript/package-lock.json` | new | config | batch | npm 生成 | none |
| `clients/typescript/tsconfig.json` | new | config | batch | 无 TS config analog | none |
| `clients/typescript/src/generated.ts` | new | model | transform | OpenAPI 生成 | none |
| `clients/typescript/src/index.ts` | new | service | request-response | 无 TS client analog | none |
| `clients/typescript/src/codec.ts` | new | utility | transform | 无 lossless codec analog | none |
| `clients/typescript/tests/contract.test.ts` | new | test | request-response | 无 TS 测试 analog | none |
| `.github/workflows/ci.yml` | modify | config | batch | 自身现有 gates | exact |
| `.github/workflows/release.yml` | new | config | batch | `.github/workflows/ci.yml` | role-match |
| `docs/api.md` | new | utility | transform | 无完整平台 API 文档 analog | none |
| `docs/dev.md` | new | utility | transform | 无持久 dev 生命周期文档 analog | none |
| `docs/installation.md` | new | utility | transform | 无三平台发行安装文档 analog | none |

统计：exact 13，role-match 9，partial 13，none 17；有可用模式 35/52。文档角色使用 utility 只是分类词汇适配，不意味着执行代码。

## Pattern Assignments

### A. 共同定义锁与短事务

**Analog:** `crates/ontoforge-storage/src/reload.rs`，git tracked。
**Apply to:** request_snapshot、必要的 storage 导出，以及 Action/history 固定定义入口的事务边界。

**Imports** (lines 37, 42-44):
```rust
use crate::error::StoreError;
use crate::scope_store;
use crate::sql_exit;
use crate::store::{ActiveDefinition, Backend, EditStore, WriteTx};
```

**Core + error handling** (lines 186-200):
```rust
pub(crate) async fn begin_definition_tx(store: &EditStore) -> Result<WriteTx<'_>, StoreError> {
    let mut tx = store.begin_write().await?;
    let millis = u64::try_from(store.settings.lock_timeout.as_millis()).unwrap_or(u64::MAX);
    let locked = async {
        sql_exit::set_local_lock_timeout(&mut tx, millis).await?;
        sql_exit::advisory_xact_lock(&mut tx).await
    }
    .await;
    match locked {
        Ok(()) => Ok(tx),
        Err(error) => {
            sql_exit::rollback(tx).await;
            Err(error)
        }
    }
}
```

**读取定义与 scope 的现成入口:** 同文件 line 265 的 `sql_exit::read_active_definition_in(tx).await?` 和 line 292 的 `scope_store::load_scope_in(tx).await?`。新增 request_snapshot 在同一个共同锁事务里调用，返回前结束事务；不得把 `prepare()` 原样搬进请求，它还有 reload 的 diff、DDL、roles 校验责任。

**复用约束:** 不给 server 暴露 SQL 或 WriteTx；返回不可变 active definition 与 scope 原始输入，再在事务外 compile_scope。PG 普通 begin 下独立 SELECT 不构成已接受的一致快照。Action 新入口接收固定定义，旧入口保留 wrapper；不删除提交 fence，不自动用新定义重试写入。history 保留当前可见性；command replay/result 保留原 receipt/time，不能套用 history 的当前行可见性重验。完整语义以 ENG-REVIEW 为准。

### B. 认证入口与脱敏错误分类

**Analog:** `crates/ontoforge-auth/src/authn/mod.rs`，git tracked。
**Apply to:** server context、REST/MCP 认证、wire error 转换。

**Imports** (lines 13-14, 23):
```rust
use ontoforge_engine::config::{AuthConfig, PrincipalAttrConfig};
use serde_json::Value;
pub use principal::{Principal, PrincipalId};
```

**Auth core** (lines 79-95):
```rust
pub async fn authenticate(&self, bearer: &str) -> Result<Principal, AuthnError> {
    let header = decode_header(bearer).map_err(|_| AuthnError::Malformed)?;
    // The route is decided by the header algorithm alone. Symmetric algorithms have no
    // OIDC route: HS256 goes to the dev verifier when the caller attached one, every
    // other symmetric algorithm is refused. Asymmetric tokens never reach the dev route.
    if header.alg.family() == AlgorithmFamily::Hmac {
        return match (&self.dev, header.alg) {
            (Some(dev), Algorithm::HS256) => dev.verify(bearer),
            _ => Err(AuthnError::UnsupportedAlgorithm),
        };
    }
    let kid = header.kid.as_deref().ok_or(AuthnError::Malformed)?;
    let verifier = self.verifier_for(bearer)?;
    verifier
        .verify(bearer, header.alg, kid, self.oidc.len() > 1, &self.attrs)
        .await
}
```

**Error 分类** (lines 27-30, 37-40):
```rust
#[derive(Debug, thiserror::Error)]
pub enum AuthnError {
    #[error("the token is malformed")]
    Malformed,
```
```rust
    #[error("the token signature is invalid")]
    InvalidSignature,
    #[error("the token has expired")]
    Expired,
```

**复用约束:** 调用库入口，不把 JWT 验证算法复制到 server。bearer 参数不含 `Bearer ` 前缀；dev verifier 只在显式 dev 配置附加。HTTP extensions 放认证后的 Principal，随后从原子 snapshot 编译授权。不得信客户端 principal/roles/scope/limits；session ID 不能替代认证。类型化错误只作分类参考，wire envelope 还须新增稳定 code/apiName/request_id 及 rejection 映射，不能直接 Display 所有后端错误。

### C. 显式拥有 monitor，异步关闭返回错误

**Analog:** `crates/ontoforge-engine/src/trino/l2_monitor.rs`，git tracked。
**Apply to:** server state/lifecycle/health、CLI serve。

**Imports** (lines 3-8):
```rust
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::{Method, RequestBuilder};
use tokio::task::JoinHandle;
```

**Core + error handling** (lines 40-53):
```rust
pub async fn shutdown(mut self) -> Result<(), EngineError> {
    self.stop();
    if let Some(task) = self.task.take() {
        let _ = task.await;
    }
    self.pending
        .cancel(Some(self.generation))
        .await
        .and_then(|()| self.pending.confirm_cleared_through(self.generation))
        .map_err(|_| EngineError::Unavailable {
            reason: "L2 probe cleanup failed".to_owned(),
            retryable: true,
        })
}
```

**Validation** (lines 104-111):
```rust
if options.interval.is_zero()
    || options.interval > Duration::from_secs(30)
    || options.check_timeout.is_zero()
    || options.check_timeout > Duration::from_secs(20)
{
    return Err(EngineError::Config(
        "invalid L2 ACL monitor bounds".to_owned(),
    ));
}
```

**复用约束:** 宿主持有 `L2AclMonitor`，显式调用其 shutdown。这里内部 task.abort 只适合 monitor，不可推广到已接受 Action。宿主按 ENG-REVIEW 顺序停止 admission、等 Action 及依赖读取、取消独立读/SSE、排空 audit、关 monitor/DB；清理失败或 deadline 超时不能成功退出。已配置 L2 故障仅核心健康时 HTTP200 degraded，核心故障503；未配置 L2 不算故障。不能把这一 monitor 模式当作现成 axum lifecycle。

### D. serde tagged union 与拒绝未知字段

**Analog:** `crates/ontoforge-query/src/ast.rs`，git tracked。
**Apply to:** API DTO、wire roundtrip、OpenAPI schema 的输入合同。

**Imports + core** (lines 1-14):
```rust
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ObjectSet {
    Pk {
        object_type: ObjectTypeName,
        pk: Value,
    },
    Type {
        object_type: ObjectTypeName,
    },
```

**分页结构** (lines 154-159):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PageSpec {
    pub first: u64,
    pub after: Option<String>,
}
```

**复用约束:** serde 的 enum rename_all 不自动决定 variant 内字段名；以实际 wire roundtrip 为准，不凭外观推断 object_type 已是 objectType。递归 ObjectSet/WhereClause、任意 JSON、nullable/required、additionalProperties 必须验证生成 OpenAPI3.1 与实际 serde 一致。API 应为轻量 DTO，不直接 re-export query 使 client 拉入 engine/storage。物理名不能进入 L0 元数据。decimal 字符串和当前整数 JSON 编码按 RESEARCH 保留，TS 另做无损 codec。

### E. 真进程、真实持久 store、重启与有界等待

**Analog:** `crates/ontoforge/tests/cli_webhook_worker.rs`，git tracked。
**Apply to:** server support/HTTP、CLI stdio/dev、Rust client contract、snapshot/Action 测试 fixture。

**Imports** (lines 1-3, 13-15):
```rust
use std::collections::{BTreeMap, BTreeSet};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
```
```rust
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
```

**进程启动** (lines 128-147):
```rust
struct Worker(Child);

impl Worker {
    fn start(config: &Utf8PathBuf) -> Self {
        Self(
            Command::new(env!("CARGO_BIN_EXE_ontoforge"))
                .args(["webhook-worker", "--server-config", config.as_str()])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn worker"),
        )
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
```

**持久 fixture:** lines 42-62 用 TempDir/edit.db、EditStore::open、load_from_memory、reload 生成真实 definition；lines 89-114 调真实 ActionService.execute。**重启结构:** lines 231-253 同配置启动第一进程、终止、启动第二进程并验证持久事件；**有界等待:** lines 150-161 使用 tokio::time::timeout 包围轮询。

**复用约束:** kill/wait 的 Drop 是测试泄漏防护，优雅关闭验收须发送正常停止信号并断言成功退出及 durable audit。stdio 要 piped stdout 验证纯协议，不能照抄 null stdout；stderr 须消费避免 pipe 填满。最终 HTTP/MCP/client 验收走真实 listener 和真实业务；wiremock 可控上游失败，不替代 Trino 健康最终证据。双 store race 用 barrier/通知建立确定时序，不靠 sleep 碰运气。

### 配置、CLI 与 release 的现有落点

- `Cargo.toml` 已有 workspace edition2024、rust-version1.98、共享 lint、reqwest 禁默认 features + rustls、tokio 的 rt/time/sync。新增依赖沿 workspace 模式；按 RESEARCH 补 features 并由 Cargo 更新 lock，不手写 lock。
- `crates/ontoforge/src/lib.rs:26-35` 是 `Command::... => module::run(&args)` 分发；`crates/ontoforge/src/cli.rs:21` 是 `Command` enum。注册 Serve/MCP 及模块须接入这些真实位置；不需直接改 main.rs 才能加命令。
- `.github/workflows/ci.yml:23-38` 是 OS matrix、SHA pinned actions、Rust1.98/rustfmt/clippy；`:39-57` 是 fmt/TLS/Clippy/workspace test gates。release 沿此结构补三平台 artifact/package/checksum/smoke，不能把既有双平台 CI 当三平台发行证据。上述路径均已在本轮 `git ls-files` 输出验证。
- 三个 api/server/client `src/lib.rs` 当前都只有 CRATE_NAME 与包名前缀测试，不是已完成业务的 analog；交付时纠正其过时 Phase 注释。

## Shared Patterns

### 请求认证与授权

使用 B 的 Authenticator，再用 A 取 coherent snapshot，在事务外编译本次 principal 的 scope；REST/MCP 调同一业务入口。MCP list/call 共用授予与名称检查，先过滤再分页，直接调用隐藏工具必须拒绝。不得 session 缓存权限。

### 错误、审计、生命周期

沿用类型化 Result/thiserror、`?` 和显式失败分支。wire envelope 是新适配层，覆盖 JsonRejection/body limit/404 等框架拒绝。read audit 由 store 的既有 worker 执行，server 必须显式 drain；本轮定位 `crates/ontoforge-storage/src/read_audit.rs:232` 的 `shutdown`，不重建队列。Action receipt/edit log 共用已有事务路径。

### 验证与测试

DTO 用 D 的 deny_unknown_fields/serde 结构，但 schema 不替代业务验证。动态 Action schema 只投影真实参数定义；调用仍走 ActionService。沿 E 建真实进程 fixture，snapshot/权限/Action/生命周期依工程审查强制 TDD。服务 suite 必须创建并确认非零测试发现；spike5/5只证明可行性。

## No Analog Found

| 文件组 | 缺口与 planner 依据 |
|---|---|
| server rest/mcp、CLI stdio | 无 tracked axum/rmcp transport；用 RESEARCH 的锁定库接口。stdio 桥接已有 `/mcp`，不启动独立 store/runtime |
| OpenAPI 与生成 schema tests | 无 utoipa analog；固定3.1.0，覆盖递归/nullable/required/JSON/数字与 tsc |
| Rust REST client | 当前只有占位；共享 API DTO、每实例 bearer、typed errors、取消、每端点组 contract |
| TS package/codegen/codec/tests | 无 TS 工程；用 openapi-typescript + openapi-fetch、npm lock、strict/noUncheckedIndexedAccess、无损 long/seq/version |
| 新 docs | 无完整平台 API/dev/release 文档；从实际端点和可运行命令生成说明，避免与合同复制漂移 |
| lock files | 包管理器生成，不能人工套源码模式 |

MCP 研究 spike 当前未被 git tracked，不能作为 Pattern Assignments analog，也不能让 executor 把正式实现写回 spike。只消费 RESEARCH 中已核验的3.8.0接口结论：HTTP Parts extensions 身份、旧协议 session 主体绑定/回收、新协议 metadata headers、并发取消通道、禁 redirect、禁自动 reinit 重发写入。生产 schema/权限/生命周期须另有正式测试。

## Metadata

- **Analog search scope:** `crates/`、`.github/workflows/`、workspace manifests；恰好5个主要业务 analog，另外读取占位、配置和既有 CLI 注册点。
- **Files scanned:** 完整源码读取8个非占位实现文件，3个占位 lib；配置/文本搜索另计。主要 analog 5个，均来自 tracked tree，不含 runtime mirror。
- **环境:** 根 `AGENTS.md`、`.codegraph/`、`.codex/skills/`、`.agents/skills/` 未发现；遵守会话内 AGENTS 规则。使用 PowerShell/rg/git 与 apply_patch；专用 Read/Write 工具未暴露。
- **GSD:** 本任务为 `gsd-plan-phase` 派生 pattern mapping，已读取该 skill；唯一产物为本文件。共享 STATE/progress 与其他工作者改动不在本任务写入范围。
- **Validation:** 未运行业务测试；未运行 ralph-loop，也不声称它已安装。没有新增产品合同待决项。
- **Pattern extraction date:** 2026-10-29 (Asia/Shanghai)。
