use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[tokio::test]
async fn real_stdio_process_bridges_authenticated_http_and_shuts_down_on_eof() {
    let (url, stop, task, started, finished) =
        ontoforge_rmcp_shell_spike::start_with_signals().await;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ontoforge-rmcp-shell-spike"))
        .arg(&url)
        .env("SPIKE_TOKEN", "alice")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"stdio-spike","version":"0"}}});
    input
        .write_all(format!("{init}\n").as_bytes())
        .await
        .unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), output.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let first: Value = serde_json::from_str(&first).expect("stdout contains only JSON-RPC");
    assert!(first.get("result").is_some(), "{first}");
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n")
        .await
        .unwrap();
    let list = tokio::time::timeout(Duration::from_secs(5), output.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let list: Value = serde_json::from_str(&list).unwrap();
    assert_eq!(list["result"]["tools"][0]["name"], "action_launch");
    input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"slow\",\"arguments\":{}}}\n").await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), started.cancelled())
        .await
        .unwrap();
    input.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":{\"requestId\":3,\"reason\":\"stdio-test\"}}\n").await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), finished.cancelled())
        .await
        .unwrap();
    drop(input);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
}
