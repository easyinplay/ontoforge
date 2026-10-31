use ontoforge::serve::run_with_shutdown;

mod cli {
    pub use ontoforge::cli::*;
}

mod serve_fixture {
    #![allow(clippy::all)]
    include!("unit/serve.test.rs");

    #[tokio::test]
    async fn shutdown_finishes_accepted_action_before_dependency_cancel() {
        let f = fixture().await;
        let bind = unused_address();
        let serve_args = args(&f, bind);
        let (stop, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            run_with_shutdown(&serve_args, async {
                let _ = rx.await;
            })
            .await
        });
        assert!(wait_listening(bind, &task).await);
        let mut stream = std::net::TcpStream::connect(bind).unwrap();
        let body = serde_json::to_string(&serde_json::json!({
            "commandId": "shutdown-action",
            "items": [{"params": {"flight": "F-1001", "delayMinutes": 2}}]
        }))
        .unwrap();
        write!(
            stream,
            "POST /api/actions/delayFlight HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            f.token,
            body.len(),
            body
        )
        .unwrap();
        stream
            .shutdown(std::net::Shutdown::Both)
            .expect("client disconnect is part of the accepted-action path");
        drop(stream);
        stop.send(()).unwrap();
        assert_eq!(task.await.unwrap(), 0);
    }

    #[tokio::test]
    async fn shutdown_cancels_reads_sse_then_durably_drains_audit() {
        let f = fixture().await;
        let bind = unused_address();
        let serve_args = args(&f, bind);
        let (stop, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            run_with_shutdown(&serve_args, async {
                let _ = rx.await;
            })
            .await
        });
        assert!(wait_listening(bind, &task).await);
        let (_, request_id, _) = http(bind, Some(&f.token)).await;
        stop.send(()).unwrap();
        assert_eq!(task.await.unwrap(), 0);
        let pool = sqlx::SqlitePool::connect_with(
            ontoforge_storage::store::sqlite_connect_options(&f.path),
        )
        .await
        .unwrap();
        let row = sqlx::query("SELECT request_id, status FROM of_read_audit")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("request_id"), request_id);
        assert_eq!(row.get::<String, _>("status"), "success");
        pool.close().await;
    }

    #[tokio::test]
    async fn shutdown_timeout_sink_and_cleanup_errors_exit_nonzero() {
        let f = fixture().await;
        let bind = unused_address();
        let serve_args = args(&f, bind);
        let (stop, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            run_with_shutdown(&serve_args, async {
                let _ = rx.await;
            })
            .await
        });
        assert!(wait_listening(bind, &task).await);
        let pool = sqlx::SqlitePool::connect_with(
            ontoforge_storage::store::sqlite_connect_options(&f.path),
        )
        .await
        .unwrap();
        sqlx::query("DROP TABLE of_read_audit")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let _ = http(bind, Some(&f.token)).await;
        stop.send(()).unwrap();
        assert_ne!(task.await.unwrap(), 0);
    }
}
