use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
use ontoforge_engine::trino::l2_monitor::L2MonitorOptions;
use serde_json::json;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

#[derive(Default)]
struct Fixture {
    loose: AtomicBool,
    leak: AtomicBool,
    finished: AtomicBool,
    absent: AtomicBool,
    deletes: AtomicUsize,
    submissions: AtomicUsize,
    cancel_status: AtomicU16,
    foreign_uri: Mutex<Option<String>>,
    origin: Mutex<String>,
    delayed: AtomicBool,
    observations: AtomicUsize,
    unknown: AtomicBool,
    finish_after_visibility: AtomicBool,
    l2_observed: AtomicBool,
}

impl Respond for Fixture {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        if request.method == "DELETE" {
            self.deletes.fetch_add(1, Ordering::SeqCst);
            let status = self.cancel_status.load(Ordering::SeqCst);
            return ResponseTemplate::new(if status == 0 { 204 } else { status });
        }
        let sql = String::from_utf8_lossy(&request.body);
        let user = request
            .headers
            .get("x-trino-user")
            .expect("identity")
            .to_str()
            .expect("user");
        let response = if request.method == "GET" || sql.contains("count(*)") {
            let number = if request.method == "POST" {
                self.submissions.fetch_add(1, Ordering::SeqCst) + 1
            } else {
                self.submissions.load(Ordering::SeqCst)
            };
            let id = format!("active_{number}");
            if self.finished.load(Ordering::SeqCst) {
                json!({"id":id,"stats":{"state":"FINISHED"}})
            } else {
                let next = self
                    .foreign_uri
                    .lock()
                    .expect("uri")
                    .clone()
                    .unwrap_or_else(|| {
                        format!("{}/v1/statement/{id}", self.origin.lock().expect("origin"))
                    });
                json!({"id":id,"stats":{"state":if self.unknown.load(Ordering::SeqCst) {"UNKNOWN"} else if request.method == "POST" {"QUEUED"} else {"RUNNING"}},"nextUri":next})
            }
        } else if sql.contains("system.runtime.queries") {
            self.observations.fetch_add(1, Ordering::SeqCst);
            if user == "l2" {
                self.l2_observed.store(true, Ordering::SeqCst);
            }
            let id = sql.split('\'').nth(1).expect("query ID");
            let rows = if self.absent.load(Ordering::SeqCst)
                || (user == "l2" && !self.leak.load(Ordering::SeqCst))
            {
                json!([])
            } else {
                json!([[
                    id,
                    if self.finish_after_visibility.load(Ordering::SeqCst)
                        && self.l2_observed.load(Ordering::SeqCst)
                    {
                        "FINISHED"
                    } else {
                        "RUNNING"
                    }
                ]])
            };
            json!({"columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}],"data":rows})
        } else if user == "l2"
            && (sql.contains("\"outside\"") || sql.contains("\"editstore\""))
            && !self.loose.load(Ordering::SeqCst)
        {
            json!({"error":{"errorName":"PERMISSION_DENIED","message":"private SQL"}})
        } else {
            json!({"columns":[{"name":"ok","type":"integer"}],"data":[[1]]})
        };
        let template = ResponseTemplate::new(200).set_body_json(response);
        if self.delayed.load(Ordering::SeqCst) && sql.contains("system.runtime.queries") {
            template.set_delay(Duration::from_millis(400))
        } else {
            template
        }
    }
}

#[derive(Clone)]
struct SharedFixture(Arc<Fixture>);
impl Respond for SharedFixture {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        self.0.respond(request)
    }
}

fn config(url: &str, sql: Option<&str>) -> String {
    let probe = sql.map_or_else(String::new, |sql| {
        format!("[trino.l2_acl]\nactive_query_sql = {sql:?}\n")
    });
    format!(
        "[trino]\nurl={url:?}\n[trino.identities.ontology]\nuser='ontology'\n[trino.identities.l2]\nuser='l2'\n{probe}[trino.l2_acl.allowed]\ncatalog='demo'\nschema='tiny'\ntable='nation'\n[trino.l2_acl.outside]\ncatalog='outside'\nschema='tiny'\ntable='nation'\n[trino.l2_acl.edit]\ncatalog='editstore'\nschema='tiny'\ntable='nation'\n"
    )
}

fn engines(url: &str, sql: Option<&str>) -> (Arc<TrinoEngine>, Arc<TrinoEngine>) {
    let cfg = parse_server_config(&config(url, sql))
        .expect("config")
        .trino
        .expect("trino");
    let built = build_trino_engines(&cfg, Utf8Path::new("."), &|_: &str| None).expect("engines");
    (Arc::new(built.l2.expect("l2")), Arc::new(built.ontology))
}

fn options() -> L2MonitorOptions {
    L2MonitorOptions {
        interval: Duration::from_millis(50),
        check_timeout: Duration::from_millis(700),
    }
}

async fn fixture() -> (MockServer, Arc<Fixture>) {
    let server = MockServer::start().await;
    let fixture = Arc::new(Fixture::default());
    *fixture.origin.lock().expect("origin") = server.uri();
    Mock::given(|_: &Request| true)
        .respond_with(SharedFixture(fixture.clone()))
        .mount(&server)
        .await;
    (server, fixture)
}

const PROBE: &str = "SELECT count(*) FROM demo.tiny.nation";

async fn wait_status(l2: &TrinoEngine, wanted: L2HealthStatus) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while l2.l2_status() != Some(wanted) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("bounded health transition");
}

#[tokio::test]
async fn monitor_startup_owns_fresh_probe_limits_and_cleanup() {
    let (server, fixture) = fixture().await;
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    let monitor = l2
        .clone()
        .start_l2_acl_monitor(ontology)
        .await
        .expect("monitor");
    assert_eq!(l2.l2_status(), Some(L2HealthStatus::Enabled));
    assert_eq!(fixture.submissions.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.deletes.load(Ordering::SeqCst), 1);
    let requests = server.received_requests().await.expect("requests");
    let submission = requests
        .iter()
        .find(|r| String::from_utf8_lossy(&r.body).contains("count(*)"))
        .expect("probe POST");
    let limits = submission
        .headers
        .get("x-trino-session")
        .expect("session limits")
        .to_str()
        .expect("header");
    assert!(limits.contains("query_max_run_time=25s"), "{limits}");
    assert!(limits.contains("query_max_execution_time=25s"), "{limits}");
    assert!(String::from_utf8_lossy(&submission.body).contains("LIMIT 2"));
    monitor.shutdown().await.expect("shutdown");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::Stopped
        })
    );
}

#[tokio::test]
async fn monitor_missing_or_unsafe_probe_never_submits() {
    for sql in [
        None,
        Some("DELETE FROM demo.tiny.nation"),
        Some("SELECT * FROM system.runtime.queries"),
        Some("SELECT * FROM outside.tiny.nation"),
    ] {
        let (server, _) = fixture().await;
        let (l2, ontology) = engines(&server.uri(), sql);
        let monitor = l2
            .clone()
            .start_l2_acl_monitor(ontology)
            .await
            .expect("disabled monitor");
        assert!(matches!(
            l2.l2_status(),
            Some(L2HealthStatus::Disabled { .. })
        ));
        assert!(
            server
                .received_requests()
                .await
                .expect("requests")
                .is_empty()
        );
        monitor.shutdown().await.expect("shutdown");
    }
}

#[tokio::test]
async fn monitor_finished_missing_and_leaked_query_disable() {
    for kind in 0..5 {
        let (server, fixture) = fixture().await;
        match kind {
            0 => fixture.finished.store(true, Ordering::SeqCst),
            1 => fixture.absent.store(true, Ordering::SeqCst),
            2 => fixture.leak.store(true, Ordering::SeqCst),
            3 => fixture.unknown.store(true, Ordering::SeqCst),
            _ => fixture
                .finish_after_visibility
                .store(true, Ordering::SeqCst),
        }
        let (l2, ontology) = engines(&server.uri(), Some(PROBE));
        let monitor = l2
            .clone()
            .start_l2_acl_monitor(ontology)
            .await
            .expect("disabled monitor");
        assert!(matches!(
            l2.l2_status(),
            Some(L2HealthStatus::Disabled { .. })
        ));
        monitor.shutdown().await.expect("shutdown");
    }
}

#[tokio::test]
async fn monitor_cancel_failure_disables_but_404_is_cleanup_success() {
    for code in [500, 404] {
        let (server, fixture) = fixture().await;
        fixture.cancel_status.store(code, Ordering::SeqCst);
        let (l2, ontology) = engines(&server.uri(), Some(PROBE));
        let monitor = l2
            .clone()
            .start_l2_acl_monitor(ontology)
            .await
            .expect("monitor");
        assert_eq!(
            l2.l2_status(),
            Some(if code == 404 {
                L2HealthStatus::Enabled
            } else {
                L2HealthStatus::Disabled {
                    reason: L2DisableReason::CleanupFailed,
                }
            })
        );
        if code == 404 {
            monitor.shutdown().await.expect("shutdown");
        } else {
            assert!(monitor.shutdown().await.is_err());
        }
    }
}

#[tokio::test]
async fn monitor_periodic_failure_and_fresh_recovery() {
    let (server, fixture) = fixture().await;
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    fixture.loose.store(true, Ordering::SeqCst);
    let monitor = l2
        .clone()
        .start_l2_acl_monitor_with_options(ontology, options())
        .await
        .expect("monitor");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::NegativeProbeFailed
        })
    );
    fixture.loose.store(false, Ordering::SeqCst);
    wait_status(&l2, L2HealthStatus::Enabled).await;
    fixture.loose.store(true, Ordering::SeqCst);
    wait_status(
        &l2,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::NegativeProbeFailed,
        },
    )
    .await;
    fixture.loose.store(false, Ordering::SeqCst);
    wait_status(&l2, L2HealthStatus::Enabled).await;
    assert!(fixture.submissions.load(Ordering::SeqCst) >= 4);
    monitor.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn monitor_duplicate_and_drop_cannot_reenable() {
    let (server, fixture) = fixture().await;
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    let monitor = l2
        .clone()
        .start_l2_acl_monitor_with_options(ontology.clone(), options())
        .await
        .expect("monitor");
    assert!(
        l2.clone()
            .start_l2_acl_monitor(ontology.clone())
            .await
            .is_err()
    );
    drop(monitor);
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::Stopped
        })
    );
    assert_eq!(fixture.submissions.load(Ordering::SeqCst), 1);
    assert_eq!(
        l2.recheck_l2_acl(&ontology, "active_1").await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::Stopped
        }
    );
}

#[tokio::test]
async fn monitor_foreign_next_uri_never_receives_get_or_cancel() {
    let (server, fixture) = fixture().await;
    let foreign = MockServer::start().await;
    *fixture.foreign_uri.lock().expect("uri") = Some(format!("{}/steal", foreign.uri()));
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    let monitor = l2
        .clone()
        .start_l2_acl_monitor(ontology)
        .await
        .expect("monitor");
    assert!(matches!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled { .. })
    ));
    monitor.shutdown().await.expect("shutdown");
    assert!(
        foreign
            .received_requests()
            .await
            .expect("requests")
            .is_empty()
    );
}

#[tokio::test]
async fn monitor_cancelled_startup_cleans_up_and_stays_disabled() {
    let (server, fixture) = fixture().await;
    fixture.delayed.store(true, Ordering::SeqCst);
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    assert!(
        tokio::time::timeout(
            Duration::from_millis(80),
            l2.clone().start_l2_acl_monitor(ontology.clone())
        )
        .await
        .is_err()
    );
    wait_status(
        &l2,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::Stopped,
        },
    )
    .await;
    tokio::time::timeout(Duration::from_secs(1), async {
        while fixture.deletes.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("probe cancelled");
    fixture.delayed.store(false, Ordering::SeqCst);
    l2.start_l2_acl_monitor(ontology)
        .await
        .expect("new monitor after cancellation")
        .shutdown()
        .await
        .expect("shutdown");
}

#[tokio::test]
async fn monitor_whole_cycle_deadline_disables_and_cleans_up() {
    let (server, fixture) = fixture().await;
    fixture.delayed.store(true, Ordering::SeqCst);
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    let opts = L2MonitorOptions {
        interval: Duration::from_secs(30),
        check_timeout: Duration::from_millis(50),
    };
    let monitor = l2
        .clone()
        .start_l2_acl_monitor_with_options(ontology, opts)
        .await
        .expect("monitor");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::TimedOut
        })
    );
    assert_eq!(fixture.deletes.load(Ordering::SeqCst), 1);
    monitor.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn monitor_cross_coordinator_is_invalid_before_network() {
    let (server, _) = fixture().await;
    let (other, _) = fixture().await;
    let (l2, _) = engines(&server.uri(), Some(PROBE));
    let (_, ontology) = engines(&other.uri(), Some(PROBE));
    let monitor = l2
        .clone()
        .start_l2_acl_monitor(ontology)
        .await
        .expect("disabled monitor");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::InvalidProbe
        })
    );
    assert!(
        server
            .received_requests()
            .await
            .expect("requests")
            .is_empty()
    );
    assert!(
        other
            .received_requests()
            .await
            .expect("requests")
            .is_empty()
    );
    monitor.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn monitor_shutdown_during_periodic_check_cannot_reenable() {
    let (server, fixture) = fixture().await;
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    let monitor = l2
        .clone()
        .start_l2_acl_monitor_with_options(ontology, options())
        .await
        .expect("monitor");
    fixture.delayed.store(true, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(2), async {
        while fixture.observations.load(Ordering::SeqCst) < 4 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("periodic check entered visibility observation");
    monitor
        .shutdown()
        .await
        .expect("shutdown cancels active probe");
    tokio::time::sleep(Duration::from_millis(450)).await;
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::Stopped
        })
    );
    assert_eq!(fixture.deletes.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn monitor_deadline_includes_waiting_for_recheck_lock() {
    let (server, fixture) = fixture().await;
    let (l2, ontology) = engines(&server.uri(), Some(PROBE));
    fixture.delayed.store(true, Ordering::SeqCst);
    let manual_l2 = l2.clone();
    let manual_ontology = ontology.clone();
    let manual =
        tokio::spawn(async move { manual_l2.recheck_l2_acl(&manual_ontology, "manual_1").await });
    tokio::time::timeout(Duration::from_secs(1), async {
        while fixture.observations.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("manual recheck holds lock");
    let opts = L2MonitorOptions {
        interval: Duration::from_secs(30),
        check_timeout: Duration::from_millis(50),
    };
    let monitor = tokio::time::timeout(
        Duration::from_millis(250),
        l2.clone().start_l2_acl_monitor_with_options(ontology, opts),
    )
    .await
    .expect("lock wait bounded")
    .expect("monitor");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::TimedOut
        })
    );
    manual.await.expect("manual join");
    assert_eq!(
        l2.l2_status(),
        Some(L2HealthStatus::Disabled {
            reason: L2DisableReason::TimedOut
        }),
        "older manual success must not overwrite newer timeout"
    );
    monitor.shutdown().await.expect("shutdown");
}
