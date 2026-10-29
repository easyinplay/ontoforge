use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::{Value, json};

struct Running {
    child: Child,
    base: String,
}

impl Running {
    fn stop(&mut self) {
        if self.child.try_wait().expect("child status").is_none() {
            self.child.kill().expect("stop dev server");
            let _ = self.child.wait();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop();
    }
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ontoforge"))
}

fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("free port")
        .local_addr()
        .expect("address")
        .port()
}

fn start(root: &Path, reset: bool) -> Running {
    let parent = root.parent().expect("root parent");
    let bind = format!("127.0.0.1:{}", port());
    let stderr =
        fs::File::create(parent.join(format!("ontoforge-{}.log", port()))).expect("stderr log");
    let mut command = Command::new(binary());
    command
        .args(["serve", "--dev", "--data-dir"])
        .arg(root)
        .args(["--bind", &bind])
        .stdout(Stdio::null())
        .stderr(stderr);
    if reset {
        command.arg("--reset");
    }
    let child = command.spawn().expect("spawn dev server");
    let mut running = Running {
        child,
        base: format!("http://{bind}"),
    };
    wait_ready(&mut running);
    running
}

fn wait_ready(running: &mut Running) {
    for _ in 0..100 {
        if running.child.try_wait().expect("child status").is_some() {
            panic!("dev server exited before becoming ready");
        }
        let address = running.base.strip_prefix("http://").expect("HTTP base");
        if std::net::TcpStream::connect(address).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("dev server did not become ready");
}

fn token(root: &Path) -> String {
    let output = Command::new(binary())
        .args(["dev-token", "--server-config"])
        .arg(root.join("server.toml"))
        .args(["--sub", "alice", "--groups", "dispatchers"])
        .output()
        .expect("issue token");
    assert!(
        output.status.success(),
        "token stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("token utf8")
        .trim()
        .to_owned()
}

async fn get(base: &str, token: &str, path: &str) -> (StatusCode, Value) {
    let response = reqwest::Client::new()
        .get(format!("{base}{path}"))
        .bearer_auth(token)
        .send()
        .await
        .expect("GET");
    let status = response.status();
    (status, response.json().await.expect("JSON response"))
}

async fn post(base: &str, token: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = reqwest::Client::new()
        .post(format!("{base}{path}"))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .expect("POST");
    let status = response.status();
    (status, response.json().await.expect("JSON response"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dev_restart_preserves_action_history_and_receipt() {
    let parent = tempfile::tempdir().expect("temp parent");
    let root = parent.path().join("dev");
    let mut first = start(&root, false);
    let token = token(&root);
    let command = "dev-restart-preserve";
    let (status, receipt) = post(
        &first.base,
        &token,
        "/api/actions/delayFlight",
        json!({"commandId": command, "items": [{"params": {"flight": "F-1001", "delayMinutes": 15}}]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    first.stop();
    let mut second = start(&root, false);
    let (status, replay) = get(&second.base, &token, &format!("/api/commands/{command}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay, receipt);
    let (status, history) = get(
        &second.base,
        &token,
        "/api/objects/Flight/F-1001/history?limit=10",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        history["entries"]
            .as_array()
            .expect("history entries")
            .iter()
            .any(|entry| entry["action"] == "delayFlight")
    );
    second.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dev_empty_seed_is_idempotent_and_reset_is_explicit() {
    let parent = tempfile::tempdir().expect("temp parent");
    let root = parent.path().join("dev");
    let mut first = start(&root, false);
    let auth_key = fs::read(root.join("dev-token.key")).expect("auth key");
    first.stop();
    let mut second = start(&root, false);
    assert_eq!(
        auth_key,
        fs::read(root.join("dev-token.key")).expect("auth key")
    );
    second.stop();
    let mut reset = start(&root, true);
    assert_ne!(
        auth_key,
        fs::read(root.join("dev-token.key")).expect("reset auth key")
    );
    reset.stop();
}

#[test]
fn dev_active_instance_and_unsafe_reset_are_rejected() {
    let parent = tempfile::tempdir().expect("temp parent");
    let root = parent.path().join("dev");
    let mut first = start(&root, false);
    let output = Command::new(binary())
        .args(["serve", "--dev", "--reset", "--data-dir"])
        .arg(&root)
        .args(["--bind", &format!("127.0.0.1:{}", port())])
        .output()
        .expect("spawn reset");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("owns"));
    first.stop();

    let unsafe_root = parent.path().join("unsafe");
    fs::create_dir_all(&unsafe_root).expect("unsafe root");
    fs::write(unsafe_root.join("user-data.txt"), "keep").expect("user data");
    let output = Command::new(binary())
        .args(["serve", "--dev", "--data-dir"])
        .arg(&unsafe_root)
        .args(["--bind", &format!("127.0.0.1:{}", port())])
        .output()
        .expect("spawn unsafe");
    assert!(!output.status.success());
    assert!(unsafe_root.join("user-data.txt").exists());
}

#[test]
fn dev_distributed_binary_query_action_history_without_checkout() {
    let parent = tempfile::tempdir().expect("temp parent");
    let isolated = parent.path().join("bin");
    fs::create_dir_all(&isolated).expect("isolated dir");
    let copied = isolated.join("ontoforge.exe");
    fs::copy(binary(), &copied).expect("copy binary");
    let mut child = Command::new(&copied)
        .current_dir(&isolated)
        .args(["serve", "--dev", "--bind", &format!("127.0.0.1:{}", port())])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn copied binary");
    std::thread::sleep(Duration::from_secs(2));
    assert!(child.try_wait().expect("child status").is_none());
    child.kill().expect("stop copied binary");
    let _ = child.wait();
    assert!(isolated.join(".ontoforge/dev/server.toml").exists());
}

#[test]
fn dev_demo_restart_and_reset_preserve_then_reinitialize() {
    let parent = tempfile::tempdir().expect("temp parent");
    let root = parent.path().join("dev");
    let mut first = start(&root, false);
    first.stop();
    let mut reset = start(&root, true);
    assert!(root.join("editstore.db").exists());
    reset.stop();
}
