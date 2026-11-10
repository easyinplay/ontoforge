---
phase: 08-production-webhook-end-to-end-delivery
reviewed: 2026-11-08T20:35:58Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - .github/workflows/ci.yml
  - Cargo.lock
  - ci/verify-evidence-archive.sh
  - crates/ontoforge/Cargo.toml
  - crates/ontoforge/WEBHOOK-WORKER.md
  - crates/ontoforge/examples/webhook_receiver.rs
  - crates/ontoforge/tests/webhook_e2e_pg.rs
findings:
  critical: 0
  warning: 4
  info: 6
  total: 10
  necessity: 1
status: issues_found
---

# Phase 08: Code Review Report

**Reviewed:** 2026-11-08T20:35:58Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

审查范围是 Phase 08 新增的 7 个文件：独立的 `phase8-webhook-e2e` CI job、接收端 example target、被 `#[ignore]` 的 PG 端到端用例、证据归档校验器、`axum` 的 dev-dependency，以及 `WEBHOOK-WORKER.md` 的追加小节。

整体形态是站得住的：接收端只记录原始事实、不自判成败，判定完全落在 ledger 原始记录与 PostgreSQL `of_webhook_outbox` 终态两处独立来源上；缺环境变量时 panic 而非静默跳过；`axum` 只进 dev-dependencies，依赖图未变（`Cargo.lock` 只有一行 `"axum",` 增量）；第三方 action 全部固定到 commit SHA；新 job 有 `timeout-minutes: 30`、有上传前凭据扫描、有 `always()` 容器清理。

问题集中在**证据目录的状态管理**上：用例从不在运行前重置 `target/phase8-webhook`，而接收端的 ledger 是追加写的、ready 文件是复用写出的，于是在同一工作区重复运行该用例时，会以两种不同的错误表象失败，且真因被掩盖。这对 CI 的一次性工作区无影响（每次 run 都是全新 checkout），但对任何本地复跑、CI 重跑同一 job、或后续 Phase 9 复用同一 harness 的场景是实打实的缺陷。另一处是接收端把 ledger 写入放在互斥锁之外，文件顺序在并发请求下可以与 `seq` 顺序脱钩，而用例断言的恰恰是「接收端按序编号」。

按 phase 明确持有的边界核对过以下各项，均未发现问题，不作为缺陷报告：`crates/ontoforge/examples/webhook_receiver.rs` 未注册为 `ontoforge` 子命令（`rg -n "webhook_receiver|webhook-receiver" crates/ontoforge/src/` 无匹配），未新增 workspace member，`ci/release-package.ps1` 只打包 binary + manifest，example 天然不入发行包；`crates/ontoforge/tests/webhook_e2e_pg.rs` 确为 `#[ignore]`，缺 `ONTOFORGE_PG_URL` / `ONTOFORGE_WEBHOOK_RECEIVER_BIN` 时走 `expect` panic（第 542-544 行），不会静默跳过；`crates/ontoforge-action/src/outbox.rs`、`crates/ontoforge-engine/src/config.rs`、`crates/ontoforge-storage/` 未被本 phase 改动；`ci/verify-evidence-archive.sh` 未被任何 workflow 引用；`crates/ontoforge/WEBHOOK-WORKER.md` 的 diff 为 25 行新增、**0 行删除**（`git diff --numstat` 实测），既有段落逐字未改。

Necessity 判定依据 `~/.claude/gsd-core/references/necessity-review.md`（已读取），分类见 WR-04。

## Warnings

### WR-01: 陈旧的 `receiver.ready` 会让本次用例指向一个已死的回环端口

**File:** `crates/ontoforge/tests/webhook_e2e_pg.rs:259-273`（配合 `:224-239`、`crates/ontoforge/examples/webhook_receiver.rs:155-157`）
**Category:** 运行时健壮性
**Issue:** `wait_for_ready_file` 的判定是「文件存在且内容非空」，只要读到就立即返回。而接收端是在 bind 成功之后才写 ready 文件，用例是在 spawn 之后才去读。同一工作区里跑过一次之后，`dir/receiver.ready` 会留在磁盘上（内容是上一次那个进程的 `http://127.0.0.1:<port>`）。第二次运行时，用例极可能在新的接收端还没写完 ready 文件之前就读到了这个陈旧文件，于是：

1. `assert!(target.starts_with("http://127.0.0.1:"))` 照样通过（陈旧 URL 也是回环地址）；
2. worker 被指向一个早已退出的端口，连接被拒，接收端一个字也收不到；
3. 用例在 `wait_for_delivery` 里空转满 60 秒，最后 panic 为 `the receiver recorded no requests at all: <stderr>`。

真正的原因是「URL 是上一轮的」，但报错指向「接收端没记录到请求」，排查会被带偏。CI 每次 run 是全新 checkout，所以线上没暴露；本地复跑、CI 重跑同一 job、或 Phase 9 复用这个 harness 时必然踩到。

**Fix:** 在 spawn 接收端之前清掉陈旧的 ready 文件（同时清理 ledger 与日志，见 WR-02），或者让接收端在发布 URL 之前先删除旧文件：

```rust
// crates/ontoforge/tests/webhook_e2e_pg.rs，spawn_receiver 之前
for stale in ["ledger.jsonl", "receiver.ready", "receiver.log", "receiver.err.log"] {
    let _ = std::fs::remove_file(dir.join(stale));
}
```

```rust
// crates/ontoforge/examples/webhook_receiver.rs，发布 ready 文件之前
let _ = std::fs::remove_file(&args.ready_file);
```

### WR-02: ledger 是追加写的，用例从不清空，第二次运行必然在去重集合断言上失败

**File:** `crates/ontoforge/tests/webhook_e2e_pg.rs:104-110`、`:277-290`、`:435-440`；`crates/ontoforge/examples/webhook_receiver.rs:135-141`
**Category:** 运行时健壮性
**Issue:** 接收端用 `OpenOptions::new().create(true).append(true)` 打开 ledger，每收到一个请求追加一行；用例侧的 `evidence_dir()` 只 `create_dir_all`，从不删除既有文件。于是第二次运行的 ledger 里同时躺着上一轮（`ulid::Ulid::generate()` 生成的、每轮都不同的 `event_id`）和这一轮的记录。

直接后果是 `assert_ledger_evidence` 里的集合相等断言失败：

```rust
let recorded: BTreeSet<String> = unique_event_ids(records);   // 上一轮 + 这一轮
let expected: BTreeSet<String> = submitted.iter().cloned().collect();
assert_eq!(recorded, expected, "the receiver must see every submitted event exactly once");
```

而这条断言正是 D3（多事件去重集合与至少一次送达）的机器判定点——它失败时报的是「集合不相等」，真因却是「上一轮的记录还在」。同样的污染还会让 `assert_event_evidence` 的 `delivered[0].status_code == 503` 变得不可靠（若上一轮出现过同一个 `event_id`，理论上前缀会被污染；`event_id` 每轮唯一，所以这条实际不触发，但 `attempt_seq` 的语义已经不再干净）。

**Fix:** 用例启动时重置证据目录里的 ledger（最直接的是 spawn 前删除 `ledger.jsonl`，见 WR-01 的清理块）；若希望接收端自身保证「每次运行从空 ledger 开始」，则把 ledger 的打开方式从 append 改为 truncate，但那样会丢掉「逐条追加」这一证据语义，不推荐。

### WR-03: 接收端在互斥锁之外写 ledger，文件顺序可以与 `seq` 顺序脱钩

**File:** `crates/ontoforge/examples/webhook_receiver.rs:99-141`；断言方 `crates/ontoforge/tests/webhook_e2e_pg.rs:418-423`
**Category:** 并发正确性
**Issue:** `seq` 的分配在 `shared.state.lock()` 内完成，但 ledger 的写入（open / `write_all` / `flush`）发生在这个临界区**之外**。`#[tokio::main]` 默认是多线程运行时，多个请求会并发进入 handler：线程 A 拿到 `seq=1` 后释放锁，线程 B 拿到 `seq=2`，B 先落盘、A 后落盘 → 文件里的行序是 `2,1`，而 `seq` 是 `1,2`。

用例对此有明确断言：

```rust
for pair in records.windows(2) {
    assert!(pair[1].seq > pair[0].seq, "the receiver numbers its records in order: {pair:?}");
}
```

也就是说，只要接收端同时处理两个请求，这条断言就会以「接收端编号乱序」的形态随机变红。今天不会触发，是因为 `WebhookWorker::poll_at_with_after_send`（`crates/ontoforge-action/src/outbox.rs:126-171`）在一个 poll 内是逐条 `await` 串行发送的——但这条性质属于**被测系统的实现细节**，而不是接收端 harness 的契约；接收端是一个独立 HTTP 服务，Phase 9 一旦引入并发投递或第二个 worker 实例，这里就立刻变成 CI 随机失败。

**Fix:** 把落盘动作纳入同一把锁，保证文件顺序恒等于 `seq` 顺序（顺序写入时不影响并发处理请求体解析）：

```rust
let (seq, attempt_seq, status_code) = {
    let mut state = shared.state.lock().expect("receiver state");
    // ... 计数与状态码计算 ...
    let record = LedgerRecord { /* ... */ };
    let mut line = serde_json::to_vec(&record).expect("ledger record");
    line.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true).append(true).open(&shared.ledger).expect("ledger file");
    file.write_all(&line).expect("ledger write");
    file.flush().expect("ledger flush");
    (seq, attempt_seq, status_code)
};
```

### WR-04: Impossible-state guard — 「接收端是独立进程」的 PID 断言无法失败

**File:** `crates/ontoforge/tests/webhook_e2e_pg.rs:583-587`
**Category:** necessity
**Issue:** 这条断言想守住的状态（接收端与用例同进程）**不可能发生**：接收端是通过 `std::process::Command::spawn` 拉起的（`crates/ontoforge/tests/webhook_e2e_pg.rs:224-239`），`Child::id()` 返回的是子进程 PID，与父进程 PID 天然不同，两者在构造上就不可能相等。换句话说，这条断言在任何输入下都恒为真，它没有为「接收端是独立进程」提供任何证据。

真正提供证据的是另外两处：`Command::spawn` 这个调用点本身（独立 OS 进程），以及 ledger 每条记录里的 `remote_addr` 以 `127.0.0.1:` 开头（独立 TCP 连接）。断言还放在全部投递完成之后，即使它可失败，也已经来不及守住任何东西。

**Fix:** 删除这条恒真断言，把「独立进程」这一 D-01 边界的证据落在 spawn 调用点与 `remote_addr` 断言上；若希望保留一条可失败的机器判定，改为断言「ledger 中的 `remote_addr` 端口与用例自身持有的端口不同」这类可证伪的事实。

## Info

### IN-01: `read_ledger` 与 `ledger_lines` 是两份几乎相同的读文件逻辑，语义却不同

**File:** `crates/ontoforge/tests/webhook_e2e_pg.rs:277-290`、`:308-317`
**Issue:** 两个函数都用 `read_to_string` + `lines()`，但只有 `read_ledger` 会丢弃尾部半行（`if !text.ends_with('\n') { lines.pop(); }`），`ledger_lines` 会把半行原样返回。凭据扫描那条路径（`assert_ledger_has_no_credential`）因此看到的是与判定路径不同的文件内容。半行只可能来自并发写入，对「找不找得到密码」无影响，但两份语义不一致的读取器是后续改动的隐患。

**Fix:** 让 `ledger_lines` 复用 `read_ledger` 的切分逻辑（例如抽一个 `complete_lines(&str) -> Vec<&str>` 供两者共用），或在 `ledger_lines` 上注明「此处刻意保留半行」。

### IN-02: `ci/verify-evidence-archive.sh` 的错误归因不精确

**File:** `ci/verify-evidence-archive.sh:125-141`
**Issue:** 索引 JSON 合法但缺 `file_count` 时，报错文案是 `the index is not valid JSON`（exit 7），与实际原因不符；反过来，索引真的无法解析时，`want_sha=$(jq_text '.archive.sha256 // empty' "$index")`（第 136 行）没有 `|| fail 7` 兜底，jq 失败会被吞成空串，最终以 exit 8（`archive.sha256 does not match`）报出，把「索引坏」说成「摘要不符」。

**Fix:** 把文案改成 `the index does not declare a usable file_count`；给第 136-137 行的两次 `jq_text` 补上 `|| fail 7 "the index is not valid JSON: $index"`。

### IN-03: `craft_symlink_tar` 写的 ustar magic 不是 POSIX 形式

**File:** `ci/verify-evidence-archive.sh:233-235`
**Issue:** 头部第 257-262 字节写的是 `ustar` + 两个空格（`printf '%-6s' 'ustar'`），即 GNU tar 容错接受的老 GNU 形式；POSIX 的形式是 `ustar\0`。GNU tar 能解析（自检场景 (c) 因此能触发 exit 4），但 BSD tar（macOS 自带）可能直接判为损坏归档，使该场景在 macOS 上得不到 exit 4。脚本本身只在 Linux CI 与 Windows/Cygwin 上使用，影响有限，但 08-02-SUMMARY 声称的是「同一份脚本行为一致」，此处是那个说法的例外。

**Fix:** 把 magic 写成 `ustar\0` + version `00`（`printf 'ustar\000'; printf '00'`），或在该脚本头部注明「仅 GNU tar 环境」。

### IN-04: 新的 ignored 用例目标没有登记进共享的「ignored 用例确实存在」清单

**File:** `.github/workflows/ci.yml:229-257`（共享清单）、`:404-415`（新 job 的本地门）
**Issue:** `integration` job 里维护着一份 targets 数组（`ontoforge-engine trino_it` … `ontoforge-action trino_action_it`），用来保证「绿色不等于什么都没跑」。新目标 `ontoforge webhook_e2e_pg` 不在这份清单里，而是靠 `phase8-webhook-e2e` job 内部自己的 `--list` 计数门（`:411-414`）覆盖。覆盖是等价的，但两份清单从此可以各自漂移：将来有人删掉新 job 里那三行 `--list` 门，就再没有机器判定保证这条用例还在。

**Fix:** 把 `ontoforge webhook_e2e_pg` 加进共享清单（该清单所在 job 是 `integration`，跨 job 复用需要另想办法，例如在 `phase8-webhook-e2e` 里也维护一份局部清单），或在 `phase8-webhook-e2e` 的 job 注释里写明它自带清单门、不进共享清单的原因。

### IN-05: ready 文件的临时名用 `with_extension("tmp")`，同目录下同 stem 的两个文件会互相覆盖

**File:** `crates/ontoforge/examples/webhook_receiver.rs:155`
**Issue:** `args.ready_file.with_extension("tmp")` 把 `receiver.ready` 变成 `receiver.tmp`。同一目录里若存在另一个 `.ready` 路径（例如 `worker.ready` 与 `receiver.ready` 同 stem 的情形）会写到同一个临时文件。目录约定（一个 harness 一个目录）下不会触发。

**Fix:** 用 `ready_file.with_file_name(format!("{}.tmp", ready_file.file_name()...))` 之类的派生名，或直接写 `<ready_file>.tmp`。

### IN-06: 用例在 PostgreSQL 里建的两个 schema 用完不删

**File:** `crates/ontoforge/tests/webhook_e2e_pg.rs:546-548`、`:596-600`
**Issue:** `of_t_<ulid>_edit` / `of_t_<ulid>_sys` 每个 `event_id` 组合都是新的 schema（ULID 保证唯一），用例结束时既不 `DROP SCHEMA` 也不关 pool。CI 里容器整体销毁所以无碍；本地对着开发库反复跑会在 `editstore` 里堆积 schema。

**Fix:** 用例结尾加一段 best-effort 清理（`DROP SCHEMA IF EXISTS ... CASCADE`），失败也不影响判定。

---

_Reviewed: 2026-11-08T20:35:58Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
