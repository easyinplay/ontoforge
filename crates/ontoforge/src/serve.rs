//! The HTTP host command and its injectable shutdown boundary.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use camino::Utf8Path;
use ontoforge_auth::authn::Authenticator;
use ontoforge_auth::authn::dev::{DevKey, DevMode, DevVerifier};
use ontoforge_engine::AnyEngine;
use ontoforge_engine::config::{AuthConfig, build_trino_engines};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_server::ServerHost;
use ontoforge_storage::store::EditStore;

use crate::cli::ServeArgs;
use crate::reload::load_server;

pub fn run(args: &ServeArgs) -> u8 {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        eprintln!("cannot start HTTP server runtime");
        return 3;
    };
    let signal_failed = Arc::new(AtomicBool::new(false));
    let failed = Arc::clone(&signal_failed);
    let exit = runtime.block_on(async {
        run_with_shutdown(args, async move {
            if tokio::signal::ctrl_c().await.is_err() {
                failed.store(true, Ordering::Relaxed);
                eprintln!("cannot listen for HTTP server shutdown signal");
            }
        })
        .await
    });
    if signal_failed.load(Ordering::Relaxed) {
        3
    } else {
        exit
    }
}

pub async fn run_with_shutdown(
    args: &ServeArgs,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> u8 {
    if let Some(path) = &args.export_openapi {
        return match std::fs::write(path, ontoforge_server::api::openapi::render()) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("cannot write OpenAPI document: {error}");
                1
            }
        };
    }
    let Some(server_config) = &args.server_config else {
        eprintln!("HTTP server needs --server-config unless --export-openapi is used");
        return 1;
    };
    let Ok(server) = load_server(server_config) else {
        eprintln!("invalid server configuration for HTTP server");
        return 1;
    };
    let dir = server_config
        .parent()
        .filter(|dir| !dir.as_str().is_empty())
        .unwrap_or(Utf8Path::new("."));
    let Some(query) = &server.config.query else {
        eprintln!("HTTP server needs signing_key_file in [query]");
        return 1;
    };
    let Ok(signing_key) = query.load_signing_key(dir) else {
        eprintln!("cannot load HTTP server query signing key");
        return 1;
    };
    let auth_config = server.config.auth.unwrap_or_else(|| AuthConfig {
        oidc: Vec::new(),
        principal_attrs: BTreeMap::new(),
        dev: None,
    });
    let Ok(mut auth) = Authenticator::from_config(&auth_config, dir) else {
        eprintln!("invalid authentication configuration");
        return 1;
    };
    if args.dev {
        let Some(file) = auth_config
            .dev
            .as_ref()
            .and_then(|dev| dev.key_file.as_deref())
        else {
            eprintln!("--dev needs key_file in [auth.dev]");
            return 1;
        };
        let Ok(key) = DevKey::load_or_create(&dir.join(file)) else {
            eprintln!("cannot open development signing key");
            return 1;
        };
        auth = auth.with_dev(DevVerifier::new(
            DevMode::enable_insecure(),
            key,
            &auth_config.principal_attrs,
        ));
    }
    let engine = if let Some(config) = server.config.trino {
        let Ok(engines) = build_trino_engines(&config, dir, &|name| std::env::var(name).ok())
        else {
            eprintln!("cannot initialize configured query engine");
            return 1;
        };
        AnyEngine::Trino(engines.ontology)
    } else {
        let Ok(engine) = SqliteEngine::in_memory().await else {
            eprintln!("cannot initialize query engine");
            return 3;
        };
        AnyEngine::Sqlite(engine)
    };
    let Ok(listener) = tokio::net::TcpListener::bind(args.bind).await else {
        eprintln!("cannot bind HTTP listener");
        return 3;
    };
    let Ok(store) = EditStore::open(server.settings).await else {
        eprintln!("cannot open HTTP server edit store");
        return 3;
    };
    let store = Arc::new(store);
    let host = ServerHost::new(
        Arc::clone(&store),
        Arc::new(engine),
        auth,
        auth_config,
        signing_key,
    );
    let Ok(host) = host else {
        let _ = store.read_audit().shutdown().await;
        eprintln!("cannot initialize HTTP server signing key");
        return 3;
    };
    match Arc::new(host).serve(listener, shutdown).await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            3
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/serve.test.rs"]
mod tests;
