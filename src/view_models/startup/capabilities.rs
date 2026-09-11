//! Background-tool reports, remedies and retry generations (ADR 0066).

use std::fmt::Write as _;
use std::time::SystemTime;

use super::StartupAvailability;
pub(crate) use crate::application::capability::CapabilityAction;
use crate::application::capability::{
    CapabilityFailure, CapabilityObservation, CapabilitySnapshot, Dependency,
};

#[derive(Clone, Debug)]
pub struct CapabilityActionDisplay {
    pub action: CapabilityAction,
    pub label: &'static str,
    pub a11y_label: String,
    pub availability: StartupAvailability,
}

pub struct CapabilityRowDisplay {
    pub label: String,
    pub actions: Vec<CapabilityActionDisplay>,
}

#[derive(Clone, Debug)]
pub struct CapabilityReportVm {
    pub observations: CapabilitySnapshot,
    pub worker_available: bool,
    generation: u64,
    running: Option<(Dependency, u64)>,
    last_check: Option<(Dependency, u64, SystemTime)>,
}

impl CapabilityReportVm {
    pub const TITLE: &'static str = "Background tools";

    #[must_use]
    pub fn new(observations: CapabilitySnapshot, worker_available: bool) -> Self {
        Self {
            observations,
            worker_available,
            generation: 0,
            running: None,
            last_check: None,
        }
    }

    pub fn issues(&self) -> impl Iterator<Item = &CapabilityObservation> {
        self.observations
            .values()
            .filter(|observation| observation.failure.is_some())
    }

    #[must_use]
    pub fn rows(&self, expanded: bool) -> Vec<CapabilityRowDisplay> {
        self.issues()
            .map(|issue| {
                let intents = if expanded {
                    vec![CapabilityAction::CheckAgain(issue.dependency)]
                } else {
                    vec![
                        CapabilityAction::Configure(issue.dependency),
                        CapabilityAction::CheckAgain(issue.dependency),
                    ]
                };
                CapabilityRowDisplay {
                    label: if expanded {
                        dependency_title(issue.dependency).to_owned()
                    } else {
                        observation_text(issue)
                    },
                    actions: intents
                        .into_iter()
                        .map(|intent| self.action(intent))
                        .collect(),
                }
            })
            .collect()
    }

    #[must_use]
    pub fn action(&self, action: CapabilityAction) -> CapabilityActionDisplay {
        let (label, a11y_label, availability) = match action {
            CapabilityAction::Configure(dependency) => (
                "Open report",
                format!("Open {} report in Settings", dependency_title(dependency)),
                StartupAvailability::Available,
            ),
            CapabilityAction::CopyReport => (
                "Copy report",
                "Copy the complete background tools report".into(),
                StartupAvailability::Available,
            ),
            CapabilityAction::CheckAgain(dependency) => {
                let availability = if self.running.is_some_and(|(active, _)| active == dependency) {
                    StartupAvailability::Working
                } else if self.running.is_none()
                    && self.worker_available
                    && self
                        .observations
                        .get(&dependency)
                        .is_some_and(|observation| observation.failure.is_some())
                {
                    StartupAvailability::Available
                } else {
                    StartupAvailability::Unavailable
                };
                (
                    if availability == StartupAvailability::Working {
                        "Checking…"
                    } else {
                        "Check again"
                    },
                    format!("Check {} again", dependency_title(dependency)),
                    availability,
                )
            }
        };
        CapabilityActionDisplay {
            action,
            label,
            a11y_label,
            availability,
        }
    }

    pub fn begin(&mut self, dependency: Dependency) -> Option<u64> {
        if self
            .action(CapabilityAction::CheckAgain(dependency))
            .availability
            != StartupAvailability::Available
        {
            return None;
        }
        self.generation += 1;
        self.running = Some((dependency, self.generation));
        Some(self.generation)
    }

    /// Call before installing a result; a stale completion never installs resources.
    pub fn complete(&mut self, dependency: Dependency, generation: u64) -> bool {
        if self.running != Some((dependency, generation)) {
            return false;
        }
        self.running = None;
        true
    }

    pub fn record_completion(&mut self, dependency: Dependency, generation: u64, at: SystemTime) {
        self.last_check = Some((dependency, generation, at));
    }

    #[must_use]
    pub fn feedback(&self) -> Option<String> {
        let (dependency, generation, at) = self.last_check?;
        let at: chrono::DateTime<chrono::Utc> = at.into();
        Some(format!(
            "App finished check {generation} for {} at {}.",
            dependency_title(dependency),
            at.format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }

    #[must_use]
    pub fn report(&self) -> String {
        let mut text = format!("{}\n", Self::TITLE);
        if let Some(feedback) = self.feedback() {
            let _ = writeln!(text, "{feedback}");
        }
        for observation in self.observations.values() {
            let at: chrono::DateTime<chrono::Utc> = observation.observed_at.into();
            let _ = writeln!(
                text,
                "\n[{}] {}",
                at.format("%Y-%m-%d %H:%M:%S UTC"),
                observation_text(observation)
            );
            if observation.failure.is_some() {
                let _ = writeln!(
                    text,
                    "{}\nNext: {}",
                    consequence(observation.dependency),
                    recovery(observation)
                );
            }
            if let Some(path) = &observation.resource {
                let _ = writeln!(text, "Location: {}", path.display());
            }
        }
        if !self.worker_available {
            text.push_str("\nApp cannot run checks because its independent maintenance worker is unavailable. Copy this report, free system resources and relaunch the app.\n");
        }
        text
    }
}

#[must_use]
pub fn dependency_title(dependency: Dependency) -> &'static str {
    match dependency {
        Dependency::BackgroundRuntime => "Background runtime",
        Dependency::ThumbnailMaintenance => "Thumbnail maintenance",
    }
}

#[must_use]
pub fn consequence(dependency: Dependency) -> &'static str {
    match dependency {
        Dependency::BackgroundRuntime => "App cannot run background library loads, online searches, playback commands or broadcast checks. Navigation, local search, existing playlist browsing and these repair tools remain available.",
        Dependency::ThumbnailMaintenance => "App keeps its image cache usable, but cannot finish removing old thumbnail files.",
    }
}

#[must_use]
pub fn observation_text(observation: &CapabilityObservation) -> String {
    let (subject, kind) = match observation.failure {
        Some(CapabilityFailure::RuntimeStart(kind)) => ("App could not start its background runtime", Some(kind)),
        Some(CapabilityFailure::CacheWorker(kind)) => ("App could not start its thumbnail cleanup worker", Some(kind)),
        Some(CapabilityFailure::CachePrune(kind)) => ("App could not finish scanning or removing old thumbnail files", Some(kind)),
        Some(CapabilityFailure::MaintenanceUnavailable) => ("App could not run this check on its independent maintenance worker", None),
        None => return match observation.dependency {
            Dependency::BackgroundRuntime => "App started its background runtime. This does not confirm that any external service is reachable.".into(),
            Dependency::ThumbnailMaintenance => "App completed its thumbnail cleanup scan.".into(),
        },
    };
    match kind {
        Some(kind) => format!("{subject}. Operating system error: {kind}."),
        None => format!("{subject}."),
    }
}

fn recovery(observation: &CapabilityObservation) -> &'static str {
    match observation.failure {
        Some(CapabilityFailure::CachePrune(_)) => "Check thumbnail-cache permissions and available storage, then choose Check again.",
        _ => "Free system resources if necessary, then choose Check again. App will make a fresh attempt.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::time::{Duration, SystemTime};

    fn vm() -> CapabilityReportVm {
        let entries = [
            (
                Dependency::BackgroundRuntime,
                CapabilityFailure::RuntimeStart(io::ErrorKind::Other),
            ),
            (
                Dependency::ThumbnailMaintenance,
                CapabilityFailure::CacheWorker(io::ErrorKind::PermissionDenied),
            ),
        ]
        .map(|(dependency, failure)| {
            (
                dependency,
                CapabilityObservation {
                    dependency,
                    failure: Some(failure),
                    observed_at: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
                    resource: None,
                },
            )
        });
        CapabilityReportVm::new(entries.into_iter().collect(), true)
    }

    #[test]
    fn adr_0066_independent_issues_keep_typed_remedies_and_recorded_times() {
        let mut vm = vm();
        assert_eq!(vm.issues().count(), 2);
        for row in vm.rows(false) {
            assert!(!row.label.is_empty());
            assert_eq!(row.actions.len(), 2);
            assert!(row.actions.iter().all(|action| action.availability
                == StartupAvailability::Available
                && !action.a11y_label.is_empty()));
        }
        assert!(vm.rows(true).iter().all(|row| row.actions.len() == 1));
        let report = vm.report();
        assert!(report.contains("1970-01-01 00:00:10 UTC"));
        assert!(report.contains("App could not start its background runtime"));
        for issue in vm.issues() {
            assert_eq!(
                vm.action(CapabilityAction::CheckAgain(issue.dependency))
                    .availability,
                StartupAvailability::Available
            );
            assert_eq!(
                vm.action(CapabilityAction::Configure(issue.dependency))
                    .availability,
                StartupAvailability::Available
            );
        }
        vm.observations
            .get_mut(&Dependency::BackgroundRuntime)
            .unwrap()
            .failure = None;
        assert_eq!(vm.issues().count(), 1);
        assert!(vm
            .report()
            .contains("does not confirm that any external service is reachable"));
        vm.worker_available = false;
        assert_eq!(
            vm.action(CapabilityAction::CopyReport).availability,
            StartupAvailability::Available
        );
        assert_eq!(
            vm.action(CapabilityAction::Configure(
                Dependency::ThumbnailMaintenance
            ))
            .availability,
            StartupAvailability::Available
        );
    }

    #[test]
    fn adr_0066_retry_generation_admits_one_install_and_rejects_stale_results() {
        let mut vm = vm();
        let dependency = Dependency::BackgroundRuntime;
        let generation = vm.begin(dependency).unwrap();
        assert!(vm.begin(dependency).is_none());
        assert!(vm.begin(Dependency::ThumbnailMaintenance).is_none());
        assert!(!vm.complete(dependency, generation + 1));
        assert!(!vm.complete(Dependency::ThumbnailMaintenance, generation));
        let mut installs = 0;
        for _ in 0..2 {
            if vm.complete(dependency, generation) {
                installs += 1;
            }
        }
        assert_eq!(installs, 1);
        assert!(vm
            .issues()
            .any(|issue| issue.dependency == Dependency::ThumbnailMaintenance));
    }

    #[test]
    fn adr_0066_repeated_tool_failures_keep_distinct_recorded_feedback() {
        let mut vm = vm();
        let dependency = Dependency::BackgroundRuntime;
        for expected in 1..=2 {
            let generation = vm.begin(dependency).unwrap();
            assert!(vm.complete(dependency, generation));
            vm.record_completion(dependency, generation, SystemTime::UNIX_EPOCH);
            assert!(vm
                .feedback()
                .unwrap()
                .contains(&format!("check {expected}")));
            assert!(vm.report().contains("1970-01-01 00:00:00 UTC"));
        }
    }
}
