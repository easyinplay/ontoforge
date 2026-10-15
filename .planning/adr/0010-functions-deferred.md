# ADR-0010 Functions 延后到 v2，现在只定接口

- status: accepted
- date: 2026-10-15

## Decision
MVP 只定义 `FunctionRuntime` trait 与调用契约（输入/输出类型、只读 query function vs 产生 edits 的 edit function）。v2 先做 HTTP 远程函数，再做 `wasmtime` component。函数作者语言（WASM 语言无关 / TS 优先 / Rust）到 v2 规划时再决定。
