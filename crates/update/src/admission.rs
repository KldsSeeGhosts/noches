//! Atomic exclusion between starting work and committing an update/restart.
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    entrants: usize,
    updating: bool,
    resumed: Option<Arc<dyn Fn() + Send + Sync>>,
    prepare: Option<PrepareRestart>,
}
pub type PrepareRestart =
    Arc<dyn Fn() -> futures::future::BoxFuture<'static, anyhow::Result<()>> + Send + Sync>;
#[derive(Clone, Default)]
pub struct AdmissionGate(Arc<Mutex<State>>);
pub struct WorkPermit(AdmissionGate);
pub struct UpdatePermit(AdmissionGate);

impl AdmissionGate {
    pub fn enter(&self) -> anyhow::Result<WorkPermit> {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        anyhow::ensure!(
            !state.updating,
            "Update installation is in progress; work remains queued"
        );
        state.entrants += 1;
        Ok(WorkPermit(self.clone()))
    }
    /// The registry check and closing admission share the entry lock. A run
    /// preparing asynchronously is counted even before it appears in registries.
    pub fn begin_update(&self, quiet: impl FnOnce() -> bool) -> anyhow::Result<UpdatePermit> {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        anyhow::ensure!(
            !state.updating && state.entrants == 0 && quiet(),
            "Finish active runs and close terminals before installing the update"
        );
        state.updating = true;
        Ok(UpdatePermit(self.clone()))
    }
    pub fn on_resumed(&self, callback: Arc<dyn Fn() + Send + Sync>) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .resumed = Some(callback);
    }
    pub fn on_prepare(&self, callback: PrepareRestart) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .prepare = Some(callback);
    }
    /// Retire idle cached processes while the update permit excludes new work.
    pub async fn prepare_restart(&self) -> anyhow::Result<()> {
        let prepare = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .prepare
            .clone();
        if let Some(prepare) = prepare {
            prepare().await?;
        }
        Ok(())
    }
}
impl Drop for WorkPermit {
    fn drop(&mut self) {
        self.0
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entrants -= 1;
    }
}
impl Drop for UpdatePermit {
    fn drop(&mut self) {
        let callback = {
            let mut state = self
                .0
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.updating = false;
            state.resumed.clone()
        };
        if let Some(callback) = callback {
            callback();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn in_progress_admission_prevents_update_even_before_registry_publication() {
        let gate = AdmissionGate::default();
        let work = gate.enter().unwrap();
        assert!(gate.begin_update(|| true).is_err());
        drop(work);
        let update = gate.begin_update(|| true).unwrap();
        assert!(gate.enter().is_err());
        assert!(gate.begin_update(|| true).is_err());
        drop(update);
        assert!(gate.enter().is_ok());
    }
    #[test]
    fn busy_registry_and_cancelled_install_never_strand_admission() {
        let gate = AdmissionGate::default();
        assert!(gate.begin_update(|| false).is_err());
        assert!(gate.enter().is_ok());
        let resumed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = resumed.clone();
        gate.on_resumed(Arc::new(move || {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }));
        drop(gate.begin_update(|| true).unwrap());
        assert_eq!(resumed.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(gate.enter().is_ok());
    }
}
