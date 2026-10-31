# TypeScript 7.x 与 `openapi-typescript` 兼容性核查

日期：2026-10-30  
范围：解释 `openapi-typescript@7.13.0` 在 `typescript@7.0.2` 下失败的原因，核对 TypeScript 7 API、TypeScript 6 side-by-side bridge、包解析行为和可复现方案。本文只记录研究，不修改依赖或业务实现。

## 结论

1. 失败是确定的 API/导出合同不兼容，不是 OpenAPI 文档内容问题。`openapi-typescript@7.13.0` 的 `dist/lib/ts.mjs` 在模块加载阶段执行 `ts.factory.createKeywordTypeNode(...)`；本地 `typescript@7.0.2` 的 `exports["."]` 指向 `./lib/version.cjs`，默认导出不再提供 `factory`、`SyntaxKind` 等旧 compiler API。调用因此在生成前即抛出 `TypeError`。
2. `openapi-typescript@7.13.0` 的官方 npm 元数据仍声明 peer dependency `typescript: ^5.x`。因此把应用的 compiler 升到 7.x 并不能视为该版本生成器已支持 TypeScript 7。
3. TypeScript 7.0 的官方迁移路径是保留 TypeScript 6 compiler API 与 TypeScript 7 compiler 并存。最小消费端 workaround 是让**名为 `typescript` 的包解析到 TypeScript 6**，并把 TypeScript 7 安装到另一个 alias（例如 `@typescript/native`），这样 `openapi-typescript` 的现有裸导入 `import ts from "typescript"` 仍拿到 TS6 API，而项目自己的 `tsc` 可显式使用 TS7 alias/命令。该方案是解析层 workaround，不是 `openapi-typescript` 对原生 TS7 AST API 的移植。
4. 仅增加一个名为 `typescript6` 或 `@typescript/typescript6` 的 alias 不会改变生成器的裸 specifier。Node/包管理器仍按 `import "typescript"` 查找 `node_modules/typescript`；只有把 alias 的**依赖键**设为 `typescript`（或让生成器自身携带/导出兼容 compiler）才会改变解析结果。修改 lockfile 后应清理并重新安装，以确认 hoisting/isolated install 没有留下 TS7 实例。

## 已验证事实

### 本地代码与运行时

- `clients/typescript/package.json:15-18` 同时声明 `openapi-typescript: 7.13.0` 与 `typescript: 7.0.2`。
- `clients/typescript/node_modules/typescript/package.json:37-50`：
  - `exports["."]` 为 `./lib/version.cjs`；
  - 新入口位于 `./unstable/sync`、`./unstable/async`、`./unstable/ast`、`./unstable/ast/factory` 等。
- `clients/typescript/node_modules/openapi-typescript/dist/lib/ts.mjs:1-2,11-20`：模块执行 `import ts from 'typescript'`，随后立即读取 `ts.factory`、`ts.SyntaxKind` 并创建 TypeScript AST 节点。
- 本地实测（此前记录）：`node` 导入 `typescript@7.0.2` 时 `factory`、`createPrinter`、`SyntaxKind` 均为 `undefined`；所以错误发生在 `openapi-typescript` 生成器初始化阶段。

### 官方/上游包元数据

- `npm view openapi-typescript@7.13.0 peerDependencies` 返回 `{ "typescript": "^5.x" }`。同一包的 `repository.url` 为 `https://github.com/openapi-ts/openapi-typescript.git`。
- TypeScript 7.0.2 的 npm `exports` 与本地安装一致：默认 `.` 指向 `./lib/version.cjs`，AST/compiler 能力通过 `unstable/*` 子路径暴露，而不是旧版默认 `typescript` 对象的完整 compiler API。
- TypeScript 6 side-by-side 包存在：`npm view @typescript/typescript6@6.0.2` 返回版本 `6.0.2`，并提供 `tsc6` binary；TypeScript 7.0.2 自身还将平台 native 包列为 optional dependency（本地 `package.json:65-75`）。

## 官方 TypeScript 7 说明与 API bridge

Microsoft 的 TypeScript 7.0 发布说明明确将迁移分成两类：TypeScript 7 的新 native compiler/语言服务，以及需要旧 JavaScript compiler API 的工具。对后者，官方建议在 TypeScript 7 与 TypeScript 6 之间 side-by-side 运行，并从 TypeScript 6 继续取得旧 API。发布说明中的示例依赖形态与本次 issue 讨论一致：

```json
{
  "@typescript/native": "npm:typescript@7.0.2",
  "typescript": "npm:@typescript/typescript6@6.0.2"
}
```

这里的关键不是 alias 名称，而是依赖键 `typescript`。`openapi-typescript` 的构建产物没有导入 `@typescript/native` 或 `typescript/unstable/ast`，它只导入裸 specifier `typescript`。因此只有让这个 specifier 解析到 TS6，现有代码才会拿到 `factory`/`SyntaxKind`。

**事实边界：** TypeScript 官方资料支持“TS7.0 保留 TS6 以供旧 compiler API 工具使用”以及 TS7 的新 API 通过 `unstable/*` 暴露。未找到官方承诺 `openapi-typescript@7.13.0` 已兼容 TS7，也未找到官方的自动 compatibility bridge 会把第三方裸导入重写到 `unstable/*`。后者是根据 Node package exports 和本地导入路径作出的解析推论。

## `openapi-typescript` 上游证据

- 上游 issue [#2841](https://github.com/openapi-ts/openapi-typescript/issues/2841) 的错误与本地完全相同：`ts.factory.createKeywordTypeNode` 处 `TypeError: Cannot read properties of undefined`，环境为 `openapi-typescript 7.13.0`、`typescript 7.0.2`。
- 该 issue 的讨论引用 TypeScript 官方 side-by-side 说明，并给出将 `typescript` alias 到 `@typescript/typescript6`、将 TS7 alias 到 `@typescript/native` 的消费端 workaround。讨论同时记录了 lockfile/Bun hoisting 可能使 alias 看似未生效，清理 `node_modules` 与 lockfile 后重装才可验证。
- issue 后续讨论提出的“让 `openapi-typescript` 自带 TS5 compiler 并导出 `ts`”是一个**未合并的提议**，不是当前 7.13.0 行为，也不应当当作已接受的上游方案。该提议还明确这属于 compiler isolation，而非把生成器移植到 native TS7 AST API。

## 最小可复现矩阵

| 方案 | `openapi-typescript` 的 `import "typescript"` 实际解析 | 结果 |
|---|---|---|
| 当前项目：`typescript: 7.0.2` | TS7 `lib/version.cjs` | 失败：`ts.factory` 为 `undefined` |
| 只新增 `@typescript/typescript6` 或 `typescript6`，保留 TS7 为 `typescript` | 仍为 TS7 | 失败；alias 名没有改变裸 specifier |
| `typescript: npm:@typescript/typescript6@6.0.2`，TS7 另命名为 `@typescript/native: npm:typescript@7.0.2` | TS6 | 生成器可继续使用旧 compiler API；应用需显式选择 TS7 的 CLI/API |
| 临时命令：`npx -p typescript@6 -p openapi-typescript openapi-typescript ...` | 命令隔离环境中的 TS6 | 可复现 workaround；不能证明项目 lockfile/monorepo 安装布局已正确 |
| 修改生成器源码为 `typescript/unstable/*` | 取决于完整 AST API 适配 | 未验证；不能只改一个 import，因现有代码大量使用旧 `ts.factory`/`ts.SyntaxKind` 合同 |

## 推论与未证实项

### 推论（有本地导入/exports 证据）

- “只 alias TS5 无法自动改变生成器 import”准确表述应是：alias 必须覆盖裸包名 `typescript`，且解析结果必须落到提供旧 compiler API 的版本。把 TS6 安装为另一个包名，不会让 Node 自动重写已经编译好的 `import 'typescript'`。
- 若 root 同时需要 TS7 的 `tsc`，不能继续依赖未命名的 `typescript` 作为 TS7；应使用别名命令或独立脚本显式调用 TS7。否则会解决生成器后又把项目自身 typecheck 降回 TS6。

### 未证实（截至 2026-10-30）

- 未证实 TypeScript 7.1 或更晚版本的 native API 已被 `openapi-typescript` 7.x 适配；当前上游 issue 只证明 7.13.0 在 7.0.2 失败。
- 未证实 `openapi-typescript` 是否会在下一个 major 版本移除 TypeScript runtime dependency、改用字符串模板，或内置 compiler。issue 中这些是讨论中的候选方案，不是发布合同。
- 未运行 pnpm/Bun/npm 的所有 hoisting 模式；具体 alias 语法应在目标包管理器中通过干净 lockfile 安装和 `node -e "import('typescript')..."` 复测。

## 来源

### Web / 上游

1. Microsoft TypeScript 7.0 发布说明（含 side-by-side TypeScript 6 迁移路径）：https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/
2. `openapi-typescript` issue #2841（TS7.0.2 复现、side-by-side workaround、后续讨论）：https://github.com/openapi-ts/openapi-typescript/issues/2841
3. `openapi-typescript` package metadata（main 分支，7.13.0 package.json，peer dependency `typescript: ^5.x`）：https://github.com/openapi-ts/openapi-typescript/blob/main/packages/openapi-typescript/package.json
4. TypeScript 7.0.2 package metadata（exports 与 `unstable/*` 子路径）：https://github.com/microsoft/TypeScript/blob/v7.0.2/package.json
5. npm metadata：`openapi-typescript@7.13.0` peer dependency：https://www.npmjs.com/package/openapi-typescript/v/7.13.0

### 本地证据

- `clients/typescript/package.json:15-18`
- `clients/typescript/node_modules/typescript/package.json:37-50,65-75`
- `clients/typescript/node_modules/openapi-typescript/dist/lib/ts.mjs:1-2,11-20`

## 给主会话的决策输入

- 当前冲突应记录为“TS7 runtime compiler API 与 `openapi-typescript@7.13.0` 的 peer/runtime 合同不兼容”，不应将生成失败归因于 OpenAPI schema。
- 最小、可回滚、无需改业务实现的验证路径是消费端 side-by-side：`typescript` 解析到 TS6，TS7 通过别名保留；验证前清理 lockfile/`node_modules` 并检查 `import('typescript')` 的 `factory` 与 `SyntaxKind`。
- 不应把 issue 评论中的未来方案或前次 Grok 会话内容写成上游推荐。当前上游事实只有失败复现、TS6 alias workaround 讨论，以及尚未合并的 compiler isolation/模板化提议。
