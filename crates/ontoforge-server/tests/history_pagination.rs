mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use ontoforge_server::ServerLimits;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::store::sqlite_connect_options;
use ontoforge_storage::value::PropertyValue;
use reqwest::StatusCode;
use serde_json::{Value, json};
use support::{Fixture, fixture};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

struct Reply {
    status: StatusCode,
    id: String,
    body: Value,
    bytes: usize,
}

async fn append_record(f: &Fixture, names: &[String]) -> Vec<Value> {
    let active = f.store.active_definition().await.unwrap();
    let receipt = f
        .store
        .commit(Commit {
            actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
            action: "history-pages".to_owned(),
            def_version: active.def_version,
            params: json!({"private": "must-not-leak"}),
            edits: names
                .iter()
                .map(|name| Edit::Object {
                    type_name: "Airport".to_owned().try_into().unwrap(),
                    pk: PropertyValue::String("PAG".to_owned()),
                    op: ObjectOp::Set(BTreeMap::from([(
                        "name".to_owned().try_into().unwrap(),
                        PropertyValue::String(name.clone()),
                    )])),
                })
                .collect(),
        })
        .await
        .unwrap();
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path))
        .await
        .unwrap();
    let committed_at: String =
        sqlx::query_scalar("SELECT committed_at FROM of_edit_log WHERE seq = ?")
            .bind(receipt.seq)
            .fetch_one(&pool)
            .await
            .unwrap();
    pool.close().await;
    names
        .iter()
        .zip(&receipt.versions)
        .rev()
        .map(|(name, version)| {
            json!({
                "seq": receipt.seq, "eventId": receipt.event_id.to_string(),
                "actor": "ontoforge-dev|alice", "action": "history-pages",
                "committedAt": committed_at, "op": "set", "version": version,
                "values": {"name": name}, "unset": [], "maskedChanges": [], "params": null
            })
        })
        .collect()
}

async fn fetch_pages(f: Fixture, limits: ServerLimits, paths: &[String]) -> Vec<Reply> {
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path))
        .await
        .unwrap();
    let commits_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::new(f.host).serve_with_limits(
        listener,
        async {
            let _ = rx.await;
        },
        limits,
    ));
    let client = reqwest::Client::new();
    let mut replies = Vec::new();
    for path in paths {
        let response = client
            .get(format!("http://{address}{path}"))
            .bearer_auth(&f.token)
            .send()
            .await
            .unwrap();
        let status = response.status();
        let id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let body = response.bytes().await.unwrap();
        replies.push(Reply {
            status,
            id,
            body: serde_json::from_slice(&body).unwrap(),
            bytes: body.len(),
        });
    }
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    let audit_ids: Vec<String> = sqlx::query_scalar("SELECT request_id FROM of_read_audit")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(audit_ids.len(), replies.len());
    for reply in &replies {
        assert!(audit_ids.contains(&reply.id));
    }
    let commits_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(commits_after, commits_before);
    pool.close().await;
    replies
}

fn page(entries: Vec<Value>, next: Option<i64>) -> Value {
    json!({"entries": Value::Array(entries), "truncated": next.is_some(), "nextBeforeSeq": next})
}

fn path(limit: u32, before: Option<i64>) -> String {
    let mut path = format!("/api/objects/Airport/PAG/history?limit={limit}");
    if let Some(before) = before {
        write!(path, "&beforeSeq={before}").unwrap();
    }
    path
}

fn assert_page(reply: &Reply, expected: &Value) {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["truncated"], expected["truncated"]);
    assert_eq!(reply.body["nextBeforeSeq"], expected["nextBeforeSeq"]);
    let actual_entries = reply.body["entries"].as_array().unwrap();
    let expected_entries = expected["entries"].as_array().unwrap();
    assert_eq!(actual_entries.len(), expected_entries.len());
    for (actual, expected) in actual_entries.iter().zip(expected_entries) {
        assert_eq!(actual, expected);
    }
}

#[tokio::test]
async fn exact_limit_terminal_page_has_no_continuation() {
    let f = fixture().await;
    let entries = append_record(&f, &["only".to_owned()]).await;
    let replies = fetch_pages(f, ServerLimits::default(), &[path(1, None)]).await;
    assert_page(&replies[0], &page(entries, None));
}

#[tokio::test]
async fn zero_limit_is_normalized_before_continuation_is_decided() {
    let f = fixture().await;
    let entries = append_record(&f, &["only".to_owned()]).await;
    let replies = fetch_pages(f, ServerLimits::default(), &[path(0, None)]).await;
    assert_page(&replies[0], &page(entries, None));
}

#[tokio::test]
async fn limit_above_500_reports_and_follows_the_real_continuation() {
    let f = fixture().await;
    let mut records = Vec::new();
    for _ in 0..501 {
        records.push(append_record(&f, &["value".to_owned()]).await);
    }
    records.reverse();
    let next = records[499][0]["seq"].as_i64().unwrap();
    let replies = fetch_pages(
        f,
        ServerLimits::default(),
        &[path(1000, None), path(1000, Some(next))],
    )
    .await;
    let expected_first = records[..500].iter().flatten().cloned().collect();
    assert_page(&replies[0], &page(expected_first, Some(next)));
    assert_page(&replies[1], &page(records[500].clone(), None));
}

#[tokio::test]
async fn soft_entry_limit_never_splits_a_record_group() {
    let f = fixture().await;
    let older = append_record(&f, &["old".to_owned()]).await;
    let latest = append_record(&f, &["first".to_owned(), "second".to_owned()]).await;
    let next = latest[0]["seq"].as_i64().unwrap();
    let replies = fetch_pages(
        f,
        ServerLimits::default(),
        &[path(1, None), path(1, Some(next))],
    )
    .await;
    assert_page(&replies[0], &page(latest, Some(next)));
    assert_page(&replies[1], &page(older, None));
}

#[tokio::test]
async fn utf8_budget_includes_the_complete_normalized_page() {
    for short_by in [0, 1] {
        let f = fixture().await;
        let entries = append_record(&f, &["北京".repeat(100), "short".to_owned()]).await;
        let expected = page(entries, None);
        let bytes = serde_json::to_vec(&expected).unwrap().len();
        let mut limits = ServerLimits::default();
        limits.execution.max_decoded_bytes = bytes - short_by;
        let replies = fetch_pages(f, limits, &[path(1, None)]).await;
        if short_by == 0 {
            assert_page(&replies[0], &expected);
            assert_eq!(replies[0].bytes, bytes);
        } else {
            assert_eq!(replies[0].status, StatusCode::BAD_REQUEST);
            assert_eq!(
                replies[0].body,
                json!({"code": "COST_EXCEEDED", "apiName": "Airport", "request_id": replies[0].id})
            );
        }
    }
}

#[tokio::test]
async fn byte_limited_pages_preserve_complete_groups_without_skipping_entries() {
    let f = fixture().await;
    let oldest = append_record(&f, &["一".repeat(100), "short".to_owned()]).await;
    let middle = append_record(&f, &["二".repeat(100), "short".to_owned()]).await;
    let newest = append_record(&f, &["三".repeat(100), "short".to_owned()]).await;
    let first_cursor = newest[0]["seq"].as_i64().unwrap();
    let second_cursor = middle[0]["seq"].as_i64().unwrap();
    let expected = [
        page(newest, Some(first_cursor)),
        page(middle, Some(second_cursor)),
        page(oldest, None),
    ];
    let cap = expected
        .iter()
        .map(|page| serde_json::to_vec(page).unwrap().len())
        .max()
        .unwrap();
    let mut limits = ServerLimits::default();
    limits.execution.max_decoded_bytes = cap;
    let replies = fetch_pages(
        f,
        limits,
        &[
            path(100, None),
            path(100, Some(first_cursor)),
            path(100, Some(second_cursor)),
        ],
    )
    .await;
    for (reply, expected) in replies.iter().zip(&expected) {
        assert_page(reply, expected);
        assert!(reply.bytes <= cap);
    }
}

#[tokio::test]
async fn oversized_first_group_returns_a_safe_error_without_partial_entries() {
    let f = fixture().await;
    append_record(&f, &["older".to_owned()]).await;
    append_record(
        &f,
        &["secret-history-payload".repeat(100), "short".to_owned()],
    )
    .await;
    let mut limits = ServerLimits::default();
    limits.execution.max_decoded_bytes = 1024;
    let replies = fetch_pages(f, limits, &[path(100, None)]).await;
    assert_eq!(replies[0].status, StatusCode::BAD_REQUEST);
    assert_eq!(
        replies[0].body,
        json!({"code": "COST_EXCEEDED", "apiName": "Airport", "request_id": replies[0].id})
    );
}
