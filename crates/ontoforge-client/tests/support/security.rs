use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
use ontoforge_schema::load_scope_dir;
use ontoforge_storage::store::sqlite_connect_options;
use serde_json::Value;
use sqlx::{QueryBuilder, Sqlite};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn quoted_identifier(name: &str) -> String {
    assert!(
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "unexpected fixture schema identifier: {name}"
    );
    format!("\"{name}\"")
}

fn quoted_table(name: &str) -> String {
    assert!(
        matches!(
            name,
            "of_edit_log"
                | "of_edit_log_entry"
                | "of_edit_history"
                | "of_action_command"
                | "of_webhook_outbox"
                | "of_obj_aircraft"
                | "of_obj_airport"
                | "of_obj_flight"
                | "of_obj_maintenance_record"
                | "of_obj_pilot"
                | "of_lnk_flight_crew"
        ),
        "unexpected aviation fixture table: {name}"
    );
    quoted_identifier(name)
}

impl super::Running {
    pub fn principal_token(&self, subject: &str, groups: Vec<String>) -> String {
        let key_path = self.path.parent().unwrap().join("token.key");
        let key = DevKey::load_or_create(&key_path).unwrap();
        issue_dev_token(
            &key,
            &DevClaims {
                sub: subject.to_owned(),
                groups,
                attrs: serde_json::Map::new(),
                ttl: std::time::Duration::from_secs(600),
            },
        )
        .unwrap()
    }

    pub async fn revoke_dispatchers(&self) {
        let mut scope = load_scope_dir(camino::Utf8Path::new(super::SCOPE))
            .scope
            .unwrap();
        scope
            .bindings
            .retain(|name, _| name.as_str() != "dispatcher");
        let props = std::collections::BTreeMap::new();
        let plan = self.store.scope_plan(&scope, &props).await.unwrap();
        self.store
            .scope_apply(&scope, &props, &plan.hash, "sdk-revoke")
            .await
            .unwrap();
    }

    pub async fn durable_state(&self) -> BTreeMap<String, (i64, i64)> {
        let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&self.path))
            .await
            .unwrap();
        let mut state = BTreeMap::new();
        for table in [
            "of_edit_log",
            "of_edit_log_entry",
            "of_edit_history",
            "of_action_command",
            "of_webhook_outbox",
        ] {
            let mut query = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM ");
            query.push(quoted_table(table));
            let count = query.build_query_scalar().fetch_one(&pool).await.unwrap();
            state.insert(table.to_owned(), (count, 0));
        }
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND (name GLOB 'of_obj_*' OR name GLOB 'of_lnk_*') ORDER BY name"
        ).fetch_all(&pool).await.unwrap();
        for table in tables {
            let mut query =
                QueryBuilder::<Sqlite>::new("SELECT COUNT(*), COALESCE(SUM(of_version), 0) FROM ");
            query.push(quoted_table(&table));
            let row: (i64, i64) = query.build_query_as().fetch_one(&pool).await.unwrap();
            state.insert(table, row);
        }
        pool.close().await;
        state
    }

    pub async fn durable_rows(&self) -> BTreeMap<String, Value> {
        let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&self.path))
            .await
            .unwrap();
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND (name GLOB 'of_obj_*' OR name GLOB 'of_lnk_*' OR name IN ('of_edit_log', 'of_edit_log_entry', 'of_edit_history', 'of_action_command', 'of_webhook_outbox')) ORDER BY name"
        ).fetch_all(&pool).await.unwrap();
        let mut state = BTreeMap::new();
        for table in tables {
            let columns: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                    .bind(&table)
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            let mut query = QueryBuilder::<Sqlite>::new(
                "SELECT json_group_array(json(row_json)) FROM (SELECT json_object(",
            );
            for (index, name) in columns.into_iter().enumerate() {
                if index > 0 {
                    query.push(", ");
                }
                let identifier = quoted_identifier(&name);
                query
                    .push_bind(name)
                    .push(", CAST(")
                    .push(identifier)
                    .push(" AS TEXT)");
            }
            query
                .push(") AS row_json FROM ")
                .push(quoted_table(&table))
                .push(" ORDER BY rowid)");
            let encoded: String = query.build_query_scalar().fetch_one(&pool).await.unwrap();
            state.insert(table, serde_json::from_str(&encoded).unwrap());
        }
        pool.close().await;
        state
    }
}

pub struct FaultProxy {
    pub url: String,
    pub requests: Arc<AtomicUsize>,
    pub forwards: Arc<AtomicUsize>,
    pub receipt: Arc<Mutex<Option<Value>>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl FaultProxy {
    pub async fn start(upstream: String, drop_after_commit: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let forwards = Arc::new(AtomicUsize::new(0));
        let receipt = Arc::new(Mutex::new(None));
        let seen = Arc::clone(&requests);
        let sent = Arc::clone(&forwards);
        let captured = Arc::clone(&receipt);
        let task = tokio::spawn(async move {
            let http = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap();
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::time::timeout(std::time::Duration::from_secs(4), async {
                    let mut bytes = Vec::new();
                    let (header_end, length) = loop {
                        let mut chunk = [0u8; 4096];
                        let n = socket.read(&mut chunk).await.unwrap();
                        assert_ne!(n, 0, "request must contain complete headers");
                        bytes.extend_from_slice(&chunk[..n]);
                        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                            let head = String::from_utf8_lossy(&bytes[..end]);
                            let length = head
                                .lines()
                                .find_map(|line| {
                                    let (name, value) = line.split_once(':')?;
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().unwrap())
                                })
                                .unwrap();
                            break (end + 4, length);
                        }
                    };
                    while bytes.len() < header_end + length {
                        let mut chunk = [0u8; 4096];
                        let n = socket.read(&mut chunk).await.unwrap();
                        assert_ne!(n, 0);
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    seen.fetch_add(1, Ordering::SeqCst);
                    if drop_after_commit {
                        let head = String::from_utf8_lossy(&bytes[..header_end]);
                        let path = head
                            .lines()
                            .next()
                            .unwrap()
                            .split_whitespace()
                            .nth(1)
                            .unwrap();
                        let authorization = head
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("authorization")
                                    .then(|| value.trim())
                            })
                            .unwrap();
                        sent.fetch_add(1, Ordering::SeqCst);
                        let response = http
                            .post(format!("{upstream}{path}"))
                            .header("authorization", authorization)
                            .header("content-type", "application/json")
                            .body(bytes[header_end..header_end + length].to_vec())
                            .send()
                            .await
                            .unwrap();
                        assert_eq!(response.status(), reqwest::StatusCode::OK);
                        let response = response.json::<Value>().await.unwrap();
                        *captured.lock().unwrap() = Some(response);
                    }
                    socket.shutdown().await.unwrap();
                })
                .await
                .expect("proxy request timed out");
            }
        });
        Self {
            url,
            requests,
            forwards,
            receipt,
            task: Some(task),
        }
    }

    pub async fn finish(mut self) {
        let task = self.task.take().unwrap();
        task.abort();
        if let Err(error) = task.await {
            assert!(error.is_cancelled(), "proxy task failed: {error}");
        }
    }
}

impl Drop for FaultProxy {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
