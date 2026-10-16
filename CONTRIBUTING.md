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

## 编辑器接入

- 定义文档与 manifest 的 JSON Schema 位于 `schemas/`，由 `ontoforge schema export` 生成；IR 改动后需重新导出。
- yaml-language-server 的 modeline（`# yaml-language-server: $schema=...`）只作用于它所在的那个 YAML document，不会延续到 `---` 之后的 document，见 https://github.com/redhat-developer/vscode-yaml/issues/1066 。因此 `examples/ontologies/aviation` 的定义文件在每个 `---` 的下一行都写一次定义 modeline。新增 document 时同样写在 `---` 的下一行，测试 `aviation_definition_documents_start_with_modeline` 会检查。
- 多文档文件以项目级 `yaml.schemas` 为准。仓库提供 `.zed/settings.json`（Zed，位于 `lsp.yaml-language-server.settings.yaml.schemas`）与 `.vscode/settings.json`（VS Code + Red Hat YAML 扩展，`yaml.schemas`），把 `./schemas/ontoforge-definition.schema.json` 关联到 `examples/ontologies/**/*.yaml`，并用 `!examples/ontologies/**/ontology.yaml` 排除 manifest。
- manifest 通过自己第 1 行的 manifest modeline 关联 manifest schema（modeline 优先于 settings）。没有 modeline 的 manifest 不关联任何 schema，不会被误按定义 schema 校验。
- 两份配置必须保持一致，且只写严格 JSON（不写注释），由测试 `editor_configs_associate_definition_schema_with_examples` 检查。
- JetBrains IDE 不读取上述两个文件：在 Settings → Languages & Frameworks → Schemas and DTDs → JSON Schema Mappings 中手动添加映射，`schemas/ontoforge-definition.schema.json` 对应定义文件，`schemas/ontoforge-manifest.schema.json` 对应 `ontology.yaml`。
- 补全触发：yaml-language-server 不声明补全触发字符，输入 `kind: ` 后不会自动弹出补全，需要按 Ctrl+Space（或键入值的首字母）。
