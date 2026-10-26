//! Bounded read-audit acceptance and asynchronous persistence.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, SqlitePool};
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinHandle;
use ulid::Ulid;

use crate::error::StoreError;
use crate::store::Db;

#[derive(Debug, Clone, Copy)]
pub struct ReadAuditSettings {
    pub capacity: usize,
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub max_pending_age: Duration,
    pub retention: Duration,
}

impl Default for ReadAuditSettings {
    fn default() -> Self {
        Self {
            capacity: 1024,
            batch_size: 128,
            flush_interval: Duration::from_millis(250),
            max_pending_age: Duration::from_millis(1000),
            retention: Duration::from_hours(30 * 24),
        }
    }
}

impl ReadAuditSettings {
    pub fn validate(self) -> Result<(), StoreError> {
        if !(1..=65_536).contains(&self.capacity)
            || self.batch_size == 0
            || self.batch_size > self.capacity
            || !(Duration::from_millis(1)..=Duration::from_millis(1000))
                .contains(&self.flush_interval)
            || self.max_pending_age < self.flush_interval
            || self.max_pending_age > Duration::from_millis(1000)
            || !(Duration::from_hours(24)..=Duration::from_hours(3650 * 24))
                .contains(&self.retention)
        {
            return Err(StoreError::Config("invalid read audit limits".to_owned()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditUnavailable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadAuditStatus {
    Success,
    Denied,
    SourceError,
    Cancelled,
}

impl ReadAuditStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Denied => "denied",
            Self::SourceError => "source_error",
            Self::Cancelled => "cancelled",
        }
    }
}

pub struct ReadAuditMetadata {
    pub principal: String,
    pub tool: String,
    pub object_type: Option<String>,
    pub condition_api_names: Vec<String>,
    pub effective_roles: Vec<String>,
    pub def_version: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct ReadAuditHealth {
    pub available: bool,
    pub pending: usize,
    pub oldest_pending: Option<Duration>,
}

struct State {
    available: AtomicBool,
    slots: Arc<Semaphore>,
    pending: Mutex<BTreeMap<u64, Instant>>,
    next_id: AtomicU64,
    max_pending_age: Duration,
}

impl State {
    fn health(&self) -> ReadAuditHealth {
        let pending = self.pending.lock().unwrap();
        let oldest = pending.values().min().map(Instant::elapsed);
        ReadAuditHealth {
            available: self.available.load(Ordering::Acquire)
                && oldest.is_none_or(|age| age <= self.max_pending_age),
            pending: pending.len(),
            oldest_pending: oldest,
        }
    }

    fn forget(&self, id: u64) {
        self.pending.lock().unwrap().remove(&id);
    }
}

struct WorkerLiveness(Arc<State>);

impl Drop for WorkerLiveness {
    fn drop(&mut self) {
        self.0.available.store(false, Ordering::Release);
    }
}

struct Record {
    at: String,
    request_id: String,
    metadata: ReadAuditMetadata,
    sql_sha256: String,
    row_count: i64,
    elapsed_ms: i64,
    status: ReadAuditStatus,
}

struct Queued {
    id: u64,
    record: Record,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

enum SinkPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

/// One store owns one worker. Dropping the store closes the channel and drains accepted rows.
pub struct ReadAudit {
    state: Arc<State>,
    sender: Mutex<Option<mpsc::Sender<Queued>>>,
    worker: tokio::sync::Mutex<Option<JoinHandle<Result<(), AuditUnavailable>>>>,
}

impl ReadAudit {
    pub(crate) fn start(db: &Db, settings: ReadAuditSettings) -> Self {
        let state = Arc::new(State {
            available: AtomicBool::new(true),
            slots: Arc::new(Semaphore::new(settings.capacity)),
            pending: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            max_pending_age: settings.max_pending_age,
        });
        let (sender, receiver) = mpsc::channel(settings.capacity);
        let sink = match db {
            Db::Sqlite(pool) => SinkPool::Sqlite(pool.clone()),
            Db::Postgres { pool, .. } => SinkPool::Postgres(pool.clone()),
        };
        let worker_state = Arc::clone(&state);
        let handle =
            tokio::spawn(async move { run_worker(receiver, sink, worker_state, settings).await });
        Self {
            state,
            sender: Mutex::new(Some(sender)),
            worker: tokio::sync::Mutex::new(Some(handle)),
        }
    }

    pub fn health(&self) -> ReadAuditHealth {
        self.state.health()
    }

    pub fn reserve(&self, metadata: ReadAuditMetadata) -> Result<ReadAuditGuard, AuditUnavailable> {
        if !self.state.health().available {
            return Err(AuditUnavailable);
        }
        let permit = Arc::clone(&self.state.slots)
            .try_acquire_owned()
            .map_err(|_| AuditUnavailable)?;
        let sender = self
            .sender
            .lock()
            .unwrap()
            .clone()
            .ok_or(AuditUnavailable)?;
        if sender.is_closed() || !self.state.health().available {
            return Err(AuditUnavailable);
        }
        Ok(ReadAuditGuard {
            state: Arc::clone(&self.state),
            sender,
            id: self.state.next_id.fetch_add(1, Ordering::Relaxed),
            started: Instant::now(),
            at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            request_id: Ulid::generate().to_string(),
            metadata: Some(metadata),
            sql_sha256: "no-sql".to_owned(),
            permit: Some(permit),
        })
    }

    /// Stop admitting reads, then wait for all accepted rows to reach the durable sink.
    pub async fn shutdown(&self) -> Result<(), AuditUnavailable> {
        self.state.available.store(false, Ordering::Release);
        self.sender.lock().unwrap().take();
        let Some(handle) = self.worker.lock().await.take() else {
            return Ok(());
        };
        handle.await.map_err(|_| AuditUnavailable)?
    }
}

/// A reservation lives across awaits. Dropping it during an in-flight read records cancellation.
pub struct ReadAuditGuard {
    state: Arc<State>,
    sender: mpsc::Sender<Queued>,
    id: u64,
    started: Instant,
    at: String,
    request_id: String,
    metadata: Option<ReadAuditMetadata>,
    sql_sha256: String,
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl ReadAuditGuard {
    pub fn observe_sql(&mut self, sql: &str) {
        self.sql_sha256 = format!("{:x}", Sha256::digest(sql.as_bytes()));
    }

    pub fn finish(
        mut self,
        status: ReadAuditStatus,
        row_count: usize,
    ) -> Result<(), AuditUnavailable> {
        self.enqueue(status, row_count)
    }

    fn enqueue(
        &mut self,
        status: ReadAuditStatus,
        row_count: usize,
    ) -> Result<(), AuditUnavailable> {
        let Some(permit) = self.permit.take() else {
            return Ok(());
        };
        if !self.state.health().available {
            return Err(AuditUnavailable);
        }
        let record = Record {
            at: self.at.clone(),
            request_id: self.request_id.clone(),
            metadata: self.metadata.take().expect("audit metadata reserved"),
            sql_sha256: self.sql_sha256.clone(),
            row_count: i64::try_from(row_count).map_err(|_| AuditUnavailable)?,
            elapsed_ms: i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX),
            status,
        };
        self.state
            .pending
            .lock()
            .unwrap()
            .insert(self.id, Instant::now());
        if self
            .sender
            .try_send(Queued {
                id: self.id,
                record,
                _permit: permit,
            })
            .is_err()
        {
            self.state.forget(self.id);
            self.state.available.store(false, Ordering::Release);
            return Err(AuditUnavailable);
        }
        Ok(())
    }
}

impl Drop for ReadAuditGuard {
    fn drop(&mut self) {
        if self.permit.is_some() && self.enqueue(ReadAuditStatus::Cancelled, 0).is_err() {
            eprintln!("read audit unavailable while recording cancellation");
        }
    }
}

async fn run_worker(
    mut receiver: mpsc::Receiver<Queued>,
    sink: SinkPool,
    state: Arc<State>,
    settings: ReadAuditSettings,
) -> Result<(), AuditUnavailable> {
    let _liveness = WorkerLiveness(Arc::clone(&state));
    let mut interval = tokio::time::interval(settings.flush_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut batch = Vec::with_capacity(settings.batch_size);
    loop {
        let mut due = false;
        tokio::select! {
            item = receiver.recv() => match item {
                Some(item) => batch.push(item),
                None => break,
            },
            _ = interval.tick(), if !batch.is_empty() => due = true,
        }
        if due || batch.len() == settings.batch_size {
            flush(&sink, &state, settings, &mut batch).await?;
        }
    }
    if !batch.is_empty() {
        flush(&sink, &state, settings, &mut batch).await?;
    }
    Ok(())
}

async fn flush(
    sink: &SinkPool,
    state: &State,
    settings: ReadAuditSettings,
    batch: &mut Vec<Queued>,
) -> Result<(), AuditUnavailable> {
    let mut failed_since = None;
    loop {
        let write = match sink {
            SinkPool::Sqlite(pool) => write_sqlite(pool, settings.retention, batch).await,
            SinkPool::Postgres(pool) => write_postgres(pool, settings.retention, batch).await,
        };
        if write.is_ok() {
            for item in batch.drain(..) {
                state.forget(item.id);
            }
            state.available.store(true, Ordering::Release);
            return Ok(());
        }
        state.available.store(false, Ordering::Release);
        let first = failed_since.get_or_insert_with(Instant::now);
        if first.elapsed() >= settings.max_pending_age {
            for item in batch.drain(..) {
                state.forget(item.id);
            }
            eprintln!("read audit sink did not recover before the pending age limit");
            return Err(AuditUnavailable);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn write_sqlite(pool: &SqlitePool, retention: Duration, batch: &[Queued]) -> Result<(), ()> {
    let mut tx = pool.begin().await.map_err(|_| ())?;
    let cutoff = (Utc::now() - chrono::Duration::from_std(retention).map_err(|_| ())?)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    sqlx::query("DELETE FROM of_read_audit WHERE at < ?")
        .bind(cutoff)
        .execute(&mut *tx)
        .await
        .map_err(|_| ())?;
    for item in batch {
        let row = &item.record;
        sqlx::query(
            "INSERT INTO of_read_audit (at, request_id, principal, tool, object_type, \
             condition_api_names, effective_roles, sql_sha256, row_count, elapsed_ms, \
             def_version, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&row.at)
        .bind(&row.request_id)
        .bind(&row.metadata.principal)
        .bind(&row.metadata.tool)
        .bind(&row.metadata.object_type)
        .bind(serde_json::to_string(&row.metadata.condition_api_names).map_err(|_| ())?)
        .bind(serde_json::to_string(&row.metadata.effective_roles).map_err(|_| ())?)
        .bind(&row.sql_sha256)
        .bind(row.row_count)
        .bind(row.elapsed_ms)
        .bind(row.metadata.def_version)
        .bind(row.status.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|_| ())?;
    }
    tx.commit().await.map_err(|_| ())
}

async fn write_postgres(pool: &PgPool, retention: Duration, batch: &[Queued]) -> Result<(), ()> {
    let cutoff = Utc::now() - chrono::Duration::from_std(retention).map_err(|_| ())?;
    let mut tx = pool.begin().await.map_err(|_| ())?;
    sqlx::query("DELETE FROM of_read_audit WHERE at < $1")
        .bind(cutoff)
        .execute(&mut *tx)
        .await
        .map_err(|_| ())?;
    for item in batch {
        let row = &item.record;
        let at = chrono::DateTime::parse_from_rfc3339(&row.at).map_err(|_| ())?;
        sqlx::query(
            "INSERT INTO of_read_audit (at, request_id, principal, tool, object_type, \
             condition_api_names, effective_roles, sql_sha256, row_count, elapsed_ms, \
             def_version, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(at)
        .bind(&row.request_id)
        .bind(&row.metadata.principal)
        .bind(&row.metadata.tool)
        .bind(&row.metadata.object_type)
        .bind(sqlx::types::Json(&row.metadata.condition_api_names))
        .bind(sqlx::types::Json(&row.metadata.effective_roles))
        .bind(&row.sql_sha256)
        .bind(row.row_count)
        .bind(row.elapsed_ms)
        .bind(row.metadata.def_version)
        .bind(row.status.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|_| ())?;
    }
    tx.commit().await.map_err(|_| ())
}
