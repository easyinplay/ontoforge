//! Host-owned legacy sessions, including initialization and resource reclamation.

use futures_util::Stream;
use rmcp::model::{ClientJsonRpcMessage, GetExtensions, ServerJsonRpcMessage};
use rmcp::transport::streamable_http_server::session::{
    ServerSseMessage, SessionId, SessionManager,
    local::{LocalSessionManager, LocalSessionManagerError},
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionVerifyError {
    Missing,
    Mismatched,
}

struct Entry {
    owner: Option<String>,
    touched: Instant,
    cancellation: CancellationToken,
}

/// The request owns the guard; extensions only share its pending cancellation.
#[derive(Clone)]
pub(crate) struct InitializationSlot(Arc<Mutex<Option<CancellationToken>>>);

pub(crate) struct InitializationGuard {
    slot: InitializationSlot,
    cleanup: Arc<tokio::sync::Notify>,
}

impl InitializationGuard {
    pub(crate) fn slot(&self) -> InitializationSlot {
        self.slot.clone()
    }
    pub(crate) fn disarm(&self) {
        self.slot
            .0
            .lock()
            .expect("initialization slot poisoned")
            .take();
    }
}

impl Drop for InitializationGuard {
    fn drop(&mut self) {
        if let Some(cancellation) = self
            .slot
            .0
            .lock()
            .expect("initialization slot poisoned")
            .take()
        {
            cancellation.cancel();
            self.cleanup.notify_one();
        }
    }
}

pub struct BoundedSessionManager {
    pub(crate) inner: LocalSessionManager,
    entries: Mutex<HashMap<SessionId, Entry>>,
    mutation: tokio::sync::Mutex<()>,
    capacity: usize,
    ttl: Duration,
    stopped: AtomicBool,
    stop: CancellationToken,
    cleanup: Arc<tokio::sync::Notify>,
    reaper: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Default for BoundedSessionManager {
    fn default() -> Self {
        Self::new(256, Duration::from_mins(30))
    }
}

impl BoundedSessionManager {
    pub(crate) fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            inner: LocalSessionManager::default(),
            entries: Mutex::new(HashMap::new()),
            mutation: tokio::sync::Mutex::new(()),
            capacity: capacity.max(1),
            ttl,
            stopped: AtomicBool::new(false),
            stop: CancellationToken::new(),
            cleanup: Arc::new(tokio::sync::Notify::new()),
            reaper: Mutex::new(None),
        }
    }

    pub(crate) fn initialization_guard(&self) -> InitializationGuard {
        InitializationGuard {
            slot: InitializationSlot(Arc::new(Mutex::new(None))),
            cleanup: Arc::clone(&self.cleanup),
        }
    }

    pub(crate) fn bind(&self, id: &str, owner: &str) -> Result<(), SessionVerifyError> {
        let mut entries = self.entries.lock().expect("session owners poisoned");
        let entry = entries.get_mut(id).ok_or(SessionVerifyError::Missing)?;
        if owner.is_empty()
            || entry.cancellation.is_cancelled()
            || entry.touched.elapsed() > self.ttl
        {
            return Err(SessionVerifyError::Missing);
        }
        if entry
            .owner
            .as_deref()
            .is_some_and(|existing| existing != owner)
        {
            return Err(SessionVerifyError::Mismatched);
        }
        entry.owner = Some(owner.to_owned());
        entry.touched = Instant::now();
        Ok(())
    }

    pub(crate) fn verify(&self, id: &str, owner: &str) -> Result<(), SessionVerifyError> {
        let mut entries = self.entries.lock().expect("session owners poisoned");
        let entry = entries.get_mut(id).ok_or(SessionVerifyError::Missing)?;
        if entry.cancellation.is_cancelled()
            || entry.touched.elapsed() > self.ttl
            || entry.owner.is_none()
        {
            return Err(SessionVerifyError::Missing);
        }
        if entry.owner.as_deref() != Some(owner) {
            return Err(SessionVerifyError::Mismatched);
        }
        entry.touched = Instant::now();
        Ok(())
    }

    pub(crate) fn start(self: &Arc<Self>) {
        let mut task = self.reaper.lock().expect("session reaper poisoned");
        if task.is_some() || self.stopped.load(Ordering::Acquire) {
            return;
        }
        let manager = Arc::downgrade(self);
        let stop = self.stop.clone();
        let cleanup = Arc::clone(&self.cleanup);
        let period = self
            .ttl
            .min(Duration::from_secs(30))
            .max(Duration::from_millis(1));
        *task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(period);
            loop {
                tokio::select! {
                    biased;
                    () = stop.cancelled() => break,
                    () = cleanup.notified() => {},
                    _ = interval.tick() => {},
                }
                let Some(manager) = manager.upgrade() else {
                    break;
                };
                let _mutation = manager.mutation.lock().await;
                let _ = manager.expire_locked().await;
            }
        }));
    }

    async fn close_locked(&self, id: &SessionId) -> Result<(), LocalSessionManagerError> {
        let cancellation = self
            .entries
            .lock()
            .expect("session owners poisoned")
            .get(id)
            .map(|entry| entry.cancellation.clone());
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
            self.cleanup.notify_one();
        }
        // Keep the entry counted if this await is cancelled. The reaper sees its
        // cancelled token and retries; removal from both maps has no await.
        let mut sessions = self.inner.sessions.write().await;
        sessions.remove(id);
        self.entries
            .lock()
            .expect("session owners poisoned")
            .remove(id);
        // Dropping the handle closes its sender; cancellation also wakes workers
        // with cloned handles. Avoid a potentially blocked Close enqueue.
        Ok(())
    }

    async fn expire_locked(&self) -> Result<(), LocalSessionManagerError> {
        let expired: Vec<_> = self
            .entries
            .lock()
            .expect("session owners poisoned")
            .iter()
            .filter(|(_, entry)| {
                entry.cancellation.is_cancelled() || entry.touched.elapsed() > self.ttl
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in expired {
            self.close_locked(&id).await?;
        }
        Ok(())
    }

    pub(crate) async fn shutdown(&self) -> Result<(), ()> {
        self.stopped.store(true, Ordering::Release);
        self.stop.cancel();
        let task = self.reaper.lock().expect("session reaper poisoned").take();
        let joined = if let Some(task) = task {
            task.await.map_err(|_| ())
        } else {
            Ok(())
        };
        let _mutation = self.mutation.lock().await;
        let ids: Vec<_> = self
            .entries
            .lock()
            .expect("session owners poisoned")
            .keys()
            .cloned()
            .collect();
        for id in ids {
            self.close_locked(&id).await.map_err(|_| ())?;
        }
        joined
    }
}

impl SessionManager for BoundedSessionManager {
    type Error = LocalSessionManagerError;
    type Transport = <LocalSessionManager as SessionManager>::Transport;

    async fn create_session(&self) -> Result<(SessionId, Self::Transport), Self::Error> {
        let _mutation = self.mutation.lock().await;
        if self.stopped.load(Ordering::Acquire) {
            return Err(LocalSessionManagerError::SessionNotFound("closed".into()));
        }
        self.expire_locked().await?;
        let oldest = {
            let entries = self.entries.lock().expect("session owners poisoned");
            (entries.len() >= self.capacity)
                .then(|| {
                    entries
                        .iter()
                        .min_by_key(|(_, entry)| entry.touched)
                        .map(|(id, _)| id.clone())
                })
                .flatten()
        };
        if let Some(id) = oldest {
            self.close_locked(&id).await?;
        }
        let (id, transport) = self.inner.create_session().await?;
        self.entries
            .lock()
            .expect("session owners poisoned")
            .insert(
                id.clone(),
                Entry {
                    owner: None,
                    touched: Instant::now(),
                    cancellation: transport.cancel_token(),
                },
            );
        Ok((id, transport))
    }

    async fn initialize_session(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<ServerJsonRpcMessage, Self::Error> {
        if let ClientJsonRpcMessage::Request(request) = &message
            && let Some(slot) = request
                .request
                .extensions()
                .get::<axum::http::request::Parts>()
                .and_then(|parts| parts.extensions.get::<InitializationSlot>())
        {
            *slot.0.lock().expect("initialization slot poisoned") = self
                .entries
                .lock()
                .expect("session owners poisoned")
                .get(id)
                .map(|entry| entry.cancellation.clone());
        }
        let result = self.inner.initialize_session(id, message).await;
        if result.is_err() {
            self.close_session(id).await?;
        }
        result
    }

    async fn has_session(&self, id: &SessionId) -> Result<bool, Self::Error> {
        self.inner.has_session(id).await
    }
    async fn close_session(&self, id: &SessionId) -> Result<(), Self::Error> {
        let _mutation = self.mutation.lock().await;
        self.close_locked(id).await
    }
    async fn create_stream(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        self.inner.create_stream(id, message).await
    }
    async fn accept_message(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        self.inner.accept_message(id, message).await
    }
    async fn create_standalone_stream(
        &self,
        id: &SessionId,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        self.inner.create_standalone_stream(id).await
    }
    async fn resume(
        &self,
        id: &SessionId,
        last_event_id: String,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + Sync + 'static, Self::Error> {
        self.inner.resume(id, last_event_id).await
    }
}
