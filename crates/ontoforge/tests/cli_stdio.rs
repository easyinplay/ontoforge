use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

#[path = "../../ontoforge-server/tests/support/mod.rs"]
mod support;

mod stdio_security_support;

#[tokio::test]
async fn live_stdio_two_principals_validate_action_history_and_rest_receipt() {
    use serde_json::json;
    use stdio_security_support::{
        Cli, DEADLINE, action, counts, durable_rows, http_json, principal_token,
    };
    let mut fixture = support::fixture().await;
    fixture.token = principal_token(&fixture, "alice", &["dispatchers", "crew-schedulers"]);
    let mia = principal_token(&fixture, "mia", &[]);
    let running = fixture.start().await;
    for (token, allowed, denied, property, action_name) in [
        (
            &running.token,
            "Flight",
            "Aircraft",
            "flightId",
            "action_delayFlight",
        ),
        (
            &mia,
            "Aircraft",
            "Flight",
            "tailNumber",
            "action_recordMaintenance",
        ),
    ] {
        check_stdio_principal_reads(&running, token, allowed, denied, property, action_name).await;
    }
    let mut cli = Cli::spawn(&format!("{}/mcp", running.url), &running.token);
    cli.initialize().await;
    let before = durable_rows(&running.path).await;
    let before_counts = counts(&running.path).await;
    let mut validate = action(30, "stdio-success-command");
    validate["params"]["arguments"]["validateOnly"] = json!(true);
    let validated = cli.request(&validate).await;
    assert_eq!(validated["structuredContent"], json!({"validated":true}));
    assert_eq!(
        durable_rows(&running.path).await,
        before,
        "authorized validation must not write any durable business row"
    );

    let committed = cli.request(&action(31, "stdio-success-command")).await;
    let receipt = committed["structuredContent"]["receipt"].clone();
    assert!(receipt["eventId"].is_string());
    assert!(receipt["committedAt"].is_string());
    assert_eq!(receipt["commandId"], "stdio-success-command");
    let after = counts(&running.path).await;
    assert_eq!(
        std::array::from_fn::<_, 5, _>(|index| after[index] - before_counts[index]),
        [1; 5]
    );
    let after_rows = durable_rows(&running.path).await;
    let history = cli
        .tool(32, "history", json!({"objectType":"Flight","pk":"F-1001"}))
        .await;
    let rest_history = http_json(
        running
            .client
            .get(format!("{}/api/objects/Flight/F-1001/history", running.url))
            .bearer_auth(&running.token),
    )
    .await;
    assert_eq!(history, rest_history);
    assert_eq!(history["entries"].as_array().unwrap().len(), 1);
    let result = http_json(
        running
            .client
            .get(format!(
                "{}/api/commands/stdio-success-command",
                running.url
            ))
            .bearer_auth(&running.token),
    )
    .await;
    assert_eq!(result, receipt);
    let replay = http_json(running.client.post(format!("{}/api/actions/delayFlight", running.url)).bearer_auth(&running.token)
        .json(&json!({"commandId":"stdio-success-command","items":[{"params":{"flight":"F-1001","delayMinutes":5}}]}))).await;
    assert_eq!(
        replay, receipt,
        "REST must replay the same full stdio receipt"
    );
    assert_eq!(durable_rows(&running.path).await, after_rows);
    check_mia_denials_preserve_alice_receipt(&running, &mia, &receipt).await;
    assert!(cli.finish(&running.token).await.success());
    let audit = tokio::time::timeout(DEADLINE, running.finish_actions())
        .await
        .unwrap();
    for principal in ["ontoforge-dev|alice", "ontoforge-dev|mia"] {
        assert!(
            audit
                .iter()
                .any(|entry| entry["principal"] == principal && entry["status"] == "success")
        );
    }
}

async fn check_mia_denials_preserve_alice_receipt(
    running: &support::Running,
    mia: &str,
    receipt: &serde_json::Value,
) {
    use serde_json::{Value, json};
    use stdio_security_support::{Cli, action, durable_rows};
    let before = durable_rows(&running.path).await;
    let mut cli = Cli::spawn(&format!("{}/mcp", running.url), mia);
    cli.initialize().await;
    let submit = action(40, "stdio-mia-denied-command");
    let mut validate = action(41, "stdio-mia-denied-validation");
    validate["params"]["arguments"]["validateOnly"] = json!(true);
    let history = json!({"jsonrpc":"2.0","id":42,"method":"tools/call","params":{
        "name":"history","arguments":{"objectType":"Flight","pk":"F-1001"}
    }});
    let replay = action(43, "stdio-success-command");
    for request in [submit, validate, history, replay] {
        cli.send(&request);
        let response = cli.receive().await.expect("Mia direct-call response");
        assert_eq!(response["id"], request["id"]);
        assert_eq!(response["error"]["code"], -32603, "{response}");
        let message = response["error"]["message"].as_str().unwrap();
        let envelope: Value = serde_json::from_str(
            message
                .strip_prefix("Mcp error: -32602: NOT_FOUND(")
                .and_then(|data| data.strip_suffix(')'))
                .expect("bridge must preserve the remote NOT_FOUND rejection in its error message"),
        )
        .unwrap();
        assert_eq!(envelope["code"], "NOT_FOUND");
        assert_eq!(envelope["apiName"], request["params"]["name"]);
        assert!(response.get("result").is_none());
        assert_alice_receipt_unchanged(running, &before, receipt).await;
    }
    assert!(cli.finish(mia).await.success());

    for validate_only in [false, true] {
        let request = running
            .client
            .post(format!("{}/api/actions/delayFlight", running.url))
            .bearer_auth(mia)
            .json(&json!({
                "commandId":"http-mia-denied-command","validateOnly":validate_only,
                "items":[{"params":{"flight":"F-1001","delayMinutes":5}}]
            }));
        assert_http_not_found(request, "delayFlight").await;
        assert_alice_receipt_unchanged(running, &before, receipt).await;
    }
    for (path, api_name) in [
        ("/api/objects/Flight/F-1001/history", "Flight"),
        ("/api/commands/stdio-success-command", "commands"),
    ] {
        assert_http_not_found(
            running
                .client
                .get(format!("{}{path}", running.url))
                .bearer_auth(mia),
            api_name,
        )
        .await;
        assert_alice_receipt_unchanged(running, &before, receipt).await;
    }
}

async fn assert_http_not_found(request: reqwest::RequestBuilder, api_name: &str) {
    use serde_json::Value;
    use stdio_security_support::DEADLINE;
    let body: Value = tokio::time::timeout(DEADLINE, async {
        let response = request.send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        response.json().await.unwrap()
    })
    .await
    .expect("Mia HTTP rejection deadline");
    assert_eq!(body["code"], "NOT_FOUND");
    assert_eq!(body["apiName"], api_name);
}

async fn assert_alice_receipt_unchanged(
    running: &support::Running,
    before: &std::collections::BTreeMap<String, serde_json::Value>,
    receipt: &serde_json::Value,
) {
    use stdio_security_support::{durable_rows, http_json};
    assert_eq!(
        &durable_rows(&running.path).await,
        before,
        "each unauthorized direct call must preserve all business, command and receipt rows"
    );
    let result = http_json(
        running
            .client
            .get(format!(
                "{}/api/commands/stdio-success-command",
                running.url
            ))
            .bearer_auth(&running.token),
    )
    .await;
    assert_eq!(
        &result, receipt,
        "Alice must retain access to her original receipt and time"
    );
}

async fn check_stdio_principal_reads(
    running: &support::Running,
    token: &str,
    allowed: &str,
    denied: &str,
    property: &str,
    action_name: &str,
) {
    use serde_json::json;
    use stdio_security_support::{Cli, http_json};
    let mut cli = Cli::spawn(&format!("{}/mcp", running.url), token);
    cli.initialize().await;
    let tools = cli
        .request(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .await;
    let names: Vec<_> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&action_name));
    let forbidden_action = if action_name == "action_delayFlight" {
        "action_recordMaintenance"
    } else {
        "action_delayFlight"
    };
    assert!(!names.contains(&forbidden_action));
    for name in ["describe", "query", "history"] {
        assert!(names.contains(&name));
    }
    let metadata = cli.tool(3, "describe", json!({})).await;
    let rest_metadata = http_json(
        running
            .client
            .get(format!("{}/api/ontology", running.url))
            .bearer_auth(token),
    )
    .await;
    for field in ["apiName", "defVersion", "links", "engineStatus"] {
        assert_eq!(metadata[field], rest_metadata[field]);
    }
    for description in [&metadata, &rest_metadata] {
        let types = description["objectTypes"].as_array().unwrap();
        assert!(types.iter().any(|ty| ty["apiName"] == allowed));
        assert!(!types.iter().any(|ty| ty["apiName"] == denied));
    }
    let query = json!({"set":{"kind":"type","object_type":allowed},"select":[property],"page":{"first":100}});
    let result = cli.tool(4, "query", json!({"request":query})).await;
    let rest = http_json(
        running
            .client
            .post(format!("{}/api/query", running.url))
            .bearer_auth(token)
            .json(&query),
    )
    .await;
    assert!(!result["rows"].as_array().unwrap().is_empty());
    assert_eq!(result["rows"], rest["rows"]);
    assert!(cli.finish(token).await.success());
}

#[tokio::test]
async fn live_stdio_online_bad_token_never_reaches_business_or_leaks_secret() {
    use stdio_security_support::{Cli, DEADLINE, Mode, Proxy, durable_rows};
    let running = support::fixture().await.start().await;
    let before = durable_rows(&running.path).await;
    let proxy = Proxy::start(&running.url, Mode::Pass).await;
    let sentinel = "stdio-online-invalid-token-sentinel";
    let mut cli = Cli::spawn(&proxy.url, sentinel);
    assert!(
        cli.receive().await.is_none(),
        "failed authentication must leave stdout empty"
    );
    assert!(!cli.finish(sentinel).await.success());
    proxy.settle().await;
    assert_eq!(proxy.count("initialize"), 1);
    assert_eq!(proxy.count("tools/call"), 0);
    assert_eq!(proxy.evidence.lock().unwrap().requests[0].status, Some(401));
    assert_eq!(durable_rows(&running.path).await, before);
    proxy.finish().await;
    assert!(
        tokio::time::timeout(DEADLINE, running.finish())
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn live_stdio_real_redirect_never_connects_to_credential_trap() {
    use std::sync::atomic::Ordering;
    use stdio_security_support::{Cli, RedirectTrap};
    let sentinel = "stdio-redirect-credential-sentinel";
    let trap = RedirectTrap::start(sentinel.to_owned()).await;
    let mut cli = Cli::spawn(&trap.url, sentinel);
    assert!(
        cli.receive().await.is_none(),
        "redirect failure must leave stdout empty"
    );
    assert!(!cli.finish(sentinel).await.success());
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(trap.origin_requests.load(Ordering::SeqCst), 1);
    assert_eq!(trap.trap_connections.load(Ordering::SeqCst), 0);
    trap.finish().await;
}

#[tokio::test]
async fn live_stdio_closed_upstream_session_never_reinitializes_or_replays() {
    use serde_json::json;
    use stdio_security_support::{
        Cli, DEADLINE, LEGACY_VERSION, Mode, Proxy, action, counts, durable_rows,
    };

    let running = support::fixture().await.start().await;
    let before = counts(&running.path).await;
    let before_rows = durable_rows(&running.path).await;
    let proxy = Proxy::start(&running.url, Mode::Pass).await;
    let mut cli = Cli::spawn(&proxy.url, &running.token);
    cli.initialize().await;
    let session = proxy
        .evidence
        .lock()
        .unwrap()
        .session
        .clone()
        .expect("real legacy session");
    let closed = tokio::time::timeout(
        DEADLINE,
        running
            .client
            .delete(&proxy.url)
            .bearer_auth(&running.token)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", LEGACY_VERSION)
            .header("mcp-session-id", &session)
            .send(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        closed.status().is_success(),
        "real host DELETE: {}",
        closed.status()
    );

    cli.send(&action(71, "bridge-expired-session"));
    cli.expect_failure(&json!(71)).await;
    proxy.settle().await;
    assert_eq!(proxy.count("server/discover"), 0);
    assert_eq!(proxy.count("initialize"), 1);
    assert_eq!(proxy.count("notifications/initialized"), 1);
    assert_eq!(proxy.count("tools/call"), 1);
    {
        let evidence = proxy.evidence.lock().unwrap();
        let failed = evidence
            .requests
            .iter()
            .find(|r| r.method == "tools/call")
            .unwrap();
        assert_eq!(failed.verb, "POST");
        assert_eq!(failed.status, Some(404), "404 must come from the real host");
        assert_eq!(failed.session.as_deref(), Some(session.as_str()));
        let deleted = evidence
            .requests
            .iter()
            .find(|r| r.verb == "DELETE")
            .unwrap();
        assert_eq!(deleted.session.as_deref(), Some(session.as_str()));
        assert!(deleted.status.unwrap() < 300);
    }
    assert_eq!(counts(&running.path).await, before);
    assert_eq!(durable_rows(&running.path).await, before_rows);
    cli.finish(&running.token).await;
    proxy.finish().await;
    tokio::time::timeout(DEADLINE, running.finish())
        .await
        .unwrap();
}

#[tokio::test]
async fn live_stdio_disconnected_before_forward_never_retries_or_commits() {
    live_stdio_fault_no_retry(false).await;
}

#[tokio::test]
async fn live_stdio_lost_commit_response_requires_explicit_replay() {
    live_stdio_fault_no_retry(true).await;
}

#[allow(
    clippy::too_many_lines,
    reason = "one fault and explicit replay scenario keeps its evidence assertions together"
)]
async fn live_stdio_fault_no_retry(after_commit: bool) {
    use serde_json::json;
    use stdio_security_support::{Cli, DEADLINE, Mode, Proxy, action, counts, durable_rows};

    let running = support::fixture().await.start().await;
    let before = counts(&running.path).await;
    let before_rows = durable_rows(&running.path).await;
    let mode = if after_commit {
        Mode::DropAfterCommit
    } else {
        Mode::DropBeforeForward
    };
    let proxy = Proxy::start(&running.url, mode).await;
    let mut cli = Cli::spawn(&proxy.url, &running.token);
    cli.initialize().await;
    cli.send(&action(72, "bridge-fault-command"));
    proxy.injected().await;
    cli.expect_failure(&json!(72)).await;
    proxy.settle().await;
    assert_eq!(
        proxy.count("tools/call"),
        1,
        "no implicit business replay after disconnect"
    );
    assert_eq!(proxy.count("server/discover"), 0);
    assert_eq!(
        proxy.count("initialize"),
        1,
        "real Initialize lifecycle without reinitialization"
    );
    assert_eq!(proxy.count("notifications/initialized"), 1);
    {
        let evidence = proxy.evidence.lock().unwrap();
        let session = evidence.session.as_ref().expect("real host session");
        let call = evidence
            .requests
            .iter()
            .find(|r| r.method == "tools/call")
            .unwrap();
        assert_eq!(call.verb, "POST");
        assert_eq!(call.session.as_ref(), Some(session));
        if !after_commit {
            assert_eq!(call.status, None, "request was never forwarded");
        }
    }
    let committed = counts(&running.path).await;
    let committed_rows = durable_rows(&running.path).await;
    assert_eq!(
        std::array::from_fn::<_, 5, _>(|index| committed[index] - before[index]),
        [i64::from(after_commit); 5]
    );
    cli.finish(&running.token).await;

    if after_commit {
        assert_eq!(before_rows["of_obj_flight"], json!([]));
        let flights = committed_rows["of_obj_flight"].as_array().unwrap();
        assert_eq!(flights.len(), 1);
        assert_eq!(flights[0]["flight_id"], "F-1001");
        assert_eq!(flights[0]["of_version"], "1");
        assert_eq!(flights[0]["status"], "DELAYED");
        assert_eq!(flights[0]["delay_minutes"], "5");
        for (table, rows) in &before_rows {
            if (table.starts_with("of_obj_") || table.starts_with("of_lnk_"))
                && table != "of_obj_flight"
            {
                assert_eq!(&committed_rows[table], rows, "unrelated overlay {table}");
            }
        }
        let receipt = proxy.evidence.lock().unwrap().receipt.clone().unwrap();
        assert!(receipt["committedAt"].is_string());
        let replay_proxy = Proxy::start(&running.url, Mode::Pass).await;
        let mut replay = Cli::spawn(&replay_proxy.url, &running.token);
        replay.initialize().await;
        replay.send(&action(73, "bridge-fault-command"));
        let response = replay.receive().await.unwrap();
        assert_eq!(response["id"], 73);
        assert!(response.get("error").is_none(), "{response}");
        assert_eq!(
            response["result"]["structuredContent"]["receipt"], receipt,
            "original receipt and committedAt survive explicit replay"
        );
        assert_eq!(
            counts(&running.path).await,
            committed,
            "replay must not add log, entry, history, command or outbox rows"
        );
        assert_eq!(
            durable_rows(&running.path).await,
            committed_rows,
            "explicit replay must preserve every durable row, property and version"
        );
        assert_eq!(replay_proxy.count("server/discover"), 0);
        assert_eq!(replay_proxy.count("initialize"), 1);
        assert_eq!(replay_proxy.count("tools/call"), 1);
        assert_eq!(
            proxy.count("tools/call"),
            1,
            "original connection remains unretried"
        );
        replay.finish(&running.token).await;
        replay_proxy.finish().await;
    } else {
        assert_eq!(
            committed_rows, before_rows,
            "pre-forward disconnect must preserve every durable row, property and version"
        );
    }
    proxy.finish().await;
    if after_commit {
        tokio::time::timeout(DEADLINE, running.finish_actions())
            .await
            .unwrap();
    } else {
        tokio::time::timeout(DEADLINE, running.finish())
            .await
            .unwrap();
    }
}

struct BridgeChild(std::process::Child);

impl Drop for BridgeChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if matches!(self.0.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            std::thread::panicking(),
            "stdio child could not be reaped within cleanup deadline"
        );
        eprintln!("stdio child could not be reaped within cleanup deadline");
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
