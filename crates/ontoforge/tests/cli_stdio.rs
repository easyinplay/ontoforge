use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

#[test]
fn stdio_real_process_describe_query_action_dryrun_history() {
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n"
    );
    let output = run_mcp("http://127.0.0.1:1/mcp", "stdio-secret-sentinel", input);
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "stdout must contain only MCP frames"
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("stdio-secret-sentinel"));
}

#[test]
fn stdio_server_absent_bad_token_redirect_and_expired_session() {
    let absent = run_mcp("http://127.0.0.1:1/mcp", "bad-token-sentinel", "");
    assert!(!absent.status.success());
    assert!(absent.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&absent.stderr).contains("bad-token-sentinel"));

    let redirect = run_mcp(
        "http://user:pass@127.0.0.1:1/mcp",
        "redirect-secret-sentinel",
        "",
    );
    assert!(!redirect.status.success());
    assert!(redirect.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&redirect.stderr).contains("redirect-secret-sentinel"));
}

#[test]
fn stdio_long_query_cancel_eof_and_shutdown() {
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":41,\"method\":\"tools/call\",\"params\":{\"name\":\"query\"}}\n",
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":{\"requestId\":41}}\n"
    );
    let started = std::time::Instant::now();
    let output = run_mcp("http://127.0.0.1:1/mcp", "shutdown-secret-sentinel", input);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("shutdown-secret-sentinel"));
}

fn run_mcp(url: &str, token: &str, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(["mcp", "--server-url", url])
        .env("ONTOFORGE_TOKEN", token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ontoforge");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write MCP input");
    drop(child.stdin.take());
    child.wait_with_output().expect("wait ontoforge")
}
