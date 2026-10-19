//! Cancelling a Trino query that nobody wants any more (ENGINE-01).
//!
//! A query keeps running on the coordinator until its results are consumed or someone sends
//! `DELETE nextUri`. [`QueryGuard`] remembers the `DELETE` for the newest `nextUri` and sends
//! it at the three moments a query is abandoned: the timeout fires, a limit cuts the result
//! short, or the caller drops the query future (the client disconnected).
//!
//! The guard only ever holds a request built for a `nextUri` that passed the same-origin
//! check, so a cancel cannot carry credentials to another host.

use std::time::Duration;

use reqwest::RequestBuilder;
use tokio::runtime::Handle;

/// How long a cancel may take. Trino answers 204, or 404 when the query is already gone;
/// both count as done, and a coordinator that does not answer in time is given up on.
const DELETE_TIMEOUT: Duration = Duration::from_secs(2);

/// Cancels the query it watches unless it was finished first.
pub(super) struct QueryGuard {
    /// The `DELETE` for the newest `nextUri`; `None` before the first one and after the last.
    pending: Option<RequestBuilder>,
    finished: bool,
}

impl QueryGuard {
    pub(super) fn new() -> Self {
        Self {
            pending: None,
            finished: false,
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
    /// to see the outcome (timeout, limits, protocol errors). Best effort: the answer is not
    /// inspected, only awaited for at most [`DELETE_TIMEOUT`].
    pub(super) async fn cancel_now(&mut self) {
        self.finished = true;
        if let Some(delete) = self.pending.take() {
            send_delete(delete).await;
        }
    }
}

impl Drop for QueryGuard {
    /// A guard dropped before the query ended means the caller walked away. The cancel is
    /// fire-and-forget on the current runtime; without one there is nobody to run it and the
    /// server's own `query_max_execution_time` is the only bound left.
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Some(delete) = self.pending.take()
            && let Ok(runtime) = Handle::try_current()
        {
            runtime.spawn(send_delete(delete));
        }
    }
}

async fn send_delete(delete: RequestBuilder) {
    // 2xx and 404 are success, anything else (or no answer) is as good as we can do.
    let _ = tokio::time::timeout(DELETE_TIMEOUT, delete.send()).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_an_unfinished_guard_without_a_runtime_does_not_panic() {
        let delete = reqwest::Client::new().delete("http://127.0.0.1:9/v1/statement/q1/1");
        let mut guard = QueryGuard::new();
        guard.advance(Some(delete));
        // This thread has no tokio runtime: the cancel is skipped.
        std::thread::spawn(move || drop(guard))
            .join()
            .expect("drop must not panic");
    }

    #[test]
    fn a_finished_guard_holds_nothing_to_send() {
        let delete = reqwest::Client::new().delete("http://127.0.0.1:9/v1/statement/q1/1");
        let mut guard = QueryGuard::new();
        guard.advance(Some(delete));
        guard.finish();
        assert!(guard.pending.is_none());
        assert!(guard.finished);
    }
}
