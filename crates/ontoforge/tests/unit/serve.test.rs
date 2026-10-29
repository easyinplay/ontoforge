use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use ontoforge_auth::authn::Authenticator;
use ontoforge_auth::authn::dev::{DevClaims, DevKey, DevMode, DevVerifier, issue_dev_token};
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_query::TokenCodec;
use ontoforge_query::cursor::{VersionClaims, VersionTokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use ontoforge_storage::value::PropertyValue;
use serde_json::{Value, json};
use sqlx::Row;
use tokio::sync::oneshot;

use super::run_with_shutdown;
use crate::cli::{Cli, ServeArgs};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);
const QUERY_KEY: [u8; 32] = [0xa5; 32];

struct Fixture {
    _dir: tempfile::TempDir,
    config: Utf8PathBuf,
    path: Utf8PathBuf,
    token: String,
}

fn settings(path: &Utf8Path) -> StoreSettings {
    StoreSettings {
        backend: BackendSettings::Sqlite {
            path: path.to_owned(),
        },
        lock_timeout: Duration::from_secs(3),
        probe_timeout: Duration::from_secs(3),
        trino_catalog: None,
    }
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
    let path = root.join("edit.db");
    let config = root.join("server.toml");
    std::fs::write(&config, "[query]\nsigning_key_file = \"query.key\"\n[store]\nkind = \"sqlite\"\npath = \"edit.db\"\n[auth.dev]\nkey_file = \"dev.key\"\n[auth.principal_attrs.homeAirports]\nclaim = \"airports\"\ntype = \"string_list\"\n").unwrap();
    std::fs::write(root.join("query.key"), QUERY_KEY).unwrap();
    let store = EditStore::open(settings(&path)).await.unwrap();
    let ontology = load_dir(Utf8Path::new(AVIATION)).ontology.unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let scope = load_scope_dir(Utf8Path::new(SCOPE)).scope.unwrap();
    let auth: AuthConfig = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    }))
    .unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&auth)),
        scope: &scope,
        sources: None,
    })
    .unwrap();
    let props = compiled.row_filter_properties();
    let plan = store.scope_plan(&scope, &props).await.unwrap();
    store
        .scope_apply(&scope, &props, &plan.hash, "serve-test")
        .await
        .unwrap();
    store
        .commit(Commit {
            actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
            action: "createAirport".to_owned(),
            def_version: store.active_definition().await.unwrap().def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: ObjectTypeName::try_from("Airport".to_owned()).unwrap(),
                pk: PropertyValue::String("BER".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    PropertyName::try_from("name".to_owned()).unwrap(),
                    PropertyValue::String("Berlin".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    store.read_audit().shutdown().await.unwrap();
    let key = DevKey::load_or_create(&root.join("dev.key")).unwrap();
    let token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "alice".to_owned(),
            groups: vec!["dispatchers".to_owned()],
            attrs: serde_json::Map::new(),
            ttl: Duration::from_secs(300),
        },
    )
    .unwrap();
    Fixture {
        _dir: dir,
        config,
        path,
        token,
    }
}

fn unused_address() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

fn args(f: &Fixture, bind: SocketAddr) -> ServeArgs {
    ServeArgs {
        server_config: Some(f.config.clone()),
        bind,
        dev: true,
        export_openapi: None,
    }
}

async fn http(bind: SocketAddr, token: Option<&str>) -> (u16, String, Value) {
    let authorization = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        let mut stream = std::net::TcpStream::connect_timeout(&bind, Duration::from_secs(2)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        write!(stream, "GET /api/objects/Airport/BER HTTP/1.1\r\nHost: localhost\r\n{authorization}Connection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
        let request_id = headers.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("x-request-id").then(|| value.trim().to_owned())
        }).unwrap();
        (status, request_id, serde_json::from_str(body).unwrap())
    }).await.unwrap()
}

async fn wait_listening(bind: SocketAddr, task: &tokio::task::JoinHandle<u8>) -> bool {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if tokio::net::TcpStream::connect(bind).await.is_ok() {
                return true;
            }
            if task.is_finished() {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or(false)
}

async fn read_across_start(f: &Fixture) -> Value {
    let bind = unused_address();
    let serve_args = args(f, bind);
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        run_with_shutdown(&serve_args, async {
            let _ = rx.await;
        })
        .await
    });
    assert!(
        wait_listening(bind, &task).await,
        "configured query key must start the authenticated host"
    );
    let (status, _, body) = http(bind, Some(&f.token)).await;
    assert_eq!(status, 200);
    stop.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap(),
        0
    );
    body
}

#[tokio::test]
async fn serve_query_key_survives_restart_and_is_independent_of_auth() {
    let mut f = fixture().await;
    let first = read_across_start(&f).await;
    let second = read_across_start(&f).await;
    let token = first["_version"].as_str().unwrap();
    assert_eq!(
        second["_version"], token,
        "same persistent key must retain version tokens across host restarts"
    );

    let store = EditStore::open(settings(&f.path)).await.unwrap();
    let snapshot = store.request_snapshot().await.unwrap();
    let auth: AuthConfig = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    }))
    .unwrap();
    let key = DevKey::load_or_create(&f.config.with_file_name("dev.key")).unwrap();
    let principal = Authenticator::from_config(&auth, f.config.parent().unwrap())
        .unwrap()
        .with_dev(DevVerifier::new(
            DevMode::enable_insecure(),
            key,
            &auth.principal_attrs,
        ))
        .authenticate(&f.token)
        .await
        .unwrap();
    let scope = compile_scope(&ScopeInputs {
        ontology: &snapshot.active.ontology,
        profile: None,
        attrs: &attr_types(Some(&auth)),
        scope: &snapshot.scope,
        sources: None,
    })
    .unwrap()
    .for_principal(&principal);
    let ty = ObjectTypeName::try_from("Airport".to_owned()).unwrap();
    let pk = PropertyValue::String("BER".to_owned());
    let edit = store
        .read_object_edit(&snapshot.active, &ty, &pk)
        .await
        .unwrap()
        .unwrap();
    let claims = VersionClaims {
        principal: &principal,
        scope: &scope,
        object_type: &ty,
        pk: &pk,
        def_version: snapshot.active.def_version,
        source: None,
        edit: Some(&edit.row),
    };
    let signer = TokenCodec::new(QUERY_KEY.to_vec()).unwrap();
    assert_eq!(
        VersionTokenCodec::new(&signer).verify(token, &claims),
        Ok(())
    );
    let wrong = TokenCodec::new(vec![0x5a; 32]).unwrap();
    assert!(
        VersionTokenCodec::new(&wrong)
            .verify(token, &claims)
            .is_err()
    );
    store.read_audit().shutdown().await.unwrap();

    std::fs::remove_file(f.config.with_file_name("dev.key")).unwrap();
    let replacement = DevKey::load_or_create(&f.config.with_file_name("dev.key")).unwrap();
    f.token = issue_dev_token(
        &replacement,
        &DevClaims {
            sub: "alice".to_owned(),
            groups: vec!["dispatchers".to_owned()],
            attrs: serde_json::Map::new(),
            ttl: Duration::from_secs(300),
        },
    )
    .unwrap();
    assert_eq!(
        read_across_start(&f).await["_version"],
        token,
        "auth key replacement must not replace the independent query key"
    );
    std::fs::write(f.config.with_file_name("query.key"), [0x5a; 32]).unwrap();
    assert_ne!(read_across_start(&f).await["_version"], token);
}

#[tokio::test]
async fn serve_query_key_rejects_missing_short_and_oversized_files_before_listen() {
    let f = fixture().await;
    let original = std::fs::read_to_string(&f.config).unwrap();
    for bytes in [None, Some(vec![1; 31]), Some(vec![1; 64 * 1024 + 1])] {
        let path = f.config.with_file_name("query.key");
        match bytes {
            Some(bytes) => std::fs::write(&path, bytes).unwrap(),
            None => std::fs::remove_file(&path).unwrap(),
        }
        let bind = unused_address();
        assert_ne!(
            tokio::time::timeout(
                Duration::from_secs(3),
                run_with_shutdown(&args(&f, bind), std::future::pending())
            )
            .await
            .expect("invalid key must fail before awaiting shutdown"),
            0
        );
        assert!(tokio::net::TcpStream::connect(bind).await.is_err());
    }
    std::fs::write(f.config.with_file_name("query.key"), QUERY_KEY).unwrap();
    std::fs::write(
        &f.config,
        original.replace("[query]\nsigning_key_file = \"query.key\"\n", ""),
    )
    .unwrap();
    let bind = unused_address();
    assert_ne!(run_with_shutdown(&args(&f, bind), async {}).await, 0);
    assert!(tokio::net::TcpStream::connect(bind).await.is_err());
}

#[tokio::test]
async fn serve_args_validate_config_and_address() {
    let parsed = Cli::try_parse_from([
        "ontoforge",
        "serve",
        "--server-config",
        "server.toml",
        "--bind",
        "127.0.0.1:8081",
    ]);
    assert!(
        parsed.is_ok(),
        "valid serve args must dispatch to the HTTP host"
    );
    assert!(
        Cli::try_parse_from([
            "ontoforge",
            "serve",
            "--server-config",
            "server.toml",
            "--bind",
            "bad-address"
        ])
        .is_err()
    );
    assert!(Cli::try_parse_from(["ontoforge", "serve"]).is_err());
    let f = fixture().await;
    let mut bad = args(&f, unused_address());
    bad.server_config = Some(f.config.with_file_name("missing.toml"));
    assert_ne!(run_with_shutdown(&bad, async {}).await, 0);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    assert_ne!(
        run_with_shutdown(&args(&f, listener.local_addr().unwrap()), async {}).await,
        0
    );
    let bind = unused_address();
    let mut production = args(&f, bind);
    production.dev = false;
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        run_with_shutdown(&production, async {
            let _ = rx.await;
        })
        .await
    });
    assert!(wait_listening(bind, &task).await);
    let (status, _, body) = http(bind, Some(&f.token)).await;
    assert_eq!(
        status, 401,
        "auth.dev configuration alone must not enable development verification"
    );
    assert_eq!(body["code"], "UNAUTHENTICATED");
    stop.send(()).unwrap();
    assert_eq!(task.await.unwrap(), 0);
}

#[tokio::test]
async fn serve_real_listener_auth_and_shutdown() {
    let f = fixture().await;
    let bind = unused_address();
    let serve_args = args(&f, bind);
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        run_with_shutdown(&serve_args, async {
            let _ = rx.await;
        })
        .await
    });
    assert!(
        wait_listening(bind, &task).await,
        "serve must start a real authenticated listener"
    );
    assert_eq!(http(bind, None).await.0, 401);
    let (status, request_id, body) = http(bind, Some(&f.token)).await;
    assert_eq!(status, 200);
    assert_eq!(body["properties"]["name"], "Berlin");
    stop.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap(),
        0
    );
    assert!(tokio::net::TcpStream::connect(bind).await.is_err());
    let reopened = EditStore::open(settings(&f.path)).await.unwrap();
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path))
        .await
        .unwrap();
    let row = sqlx::query("SELECT principal, request_id FROM of_read_audit")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("principal"), "ontoforge-dev|alice");
    assert_eq!(row.get::<String, _>("request_id"), request_id);
    reopened.read_audit().shutdown().await.unwrap();
    pool.close().await;

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
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path))
        .await
        .unwrap();
    sqlx::query("DROP TABLE of_read_audit")
        .execute(&pool)
        .await
        .unwrap();
    let status = http(bind, Some(&f.token)).await.0;
    assert!([200, 503].contains(&status));
    stop.send(()).unwrap();
    assert_ne!(
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap(),
        0,
        "failed audit drain must return nonzero"
    );
    pool.close().await;
}

#[tokio::test]
async fn openapi_export_without_store_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let first = Utf8PathBuf::from_path_buf(dir.path().join("one.json")).unwrap();
    let parsed = Cli::try_parse_from(["ontoforge", "serve", "--export-openapi", first.as_str()]);
    assert!(parsed.is_ok(), "serve must accept --export-openapi FILE");

    let second = Utf8PathBuf::from_path_buf(dir.path().join("two.json")).unwrap();
    let third = Utf8PathBuf::from_path_buf(dir.path().join("three.json")).unwrap();
    let one = ServeArgs {
        server_config: None,
        bind: unused_address(),
        dev: false,
        export_openapi: Some(first.clone()),
    };
    let two = ServeArgs {
        server_config: None,
        bind: unused_address(),
        dev: false,
        export_openapi: Some(second.clone()),
    };
    let three = ServeArgs {
        server_config: None,
        bind: unused_address(),
        dev: false,
        export_openapi: Some(third.clone()),
    };
    let (a, b, c) = tokio::join!(
        run_with_shutdown(&one, async {}),
        run_with_shutdown(&two, async {}),
        run_with_shutdown(&three, async {})
    );
    assert_eq!((a, b, c), (0, 0, 0));
    let expected = ontoforge_server::api::openapi::render();
    for path in [first, second, third] {
        assert_eq!(std::fs::read_to_string(path).unwrap(), expected);
    }
    let failed = ServeArgs {
        server_config: None,
        bind: unused_address(),
        dev: false,
        export_openapi: Some(Utf8PathBuf::from("missing-parent/openapi.json")),
    };
    assert_ne!(run_with_shutdown(&failed, async {}).await, 0);
}
