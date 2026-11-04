---
status: research-complete-runtime-blocked
queried: 2026-11-04
owner: minio_fixture_feasibility
---

# MinIO fixture 恢复可行性

## 结论

**原 JDBC catalog + MinIO 合同在 Trino 483/484 配置层可恢复；未找到可匿名获取并固定 digest 的官方预编译 MinIO artifact，不能直接恢复旧 `docker run` 路径。** 有条件的保合同路径是固定官方源码、自建测试 artifact，但仍需落实构建和运行验证，当前没有成品镜像 digest。ENGINE-05/ADR0017 gap 保留，local warehouse 证据不等于 S3 验证。

## 项目证据复用

- [PHASE-02-04-REVERIFY](PHASE-02-04-REVERIFY.md) 与 [DYNAMIC-VALIDATION](DYNAMIC-VALIDATION.md) 已给出 local CTAS 的 483/484 各 1 个实际测试；本研究没有重复代码审计。
- `../../adr/0017-engine-and-trino-connection.md` accepted CI 决策，以及 `../../phases/02-source-binding-engine/02-10-PLAN.md` Task 3 明确要求 JDBC catalog + MinIO。当前 `ci/trino/iceberg/iceberg.properties:13` 是 `local:///warehouse`。
- `02-10-SUMMARY.md:52,211–218` 披露 Docker Hub `object not found`、两 registry 匿名 pull 401，以及避免未经确认第三方镜像的供应链原因。这不是 accepted 替代 ADR。

## 官方 artifact 实查

查询时间 **2026-11-04 03:37 UTC** 起。只做 HTTP GET / GitHub API / Context7 / Web 核查。完整 URL、响应与 body SHA-256 在 `target/minio-feasibility/http-metadata.json`；匿名鉴权结果在 `registry-auth-metadata.json`，不保存或输出 bearer token。

| 对象 | 结果 | 身份与限制 |
|---|---|---|
| Docker Hub `minio/minio` repository API | 404 | 无公开 repository metadata |
| Docker registry 的 `RELEASE.2025-09-07T16-13-09Z` 与 `RELEASE.2025-10-15T17-29-55Z` manifest | 匿名 token endpoint 200，`granted_access=[]`；两个 manifest 均 401 | **没有可记录 manifest digest 或 linux/amd64 子 manifest** |
| Quay `minio/minio` 同两 tag manifest | challenge 后申请匿名 token 成功，但 repository `actions=[]`；两个 manifest 均 401 | 不是误读初始认证 challenge；无可获取 manifest/digest |
| Quay repository 与固定 September tag API | 401 | 未借助用户登录或 secrets |
| `dl.min.io/server/minio/release/linux-amd64/archive/` | 410 Gone | archive 入口不可用；未穷举全部历史文件，不能断言所有历史副本不存在 |
| 官方 GitHub `RELEASE.2025-10-15T17-29-55Z` | release/tag API 200，`assets=[]` | 源码 commit **`9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a`**；不是 OCI digest |

[官方 README](https://github.com/minio/minio) 明确 community source-only，不再提供预编译 binary；repository API 显示 `archived=true`。README 声称历史 binary 可参考，不等于本次访问成功。[固定 release](https://github.com/minio/minio/releases/tag/RELEASE.2025-10-15T17-29-55Z) 提供固定 tag 的源码安装路径，指出修复一项 privilege escalation；本报告不声明该版本在查询日没有其他漏洞。

**官方 `make docker` 不能原样绕过阻塞。** 该 commit 的 [Dockerfile](https://github.com/minio/minio/blob/9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a/Dockerfile) 首行是 `FROM minio/minio:latest`；[Makefile](https://github.com/minio/minio/blob/9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a/Makefile) 的 docker target 使用它，仍依赖不可取得且未固定的原镜像。原文保存在 `target/minio-feasibility/minio-{Dockerfile,Makefile}.body`。

## Trino 483/484 配置

两版本官方 S3 文档支持 Iceberg + MinIO，配置主名称为 `fs.s3.enabled=true`。[483 FileSystemConfig](https://github.com/trinodb/trino/blob/483/lib/trino-filesystem-manager/src/main/java/io/trino/filesystem/manager/FileSystemConfig.java) 和 [484 FileSystemConfig](https://github.com/trinodb/trino/blob/484/lib/trino-filesystem-manager/src/main/java/io/trino/filesystem/manager/FileSystemConfig.java) 均保留 `@LegacyConfig("fs.native-s3.enabled")`。原 PLAN 的旧开关仍兼容，无需升级 Trino 或改业务接口。

保留现有 JDBC 属性，删除 `fs.local.enabled` / `local.location`，恢复以下配置；一次性 `s3.aws-access-key` 与 `s3.aws-secret-key` 由隔离 runner 注入，本报告不写凭据：

```properties
iceberg.jdbc-catalog.default-warehouse-dir=s3://warehouse/
fs.s3.enabled=true
s3.endpoint=http://localhost:9000
s3.path-style-access=true
s3.region=us-east-1
```

`localhost` 适用于现有 Linux host network；若使用独立 Docker network，endpoint 要改为服务名。来源：[483 S3](https://github.com/trinodb/trino/blob/483/docs/src/main/sphinx/object-storage/file-system-s3.md)、[484 S3](https://github.com/trinodb/trino/blob/484/docs/src/main/sphinx/object-storage/file-system-s3.md)、[483 JDBC](https://github.com/trinodb/trino/blob/483/docs/src/main/sphinx/object-storage/metastores.md)、[484 Iceberg](https://github.com/trinodb/trino/blob/484/docs/src/main/sphinx/connector/iceberg.md)。已由 Context7 版本化查询与官方原文交叉核对，原文在 `target/minio-feasibility/trino*.body`。

## 可行路径和最小改动

1. **直接消费官方预编译镜像：当前阻塞。** 没有可核验 manifest，不猜 digest，不用 `latest`、不可信镜像站或另一 S3 产品冒充 MinIO。
2. **固定官方源码自建：有条件可行，未实施。** 固定上述 commit、依赖和 Go builder/运行基础镜像 digest，替换依赖 `minio/minio:latest` 的 Dockerfile，构建自己的测试 artifact。该 `go.mod` 声明 `go 1.24.0`、`toolchain go1.24.8`；这只是源码要求，尚未核实 builder artifact、构建结果或可复现性。先运行自编 binary 可以补 MinIO 行为证据，但还需版本/checksum/provenance，原容器化 nightly 的恢复仍要收口。源码可读不等于成品可运行。
3. **取得可信 artifact 后：** 修改 `ci/trino/iceberg/iceberg.properties`；在 `.github/workflows/nightly.yml` 增加隔离 MinIO、健康等待、一次性身份、AWS CLI 建 `warehouse` bucket 与 always cleanup，移除 local warehouse mount；更新 `CONTRIBUTING.md`。自建路径另需明确构建 recipe/provenance 文件。现有 JDBC schema SQL、flights CTAS 与 Rust named test 可复用。
4. **先隔离补证：** 不触发会发 issue 的 nightly。沿当前 Trino 483/484 pinned digests，各用新 PG JDBC metadata、MinIO bucket 与 Trino runtime；执行原 schema/CTAS，再在两环境变量齐备时执行 `iceberg_table_is_readable`。每版本发现数/执行数均为 1，source=29、copied=29；另保留 bucket 对象列表或 Iceberg metadata 的 `s3://warehouse/` location，排除 local fallback。清理全部专属资源并扫描日志凭据。执行须由主会话协调唯一 Cargo/Docker owner。

## 边界与流程

- 未 pull/build/run、下载容器 layers、Cargo、CTAS 或运行测试；未验证成品 checksum、OCI signature、镜像漏洞或全部历史 tag。结论仅覆盖配置兼容性和 artifact 获取可行性。
- 未改生产、CI、ADR、共享 STATE/ROADMAP/task_plan，不 commit、不发外部消息、不触发 nightly/发布。
- 已调用 `planning-with-files`，通过其 PowerShell resolver pin `.planning/milestones/v1.0-audit`。本 worker 仅维护本报告和 `target/minio-feasibility/`；共享计划归主会话 owner。旧证据复用、artifact metadata、版本核查、报告四步已完成。
- 跳过 office-hours/CEO、Phase discuss、brainstorming：已有明确合同的只读补证。TDD 不适用：文档和只读证据脚本，不改实现。Warp/ralph-loop 不可用，未伪称调用。
- `.codex/rules/web-search.md` 缺失，已读 `.claude/rules/web-search.md`；未发现 Tavily/Exa，使用 `web.run`，GitHub API 用 `gh`，库文档先 `ctx7 library` 后查询 483/484。一次 PowerShell wildcard 查询失败，已改真实目录；一次 apply_patch 同文件删除/新增被拒绝，改为顺序调用后成功。

**研究交付：COMPLETE；MinIO runtime 验证：未执行；合同 gap：未关闭。**

## 2026-11-04 构建前只读补核

主会话已核对原合同不要求官方预编译/签名镜像，固定官方源码自建无需降低 MinIO 验收标准。纯事实独立咨询在 `target/minio-restoration-independent-opinion.json`；以下只核查方案输入，不将建议或 metadata 当作构建证据。

- 官方 `docker.io/library/golang:1.24.8-bookworm` manifest 查询 200，index SHA-256 与响应头一致：`sha256:4ed690d6649d63c312b99a6120025ec79ce3b542968a37da53d6236c7c61a848`。
- 唯一 `linux/amd64` descriptor 与直接读取的 child manifest 重算一致：**`sha256:39e2f344b68e61144a5de3d3bd166ea746ee8e1965e6fc2ad046df5cd6c8369e`**。可固定 `docker.io/library/golang:1.24.8-bookworm@sha256:39e2f344b68e61144a5de3d3bd166ea746ee8e1965e6fc2ad046df5cd6c8369e` 并声明 `linux/amd64`。
- config blob 重算与 manifest 一致：`sha256:a67b28d7fd1bb163a378f3b5526c2c0c9210804f765254d86907deda4651eced`，平台 `linux/amd64`，`GOLANG_VERSION=1.24.8`、`GOTOOLCHAIN=local`。仅下载 manifest/config JSON，**7 个 filesystem layers 均未下载**，不宣称完整 pull 成功或已运行版本检查。
- index annotation 指向 [docker-library/golang 固定源码](https://github.com/docker-library/golang/tree/1538c1993363659087125a7f2a18f5136ba12de2/1.24/bookworm)，固定 revision `1538c1993363659087125a7f2a18f5136ba12de2`；官方镜像说明为 [Docker Hub golang](https://hub.docker.com/_/golang)。这证明公开来源和内容身份，不是签名或漏洞审计。
- 原始证据：`golang-builder-metadata.json`、`golang-index.json`、`golang-amd64-manifest.json`、`golang-amd64-config.json`、`golang-builder-Dockerfile.body`，均位于 `target/minio-feasibility/`。

### 本机 AWS CLI 与最小源码构建

- Windows `Get-Command aws` 无结果，默认 `C:/Program Files/Amazon/AWSCLIV2/aws.exe` 不存在。Ubuntu-26.04 WSL 的 `command -v aws` 无结果，`/usr/local/aws-cli/v2/current/bin/aws`、`/usr/local/bin/aws`、`/usr/bin/aws` 均无可执行文件。**现有可调用 AWS CLI 未找到**；没有扫描全部磁盘，不能断言任何目录都没有副本。证据 `local-tools.json`、`wsl-tools.txt`。bucket 创建客户端尚是执行准备缺口，本轮没有安装客户端或调用 AWS API。
- Windows Go 实际版本是 `go1.27.3 windows/amd64`，WSL PATH 没有 Go；没有使用宿主新版 Go 替代所要求的 1.24.8 builder。
- 固定源码 [Makefile:179](https://github.com/minio/minio/blob/9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a/Makefile#L179) 的 server 构建使用 `CGO_ENABLED=0`、`-tags kqueue`、`-trimpath` 和生成的 ldflags。`make build` 还会运行 checks 和 docs/debugging 的工具构建，最小 server artifact 可以仅执行该 server recipe。
- 固定源码 [gen-ldflags.go](https://github.com/minio/minio/blob/9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a/buildscripts/gen-ldflags.go) 只依赖 Go 标准库，但调用 Git 读取 commit/time；必须使用含 `.git` 的固定 checkout，不能直接对无 Git 信息的 tarball 调用。默认 release prefix 是 `DEVELOPMENT`，保留它不会将自建物冒充官方发布 binary。`go.mod`/`go.sum` 需原样保留，依赖下载与 checksum 校验仍需要网络；本轮未下载 Go modules。

以下为**未执行的 Linux builder 命令草案**，在固定源码 checkout 根目录执行，输出目录预先创建且可写。它复用官方 server 配方，禁自动 toolchain 升级，保留 module checksum 校验；没有调用 Dockerfile：

```sh
set -eu
test "$(git rev-parse HEAD)" = 9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a
export GOTOOLCHAIN=local
export CGO_ENABLED=0 GOOS=linux GOARCH=amd64
export GOFLAGS=-mod=readonly
export GOPROXY=https://proxy.golang.org GOSUMDB=sum.golang.org
unset GOPRIVATE GONOPROXY GONOSUMDB
go version
go mod download
go mod verify
minio_ldflags=$(MINIO_RELEASE=DEVELOPMENT go run buildscripts/gen-ldflags.go)
go build -tags kqueue -trimpath -ldflags "$minio_ldflags" -o /out/minio .
git diff --exit-code -- go.mod go.sum
sha256sum /out/minio
```

`CGO_ENABLED=0` 与官方配方一致，不需要 C ABI 链接；是否得到可用静态 executable、是否适合 scratch runtime、实际 binary 版本与完整依赖可获取性，仍需后续构建与检查。builder pin 已明确，AWS CLI 暂缺，MinIO 构建和 S3 验证仍全部未执行。

补核状态：COMPLETE。已解析 builder/local-tools JSON、校验平台 descriptor 和只读脚本语法；没有安装工具、启动服务、构建、Cargo/Docker 容器、commit 或发布。


## 独立意见与后续边界

主会话核对target/minio-restoration-independent-opinion.json：咨询只给需求、事实与约束，未给我方方案。独立推荐固定官方源码、自建测试artifact、隔离验证，保留原MinIO/S3合同。与主会话判断同向；原02-10 PLAN要求MinIO而未要求官方签名预编译镜像，因此在原授权范围自动推进，不修订验收标准。当前仍未构建或运行，bucket客户端及CI最小接入在7.1收口后细化。实际模型为MiniMax-M3.1-Flash[1m]，通过Claude Code调用。
