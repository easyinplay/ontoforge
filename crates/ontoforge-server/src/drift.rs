//! One host-owned source metadata check and immutable per-request admission views.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ontoforge_engine::bind_check::{DegradeReason, DriftReport, TargetHealth, check_drift};
use ontoforge_engine::{Engine, QueryContext, SourceTargetError};
use ontoforge_schema::ir::{Backing, BindingTarget, ProfileName};
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use tokio::sync::{Notify, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

const INTERVAL: Duration = Duration::from_secs(30);
const ROUND_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    version: i64,
    profile: Option<ProfileName>,
}

impl From<&ActiveDefinition> for Key {
    fn from(active: &ActiveDefinition) -> Self {
        Self {
            version: active.def_version,
            profile: active.ontology.default_profile.clone(),
        }
    }
}

#[derive(Debug)]
pub enum DriftView {
    NotApplicable,
    Pending,
    Checked(DriftReport),
    EngineFailed,
    CheckFailed,
}

impl DriftView {
    pub fn check(&self, target: &BindingTarget) -> Result<(), SourceTargetError> {
        match self {
            Self::Pending => Err(SourceTargetError::Pending),
            Self::NotApplicable | Self::EngineFailed | Self::CheckFailed => {
                Err(SourceTargetError::Unavailable)
            }
            Self::Checked(report) => match match target {
                BindingTarget::ObjectType(name) => report.objects.get(name),
                BindingTarget::LinkType(name) => report.links.get(name),
            } {
                Some(TargetHealth::Healthy { .. }) => Ok(()),
                Some(TargetHealth::Degraded {
                    reason: DegradeReason::SourceSchemaDrift,
                    ..
                }) => Err(SourceTargetError::SchemaDrift),
                Some(TargetHealth::Degraded {
                    reason: DegradeReason::SourceUnreadable { .. },
                    ..
                }) => Err(SourceTargetError::Unreadable),
                None => Err(SourceTargetError::Unavailable),
            },
        }
    }

    /// `Some(true)` is a complete report with target degradation, regardless of count.
    pub(crate) fn degraded(&self) -> Option<bool> {
        match self {
            Self::NotApplicable => Some(false),
            Self::Checked(report) => Some(!report.is_healthy()),
            Self::Pending | Self::EngineFailed | Self::CheckFailed => None,
        }
    }
}

#[derive(Default)]
struct Slot {
    observed: Option<Key>,
    report: Option<(Key, Arc<DriftView>)>,
}

#[derive(Default)]
struct Shared {
    slot: Mutex<Slot>,
    wake: Notify,
    stopped: std::sync::atomic::AtomicBool,
}

impl Shared {
    fn capture(&self, key: &Key) -> Arc<DriftView> {
        if self.stopped.load(std::sync::atomic::Ordering::Acquire) {
            return Arc::new(DriftView::CheckFailed);
        }
        let mut slot = self.slot.lock().expect("drift slot poisoned");
        if slot
            .observed
            .as_ref()
            .is_none_or(|old| key.version >= old.version)
        {
            slot.observed = Some(key.clone());
        }
        if let Some((checked, view)) = &slot.report
            && checked == key
        {
            return Arc::clone(view);
        }
        drop(slot);
        self.wake.notify_one();
        Arc::new(DriftView::Pending)
    }

    fn publish(&self, key: Key, view: DriftView) {
        let mut slot = self.slot.lock().expect("drift slot poisoned");
        if slot.observed.as_ref() == Some(&key) {
            slot.report = Some((key, Arc::new(view)));
        }
    }

    fn failed(&self) {
        let mut slot = self.slot.lock().expect("drift slot poisoned");
        if let Some(key) = slot.observed.clone() {
            slot.report = Some((key, Arc::new(DriftView::CheckFailed)));
        }
    }
}

struct TaskExit(Arc<Shared>);
impl Drop for TaskExit {
    fn drop(&mut self) {
        self.0
            .stopped
            .store(true, std::sync::atomic::Ordering::Release);
        self.0.failed();
    }
}

pub(crate) struct SourceDriftMonitor {
    shared: Arc<Shared>,
    stop: CancellationToken,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl SourceDriftMonitor {
    pub(crate) fn start<E: Engine + Send + Sync + 'static>(
        store: Arc<EditStore>,
        engine: Arc<E>,
    ) -> (Self, oneshot::Receiver<Result<(), ()>>) {
        let shared = Arc::new(Shared::default());
        let stop = CancellationToken::new();
        let (ready, initialized) = oneshot::channel();
        let owned_shared = Arc::clone(&shared);
        let owned_stop = stop.clone();
        let task = tokio::spawn(async move {
            let _exit = TaskExit(Arc::clone(&owned_shared));
            let mut ticks = tokio::time::interval_at(Instant::now() + INTERVAL, INTERVAL);
            ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let initial = tokio::select! {
                biased;
                () = owned_stop.cancelled() => return,
                result = round(&store, &*engine, &owned_shared) => result,
            };
            let _ = ready.send(initial);
            if initial.is_err() {
                return;
            }
            loop {
                tokio::select! {
                    biased;
                    () = owned_stop.cancelled() => return,
                    _ = ticks.tick() => {},
                    () = owned_shared.wake.notified() => {},
                }
                tokio::select! {
                    biased;
                    () = owned_stop.cancelled() => return,
                    result = round(&store, &*engine, &owned_shared) => {
                        if result.is_err() { owned_shared.failed(); }
                    },
                }
            }
        });
        (
            Self {
                shared,
                stop,
                task: Mutex::new(Some(task)),
            },
            initialized,
        )
    }

    pub(crate) fn capture(&self, active: &ActiveDefinition) -> Arc<DriftView> {
        self.shared.capture(&Key::from(active))
    }

    pub(crate) async fn shutdown(&self) -> Result<(), ()> {
        self.stop.cancel();
        let task = self.task.lock().expect("drift task poisoned").take();
        if let Some(task) = task {
            task.await.map_err(|_| ())?;
        }
        Ok(())
    }
}

async fn round<E: Engine + Sync>(store: &EditStore, engine: &E, shared: &Shared) -> Result<(), ()> {
    let deadline = Instant::now() + ROUND_TIMEOUT;
    let mut checked_key = None;
    let checked = tokio::time::timeout_at(deadline, async {
        let snapshot = store.request_snapshot().await.map_err(|_| ())?;
        let active = snapshot.active;
        let key = Key::from(active.as_ref());
        checked_key = Some(key.clone());
        // Observe without waking ourselves: only new requests need an expedited round.
        {
            let mut slot = shared.slot.lock().expect("drift slot poisoned");
            if slot
                .observed
                .as_ref()
                .is_none_or(|old| key.version >= old.version)
            {
                slot.observed = Some(key.clone());
            }
        }
        let ontology = &active.ontology;
        let objects = ontology
            .object_types
            .values()
            .filter(|ty| ty.backing == Backing::Source);
        let links = ontology
            .link_types
            .values()
            .filter(|link| link.backing == Some(Backing::Source));
        if objects.clone().next().is_none() && links.clone().next().is_none() {
            shared.publish(key, DriftView::NotApplicable);
            return Ok(());
        }
        let profile = ontology.default_profile.as_ref().ok_or(())?;
        let bindings = ontology.profile_bindings(profile).ok_or(())?;
        if objects
            .clone()
            .any(|ty| !bindings.objects.contains_key(&ty.api_name))
            || links
                .clone()
                .any(|link| !bindings.links.contains_key(&link.api_name))
        {
            return Err(());
        }
        let mut context = QueryContext::new("source-drift".to_owned(), Vec::new());
        context.timeout = deadline.saturating_duration_since(Instant::now());
        let view = match check_drift(engine, ontology, profile, &context).await {
            Ok(report) => DriftView::Checked(report),
            Err(error) if error.is_engine_wide() => DriftView::EngineFailed,
            Err(_) => DriftView::CheckFailed,
        };
        shared.publish(key, view);
        Ok(())
    })
    .await;
    match checked {
        Ok(result) => result,
        Err(_) => {
            if let Some(key) = checked_key {
                shared.publish(key, DriftView::CheckFailed);
                Ok(())
            } else {
                Err(())
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/drift.test.rs"]
mod tests;
