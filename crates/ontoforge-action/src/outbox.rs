//! Delivery of committed Action events from the durable outbox.

use std::time::Duration;

use chrono::{DateTime, Utc};
use ontoforge_storage::commit::ClaimedOutboxEvent;
use ontoforge_storage::error::StoreError;
use ontoforge_storage::store::EditStore;

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("invalid webhook configuration")]
    Config,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("delivery interrupted after the receiver responded")]
    Interrupted,
}

/// Trusted host configuration. Action requests and ontology rules cannot set a target.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    target: reqwest::Url,
    lease_duration: Duration,
    request_timeout: Duration,
    batch_limit: u32,
}

impl WebhookConfig {
    pub fn new(
        target: &str,
        lease_duration: Duration,
        request_timeout: Duration,
        batch_limit: u32,
    ) -> Result<Self, WebhookError> {
        let target = reqwest::Url::parse(target).map_err(|_| WebhookError::Config)?;
        if !matches!(target.scheme(), "http" | "https")
            || target.host().is_none()
            || !target.username().is_empty()
            || target.password().is_some()
            || target.fragment().is_some()
            || request_timeout.is_zero()
            || lease_duration <= request_timeout
            || batch_limit == 0
            || batch_limit > 100
            || chrono::Duration::from_std(lease_duration).is_err()
        {
            return Err(WebhookError::Config);
        }
        Ok(Self {
            target,
            lease_duration,
            request_timeout,
            batch_limit,
        })
    }
}

/// Sends the unchanged stored JSON body, never reads the current object state or response body.
pub struct WebhookTransport {
    client: reqwest::Client,
}

impl WebhookTransport {
    fn new() -> Result<Self, WebhookError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| WebhookError::Config)?,
        })
    }

    async fn send(&self, config: &WebhookConfig, event: &ClaimedOutboxEvent) -> Result<(), String> {
        let response = self
            .client
            .post(config.target.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header("X-Ontoforge-Event-Id", &event.event_id)
            .timeout(config.request_timeout)
            .body(event.payload.clone())
            .send()
            .await
            .map_err(|_| "transport_error".to_owned())?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("http_status_{}", response.status().as_u16()))
        }
    }
}

/// One bounded poll claims events, commits the lease, sends HTTP, then finalizes each result.
pub struct WebhookWorker<'a> {
    store: &'a EditStore,
    config: WebhookConfig,
    transport: WebhookTransport,
    worker_id: String,
}

impl<'a> WebhookWorker<'a> {
    pub fn new(store: &'a EditStore, config: WebhookConfig) -> Result<Self, WebhookError> {
        Ok(Self {
            store,
            config,
            transport: WebhookTransport::new()?,
            worker_id: ulid::Ulid::generate().to_string(),
        })
    }

    pub async fn poll_at(&self, now: DateTime<Utc>) -> Result<usize, WebhookError> {
        self.poll_at_with_after_send(now, |_| Ok(())).await
    }

    /// The callback permits a host to interrupt delivery after 2xx and before the mark.
    /// A failure leaves the lease in place for recovery after its expiry.
    pub async fn poll_at_with_after_send(
        &self,
        now: DateTime<Utc>,
        mut after_send: impl FnMut(&ClaimedOutboxEvent) -> Result<(), WebhookError>,
    ) -> Result<usize, WebhookError> {
        let until = now
            + chrono::Duration::from_std(self.config.lease_duration)
                .map_err(|_| WebhookError::Config)?;
        let events = self
            .store
            .claim_due_outbox(now, &self.worker_id, until, self.config.batch_limit)
            .await?;
        let count = events.len();
        for event in events {
            match self.transport.send(&self.config, &event).await {
                Ok(()) => {
                    after_send(&event)?;
                    self.store
                        .mark_delivered(&event.event_id, &event.lease_owner, now)
                        .await?;
                }
                Err(category) => {
                    let attempts = event.attempt_count.checked_add(1).ok_or_else(|| {
                        StoreError::Corrupt("outbox attempt count overflow".to_owned())
                    })?;
                    let backoff = 1_i64 << (attempts - 1).min(16);
                    let retry_at = now + chrono::Duration::seconds(backoff.min(300));
                    self.store
                        .record_retry(
                            &event.event_id,
                            &event.lease_owner,
                            attempts,
                            retry_at,
                            &category,
                        )
                        .await?;
                }
            }
        }
        Ok(count)
    }
}
