//! Explicit ownership for host lifecycle resources.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
use ontoforge_engine::trino::l2_monitor::L2AclMonitor;

use crate::ServerHost;

type L2StatusProvider = Arc<dyn Fn() -> Option<L2HealthStatus> + Send + Sync>;

pub struct ServerState {
    accepting: std::sync::atomic::AtomicBool,
    l2_status: RwLock<Option<L2StatusProvider>>,
    monitor: Mutex<Option<L2AclMonitor>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            accepting: std::sync::atomic::AtomicBool::new(true),
            l2_status: RwLock::new(None),
            monitor: Mutex::new(None),
        }
    }
}

impl ServerState {
    pub fn stop_admission(&self) {
        self.accepting
            .store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn accepting(&self) -> bool {
        self.accepting.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn l2_status(&self) -> Option<L2HealthStatus> {
        self.l2_status
            .read()
            .ok()
            .and_then(|provider| provider.as_ref().and_then(|provider| provider()))
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
}

static STATES: OnceLock<Mutex<HashMap<usize, Arc<ServerState>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<usize, Arc<ServerState>>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key<E>(host: &ServerHost<E>) -> usize {
    Arc::as_ptr(&host.store) as usize
}

pub(crate) fn for_host<E>(host: &ServerHost<E>) -> Arc<ServerState> {
    let mut states = registry().lock().expect("server state registry poisoned");
    Arc::clone(
        states
            .entry(key(host))
            .or_insert_with(|| Arc::new(ServerState::default())),
    )
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
