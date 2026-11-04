//! Cancelling a Trino query that nobody wants any more (ENGINE-01).
//!
//! A query keeps running on the coordinator until its results are consumed or someone sends
//! `DELETE nextUri`. [`QueryGuard`] remembers the `DELETE` for the newest `nextUri` and sends
//! it at the three moments a query is abandoned: the timeout fires, a limit cuts the result
//! short, or the caller drops the query future (the client disconnected).
//!
//! The guard only ever holds a request built for a `nextUri` that passed the same-origin
//! check, so a cancel cannot carry credentials to another host.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use reqwest::RequestBuilder;
use tokio::runtime::Handle;
use tokio::sync::{Notify, oneshot};

use crate::EngineError;

/// How long a cancel may take. Trino answers 204, or 404 when the query is already gone;
/// both count as done, and a coordinator that does not answer in time is given up on.
const DELETE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(super) struct CancellationTasks {
    active: AtomicUsize,
    changed: Notify,
    failed: AtomicBool,
}

impl CancellationTasks {
    fn start(self: &Arc<Self>, delete: RequestBuilder) -> Option<oneshot::Receiver<()>> {
        self.spawn(send_delete(delete), true)
    }

    /// Tracks completion when the cleanup owner retains and validates recoverable errors.
    pub(super) fn observe(
        self: &Arc<Self>,
        cleanup: impl Future<Output = ()> + Send + 'static,
    ) -> Option<oneshot::Receiver<()>> {
        self.spawn(
            async move {
                cleanup.await;
                true
            },
            false,
        )
    }

    fn spawn(
        self: &Arc<Self>,
        cleanup: impl Future<Output = bool> + Send + 'static,
        sticky: bool,
    ) -> Option<oneshot::Receiver<()>> {
        let Ok(runtime) = Handle::try_current() else {
            if sticky {
                self.failed.store(true, Ordering::Release);
            }
            return None;
        };
        let (done, completed) = oneshot::channel();
        self.active.fetch_add(1, Ordering::AcqRel);
        let completion = CancellationCompletion {
            tasks: Arc::clone(self),
            success: false,
            sticky,
        };
        runtime.spawn(async move {
            let mut completion = completion;
            completion.success = cleanup.await;
            let _ = done.send(());
        });
        Some(completed)
    }

    pub(super) async fn drain(&self) -> Result<(), EngineError> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.active.load(Ordering::Acquire) == 0 {
                break;
            }
            changed.await;
        }
        if self.failed.load(Ordering::Acquire) {
            Err(EngineError::Unavailable {
                retryable: false,
                reason: "Trino cancellation cleanup failed".to_owned(),
            })
        } else {
            Ok(())
        }
    }
}

struct CancellationCompletion {
    tasks: Arc<CancellationTasks>,
    success: bool,
    sticky: bool,
}

impl Drop for CancellationCompletion {
    fn drop(&mut self) {
        if self.sticky && !self.success {
            self.tasks.failed.store(true, Ordering::Release);
        }
        self.tasks.active.fetch_sub(1, Ordering::AcqRel);
        self.tasks.changed.notify_waiters();
    }
}

/// Cancels the query it watches unless it was finished first.
pub(super) struct QueryGuard {
    /// The `DELETE` for the newest `nextUri`; `None` before the first one and after the last.
    pending: Option<RequestBuilder>,
    finished: bool,
    cancellations: Arc<CancellationTasks>,
}

impl QueryGuard {
    pub(super) fn new(cancellations: Arc<CancellationTasks>) -> Self {
        Self {
            pending: None,
            finished: false,
            cancellations,
        }
    }

    /// Records the `DELETE` for the newest `nextUri`, or `None` when the page had none.
    pub(super) fn advance(&mut self, delete: Option<RequestBuilder>) {
        self.pending = delete;
    }

    /// The query ended by itself (completed, or failed on the server): nothing to cancel.
    pub(super) fn finish(&mut self) {
        self.finished = true;
        self.pending = None;
    }

    /// Cancels now and waits for the answer, for the cases where the caller is still there
    /// to see the outcome (timeout, limits, protocol errors). Cleanup failure is retained
    /// for shutdown without changing the original query outcome.
    pub(super) async fn cancel_now(&mut self) {
        self.finished = true;
        if let Some(delete) = self.pending.take()
            && let Some(completed) = self.cancellations.start(delete)
        {
            let _ = completed.await;
        }
    }
}

impl Drop for QueryGuard {
    /// A guard dropped before the query ended means the caller walked away. The cancel is
    /// tracked by the engine on the current runtime. Without a runtime there is nobody to
    /// run it and the server's own `query_max_execution_time` is the only bound left.
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Some(delete) = self.pending.take() {
            let _ = self.cancellations.start(delete);
        }
    }
}

async fn send_delete(delete: RequestBuilder) -> bool {
    matches!(tokio::time::timeout(DELETE_TIMEOUT, delete.send()).await,
        Ok(Ok(response)) if response.status().is_success() || response.status().as_u16() == 404)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_an_unfinished_guard_without_a_runtime_does_not_panic() {
        let delete = reqwest::Client::new().delete("http://127.0.0.1:9/v1/statement/q1/1");
        let mut guard = QueryGuard::new(Arc::default());
        guard.advance(Some(delete));
        // This thread has no tokio runtime: the cancel is skipped.
        std::thread::spawn(move || drop(guard))
            .join()
            .expect("drop must not panic");
    }

    #[test]
    fn a_finished_guard_holds_nothing_to_send() {
        let delete = reqwest::Client::new().delete("http://127.0.0.1:9/v1/statement/q1/1");
        let mut guard = QueryGuard::new(Arc::default());
        guard.advance(Some(delete));
        guard.finish();
        assert!(guard.pending.is_none());
        assert!(guard.finished);
    }
}
