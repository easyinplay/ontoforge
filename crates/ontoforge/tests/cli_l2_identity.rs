use ontoforge::serve::run_with_shutdown;

mod cli {
    pub use ontoforge::cli::*;
}

mod production_assembly {
    #![allow(clippy::all)]
    include!("unit/serve.test.rs");

    use ontoforge_engine::config::{build_trino_engines, parse_server_config};
    use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
    use ontoforge_server::{ServerHost, state};
    use std::fmt::Write as _;
    use std::sync::Arc;
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

    fn source_monitor_sql() -> String {
        let tables = [
            "aircraft",
            "flight_crew",
            "flights",
            "maintenance_records",
            "pilots",
        ];
        let conditions = tables.map(|table| {
            format!(r#"(("table_schema" = 'aviation') AND ("table_name" = '{table}'))"#)
        });
        format!(
            concat!(
                "SELECT \"table_schema\", \"table_name\", \"column_name\", \"data_type\", \"is_nullable\" ",
                "FROM \"demo\".\"information_schema\".\"columns\" WHERE {} ",
                "ORDER BY \"table_schema\" ASC NULLS LAST, \"table_name\" ASC NULLS LAST, ",
                "\"ordinal_position\" ASC NULLS LAST"
            ),
            conditions.join(" OR ")
        )
    }

    fn is_source_monitor(request: &Request, origin: &str) -> bool {
        let header_is = |name: &str, expected: &str| {
            request
                .headers
                .get_all(name)
                .iter()
                .map(reqwest::header::HeaderValue::as_bytes)
                .eq([expected.as_bytes()])
        };
        // wiremock records origin-form requests against localhost; Host retains the target.
        request.method == "POST"
            && request.url.as_str() == "http://localhost/v1/statement"
            && origin
                .strip_prefix("http://")
                .is_some_and(|host| header_is("host", host))
            && header_is("x-trino-user", "ontology")
            && header_is("x-trino-trace-token", "source-drift")
            && request.body == source_monitor_sql().as_bytes()
    }

    async fn non_monitor_requests(server: &MockServer) -> Vec<Request> {
        server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|request| !is_source_monitor(request, &server.uri()))
            .collect()
    }

    async fn wait_for_source_monitor(server: &MockServer) {
        let observed = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if server
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .any(|request| is_source_monitor(request, &server.uri()))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            observed.is_ok(),
            "expected the exact startup source monitor request: {:?}",
            server.received_requests().await.unwrap()
        );
    }

    #[test]
    fn source_monitor_classifier_preserves_every_nonmatching_request() {
        let origin = "http://127.0.0.1:54321";
        let mut probe = Request {
            url: "http://localhost/v1/statement".parse().unwrap(),
            method: "POST".parse().unwrap(),
            headers: reqwest::header::HeaderMap::default(),
            body: source_monitor_sql().into_bytes(),
        };
        for (name, value) in [
            ("host", "127.0.0.1:54321"),
            ("x-trino-user", "ontology"),
            ("x-trino-trace-token", "source-drift"),
        ] {
            probe.headers.insert(name, value.parse().unwrap());
        }
        assert!(is_source_monitor(&probe, origin));
        assert!(!is_source_monitor(&probe, "http://127.0.0.1:54322"));
        let mut changed = probe.clone();
        changed.method = "GET".parse().unwrap();
        assert!(!is_source_monitor(&changed, origin));
        for url in [
            "http://localhost/unknown",
            "http://localhost/v1/statement?extra=1",
            "http://other/v1/statement",
        ] {
            let mut changed = probe.clone();
            changed.url = url.parse().unwrap();
            assert!(!is_source_monitor(&changed, origin), "{url}");
        }
        for name in ["host", "x-trino-user", "x-trino-trace-token"] {
            let mut changed = probe.clone();
            changed.headers.remove(name);
            assert!(!is_source_monitor(&changed, origin), "missing {name}");
            changed.headers.insert(name, "unknown".parse().unwrap());
            assert!(!is_source_monitor(&changed, origin), "wrong {name}");
            let mut changed = probe.clone();
            changed.headers.append(name, probe.headers[name].clone());
            assert!(!is_source_monitor(&changed, origin), "duplicate {name}");
        }
        for sql in [
            source_monitor_sql().replace("'aviation'", "'outside'"),
            source_monitor_sql().replace("'pilots'", "'unknown'"),
            "SELECT * FROM demo.information_schema.columns".to_owned(),
            "SELECT 1 FROM demo.tiny.nation".to_owned(),
            "unknown".to_owned(),
            String::new(),
        ] {
            for user in ["ontology", "l2", "unknown"] {
                let mut changed = probe.clone();
                changed
                    .headers
                    .insert("x-trino-user", user.parse().unwrap());
                changed.body = sql.as_bytes().to_vec();
                assert!(!is_source_monitor(&changed, origin), "{user}: {sql}");
            }
        }
        let mut changed = probe;
        changed.body = vec![0xff];
        assert!(!is_source_monitor(&changed, origin));
    }

    fn configure_trino(f: &Fixture, server: &MockServer, l2: bool, proof: bool) {
        let mut config = std::fs::read_to_string(&f.config).unwrap();
        write!(
            config,
            "\n[trino]\nurl = \"{}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n",
            server.uri()
        )
        .unwrap();
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
                attrs: serde_json::Map::new(),
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

    async fn assert_six_tools_rejected(
        client: &reqwest::Client,
        bind: SocketAddr,
        token: &str,
        session: &str,
        trino: &MockServer,
        reason: &str,
    ) {
        let before = non_monitor_requests(trino).await.len();
        for (name, arguments) in six_tools() {
            let value = call(client, bind, token, session, name, arguments).await;
            assert!(
                value["error"].is_object(),
                "{reason}: {name} must fail closed: {value}"
            );
        }
        assert_eq!(
            non_monitor_requests(trino).await.len(),
            before,
            "{reason}: disabled L2 must not submit SQL under any identity"
        );
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
            assert_six_tools_rejected(&client, bind, &token, &session, &trino, "unproved").await;
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
        let before = non_monitor_requests(&trino).await.len();
        for (name, arguments) in six_tools() {
            let value = call(&client, bind, &token, &session, name, arguments).await;
            assert!(value["error"].is_null(), "proved {name}: {value}");
        }
        let requests = non_monitor_requests(&trino).await;
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
    async fn serve_shutdown_waits_for_trino_delete_completion() {
        shutdown_delete_case(204, Duration::from_millis(800), 0).await;
    }

    #[tokio::test]
    async fn serve_shutdown_reports_failed_or_timed_out_trino_delete() {
        shutdown_delete_case(500, Duration::ZERO, 3).await;
        shutdown_delete_case(204, Duration::from_secs(3), 3).await;
    }

    async fn shutdown_delete_case(delete_status: u16, delete_delay: Duration, expected_exit: u8) {
        use wiremock::matchers::{body_string_contains, path};

        let f = fixture().await;
        let trino = trino(false).await;
        Mock::given(method("POST"))
            .and(body_string_contains("998"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id":"read_1", "nextUri":format!("{}/read/1", trino.uri()), "stats":{"state":"RUNNING"}
            })))
            .with_priority(1).mount(&trino).await;
        Mock::given(method("GET"))
            .and(path("/read/1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"columns":[],"data":[]}))
                    .set_delay(Duration::from_secs(10)),
            )
            .mount(&trino)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/read/1"))
            .respond_with(ResponseTemplate::new(delete_status).set_delay(delete_delay))
            .with_priority(1)
            .mount(&trino)
            .await;
        configure_trino(&f, &trino, true, true);
        let bind = unused_address();
        let serve_args = args(&f, bind);
        let (stop, rx) = oneshot::channel();
        let mut task = tokio::spawn(async move {
            run_with_shutdown(&serve_args, async {
                let _ = rx.await;
            })
            .await
        });
        assert!(wait_listening(bind, &task).await);
        let token = raw_token(&f);
        let (client, session) = initialize(bind, &token).await;
        let request = tokio::spawn(async move {
            client.post(format!("http://{bind}/mcp"))
                .bearer_auth(token).header("accept", "application/json, text/event-stream")
                .header("mcp-session-id", session).header("mcp-protocol-version", "2025-03-26")
                .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"execute_query","arguments":{"query":"SELECT 998 FROM demo.tiny.nation"}}}))
                .send().await.unwrap().text().await
        });
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if trino
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .any(|request| request.method == "GET" && request.url.path() == "/read/1")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        stop.send(()).unwrap();
        if !delete_delay.is_zero() {
            assert!(
                tokio::time::timeout(Duration::from_millis(200), &mut task)
                    .await
                    .is_err(),
                "serve returned before the pending Trino DELETE could complete"
            );
        }
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(4), task)
                .await
                .unwrap()
                .unwrap(),
            expected_exit
        );
        assert!(
            trino
                .received_requests()
                .await
                .unwrap()
                .iter()
                .any(|request| request.method == "DELETE" && request.url.path() == "/read/1")
        );
        let _ = request.await;
    }

    #[tokio::test]
    async fn same_host_l2_unverified_failed_and_expired_proofs_reject_all_six_tools() {
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

        wait_for_source_monitor(&trino).await;
        assert_six_tools_rejected(&client, bind, &token, &session, &trino, "health alone").await;
        state::set_l2_engine(&store, Arc::clone(&l2)).unwrap();

        for reason in [
            L2DisableReason::NotVerified,
            L2DisableReason::AllowedProbeFailed,
            L2DisableReason::Expired,
        ] {
            assert_eq!(l2.l2_status(), Some(L2HealthStatus::Disabled { reason }));
            assert_six_tools_rejected(
                &client,
                bind,
                &token,
                &session,
                &trino,
                &format!("{reason:?}"),
            )
            .await;
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
