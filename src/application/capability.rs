//! Scoped execution dependencies and recorded observations (ADR 0066).

use std::collections::BTreeMap;
use std::io;
use std::time::SystemTime;

use tokio::sync::watch;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dependency {
    BackgroundRuntime,
    ThumbnailMaintenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityAction {
    Configure(Dependency),
    CheckAgain(Dependency),
    CopyReport,
}

/// A blocked command carries its remedy through every dispatch entry point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionUnavailable {
    pub dependency: Dependency,
    pub remedy: CapabilityAction,
}

impl ExecutionUnavailable {
    pub const RUNTIME: Self = Self {
        dependency: Dependency::BackgroundRuntime,
        remedy: CapabilityAction::CheckAgain(Dependency::BackgroundRuntime),
    };
}

impl std::fmt::Display for ExecutionUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("App cannot run this action because its background runtime is unavailable. Open Background tools in Settings and choose Check again.")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityFailure {
    RuntimeStart(io::ErrorKind),
    CacheWorker(io::ErrorKind),
    CachePrune(io::ErrorKind),
    MaintenanceUnavailable,
}

/// Store safe categories, never arbitrary error strings or configuration values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityObservation {
    pub dependency: Dependency,
    pub observed_at: SystemTime,
    pub failure: Option<CapabilityFailure>,
    pub resource: Option<std::path::PathBuf>,
}

impl CapabilityObservation {
    pub fn new(dependency: Dependency, failure: Option<CapabilityFailure>) -> Self {
        Self {
            dependency,
            observed_at: SystemTime::now(),
            failure,
            resource: None,
        }
    }

    pub fn at_path(mut self, path: &std::path::Path) -> Self {
        self.resource = Some(path.to_path_buf());
        self
    }
}

pub type CapabilitySnapshot = BTreeMap<Dependency, CapabilityObservation>;

/// The watch channel works without a Tokio context; the GPUI bridge only awaits it.
#[derive(Clone, Debug)]
pub struct CapabilityObservations(watch::Sender<CapabilitySnapshot>);

impl Default for CapabilityObservations {
    fn default() -> Self {
        Self(watch::channel(BTreeMap::new()).0)
    }
}

impl CapabilityObservations {
    pub fn record(&self, observation: CapabilityObservation) {
        self.0.send_modify(|entries| {
            entries.insert(observation.dependency, observation);
        });
    }

    pub fn subscribe(&self) -> watch::Receiver<CapabilitySnapshot> {
        self.0.subscribe()
    }

    pub fn snapshot(&self) -> CapabilitySnapshot {
        self.0.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_observations_are_independent_without_a_runtime() {
        let observations = CapabilityObservations::default();
        let receiver = observations.subscribe();
        observations.record(CapabilityObservation::new(
            Dependency::BackgroundRuntime,
            Some(CapabilityFailure::RuntimeStart(
                io::ErrorKind::PermissionDenied,
            )),
        ));
        observations.record(CapabilityObservation::new(
            Dependency::ThumbnailMaintenance,
            Some(CapabilityFailure::CacheWorker(io::ErrorKind::Other)),
        ));
        assert_eq!(receiver.borrow().len(), 2);
        observations.record(CapabilityObservation::new(
            Dependency::BackgroundRuntime,
            None,
        ));
        assert!(receiver.borrow()[&Dependency::BackgroundRuntime]
            .failure
            .is_none());
        assert!(receiver.borrow()[&Dependency::ThumbnailMaintenance]
            .failure
            .is_some());
    }
}
