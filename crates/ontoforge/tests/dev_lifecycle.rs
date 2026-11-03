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
        assert!(
            running.child.try_wait().expect("child status").is_none(),
            "dev server exited before becoming ready"
        );
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
    assert!(
        !unsafe_root.join(".ontoforge-dev.lock").exists(),
        "rejected user directory must not gain a lock artifact"
    );
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

fn rejected_start(root: &Path, reset: bool) -> std::process::Output {
    let mut command = Command::new(binary());
    command
        .args(["serve", "--dev", "--data-dir"])
        .arg(root)
        .args(["--bind", &format!("127.0.0.1:{}", port())])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if reset {
        command.arg("--reset");
    }
    let mut child = command.spawn().expect("spawn competing server");
    for _ in 0..100 {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.kill().expect("clean up unexpected competing server");
    let output = child.wait_with_output().unwrap();
    panic!(
        "competing dev server remained active: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn dev_lock_is_held_for_live_state_and_released_after_process_crash() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("dev");
    let mut first = start(&root, false);
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(".ontoforge-dev.lock"))
        .unwrap();
    assert!(
        lock.try_lock().is_err(),
        "live DevState must hold an exclusive OS file lock"
    );
    let rejected = rejected_start(&root, true);
    assert!(!rejected.status.success());
    first.stop();
    lock.try_lock()
        .expect("kernel releases lock when owner crashes");
    drop(lock);
    let mut recovered = start(&root, false);
    recovered.stop();
    assert!(
        root.join(".ontoforge-dev.lock").exists(),
        "lock inode must survive owner release"
    );
}

#[test]
fn dev_empty_lock_window_never_allows_a_second_owner_or_reset() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("dev");
    start(&root, false).stop();
    let path = root.join(".ontoforge-dev.lock");
    fs::write(&path, "").unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    lock.try_lock().unwrap();
    let before = fs::read(root.join("dev-token.key")).unwrap();
    for reset in [false, true] {
        let rejected = rejected_start(&root, reset);
        assert!(!rejected.status.success());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("owns"));
        assert_eq!(fs::read(root.join("dev-token.key")).unwrap(), before);
    }
    drop(lock);
    let mut recovered = start(&root, false);
    recovered.stop();
}

#[test]
fn dev_recovers_unowned_empty_lock_before_first_marker() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("dev");
    fs::create_dir(&root).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(root.join(".ontoforge-dev.lock"), "").unwrap();
    let mut recovered = start(&root, false);
    recovered.stop();
    assert!(root.join(".ontoforge-dev.lock").exists());
}

#[cfg(unix)]
#[test]
fn dev_new_directory_and_both_keys_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("private-parent/dev");
    fs::create_dir(root.parent().unwrap()).unwrap();
    let mut running = start(&root, false);
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for key in ["dev-token.key", "query-signing.key"] {
        assert_eq!(
            fs::metadata(root.join(key)).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    running.stop();
}

#[cfg(unix)]
#[test]
fn dev_rejects_existing_exposed_keys_without_changing_them() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("dev");
    start(&root, false).stop();
    for key in ["dev-token.key", "query-signing.key"] {
        let path = root.join(key);
        let before = fs::read(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let database = fs::read(root.join("editstore.db")).unwrap();
        for reset in [false, true] {
            let rejected = rejected_start(&root, reset);
            assert!(!rejected.status.success());
            assert!(String::from_utf8_lossy(&rejected.stderr).contains("chmod 600"));
            assert_eq!(fs::read(&path).unwrap(), before);
            assert_eq!(fs::read(root.join("editstore.db")).unwrap(), database);
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn dev_rejects_existing_public_directory_without_modifying_it() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("dev");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let rejected = rejected_start(&root, false);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("chmod 700"));
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn dev_rejects_dangling_lock_symlink_without_creating_external_target() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("dev");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let outside = directory.path().join("outside.lock");
    symlink(&outside, root.join(".ontoforge-dev.lock")).unwrap();
    let rejected = rejected_start(&root, false);
    assert!(!rejected.status.success());
    assert!(!outside.exists());
    assert!(!root.join(".ontoforge-dev-marker").exists());
    assert!(!root.join("dev-token.key").exists());
    assert!(
        fs::symlink_metadata(root.join(".ontoforge-dev.lock"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
