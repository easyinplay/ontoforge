use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{OriginalUri, State};
use axum::http::StatusCode;
use camino::Utf8PathBuf;
use ontoforge_auth::authn::Authenticator;
use ontoforge_engine::config::AuthConfig;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_server::{HostError, ServerHost};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use serde_json::json;
use tokio::sync::{Notify, Semaphore, oneshot};
use tokio_util::task::AbortOnDropHandle;

const QUERY_PATH: &str = "/v1/statement/drift_cleanup_1/1";
const WAIT: Duration = Duration::from_secs(5);

struct Upstream {
    next_uri: String,
    statements: Mutex<Vec<String>>,
    gets: Mutex<Vec<String>>,
    deletes: Mutex<Vec<String>>,
    get_entered: Notify,
    delete_entered: Notify,
    release_delete: Semaphore,
    delete_status: StatusCode,
}

async fn post_statement(
    State(upstream): State<Arc<Upstream>>,
    sql: String,
) -> axum::Json<serde_json::Value> {
    upstream.statements.lock().unwrap().push(sql);
    axum::Json(json!({
        "id": "drift_cleanup_1",
        "nextUri": upstream.next_uri,
    }))
}

async fn get_page(
    State(upstream): State<Arc<Upstream>>,
    OriginalUri(uri): OriginalUri,
) -> StatusCode {
    upstream.gets.lock().unwrap().push(uri.to_string());
    upstream.get_entered.notify_one();
    std::future::pending().await
}

async fn delete_query(
    State(upstream): State<Arc<Upstream>>,
    OriginalUri(uri): OriginalUri,
) -> StatusCode {
    upstream.deletes.lock().unwrap().push(uri.to_string());
    upstream.delete_entered.notify_one();
    upstream.release_delete.acquire().await.unwrap().forget();
    upstream.delete_status
}

#[expect(
    clippy::too_many_lines,
    reason = "keeps controlled upstream, host and cleanup ownership in one scope"
)]
async fn run_shutdown(delete_status: StatusCode) -> Result<(), HostError> {
    let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_url = format!("http://{}", upstream_listener.local_addr().unwrap());
    let upstream = Arc::new(Upstream {
        next_uri: format!("{upstream_url}{QUERY_PATH}"),
        statements: Mutex::new(Vec::new()),
        gets: Mutex::new(Vec::new()),
        deletes: Mutex::new(Vec::new()),
        get_entered: Notify::new(),
        delete_entered: Notify::new(),
        release_delete: Semaphore::new(0),
        delete_status,
    });
    let router = axum::Router::new()
        .route("/v1/statement", axum::routing::post(post_statement))
        .route(
            QUERY_PATH,
            axum::routing::get(get_page).delete(delete_query),
        )
        .with_state(Arc::clone(&upstream));
    let _upstream_task = AbortOnDropHandle::new(tokio::spawn(async move {
        axum::serve(upstream_listener, router).await.unwrap();
    }));
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    let store = Arc::new(
        EditStore::open(StoreSettings {
            backend: BackendSettings::Sqlite {
                path: root.join("edit.db"),
            },
            lock_timeout: WAIT,
            probe_timeout: WAIT,
            trino_catalog: None,
        })
        .await
        .unwrap(),
    );
    let loaded = ontoforge_schema::load_from_memory(&[
        (
            "ontology.yaml",
            "apiName: cleanupFixture\ndefaultProfile: dev\n",
        ),
        (
            "objects.yaml",
            "kind: ObjectType\napiName: Row\nbacking: source\nprimaryKey: id\nproperties:\n  id: { type: string, required: true }\n",
        ),
        (
            "bindings/dev/source.yaml",
            "kind: SourceBinding\ntarget: { objectType: Row }\nsource: { catalog: demo, schema: tiny, table: source_rows }\ncolumns:\n  id: { column: physical_id }\n",
        ),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    reload(&store, &loaded.ontology.unwrap(), &ReloadOptions::default())
        .await
        .unwrap();
    let engine = Arc::new(
        TrinoEngine::new(TrinoConnection {
            url: upstream_url.parse().unwrap(),
            user: "ontology".to_owned(),
            auth: Auth::None,
            ca_pem: None,
            connect_timeout: WAIT,
            max_page_bytes: 1024 * 1024,
            extra_origins: Vec::new(),
        })
        .unwrap(),
    );
    let config: AuthConfig = serde_json::from_value(json!({})).unwrap();
    let auth = Authenticator::from_config(&config, &root).unwrap();
    let host = Arc::new(
        ServerHost::new(
            Arc::clone(&store),
            engine,
            auth,
            config,
            b"cleanup-test-signing-key".to_vec(),
        )
        .unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (stop, stopped) = oneshot::channel();
    let mut serving = AbortOnDropHandle::new(tokio::spawn(host.serve(listener, async {
        let _ = stopped.await;
    })));
    tokio::time::timeout(WAIT, upstream.get_entered.notified())
        .await
        .expect("monitor metadata GET must be in flight");
    assert!(store.read_audit().health().available);
    assert_eq!(*upstream.gets.lock().unwrap(), [QUERY_PATH]);
    {
        let statements = upstream.statements.lock().unwrap();
        assert_eq!(statements.len(), 1);
        assert!(statements[0].contains("information_schema"));
        assert!(statements[0].contains("source_rows"));
    }
    stop.send(()).unwrap();
    tokio::time::timeout(WAIT, upstream.delete_entered.notified())
        .await
        .expect("shutdown must cancel the metadata query");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut serving)
            .await
            .is_err(),
        "host must await the blocked DELETE response"
    );
    upstream.release_delete.add_permits(1);
    let result = tokio::time::timeout(WAIT, &mut serving)
        .await
        .expect("shutdown must finish within its cleanup bound")
        .expect("host task must not panic");
    assert_eq!(*upstream.deletes.lock().unwrap(), [QUERY_PATH]);
    assert!(!store.read_audit().health().available);
    result
}

#[tokio::test]
async fn shutdown_waits_for_inflight_metadata_delete_and_closes_audit() {
    run_shutdown(StatusCode::NO_CONTENT).await.unwrap();
}

#[tokio::test]
async fn failed_metadata_delete_returns_shutdown_error_and_closes_audit() {
    assert!(matches!(
        run_shutdown(StatusCode::INTERNAL_SERVER_ERROR).await,
        Err(HostError::Shutdown)
    ));
}
