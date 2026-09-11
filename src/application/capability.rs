//! Scoped execution dependencies and recorded observations (ADR 0066).

use std::collections::BTreeMap;
use std::io;
use std::time::SystemTime;

use tokio::sync::watch;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dependency {
    BackgroundRuntime,
    ThumbnailMaintenance,
    MusicIndex,
    Playback,
    Publisher,
    Producer,
    Encoder,
    Converter,
    Presentation,
    LibraryPaths,
    Configuration(&'static str),
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

    pub const fn configured(dependency: Dependency) -> Self {
        Self {
            dependency,
            remedy: CapabilityAction::Configure(dependency),
        }
    }
}

impl std::fmt::Display for ExecutionUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let subject = match self.dependency {
            Dependency::BackgroundRuntime => return f.write_str("App cannot run this action because its background runtime is unavailable. Open Background tools in Settings and choose Check again."),
            Dependency::ThumbnailMaintenance => "thumbnail maintenance",
            Dependency::MusicIndex => "MusicIndex configuration",
            Dependency::Playback => "the configured player",
            Dependency::Publisher => "the selected publisher host",
            Dependency::Producer => "drop-file publication",
            Dependency::Encoder => "the configured encoder",
            Dependency::Converter => "the configured converter",
            Dependency::Presentation => "presentation settings",
            Dependency::LibraryPaths => "this local file binding",
            Dependency::Configuration(field) => field,
        };
        write!(f, "App cannot run this action because {subject} is unavailable. Open its report in Settings and correct the named setting or resource.")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityFailure {
    RuntimeStart(io::ErrorKind),
    CacheWorker(io::ErrorKind),
    CachePrune(io::ErrorKind),
    MaintenanceUnavailable,
    Configuration(crate::config::ConfigFieldIssue),
    Preparation,
    PathRepair,
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

/// Current resource facts shared by displayed actions and dispatch (ADR 0066).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureAvailability {
    runtime: Result<(), ExecutionUnavailable>,
    musicindex: bool,
    playback: bool,
    publisher: bool,
    producer: bool,
    encoder: bool,
}

impl FeatureAvailability {
    pub fn from_resources(
        endpoint: &crate::config::MusicIndexEndpoint,
        broadcast: &crate::config::BroadcastCapabilities,
        playback: bool,
    ) -> Self {
        Self {
            runtime: Ok(()),
            musicindex: endpoint.require().is_ok(),
            playback,
            publisher: broadcast.selected_host().is_ok(),
            producer: matches!(&broadcast.drop_directory, Ok(Some(_)))
                && broadcast.drop_file_target.is_ok(),
            encoder: matches!(&broadcast.encoder, Ok(Some(_))),
        }
    }

    pub fn with_runtime(mut self, availability: Result<(), ExecutionUnavailable>) -> Self {
        self.runtime = availability;
        self
    }

    pub fn with_observations(mut self, observations: &CapabilitySnapshot) -> Self {
        if observations
            .get(&Dependency::Producer)
            .is_some_and(|entry| entry.failure.is_some())
        {
            self.producer = false;
        }
        self
    }

    pub fn require(self, dependency: Dependency) -> Result<(), ExecutionUnavailable> {
        self.runtime?;
        let available = match dependency {
            Dependency::MusicIndex => self.musicindex,
            Dependency::Playback => self.playback,
            Dependency::Publisher => self.publisher,
            Dependency::Producer => self.producer,
            Dependency::Encoder => self.encoder,
            _ => true,
        };
        if available {
            Ok(())
        } else {
            Err(ExecutionUnavailable::configured(dependency))
        }
    }
}

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

    #[test]
    fn adr_0066_optional_dependency_pairs_preserve_independent_actions() {
        use crate::config::{ConfigSnapshot, MusicIndexEndpoint};
        let config = ConfigSnapshot::from_bytes(std::path::Path::new("config.toml"), b"musicindex_endpoint = false\n[broadcast]\nhosts = false\ndrop_directory = '/tmp/producer'\n[broadcast.encoder]\naddress = 'localhost'\n".to_vec()).unwrap();
        let features = FeatureAvailability::from_resources(
            &MusicIndexEndpoint::from_field(config.musicindex_endpoint.clone()),
            &config.broadcast(),
            true,
        );
        assert!(features.require(Dependency::MusicIndex).is_err());
        assert!(features.require(Dependency::Publisher).is_err());
        assert!(features.require(Dependency::Producer).is_ok());
        assert!(features.require(Dependency::Encoder).is_ok());
        assert!(features.require(Dependency::Playback).is_ok());
        let without_player = FeatureAvailability::from_resources(
            &"http://index.test".into(),
            &ConfigSnapshot::from_bytes(std::path::Path::new("fixture.toml"), Vec::new())
                .unwrap()
                .broadcast(),
            false,
        );
        assert!(without_player.require(Dependency::Playback).is_err());
        assert!(without_player.require(Dependency::Publisher).is_ok());
        assert!(without_player.require(Dependency::MusicIndex).is_ok());
        let no_runtime = features.with_runtime(Err(ExecutionUnavailable::RUNTIME));
        assert_eq!(
            no_runtime.require(Dependency::Playback),
            Err(ExecutionUnavailable::RUNTIME)
        );
    }

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
