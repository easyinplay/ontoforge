use ontoforge::serve::run_with_shutdown;

mod cli {
    pub use ontoforge::cli::*;
}

mod production_assembly {
    #![allow(clippy::all)]
    include!("unit/serve.test.rs");

    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    #[derive(Clone)]
    struct TrinoReplies {
        origin: String,
        fail_proof: bool,
    }

    impl Respond for TrinoReplies {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            let user = request
                .headers
                .get("x-trino-user")
                .unwrap()
                .to_str()
                .unwrap();
            let trace = request
                .headers
                .get("x-trino-trace-token")
                .unwrap()
                .to_str()
                .unwrap();
            let sql = String::from_utf8_lossy(&request.body);
            let body = if trace == "ontoforge-l2-active-probe" {
                json!({"id":"active_1","nextUri":format!("{}/active",self.origin),"stats":{"state":"RUNNING"}})
            } else if trace == "ontoforge-l2-acl-check" && sql.contains("query_id") {
                json!({"columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}],"data":if user == "ontology" {json!([["active_1","RUNNING"]])} else {json!([])}})
            } else if trace == "ontoforge-l2-acl-check"
                && user == "l2"
                && (self.fail_proof || sql.contains("\"outside\"") || sql.contains("\"editstore\""))
            {
                json!({"id":"q1","error":{"errorName":"PERMISSION_DENIED","message":"denied"},"stats":{"state":"FAILED"}})
            } else if sql.contains("information_schema") {
                json!({"columns":[],"data":[]})
            } else {
                json!({"id":"q1","columns":[{"name":"value","type":"bigint"}],"data":[[1]],"stats":{"state":"FINISHED"}})
            };
            ResponseTemplate::new(200).set_body_json(body)
        }
    }

    async fn trino(fail_proof: bool) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(TrinoReplies {
                origin: server.uri(),
                fail_proof,
            })
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        server
    }

    fn configure_trino(f: &Fixture, server: &MockServer, l2: bool, proof: bool) {
        let mut config = std::fs::read_to_string(&f.config).unwrap();
        config.push_str(&format!(
            "\n[trino]\nurl = \"{}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n",
            server.uri()
        ));
        if l2 {
            config.push_str("[trino.identities.l2]\nuser = \"l2\"\n");
        }
        if proof {
            config.push_str("[trino.l2_acl]\nactive_query_sql = \"SELECT count(*) FROM demo.tiny.nation\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n");
        }
        std::fs::write(&f.config, config).unwrap();
    }

    fn raw_token(f: &Fixture) -> String {
        let key = DevKey::load_or_create(&f.config.with_file_name("dev.key")).unwrap();
        issue_dev_token(
            &key,
            &DevClaims {
                sub: "mia".to_owned(),
                groups: vec!["dispatchers".to_owned()],
                attrs: Default::default(),
                ttl: Duration::from_secs(300),
            },
        )
        .unwrap()
    }

    async fn initialize(bind: SocketAddr, token: &str) -> (reqwest::Client, String) {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let response = client.post(format!("http://{bind}/mcp"))
            .bearer_auth(token).header("accept", "application/json, text/event-stream")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"l2-assembly","version":"0"}}}))
            .send().await.unwrap();
        assert!(response.status().is_success());
        let session = response.headers()["mcp-session-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let response = client
            .post(format!("http://{bind}/mcp"))
            .bearer_auth(token)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-session-id", &session)
            .header("mcp-protocol-version", "2025-03-26")
            .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        (client, session)
    }

    async fn call(
        client: &reqwest::Client,
        bind: SocketAddr,
        token: &str,
        session: &str,
        name: &str,
        arguments: Value,
    ) -> Value {
        let response = client.post(format!("http://{bind}/mcp"))
            .bearer_auth(token).header("accept", "application/json, text/event-stream")
            .header("mcp-session-id", session).header("mcp-protocol-version", "2025-03-26")
            .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":name,"arguments":arguments}}))
            .send().await.unwrap().text().await.unwrap();
        response
            .lines()
            .find_map(|line| {
                line.strip_prefix("data: ")
                    .and_then(|value| serde_json::from_str(value).ok())
            })
            .or_else(|| serde_json::from_str(&response).ok())
            .expect(&response)
    }

    fn six_tools() -> [(&'static str, Value); 6] {
        [
            (
                "execute_query",
                json!({"query":"SELECT 1 FROM demo.tiny.nation"}),
            ),
            (
                "explain_query",
                json!({"query":"SELECT 1 FROM demo.tiny.nation"}),
            ),
            ("list_catalogs", json!({})),
            ("list_schemas", json!({"catalog":"demo"})),
            ("list_tables", json!({"catalog":"demo","schema":"tiny"})),
            (
                "get_table_schema",
                json!({"catalog":"demo","schema":"tiny","table":"nation"}),
            ),
        ]
    }

    #[tokio::test]
    async fn serve_l2_requires_dedicated_proved_identity_for_all_six_tools() {
        for (configured, proof, fail_proof) in [
            (false, false, false),
            (true, false, false),
            (true, true, true),
        ] {
            let f = fixture().await;
            let trino = trino(fail_proof).await;
            configure_trino(&f, &trino, configured, proof);
            let bind = unused_address();
            let serve_args = args(&f, bind);
            let (stop, rx) = oneshot::channel();
            let task = tokio::spawn(async move {
                run_with_shutdown(&serve_args, async {
                    let _ = rx.await;
                })
                .await
            });
            assert!(wait_listening(bind, &task).await);
            let token = raw_token(&f);
            let (client, session) = initialize(bind, &token).await;
            let before = trino.received_requests().await.unwrap().len();
            for (name, arguments) in six_tools() {
                let value = call(&client, bind, &token, &session, name, arguments).await;
                assert!(
                    value["error"].is_object(),
                    "unproved {name} must fail closed: {value}"
                );
            }
            assert_eq!(
                trino.received_requests().await.unwrap().len(),
                before,
                "disabled L2 must not submit SQL under any identity"
            );
            assert_eq!(
                http(bind, Some(&f.token)).await.0,
                200,
                "L0 remains available"
            );
            stop.send(()).unwrap();
            assert_eq!(task.await.unwrap(), 0);
        }
    }

    #[tokio::test]
    async fn serve_l2_queries_use_l2_identity_after_monitor_attestation() {
        let f = fixture().await;
        let trino = trino(false).await;
        configure_trino(&f, &trino, true, true);
        let bind = unused_address();
        let serve_args = args(&f, bind);
        let (stop, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            run_with_shutdown(&serve_args, async {
                let _ = rx.await;
            })
            .await
        });
        assert!(wait_listening(bind, &task).await);
        let token = raw_token(&f);
        let (client, session) = initialize(bind, &token).await;
        let before = trino.received_requests().await.unwrap().len();
        for (name, arguments) in six_tools() {
            let value = call(&client, bind, &token, &session, name, arguments).await;
            assert!(value["error"].is_null(), "proved {name}: {value}");
        }
        let requests = trino.received_requests().await.unwrap();
        assert!(requests.len() >= before + 6);
        for request in &requests[before..] {
            assert_eq!(
                request.headers["x-trino-user"], "l2",
                "raw tools must never use ontology credentials"
            );
        }
        assert_eq!(http(bind, Some(&f.token)).await.0, 200);
        stop.send(()).unwrap();
        assert_eq!(task.await.unwrap(), 0);
    }

    #[tokio::test]
    async fn same_host_l2_unverified_failed_and_expired_proofs_reject_all_six_tools() {
        use ontoforge_engine::config::{build_trino_engines, parse_server_config};
        use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
        use ontoforge_server::{ServerHost, state};
        use std::sync::Arc;

        let f = fixture().await;
        let trino = trino(false).await;
        configure_trino(&f, &trino, true, true);
        let config = parse_server_config(&std::fs::read_to_string(&f.config).unwrap()).unwrap();
        let engines = build_trino_engines(
            config.trino.as_ref().unwrap(),
            f.config.parent().unwrap(),
            &|_| None,
        )
        .unwrap();
        let core = Arc::new(engines.ontology);
        let l2 = Arc::new(engines.l2.unwrap());
        let auth_config = config.auth.unwrap();
        let key = DevKey::load_or_create(&f.config.with_file_name("dev.key")).unwrap();
        let auth = Authenticator::from_config(&auth_config, f.config.parent().unwrap())
            .unwrap()
            .with_dev(DevVerifier::new(
                DevMode::enable_insecure(),
                key,
                &auth_config.principal_attrs,
            ));
        let store = Arc::new(EditStore::open(settings(&f.path)).await.unwrap());
        let host = ServerHost::new(
            Arc::clone(&store),
            Arc::clone(&core),
            auth,
            auth_config,
            QUERY_KEY.to_vec(),
        )
        .unwrap();
        assert!(
            state::set_l2_engine(&store, Arc::clone(&core)).is_err(),
            "ordinary Trino engine has no proof admission capability"
        );
        let host = host.with_l2_status_provider(|| Some(L2HealthStatus::Enabled));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind = listener.local_addr().unwrap();
        let (stop, rx) = oneshot::channel();
        let task = tokio::spawn(Arc::new(host).serve(listener, async {
            let _ = rx.await;
        }));
        let token = raw_token(&f);
        let (client, session) = initialize(bind, &token).await;

        for (name, arguments) in six_tools() {
            let value = call(&client, bind, &token, &session, name, arguments).await;
            assert!(
                value["error"].is_object(),
                "health alone cannot grant {name}: {value}"
            );
        }
        assert!(trino.received_requests().await.unwrap().is_empty());
        state::set_l2_engine(&store, Arc::clone(&l2)).unwrap();

        for reason in [
            L2DisableReason::NotVerified,
            L2DisableReason::AllowedProbeFailed,
            L2DisableReason::Expired,
        ] {
            assert_eq!(l2.l2_status(), Some(L2HealthStatus::Disabled { reason }));
            let before = trino.received_requests().await.unwrap().len();
            for (name, arguments) in six_tools() {
                let value = call(&client, bind, &token, &session, name, arguments).await;
                assert!(value["error"].is_object(), "{reason:?} {name}: {value}");
            }
            assert_eq!(
                trino.received_requests().await.unwrap().len(),
                before,
                "disabled proof must prevent every SQL request"
            );
            assert_eq!(http(bind, Some(&f.token)).await.0, 200);
            if reason == L2DisableReason::Expired {
                break;
            }
            trino.reset().await;
            Mock::given(method("POST"))
                .respond_with(TrinoReplies {
                    origin: trino.uri(),
                    fail_proof: reason == L2DisableReason::NotVerified,
                })
                .mount(&trino)
                .await;
            let status = l2.recheck_l2_acl(&core, "active_1").await;
            if reason == L2DisableReason::AllowedProbeFailed {
                assert_eq!(status, L2HealthStatus::Enabled);
                let value = call(
                    &client,
                    bind,
                    &token,
                    &session,
                    "execute_query",
                    json!({"query":"SELECT 1 FROM demo.tiny.nation"}),
                )
                .await;
                assert!(value["error"].is_null(), "recovered proof: {value}");
                tokio::time::sleep(Duration::from_secs(61)).await;
            }
        }
        stop.send(()).unwrap();
        task.await.unwrap().unwrap();
    }
}
