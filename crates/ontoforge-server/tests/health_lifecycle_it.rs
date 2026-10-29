#[path = "support/mod.rs"]
mod support;

use std::sync::Arc;
use std::time::Duration;

use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
use ontoforge_server::{HostError, ServerHost};
use reqwest::StatusCode;
use support::fixture;
use tokio::sync::oneshot;

async fn start<E: ontoforge_engine::Engine + Send + Sync + 'static>(
    host: ServerHost<E>,
) -> (
    String,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), HostError>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::new(host).serve(listener, async move {
        let _ = rx.await;
    }));
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if tokio::net::TcpStream::connect(url.strip_prefix("http://").unwrap())
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("listener must start");
    (url, stop, task)
}

#[tokio::test]
async fn health_unconfigured_optional_l2_is_healthy() {
    let f = fixture().await;
    let (url, stop, task) = start(f.host).await;
    let response = reqwest::get(format!("{url}/health")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
    stop.send(()).unwrap();
    assert_eq!(task.await.unwrap().unwrap(), ());
}

#[tokio::test]
async fn health_l2_failure_and_expiry_are_200_degraded() {
    let f = fixture().await;
    let host = f.host.with_l2_status_provider(|| {
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::Expired,
        })
    });
    let (url, stop, task) = start(host).await;
    let response = reqwest::get(format!("{url}/health")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["l2"]["reason"], "EXPIRED");
    stop.send(()).unwrap();
    assert_eq!(task.await.unwrap().unwrap(), ());
}

#[tokio::test]
async fn health_core_failure_is_503_and_recovery_is_visible() {
    let f = fixture().await;
    f.engine
        .unavailable
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let (url, stop, task) = start(f.host).await;
    let response = reqwest::get(format!("{url}/health")).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    f.engine
        .unavailable
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let response = reqwest::get(format!("{url}/health")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    stop.send(()).unwrap();
    assert_eq!(task.await.unwrap().unwrap(), ());
}

#[tokio::test]
async fn startup_partial_failure_cleans_monitor_and_store() {
    let f = fixture().await;
    let (url, stop, task) = start(f.host).await;
    drop(reqwest::get(format!("{url}/health")).await.unwrap());
    stop.send(()).unwrap();
    assert_eq!(task.await.unwrap().unwrap(), ());
    assert!(!f.store.read_audit().health().available);
}
