---
phase: 03-edit-store-scope-model
plan: 23
subsystem: storage
tags: [sqlite, dev-token, quickstart, create_dir_all, windows-acl, AUTH-02, CR-D01, WR-C01, WR-D01]
gap_closure: true

requires:
  - phase: 03-edit-store-scope-model
    provides: "DevKey::load_or_create 与 dev-token 子命令（03-12）、EditStore::open 与 reload（03-04 / 03-08）、03-21 对 open_postgres 与 import 文档的修改"
provides:
  - "EditStore::open 的 SQLite 分支在连接前创建缺少的上级目录，失败映射为 StoreError::Connect（ONT-V26，退出码 3）"
  - "DevKey::load_or_create 首次创建密钥时创建缺少的上级目录，Unix 新目录 0700，其他平台 create_dir_all"
  - "示例配置 examples/server/ontoforge-server.dev.toml 在没有 target/ontoforge-dev/ 的全新检出上可直接用于 reload、scope plan、dev-token"
  - "dev.rs 注释与 docs/edit-store.md 不再声称 Windows 上密钥文件只有本人可读，限制记为已知缺口 7"
affects: [phase-7-serve-dev, 03-VERIFICATION-gap-2]

plan_head_before: 8cbd676a2902b5ec9587c898adea8c09b1abe54b

actuals:
  tokens: 4500
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "目录创建放在库里（open / load_or_create），CLI 与 Phase 7 的 serve --dev 共用同一处修复"
    - "用普通文件占住目录名来制造「store 不可用」，替代依赖目录不存在的旧写法"

key-files:
  created: []
  modified:
    - crates/ontoforge-storage/src/store.rs
    - crates/ontoforge-auth/src/authn/dev.rs
    - crates/ontoforge/tests/cli_reload.rs
    - crates/ontoforge/tests/cli_scope.rs
    - crates/ontoforge/tests/cli_dev_token.rs
    - crates/ontoforge-storage/tests/sqlite_connection.rs
    - crates/ontoforge-auth/tests/dev_token.rs
    - examples/server/ontoforge-server.dev.toml
    - docs/edit-store.md

key-decisions:
  - "目录创建放在 auth 的 DevKey::create 而不是 dev_token.rs，一处修复覆盖 CLI 与 serve --dev（plan 允许二选一）"
  - "store.rs 用 std::fs::create_dir_all，因为工作区 tokio 没有 fs feature，且打开时只调用一次"
  - "不改 Windows ACL，只改正注释并把限制写进已知缺口 7（按 orchestrator 指示，WR-C01 / WR-D01 代码部分不在范围内）"

requirements-completed: [AUTH-02]

coverage:
  - id: D1
    description: "全新检出上按示例配置 ontoforge reload 成功并创建 target/ontoforge-dev/editstore.db，随后 scope plan 成功"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#example_config_reload_works_from_a_fresh_tree"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/sqlite_connection.rs#open_creates_the_parent_directories_of_the_sqlite_file"
        status: pass
    human_judgment: false
  - id: D2
    description: "全新检出上按示例配置 ontoforge dev-token 成功，stdout 一行 token，创建 32 字节的 dev-token.key"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#example_config_dev_token_works_from_a_fresh_tree"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#missing_key_directory_is_created"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#rejected_arguments_leave_no_key_file_behind"
        status: pass
    human_judgment: false
  - id: D3
    description: "Unix 上新建的密钥目录权限为 0700"
    requirement: AUTH-02
    verification: []
    human_judgment: true
    rationale: "断言写在 missing_key_directory_is_created 的 #[cfg(unix)] 块里，本机是 Windows，该分支没有被编译或运行；需要 Linux CI 的结果确认"
  - id: D4
    description: "示例配置头注释、docs/edit-store.md 的目录说明与已知缺口 7 如实描述首次使用行为和 Windows 密钥 ACL 限制，代码注释不再有 owner-only 的说法"
    requirement: AUTH-02
    verification:
      - kind: other
        ref: "rg -n 'restricts it to that user' crates/ontoforge-auth/src/authn/dev.rs (无匹配)；rg -n 'Authenticated Users' docs/edit-store.md crates/ontoforge-auth/src/authn/dev.rs (均有匹配)"
        status: pass
    human_judgment: true
    rationale: "文档措辞是否准确需要读者判断，grep 只能证明旧句式已删、新内容存在"

duration: 27min
completed: 2026-10-21
status: complete
---

# Phase 3 Plan 23: 示例 dev 配置在全新检出上可直接运行 Summary

**EditStore::open 与 DevKey::load_or_create 首次使用时创建缺少的上级目录（Unix 0700），示例 dev 配置从没有 target/ 的全新检出直接跑通 reload、scope plan、dev-token；Windows 密钥 ACL 的限制改为如实记录**

## Performance

- **Duration:** 27 min
- **Started:** 2026-10-21T08:20:41Z
- **Completed:** 2026-10-21T08:48Z
- **Tasks:** 3
- **Files modified:** 9

## Accomplishments

- gap 2 关闭：在 tempdir 里复制示例配置到 `examples/server/`、没有 `target/` 的树上，`reload` 退出码 0，`scope plan` 退出码 0 且有 `plan hash:` 行，`dev-token --sub mia` 退出码 0、一行 stdout、32 字节密钥。
- 修复放在库里，`serve --dev`（Phase 7）会自动受益。
- 被拒绝的 `dev-token` 调用仍不留下文件：既有测试 `rejected_arguments_leave_no_key_file_behind` 通过（参数校验先于 `load_or_create`，顺序未改）。
- `dev.rs` 的 `load_or_create` 注释删掉了「Windows 上目录 ACL 只让本人可读」，改为说明继承 ACL、`Authenticated Users` 修改权限、用户目录或手工收紧 ACL 的做法。

## Task Commits

1. **Task 1: 端到端 tracer（reload + scope plan）**
   - RED `e670bed` (test)：两个失败用例，CLI 以退出码 3 失败（`reload failed, the edit store was not changed`），库测试以 `Connect("error returned from database: (code: 14) unable to open database file")` 失败
   - GREEN `87c6ae5` (fix)：`store.rs` 的 SQLite 分支 `create_dir_all`
2. **Task 2: dev 密钥建目录与 ACL 注释**
   - RED `41061e2` (test)：`DevKey::load_or_create` 报 ``dev key file `…` cannot be created``，CLI 同因退出码 1
   - GREEN `6ae1b1c` (fix)：`create_key_dir`、`CreateError::Dir`、注释改正
3. **Task 3: 示例配置与文档** - `c652435` (docs)

**Plan metadata:** 见本 SUMMARY 之后的 `docs(03-23)` 提交。

## Files Created/Modified

- `crates/ontoforge-storage/src/store.rs` - `open` 的 SQLite 分支先建上级目录；`open` 与 `from_config` 的 doc 补一句
- `crates/ontoforge-auth/src/authn/dev.rs` - `create_key_dir`（Unix `DirBuilderExt::mode(0o700)`，其他平台 `create_dir_all`）、`CreateError::Dir`、`load_or_create` 注释
- `crates/ontoforge/tests/cli_reload.rs` - `example_config_reload_works_from_a_fresh_tree`；`store_unavailable` 改用普通文件占住目录名
- `crates/ontoforge/tests/cli_scope.rs` - `unusable_store_exits_3` 同样改法
- `crates/ontoforge/tests/cli_dev_token.rs` - `example_config_dev_token_works_from_a_fresh_tree`
- `crates/ontoforge-storage/tests/sqlite_connection.rs` - `open_creates_the_parent_directories_of_the_sqlite_file`
- `crates/ontoforge-auth/tests/dev_token.rs` - `missing_key_directory_is_created`
- `examples/server/ontoforge-server.dev.toml` - 只改头注释，两条路径行未动
- `docs/edit-store.md` - `[store] path` 注释、dev-token 一节、已知缺口 7

## Decisions Made

- 目录创建放在 auth 的 `DevKey::create` 而非 `dev_token.rs`，属 Claude's Discretion「dev 密钥存放位置」，原因见 plan objective。
- 目录创建失败的错误文本不带 OS 错误细节（dev 密钥一侧），与同文件里其他 `key_error` 一致；store 一侧带 `{e}`，路径来自部署方配置，不含秘密。
- 未改 ACL，未加 Unix 权限位与符号链接检查（WR-C01 代码部分），见已知缺口 7。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 两个既有测试依赖「目录不存在则 store 不可用」**
- **Found during:** Task 1 (GREEN 之后跑 `cargo test -p ontoforge --test cli_reload`)
- **Issue:** `store_unavailable`（cli_reload.rs）与 `unusable_store_exits_3`（cli_scope.rs）用 `path = "no-such-dir/sub/edit.db"` 制造 ONT-V26 退出码 3。open 现在会创建该目录，两个测试得到退出码 0。
- **Fix:** 先在配置目录下写一个名为 `not-a-dir` 的普通文件，路径改为 `not-a-dir/sub/edit.db`，目录无法创建，仍得到 ONT-V26 退出码 3。测试意图（store 打不开时退出码 3）不变，没有放宽断言。
- **Files modified:** `crates/ontoforge/tests/cli_reload.rs`, `crates/ontoforge/tests/cli_scope.rs`
- **Verification:** `cargo test -p ontoforge --locked` 全绿（cli_reload 8 passed，cli_scope 8 passed）
- **Committed in:** `87c6ae5`

---

**Total deviations:** 1 auto-fixed (1 Rule 1)
**Impact on plan:** 行为变化的直接后果，没有范围扩大。

## Issues Encountered

- 本机是 Windows，`#[cfg(unix)]` 的 `DirBuilderExt::mode(0o700)` 分支和对应的权限位断言没有被编译或运行（只安装了 `x86_64-pc-windows-msvc` target）。该代码只有一行链式调用，语法按标准库签名核对，但需要 Linux CI 确认。已在 coverage D3 标为需人工确认。
- 一次写示例配置头注释的脚本里，Python 字符串的 `\` 加换行把续行符折成了一行，已用 Edit 工具修正并核对 `git diff`。
- `ontoforge-engine --test drift` 这次没有失败，工作区全量测试里通过。

## Verification Run

全部为本次真实运行：

- `cargo test -p ontoforge --locked --test cli_reload example_config_reload_works_from_a_fresh_tree`：1 passed
- `cargo test -p ontoforge-storage --locked --test sqlite_connection`：4 passed（含 `open_creates_the_parent_directories_of_the_sqlite_file`）
- `cargo test -p ontoforge-auth --locked`：各测试目标全部 ok，`dev_token` 16 passed（含 `missing_key_directory_is_created`、`key_file_is_created_once_and_reused`、`key_file_of_wrong_length_is_a_config_error`）
- `cargo test -p ontoforge --locked --test cli_dev_token`：12 passed（含 `example_config_dev_token_works_from_a_fresh_tree`、`rejected_arguments_leave_no_key_file_behind`、`no_key_file_anywhere_fails`）
- `cargo test -p ontoforge --locked --test cli_reload --test cli_dev_token`（Task 3 后）：12 passed、8 passed
- `cargo test --workspace --locked`：退出码 0，合计 984 passed、0 failed、67 ignored（ignored 为需要 Docker / Trino / 外部 Postgres 的既有用例）
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：通过（用 `&&` 链判断，输出 `CLIPPY_OK`）
- `cargo fmt --all --check`：通过（输出 `FMT_OK`）
- Acceptance grep：`rg create_dir_all crates/ontoforge-storage/src/store.rs` 有匹配；`rg DirBuilderExt crates/ontoforge-auth/src/authn/dev.rs` 有匹配；`restricts it to that user` 无匹配；`Authenticated Users` 在 dev.rs 与 docs/edit-store.md 均有匹配；示例配置 `target/ontoforge-dev/` 3 处匹配，`../../target/ontoforge-dev/editstore.db` 原样；docs 含 `0700` 与 `^7\. `

## TDD Gate Compliance

两个 tdd 任务各有 `test(03-23)` 在 `fix(03-23)` 之前（e670bed 先于 87c6ae5，41061e2 先于 6ae1b1c）。RED 的失败都落在目标测试上、原因与 plan 预期一致（见 Task Commits）。没有 REFACTOR 提交。本 plan 的 GREEN 提交类型按 plan 使用 `fix`。

## Known Stubs

None。

## Threat Flags

None。新增文件系统行为（按配置路径建目录）已在 plan 的 T-03-84 / T-03-85 / T-03-86 范围内。T-03-83（Windows 密钥可被同机账号读取）按 plan 为 accept，已记入 docs/edit-store.md 已知缺口 7。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 03-VERIFICATION gaps[1] 的代码与文档部分完成；SC1 第一步与 SC5 的本地流程可从全新检出跑通。
- Linux CI 需确认 `missing_key_directory_is_created` 的 Unix 0700 断言。

---
*Phase: 03-edit-store-scope-model*
*Completed: 2026-10-21*

## Self-Check: PASSED

- 9 个 key-files 均存在于磁盘，`git log` 中可见 e670bed、87c6ae5、41061e2、6ae1b1c、c652435
- `git rev-list --count 8cbd676a..HEAD` 在写本 SUMMARY 前为 5，与 `actuals.commits` 一致
