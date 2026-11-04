use ontoforge::serve::run_with_shutdown;

mod cli {
    pub use ontoforge::cli::*;
}

mod serve_fixture {
    #![allow(clippy::all)]
    include!("unit/serve.test.rs");

    #[tokio::test]
    async fn occupied_listener_does_not_start_an_l2_probe_owner() {
        use std::fmt::Write as _;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let f = fixture().await;
        let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind = occupied.local_addr().unwrap();
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", upstream.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let stub = tokio::spawn(async move {
            loop {
                let (mut stream, _) = upstream.accept().await.unwrap();
                let mut buffer = [0; 8192];
                let _ = stream.read(&mut buffer).await.unwrap();
                observed.fetch_add(1, Ordering::SeqCst);
                stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
            }
        });
        let mut config = std::fs::read_to_string(&f.config).unwrap();
        write!(&mut config, "\n[trino]\nurl = \"{url}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n").unwrap();
        config.push_str(
            "[trino.l2_acl]\nactive_query_sql = \"SELECT count(*) FROM demo.tiny.nation\"\n",
        );
        std::fs::write(&f.config, config).unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            run_with_shutdown(&args(&f, bind), std::future::pending()),
        )
        .await;
        stub.abort();
        let _ = stub.await;
        assert_eq!(result.unwrap(), 3);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "port failure must precede all monitor startup probes"
        );
    }

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
