use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

#[path = "../../ontoforge-server/tests/support/mod.rs"]
mod support;

struct BridgeChild(std::process::Child);

impl Drop for BridgeChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn live_stdio_cancellation_maps_string_id_without_cancelling_other_read() {
    live_stdio_cancellation_roundtrip("2025-11-25").await;
}

#[tokio::test]
async fn live_stdio_legacy_initialize_negotiates_locally() {
    live_stdio_cancellation_roundtrip("2025-03-26").await;
}

async fn live_stdio_cancellation_roundtrip(version: &str) {
    use serde_json::{Value, json};
    use std::io::{BufRead, BufReader};
    use std::sync::atomic::Ordering;
    let running = support::fixture().await.start().await;
    running.engine.block.store(true, Ordering::SeqCst);
    let mut child = BridgeChild(
        Command::new(env!("CARGO_BIN_EXE_ontoforge"))
            .args(["mcp", "--server-url", &format!("{}/mcp", running.url)])
            .env("ONTOFORGE_TOKEN", &running.token)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                let _ = tx.send(value);
            }
        }
    });
    let stdin = child.0.stdin.as_mut().unwrap();
    writeln!(stdin, "{}", json!({"jsonrpc":"2.0","id":"local-init","method":"initialize","params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"stdio-test","version":"0"}}})).unwrap();
    let initialized = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(initialized["id"], "local-init");
    assert!(initialized["error"].is_null(), "{initialized}");
    assert_eq!(initialized["result"]["protocolVersion"], version);
    assert!(initialized["result"]["capabilities"]["tools"].is_object());
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    for id in [json!("local-read"), json!(777)] {
        writeln!(stdin, "{}", json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"query","arguments":{"request":{"set":{"kind":"type","object_type":"Flight"},"select":["flightNumber"],"page":{"first":3}}}}})).unwrap();
        tokio::time::timeout(Duration::from_secs(5), running.engine.entered.notified())
            .await
            .unwrap();
    }
    writeln!(stdin, "{}", json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"local-read"}})).unwrap();
    tokio::time::timeout(Duration::from_secs(2), running.engine.cancelled.notified())
        .await
        .expect("bridge must cancel the corresponding remote request");
    running.engine.block.store(false, Ordering::SeqCst);
    running.engine.release.notify_one();
    loop {
        let response = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        if response["id"] == 777 {
            assert!(
                response["error"].is_null(),
                "unrelated request must complete: {response}"
            );
            break;
        }
    }
    drop(child.0.stdin.take());
    let status = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("stdio EOF must terminate the bridge");
    assert!(status.success());
    reader.join().unwrap();
    let audit = running.finish().await;
    assert_eq!(
        audit
            .iter()
            .filter(|entry| entry["status"] == "cancelled")
            .count(),
        1
    );
    assert_eq!(
        audit
            .iter()
            .filter(|entry| entry["status"] == "success")
            .count(),
        1
    );
}

#[test]
fn stdio_absent_server_keeps_stdout_empty_and_redacts_token() {
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
fn stdio_absent_server_and_invalid_url_fail_without_leaking_token() {
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
fn stdio_absent_server_exits_with_queued_input() {
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
