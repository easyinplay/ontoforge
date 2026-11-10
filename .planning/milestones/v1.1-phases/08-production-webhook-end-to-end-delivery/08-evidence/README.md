# Phase 8 CI 证据归档

`ci-evidence.tar.gz` 冻结一次成功的 CI run 的端到端 webhook 投递证据，`ci-evidence-index.json` 逐成员记录路径、字节数与 SHA256，并记录归档自身的 SHA256 与来源边界。

## 来源

| 项 | 值 |
|---|---|
| CI run | `42515808955` — https://github.com/easyinplay/ontoforge/actions/runs/42515808955 |
| run 的 head sha | `30a05a8a43f7a8235ca8db7cc715d39e277bc8b5` |
| CI job | `phase8-webhook-e2e`（结论 `success`，job id `128838450761`） |
| artifact | `phase8-webhook-ledger` |
| 收集目录 | `target/phase8-webhook/ci` |

该 run 是本分支上第一条同时满足三个谓词的 run：`phase8-webhook-e2e` job 结论为 success、run 的 `head_sha` 是当前 HEAD 的祖先、且 `head_sha` 晚于首次把 `phase8-webhook-e2e` 引入 `.github/workflows/ci.yml` 的 commit（`f95767a`）。该 job 是本 phase 独有的，因此「含有且成功的该 job」是排除本 phase 之前旧 run 的锚。

## 成员构成（6 个普通文件）

| 成员（仓库根相对路径） | 字节数 | 内容 |
|---|---|---|
| `target/phase8-webhook/ci/ledger.jsonl` | 1866 | 接收端逐请求写入的原始 JSONL 记录：6 行，对应 3 条事件的 2 次投递（第 1 次 503、第 2 次 204） |
| `target/phase8-webhook/ci/ontoforge-server.toml` | 302 | CI 生成的受信配置快照；`[store]` 只写 `url_env = "ONTOFORGE_PG_URL"` 这个变量名，`[webhook]` 写明 target 与轮询/租约参数 |
| `target/phase8-webhook/ci/receiver.log` | 0 | 接收端 stdout（空：接收端只写 ledger，不打印） |
| `target/phase8-webhook/ci/receiver.err.log` | 0 | 接收端 stderr（空：接收端没有 panic 或错误输出） |
| `target/phase8-webhook/ci/worker.log` | 0 | worker stdout（空） |
| `target/phase8-webhook/ci/worker.err.log` | 0 | worker stderr（空：失败只经 outbox 退避重试，不打 panic） |

归档**不收**：CI 的私有 env / state / secret、auth 状态文件，以及 artifact 里存在但不属于预期证据集合的 `receiver.ready`（接收端 ready-file，只含绑定的回环 target URL）。排除项逐条记在索引的 `source_boundary.excluded` 里，不静默丢弃。

## 索引字段

| 字段 | 含义 |
|---|---|
| `schema_version` | 索引格式版本 |
| `created_at` | 索引生成时间（UTC） |
| `archive.path` / `archive.size` / `archive.sha256` | 归档文件自身相对本目录的路径、字节数与 SHA256 |
| `file_count` | 归档内的成员数，必须等于 `files` 数组长度 |
| `uncompressed_size` | 全部成员字节数之和 |
| `source_boundary.root` | 收集目录 |
| `source_boundary.runs` | 每个来源 run 的 `head_sha`、结论与成功 job id |
| `source_boundary.included` / `excluded` | 收了什么、没收什么 |
| `source_boundary.acceptance` | 边界声明：只证明归档完整性与可恢复性，不代替独立 phase 验收 |
| `files[].path` / `files[].size` / `files[].sha256` | 逐成员的路径、字节数与 SHA256，路径与归档成员路径逐字相同 |

## 核对与恢复

先核对归档自身的 SHA256，再让脚本解包复算：

```powershell
Get-FileHash .\ci-evidence.tar.gz -Algorithm SHA256
```

```bash
bash ci/verify-evidence-archive.sh ci-evidence.tar.gz ci-evidence-index.json
```

脚本把归档解到一个新的临时目录（用完即删），逐成员复算 SHA256 与字节数并与索引比对，核对成员数量，并拒绝对对路径、路径穿越、非普通文件成员（含 symlink 与硬链接）、重复成员、索引未列出的成员与成员数不一致。退出码 0 且输出 `OK: <N> members verified against <index>` 才算通过；各类失败有各自的退出码（2 输入缺失、3 绝对路径或路径穿越、4 非普通文件成员、5 重复成员、6 解包失败、7 索引自身不一致、8 归档自身摘要不符、9 索引成员缺失、10 成员 sha256 不符、11 成员字节数不符、12 归档有未列出成员、13 成员数不一致）。

校验器自身的可信度由 `bash ci/verify-evidence-archive.sh --selftest` 保证：它造一个最小 fixture，派生四个被篡改的变体（成员 sha256 被改、索引少列一个成员、成员被换成 symlink、成员路径重复），断言四个变体分别以退出码 10 / 12 / 4 / 5 被拒。

手工恢复时也必须解到**新的空目录**，成员路径保持仓库根相对路径：

```powershell
New-Item -ItemType Directory -Path .\restored-ci-evidence
tar -xzf .\ci-evidence.tar.gz -C .\restored-ci-evidence
```

## 边界声明

本归档只证明 CI 证据的完整性与可恢复性，**不代替独立 phase 验收**，也不关闭 REL-03（webhook-worker 崩溃后由 systemd unit 自动拉起属 Phase 9）。Phase 8 的结论位保持 `VERDICT: PENDING`，执行者不代签。

归档内容里不含 PostgreSQL 连接串或密码：CI 在上传前用 `bash ci/query-services.sh check-artifacts target/phase8-webhook` 以本次 job 的真实密码扫过一遍，归档前本地又复扫了一次。归档只证明证据完整性与可恢复性，不代替独立 phase 验收，也不关闭 REL-03。
