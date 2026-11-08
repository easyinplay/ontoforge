---
phase: 03-edit-store-scope-model
reviewed: 2026-10-21T16:00:00Z
depth: standard
review_kind: incremental re-review after gap closure plans 03-21, 03-22, 03-23 (diff base 21883a1)
files_reviewed: 24
files_reviewed_list:
  - crates/ontoforge-auth/src/authn/dev.rs
  - crates/ontoforge-auth/src/scope/coverage.rs
  - crates/ontoforge-auth/tests/criteria_coverage.rs
  - crates/ontoforge-auth/tests/dev_token.rs
  - crates/ontoforge-storage/src/commit.rs
  - crates/ontoforge-storage/src/migrate.rs
  - crates/ontoforge-storage/src/store.rs
  - crates/ontoforge-storage/src/value.rs
  - crates/ontoforge-storage/tests/conformance/mod.rs
  - crates/ontoforge-storage/tests/conformance_pg.rs
  - crates/ontoforge-storage/tests/import.rs
  - crates/ontoforge-storage/tests/migrate.rs
  - crates/ontoforge-storage/tests/migrate_pg.rs
  - crates/ontoforge-storage/tests/reload.rs
  - crates/ontoforge-storage/tests/sqlite_connection.rs
  - crates/ontoforge-storage/tests/support/mod.rs
  - crates/ontoforge/src/migrate.rs
  - crates/ontoforge/tests/cli_dev_token.rs
  - crates/ontoforge/tests/cli_migrate.rs
  - crates/ontoforge/tests/cli_reload.rs
  - crates/ontoforge/tests/cli_scope.rs
  - docs/edit-store.md
  - docs/scope.md
  - examples/server/ontoforge-server.dev.toml
findings:
  critical: 2
  warning: 24
  info: 6
  total: 32
status: issues_found
---

# Phase 03 代码审查报告（gap closure 增量复审）

**Reviewed:** 2026-10-21
**Depth:** standard
**Files Reviewed:** 24（`git diff 21883a1..HEAD` 触及的全部源文件、测试、文档与示例配置）
**Status:** issues_found

计数口径：`critical` 2 与 `warning` 24 包含上一轮仍未修复、被原样带入本轮的条目（见「上一轮遗留」表，按「文件未变且代码行经 grep 确认」判定）；`info` 只计本轮新发现的 6 条，上一轮的 info 不重复列出。

## Summary

03-21、03-22、03-23 三个 gap 计划针对的 6 条上一轮发现，代码层面全部有效修复，没有发现回归。我在本机跑了相关测试，全部通过：`ontoforge-auth` 的 `criteria_coverage`（6 个）与 `dev_token`（16 个），`ontoforge-storage` 的 lib（48 个）、`import`（16 个）、`migrate`（15 个）、`sqlite_connection`（4 个）。PostgreSQL 与 CLI 集成测试没有运行，下面对它们的判断只来自读代码。

| 上一轮条目 | 状态 | 依据 |
|---|---|---|
| CR-A01 decimal 经 f64 舍入 | 已修复 | `value.rs:337-341` 只收 `is_i64 / is_u64` 的 JSON 数字，带小数的数字被拒绝；数组元素走同一函数；单元测试、import 测试与 conformance 都覆盖 17 位有效数字 |
| WR-A01 PG 存不下的值 | 已修复 | `canonical_as` 拒绝 U+0000，`date` 拒绝 0 年，`timestamp` 在 UTC 转换后限定 1 到 9999 年；`check_shape` 拒绝 `action`、`Principal` id、`params` 的串与键中的 NUL（`json_contains_nul` 迭代实现，不会栈溢出） |
| WR-A03 PG 会话设置 | 已处理 | `pg_connect_options` 同时用 `options`（`-c`）和专用启动参数固定 `DateStyle`、`TimeZone`、`extra_float_digits`。我核对了 sqlx-postgres 0.9.0 的 `options()`：它对带空格的值（`ISO, MDY`）做转义，多个选项以空格分隔，所以写法正确 |
| WR-B07 migrate 不复核 role | 已修复（CLI 路径） | `apply_in` 在 `lock_def_state`、`check_hash` 和「空计划」提前返回之后、`run` 之前编译已存 role；失败映射为 `MigrateError::Definition` 并回滚。见 WR-01 与 IN-04 的限制 |
| WR-C05 整对象读取绕过 V23 | 已修复 | `coverage::problem` 对 2 段路径要求 allow-list 覆盖该类型全部属性；`walk.rs` 对 `in`、比较、宏参数、`size()` 等形态都产出 2 段路径；类型缺失按 fail-closed 处理 |
| CR-D01 全新检出上 quickstart 失败 | 已修复 | `EditStore::open` 创建 SQLite 文件的上级目录，`DevKey::create` 创建密钥的上级目录（Unix 新目录 0700）；`example_config_*_works_from_a_fresh_tree` 两个 CLI 测试覆盖 |

本轮新发现 2 个 warning（WR-01、WR-02）和 6 个 info。最重要的一条是 WR-01：新加的 migrate 角色复核与 `scope apply` 的校验口径互相排斥，对被角色引用的属性做重命名或改类型的迁移，没有一个中间状态能同时满足两边。

## Critical Issues

上一轮的两个 critical 仍然未修复，不在三个 gap 计划范围内；详情见 `git show 21883a1:.planning/phases/03-edit-store-scope-model/03-REVIEW.md`。

### CR-B01（遗留）: CI 的 openssl 守卫不会失败

**File:** `.github/workflows/ci.yml:33`
**Issue:** 第一行 `! cargo tree ... -i openssl` 在 bash `errexit` 下被忽略，只有最后一行（`native-tls`）决定步骤退出码。我重新读了当前文件，两行仍是原样。
**Fix:** 改为显式循环，对每个包 `if cargo tree ... -i "$pkg" >/dev/null 2>&1; then echo "::error::$pkg is in the dependency graph"; exit 1; fi`。

### CR-D02（遗留）: server 配置的解析错误回显被拒绝的值，可能泄漏误贴的密钥

**File:** `crates/ontoforge-engine/src/config.rs:397-439`
**Issue:** 文件自 21883a1 起未变。除 `password`、`token`、`secret`、`url` 四个未知键外，其余 `toml::de::Error` 文本原样透传，其中包含被拒绝的值（`unknown variant `hunter2secret``）。
**Fix:** 对带值的错误种类只保留键和期望类型，丢弃值，并加一个把可识别密钥填入各类型字段、断言输出不含它的测试。

## Warnings

### WR-01: migrate 的角色复核与 `scope apply` 的校验口径互斥，重命名或改类型的迁移无法在不临时撤销角色引用的情况下进行

**File:** `crates/ontoforge-storage/src/migrate.rs:403-414`；`docs/scope.md:210`（「定义变更时的复核」）
**Issue:** `apply_in` 用**新**定义编译已存的 role。`scope apply` 则要求 `--ontology` 与系统库当前生效的**旧**定义一致（`docs/scope.md:207`，`check_active_definition`），并按旧定义校验 role。对被某个 role 的 `properties` 或 `rowFilter` 引用的属性做声明式重命名（`--rename`）：

1. role 写旧名：新定义里没有旧名，migrate 报 ONT-V21 拒绝。
2. role 写新名：旧定义里没有新名，`scope apply` 报 ONT-V21 拒绝。

没有一份 scope 能同时通过两道检查。属性改类型而 `rowFilter` 的比较随类型变化时同理。文档给出的出路只覆盖「删除引用」：先去掉引用并 `scope apply`，migrate，再把新名加回去并再 `scope apply`。这意味着重命名一个被引用的属性，必须经过一个 role 暂时失去该列（或该行过滤）的窗口，而且文档没有提到重命名这个最常见的场景。方向上是 fail-closed，所以是 warning，不是 blocker；但它让 03-22 新增的复核成为这类迁移的硬阻塞，而不是安全网。

**Fix:** 二选一，并在 `docs/scope.md` 写明。
- 让 `scope apply` 接受「新定义」作为 `--ontology`，条件是该定义是 `migrate` 即将应用的目标（例如在 `scope apply` 里增加 `--for-migration`，只做编译与写入，不要求与生效定义相等，并在 `migrate --apply` 的复核里通过）。
- 或让 migrate 的复核对声明了 `--rename` 的属性先按重命名映射改写 role 的引用再编译。

至少把「重命名需要两次 scope apply，中间 role 失去该列」写进文档，并加一个覆盖该流程的 CLI 测试。

### WR-02: dev 签名密钥的保护仍然缺失（WR-C01 与 WR-D01 的代码部分）

**File:** `crates/ontoforge-auth/src/authn/dev.rs:106-130`（`read_existing`）与 `create`；`docs/edit-store.md` 已知缺口第 7 条
**Issue:** 03-23 按指示只改正了注释并把限制写入已知缺口，没有改代码。因此：
- Windows 上密钥继承目录 ACL；我上一轮在本机看到项目目录带 `Authenticated Users:(M)`，任何本机账号都能读这把对称密钥并为任意 `sub`、任意 `groups` 签发 dev token。示例配置把密钥放在 `target/ontoforge-dev/`，正好是这种位置。
- Unix 上 `read_existing` 不检查已有文件的权限位（0644 的备份恢复或 git 检出会被静默接受），也不拒绝符号链接。
- 新增的 `create_key_dir` 在 Unix 上给新目录 0700，但已存在的父目录保持原样，所以 0700 只在目录链全是新建时才成立。文档写得准确，但这是一个「默认不安全、靠文档提醒」的状态。

dev token 只被 `--dev` 启动的服务接受，配置不能单独启用，所以不是生产凭据。降级为 warning，而不是 blocker，依据正是这一点。
**Fix:**
- Unix：`File::open` 之后检查 `metadata()?.permissions().mode() & 0o077 != 0` 并拒绝（或警告），用 `symlink_metadata` 拒绝符号链接。
- Windows：创建后用 `icacls` 等价的 API 去掉继承并只授权当前用户，或让 `dev-token` 在密钥路径不在用户配置目录下时向 stderr 打印一行警告。
- 示例配置的默认 `key_file` 改到用户目录下的路径（需要 `~` 或环境变量展开支持时再做）。

## 上一轮遗留（未修复，不在 03-21 至 03-23 范围内）

下表条目经核对：对应文件在 `21883a1..HEAD` 中未变，或变更的函数与该条目无关（`commit.rs` 只改了 `check_shape` 的 NUL 检查，`migrate.rs` 只改了 `apply_in`）。我对其中几条重新用 grep 确认了现状：WR-A02（`commit.rs` 中没有 `lock_timeout`）、WR-B03（`migrate.rs:502` 仍是无条件 `*store.active.write().await = ...`）、WR-B10（`migrate.rs:482` 仍是 `unwrap_or_default()`）、WR-D02（`reload.rs:58` 与 `migrate.rs` 的 `probe_engine` 仍在 dry-run 分支之前）、WR-D03（`discover.rs:305` 仍是大小写敏感的扩展名匹配）。其余条目的细节与修复方案见上一轮报告（`git show 21883a1:.planning/phases/03-edit-store-scope-model/03-REVIEW.md`）。

| ID | 严重度 | 位置 | 一句话 |
|---|---|---|---|
| WR-A02 | warning | `commit.rs` 提交路径 | commit 不设 `lock_timeout`，被阻塞的提交会占满 5 个连接的池 |
| WR-A04 | warning | `commit.rs` `check_shape`，`history.rs` | 单个 commit 无大小上限；`history()` 的 `seqs` 不去重，超过绑定参数上限后永久不可读 |
| WR-A05 | warning | `import.rs` | `OnExisting::Update` 不能只更新部分列，也不能清空值 |
| WR-B01 | warning | `reload.rs:748-783` | 带 probe 的两个并发相同 reload 不是 single-flight，后者以 `Conflict` 失败 |
| WR-B02 | warning | `scope_store.rs` `apply_in` | `scope apply` 在不持有 state 行锁的情况下执行 DDL，有死锁窗口 |
| WR-B03 | warning | `migrate.rs:502` | `apply_drop_archived` 无条件覆盖内存中的 active 定义，应使用 `install_if_newer` |
| WR-B04 | warning | `migrate.rs` `archive_column`、`archive_object_table` | role 创建的索引不在 `PhysicalSchema` 中，迁移后残留在归档列上；SQLite 上会使 `DROP COLUMN` 失败。见 IN-04：03-22 之后 migrate 复核了 role 却仍不维护 role 索引 |
| WR-B05 | warning | `reload.rs:628-633` | 数组列经重命名或归档后，`expected_visible` 等待一个 Trino 永远不显示的列 |
| WR-B06 | warning | `migrate.rs` `rename_object_types` | 物理表名不变的对象类型重命名会产生 `RENAME TO` 同名语句 |
| WR-B08 | warning | `scope_store.rs`、`scope.rs` | `scope_apply` 不在锁内校验 role 是按当前生效定义编译的 |
| WR-B09 | warning | `migrate.rs` `compute` | 归档后缀与记录的 `def_version` 按 `active + 1` 预测，存在 pending 版本时与实际版本不符 |
| WR-B10 | warning | `migrate.rs:482` | `plan_drop_archived` 把哈希失败变成空串，`check_hash` 对空串可能放行，而这是唯一不可逆删除数据的操作 |
| WR-C02 | warning | `jwks.rs` | JWKS 刷新节流在 fetch 完成后才记录，取消请求可绕过 `min_refresh` |
| WR-C03 | warning | `oidc.rs`、`authn/mod.rs` | auth crate 不重复校验配置层的安全限制（https、`min_refresh`、`leeway`、issuer 不含 `\|`） |
| WR-C04 | warning | `rowpred/eval.rs:64-73` | 行缺少某属性时按 `Null` 求值，`x == null \|\| ...` 形式的过滤会 fail-open |
| WR-C06 | warning | `scope/compile.rs:180-184`、`merge.rs` | 编译失败的 `rowFilter` 与「无过滤」同为 `row: None`，仅靠 `compile_scope` 的整体 `Err` 兜底 |
| WR-C07 | warning | `rowpred/parse.rs:450-453` | 含 NUL 的字符串字面量通过编译，到 SQL 打印时才失败，Rust 与 SQL 两条路径不一致 |
| WR-D02 | warning | `reload.rs`、`migrate.rs`（CLI） | `reload --dry-run` 与 `migrate --plan` 仍构建 Trino 引擎并要求其密钥 |
| WR-D03 | warning | `discover.rs:298-311` | scope 加载器静默丢弃文件（扩展名大小写、符号链接、保留名） |
| WR-D04 | warning | `loader/scope.rs`、`dev_token.rs` | subject 与 dev-token 参数不规范化也不校验 |
| WR-D05 | warning | `sqlite/fixture.rs:749-758` | `normalize_timestamp` 的时钟字段接受前导 `+` |
| WR-D06 | warning | `scope.rs` | 审计 actor 来自可伪造的环境变量，Linux 上常为 `user@unknown` |

## Info

### IN-01: decimal 拒绝消息里夹着一长串空格

**File:** `crates/ontoforge-storage/src/value.rs:341`
**Issue:** 字符串 `"... a JSON number with a fraction                  passes through a binary float ..."` 在 `fraction` 与 `passes` 之间有 18 个空格，应该是 `\` 续行被写成了行内空格。该文本会原样出现在 ONT-V27 诊断和 `EditError::Value` 中；import 测试只断言 `contains("as a string")`，所以没有发现。
**Fix:** 合并成一个正常的字符串字面量（单空格），或用 `\` 续行。

### IN-02: coverage.rs 模块注释出现超长行

**File:** `crates/ontoforge-auth/src/scope/coverage.rs:7-9`
**Issue:** 03-22 在模块文档中插入一句话后没有重新折行，第 8 行把新句子和原来的 `What the criteria read has to be known statically: a computed index` 连成一行，约 140 个字符，`rustfmt` 不折行注释，所以没有被格式检查抓到。
**Fix:** 在 `reads the object as a whole ... property of its type.` 之后换行，使原句 `What the criteria read ...` 另起一行。

### IN-03: `create_key_dir` 的错误原因被丢弃

**File:** `crates/ontoforge-auth/src/authn/dev.rs:150`（`map_err(|_| CreateError::Dir)`）
**Issue:** 目录创建失败时只报 `cannot be created because its directory cannot be created`，操作员分不清是权限不足、路径上有同名文件还是磁盘问题。`EditStore::open` 的对应错误带了 `{e}`，两处不一致。
**Fix:** 让 `CreateError::Dir` 携带 `io::ErrorKind`，并在消息里写出 kind（不要写完整路径之外的内容，保持密钥材料不出现在错误里的既有约束）。

### IN-04: `apply_migration` 默认不复核 role，且 migrate 仍不为新列补 role 索引

**File:** `crates/ontoforge-storage/src/migrate.rs:333-349`、`run`（`:421` 起）
**Issue:** 两点。
1. 公共函数 `apply_migration` 把 `roles` 固定传 `None`，doc 已说明只有 CLI 走复核路径。D-18 因此依赖调用方记得传 `Some`，Phase 5 或 Phase 7 的新入口复制 `apply_migration` 就会静默跳过。
2. `reload` 在复核 role 之后调用 `missing_role_indexes` 补齐 `rowFilter` 的索引，`migrate` 复核后只执行迁移步骤，不补索引。迁移若归档了一个被 `rowFilter` 引用的列并建立同名新列，新列没有索引，直到下一次 `reload`（且受 WR-B04 影响，同名索引已存在于归档列上时也不会重建）。
**Fix:** 把 `apply_migration` 标为仅供测试（`#[doc(hidden)]` 或 `pub(crate)`），公开入口必须显式传 `RolesCheck` 或显式声明跳过；`run` 之后复用 `missing_role_indexes`。

### IN-05: PG 会话设置守卫测试在断言失败时泄漏临时数据库

**File:** `crates/ontoforge-storage/tests/conformance_pg.rs:214-285`
**Issue:** `DROP DATABASE` 在函数末尾，任何 `assert_eq!` 失败都会跳过它，留下 `of_t_<ulid>_db`。本机 `embedded-pg` 每次启动全新实例，影响小；对 CI 的共享 service container 会累积。
**Fix:** 用 drop guard，或把断言结果先收集再清理再统一断言。

### IN-06: `EditStore::open` 对只读命令也会创建目录

**File:** `crates/ontoforge-storage/src/store.rs:190-197`
**Issue:** `reload --dry-run`、`scope plan`、`migrate --plan` 也走 `EditStore::open`，路径写错（例如 `/etc/foo/edit.db` 的 typo）会在预览命令里悄悄创建目录。SQLite 文件本来就是 `create_if_missing`，所以影响仅限目录，且 `create_dir_all` 的失败消息清楚；记录在案即可。
**Fix:** 如果要保持预览命令无副作用，给 `StoreSettings` 加 `create_dirs: bool`，只读命令传 `false`。

---

_Reviewed: 2026-10-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
