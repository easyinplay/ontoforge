# 安装 Ontoforge 二进制

发行包包含 `ontoforge`（Windows 为 `ontoforge.exe`）与 `manifest.json`。每个平台另提供相同的旁置 manifest 与 archive 的 `.sha256` 文件。manifest 记录版本、源码 commit、Rust target、binary SHA256、构建 OS 与架构，Linux 还记录构建环境 glibc。

## 平台与运行环境

| 平台 | Rust target | 包格式 | native 验收环境 |
| --- | --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `.tar.gz` | GitHub `ubuntu-24.04` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` | GitHub `macos-15` |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `.zip` | GitHub `windows-2025` |

支持基线按上表的 OS/架构验收，不承诺更旧系统兼容。Linux 使用 Ubuntu 24.04 的 glibc 2.39 构建基线，具体版本写入 manifest；这是动态链接 GNU target，不是 musl/static 包，不能直接用于 Alpine。macOS 包仅供 arm64，不是 universal 或 Intel 包。Windows 使用 MSVC target。系统仍需提供对应平台的标准运行库。

运行已解包二进制无需 Rust、Cargo、Node.js、Python 或源码 checkout。`serve --dev` 内嵌演示 definition 和 fixture，自行创建可写数据目录。接入真实 source/store 时仍需按服务配置准备数据库与网络访问；TLS 的根证书来源遵循现有系统/服务配置。

## 下载与校验

从项目已发布的 GitHub Release 下载同一版本、同一 target 的 archive、`.manifest.json` 和 `.sha256`。PR 与 `workflow_dispatch` 的构建只保存在对应 Actions run 的 artifacts 中，不会创建 Release；这些验收产物的 commit 可能是 PR merge commit。

以下以 `0.1.0` 为例，选择所需版本并将文件放在同一目录。

Linux：

```sh
sha256sum -c ontoforge-0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf ontoforge-0.1.0-x86_64-unknown-linux-gnu.tar.gz
./ontoforge --version
./ontoforge --help
```

macOS：

```sh
shasum -a 256 -c ontoforge-0.1.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf ontoforge-0.1.0-aarch64-apple-darwin.tar.gz
./ontoforge --version
./ontoforge --help
```

Windows（PowerShell）：

```powershell
$archive = 'ontoforge-0.1.0-x86_64-pc-windows-msvc.zip'
$expected = (Get-Content "$archive.sha256" -Raw).Split(' ')[0]
if ((Get-FileHash $archive -Algorithm SHA256).Hash -ne $expected) { throw 'SHA256 mismatch' }
Expand-Archive $archive -DestinationPath ontoforge
Set-Location ontoforge
.\ontoforge.exe --version
.\ontoforge.exe --help
```

确认解包的 `manifest.json` 与旁置 manifest 内容相同，`version`、`target`、`commit` 对应所选 Release，binary 的 SHA256 等于 `binary_sha256`。SHA256 用于校验下载完整性，不是独立来源的签名或身份认证。

## 运行与数据保留

在新的工作目录启动服务（Windows 将 `./ontoforge` 替换为 `.\ontoforge.exe`）：

```sh
./ontoforge serve --dev --data-dir ./data --bind 127.0.0.1:8080
```

让服务自行创建 `data`，尤其 Unix 上不要预建宽权限目录。另一终端可使用 `dev-token --server-config ./data/server.toml --sub alice --groups dispatchers` 获取演示 token。REST 与 MCP 共用该服务，`mcp --server-url http://127.0.0.1:8080/mcp` 通过 `ONTOFORGE_TOKEN` 或 token file 接入；凭据不要写入日志或分享。

使用 Ctrl+C 正常关闭，随后以同一 data dir 启动，Action 修改、history 和 receipt 会保留。需要重新开始演示时，先正常停止服务，再显式添加 `--reset`；reset 清除演示修改并使旧 token 失效。完整配置和权限说明见 [dev.md](dev.md)。

## 发行验收与发布边界

`release.yml` 在三个 native runner 上实际构建 `cargo build --release --locked -p ontoforge --target <target>`，检查 OS、架构及 rustc host，再打包并重新解包校验。每个平台从 archive 安装到无 checkout 的临时目录，实际执行 version/help、query、Action、history、正常关闭重启、显式 reset 与 MCP stdio query。Windows 的 Python 标准库 helper 仅用于 CI 的隐藏 console 与 Ctrl+C 验证，不进入发行包。

只有三个 native smoke 全部成功，聚合 job 才核对本次 run 的三个独立 artifacts、版本、commit、target 和 checksum。未运行的平台不能视为已验收，具体证据见对应 Actions run。仅 `release: published` 事件可将已验证资产上传到该现有 Release，上传 job 才有 `contents: write`；同名资产冲突会失败，不覆盖。PR、手动验收、失败或取消的构建不会进入发布 job。workflow 不创建 Release、不发布 npm、不合并 PR。
