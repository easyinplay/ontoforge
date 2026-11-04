//! Long-running delivery of Action events from the durable outbox.

use std::time::Duration;

use chrono::Utc;
use ontoforge_action::outbox::{WebhookConfig, WebhookWorker};
use ontoforge_storage::store::EditStore;

use crate::cli::WebhookWorkerArgs;
use crate::reload::load_server;

pub fn run(args: &WebhookWorkerArgs) -> u8 {
    let Ok(server) = load_server(&args.server_config) else {
        eprintln!("invalid server configuration for webhook worker");
        return 1;
    };
    let Some(config) = server.config.webhook else {
        eprintln!("server configuration needs a [webhook] section");
        return 1;
    };
    let Ok(webhook) = WebhookConfig::new(
        &config.target,
        Duration::from_millis(config.lease_duration_ms),
        Duration::from_millis(config.request_timeout_ms),
        config.batch_limit,
    ) else {
        eprintln!("invalid [webhook] settings");
        return 1;
    };
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        eprintln!("cannot start webhook worker runtime");
        return 3;
    };
    runtime.block_on(async move {
        let Ok(store) = EditStore::open(server.settings).await else {
            eprintln!("cannot open webhook worker edit store");
            return 3;
        };
        let Ok(worker) = WebhookWorker::new(&store, webhook) else {
            eprintln!("cannot initialize webhook worker");
            return 3;
        };
        let interval = Duration::from_millis(config.poll_interval_ms);
        let shutdown = tokio::signal::ctrl_c();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                result = worker.poll_at(Utc::now()) => {
                    if result.is_err() {
                        eprintln!("webhook worker poll failed; retrying");
                    }
                }
                _ = &mut shutdown => return 0,
            }
            tokio::select! {
                () = tokio::time::sleep(interval) => {},
                _ = &mut shutdown => return 0,
            }
        }
    })
}
