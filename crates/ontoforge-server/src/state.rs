//! Explicit ownership for host lifecycle resources.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use ontoforge_engine::EngineError;
use ontoforge_engine::printer::l2::McpDefaults;
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
use ontoforge_engine::trino::l2_monitor::L2AclMonitor;
use ontoforge_storage::store::EditStore;

use crate::ServerHost;

type L2StatusProvider = Arc<dyn Fn() -> Option<L2HealthStatus> + Send + Sync>;

pub struct ServerState {
    accepting: std::sync::atomic::AtomicBool,
    l2_status: RwLock<Option<L2StatusProvider>>,
    l2_engine: RwLock<Option<Arc<TrinoEngine>>>,
    monitor: Mutex<Option<L2AclMonitor>>,
    drift: Mutex<Option<Arc<crate::drift::SourceDriftMonitor>>>,
    mcp_defaults: RwLock<McpDefaults>,
    pub read_cancellation: tokio_util::sync::CancellationToken,
    pub read_tasks: tokio_util::task::TaskTracker,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            accepting: std::sync::atomic::AtomicBool::new(true),
            l2_status: RwLock::new(None),
            l2_engine: RwLock::new(None),
            monitor: Mutex::new(None),
            drift: Mutex::new(None),
            mcp_defaults: RwLock::new(McpDefaults::default()),
            read_cancellation: tokio_util::sync::CancellationToken::new(),
            read_tasks: tokio_util::task::TaskTracker::new(),
        }
    }
}

impl ServerState {
    pub(crate) fn start_drift<E: ontoforge_engine::Engine + Send + Sync + 'static>(
        &self,
        host: &ServerHost<E>,
    ) -> Result<tokio::sync::oneshot::Receiver<Result<(), ()>>, ()> {
        let mut slot = self.drift.lock().map_err(|_| ())?;
        if slot.is_some() {
            return Err(());
        }
        let (monitor, ready) = crate::drift::SourceDriftMonitor::start(
            Arc::clone(&host.store),
            Arc::clone(&host.engine),
        );
        *slot = Some(Arc::new(monitor));
        Ok(ready)
    }

    pub(crate) fn drift_view(
        &self,
        active: &ontoforge_storage::store::ActiveDefinition,
    ) -> Arc<crate::drift::DriftView> {
        self.drift
            .lock()
            .expect("drift owner poisoned")
            .as_ref()
            .map_or_else(
                || Arc::new(crate::drift::DriftView::Pending),
                |monitor| monitor.capture(active),
            )
    }

    pub(crate) fn take_drift(&self) -> Option<Arc<crate::drift::SourceDriftMonitor>> {
        self.drift.lock().ok().and_then(|mut slot| slot.take())
    }

    pub fn stop_admission(&self) {
        self.accepting
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn accepting(&self) -> bool {
        self.accepting.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn cancel_reads(&self) {
        self.read_cancellation.cancel();
    }

    pub fn l2_status(&self) -> Option<L2HealthStatus> {
        if let Some(engine) = self.l2_engine() {
            return engine.l2_status();
        }
        self.l2_status
            .read()
            .ok()
            .and_then(|provider| provider.as_ref().and_then(|provider| provider()))
    }

    pub(crate) fn l2_engine(&self) -> Option<Arc<TrinoEngine>> {
        self.l2_engine.read().ok().and_then(|engine| engine.clone())
    }

    pub fn set_l2_status_provider<F>(&self, provider: F)
    where
        F: Fn() -> Option<L2HealthStatus> + Send + Sync + 'static,
    {
        if let Ok(mut slot) = self.l2_status.write() {
            *slot = Some(Arc::new(provider));
        }
    }

    pub fn set_l2_status(&self, status: Option<L2HealthStatus>) {
        self.set_l2_status_provider(move || status);
    }

    pub fn install_monitor(&self, monitor: L2AclMonitor) -> Result<(), L2AclMonitor> {
        let Ok(mut slot) = self.monitor.lock() else {
            return Err(monitor);
        };
        if slot.is_some() {
            return Err(monitor);
        }
        *slot = Some(monitor);
        Ok(())
    }

    pub fn take_monitor(&self) -> Option<L2AclMonitor> {
        self.monitor.lock().ok().and_then(|mut slot| slot.take())
    }

    pub fn mcp_defaults(&self) -> McpDefaults {
        self.mcp_defaults
            .read()
            .map(|defaults| defaults.clone())
            .unwrap_or_default()
    }

    pub fn set_mcp_defaults(&self, defaults: McpDefaults) {
        if let Ok(mut slot) = self.mcp_defaults.write() {
            *slot = defaults;
        }
    }
}

static STATES: OnceLock<Mutex<HashMap<usize, Arc<ServerState>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<usize, Arc<ServerState>>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key<E>(host: &ServerHost<E>) -> usize {
    key_store(&host.store)
}

fn key_store(store: &EditStore) -> usize {
    std::ptr::from_ref(store) as usize
}

pub(crate) fn for_host<E>(host: &ServerHost<E>) -> Arc<ServerState> {
    for_store(&host.store)
}

pub(crate) fn for_store(store: &EditStore) -> Arc<ServerState> {
    let mut states = registry().lock().expect("server state registry poisoned");
    Arc::clone(
        states
            .entry(key_store(store))
            .or_insert_with(|| Arc::new(ServerState::default())),
    )
}

pub fn set_mcp_defaults(store: &EditStore, defaults: McpDefaults) {
    for_store(store).set_mcp_defaults(defaults);
}

/// Installs only an engine with L2 proof/admission enforcement. The monitor and
/// raw tools must share this instance; an ontology engine is never a fallback.
pub fn set_l2_engine(store: &EditStore, engine: Arc<TrinoEngine>) -> Result<(), EngineError> {
    if engine.l2_status().is_none() {
        return Err(EngineError::Config("not an L2 engine".to_owned()));
    }
    let state = for_store(store);
    let mut slot = state
        .l2_engine
        .write()
        .map_err(|_| EngineError::Config("L2 capability unavailable".to_owned()))?;
    *slot = Some(engine);
    Ok(())
}

pub(crate) fn remove<E>(host: &ServerHost<E>) {
    if let Ok(mut states) = registry().lock() {
        states.remove(&key(host));
    }
}

pub(crate) fn reason_code(reason: L2DisableReason) -> &'static str {
    match reason {
        L2DisableReason::NotConfigured => "NOT_CONFIGURED",
        L2DisableReason::NotVerified => "NOT_VERIFIED",
        L2DisableReason::InvalidProbe => "INVALID_PROBE",
        L2DisableReason::ObjectUnconfirmed => "OBJECT_UNCONFIRMED",
        L2DisableReason::AllowedProbeFailed => "ALLOWED_PROBE_FAILED",
        L2DisableReason::NegativeProbeFailed => "NEGATIVE_PROBE_FAILED",
        L2DisableReason::OtherQueryUnconfirmed => "OTHER_QUERY_UNCONFIRMED",
        L2DisableReason::QueryVisibilityFailed => "QUERY_VISIBILITY_FAILED",
        L2DisableReason::TimedOut => "TIMED_OUT",
        L2DisableReason::Expired => "EXPIRED",
        L2DisableReason::Stopped => "STOPPED",
        L2DisableReason::CleanupFailed => "CLEANUP_FAILED",
    }
}
