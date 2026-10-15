# 贡献指南

## 本地门禁

提交前在仓库根目录运行以下三条命令，与 CI（Windows 与 Ubuntu）一致，不依赖 Docker：

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

工具链由 `rust-toolchain.toml` 固定为 1.98.0，CI 使用同一版本。

## 快照测试（insta）

诊断输出、CLI 输出等用 `insta` 快照固定。

- 更新快照：`INSTA_UPDATE=always cargo test -p <crate>`。本机未装 `cargo-insta` 也可使用。
- 更新后用 `git diff` 逐个审阅 `.snap` 的变化，确认是预期改动再提交。
- CI 环境（`CI=true`）下 insta 不写快照，缺失或不一致的快照会让测试失败，这是期望行为。
- 快照里不要出现绝对路径、耗时或机器相关内容。

## 行尾与环境变量

- 行尾由 `.gitattributes` 统一为 LF，不要提交 CRLF 的 fixture 或快照。
- 测试里不得调用 `std::env::set_var`：edition 2024 下它是 `unsafe`，会被 `unsafe_code = "forbid"` 拦截。需要给子进程设环境变量时使用 `Command::env`。

## invalid 样例

- 位置：`examples/ontologies/invalid/<code>-<slug>/`。
- 目录名前缀决定期望的诊断码，例如 `v04-unknown-field` 期望 `ONT-V04`。
- 新增样例只需加目录并生成快照，不需要改测试代码。

## 诊断码

诊断码（`ONT-V*`、`ONT-E*`、`ONT-W*`）一经发布不得复用或改义。废弃的诊断码保留编号，不再分配给新诊断。

## 命名与来源

- 命名遵循 ADR-0002（crate 前缀 `ontoforge-`）。
- 禁止沿用参考仓库的标识符，禁止复制参考仓库的代码。
- ontology 定义文档的格式以 ADR-0012 为准。
