//! Typed startup admission and one safe report formatter (ADR 0066).

use std::fmt::Write as _;
use std::time::SystemTime;

use crate::db::startup::DbStage;
use crate::startup::{CoreCheckOutcome, IssueSeverity, StartupStage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupAction {
    CopyReport,
    CheckAgain,
    OpenApp,
    Quit,
    Details,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupAvailability {
    Available,
    Working,
    Unavailable,
}
#[derive(Clone, Debug)]
pub struct StartupActionDisplay {
    pub action: StartupAction,
    pub label: &'static str,
    pub a11y_label: &'static str,
    pub availability: StartupAvailability,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StartupWork {
    Check,
    Open,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StartupPhase {
    Idle,
    Running(StartupWork),
    Finished {
        work: StartupWork,
        completed_at: SystemTime,
    },
    Mounted,
    Closed,
}

pub struct StartupReportVm {
    pub generation: u64,
    phase: StartupPhase,
    pub worker_available: bool,
    pub details: bool,
    pub outcome: CoreCheckOutcome,
}
impl StartupReportVm {
    #[must_use]
    pub fn new(worker_available: bool) -> Self {
        Self {
            generation: 0,
            phase: StartupPhase::Idle,
            worker_available,
            details: true,
            outcome: CoreCheckOutcome::pending(),
        }
    }
    pub fn begin(&mut self, action: StartupAction) -> Option<u64> {
        let work = match action {
            StartupAction::CheckAgain => StartupWork::Check,
            StartupAction::OpenApp => StartupWork::Open,
            _ => return None,
        };
        if self.action(action).availability != StartupAvailability::Available {
            return None;
        }
        self.generation += 1;
        self.phase = StartupPhase::Running(work);
        Some(self.generation)
    }
    #[must_use]
    pub fn accepts(&self, generation: u64) -> bool {
        matches!(self.phase, StartupPhase::Running(_)) && self.generation == generation
    }
    pub fn complete(&mut self, generation: u64, outcome: CoreCheckOutcome) -> bool {
        self.complete_at(generation, outcome, SystemTime::now())
    }
    fn complete_at(
        &mut self,
        generation: u64,
        outcome: CoreCheckOutcome,
        completed_at: SystemTime,
    ) -> bool {
        if !self.accepts(generation) {
            return false;
        }
        let StartupPhase::Running(work) = self.phase else {
            return false;
        };
        self.phase = StartupPhase::Finished { work, completed_at };
        self.outcome = outcome;
        true
    }
    pub fn mount(&mut self, generation: u64) -> bool {
        if !self.accepts(generation) {
            return false;
        }
        self.phase = StartupPhase::Mounted;
        true
    }
    pub fn close(&mut self) {
        self.phase = StartupPhase::Closed;
        self.generation += 1;
    }
    #[must_use]
    pub fn title(&self) -> &'static str {
        if matches!(self.phase, StartupPhase::Running(_)) {
            "Checking startup requirements"
        } else if self.outcome.can_open() {
            "Ready to open the app"
        } else {
            "App needs attention before it can open"
        }
    }
    #[must_use]
    pub fn summary(&self) -> String {
        if matches!(self.phase, StartupPhase::Running(_)) {
            return "App is checking configuration, music storage and SQLite. You can close this window; the current storage operation will finish before the process exits.".into();
        }
        self.outcome.issues.iter().find(|i| i.severity == IssueSeverity::Blocked)
            .map_or_else(|| "Core checks passed. Open app will revalidate these settings and prepare any new resources.".into(), |i| format!("{}: {} {}", subject(i.stage), i.cause, i.next_action))
    }
    #[must_use]
    pub fn report(&self) -> String {
        let report = format_report(&self.outcome);
        match self.feedback() {
            Some(feedback) => format!("{feedback}\n\n{report}"),
            None => report,
        }
    }
    /// Keep completion visible even when consecutive checks return the same error.
    #[must_use]
    pub fn feedback(&self) -> Option<String> {
        let (work, completed_at) = match self.phase {
            StartupPhase::Running(StartupWork::Check) => {
                return Some(format!(
                    "App is checking startup requirements (check {}).",
                    self.generation
                ));
            }
            StartupPhase::Running(StartupWork::Open) => {
                return Some(format!(
                    "App is preparing to open (request {}).",
                    self.generation
                ));
            }
            StartupPhase::Finished { work, completed_at } => (work, completed_at),
            StartupPhase::Idle | StartupPhase::Mounted | StartupPhase::Closed => return None,
        };
        let subject = match work {
            StartupWork::Check
                if self
                    .outcome
                    .issues
                    .iter()
                    .any(|issue| issue.stage == StartupStage::Worker) =>
            {
                "App could not complete check"
            }
            StartupWork::Check => "App finished check",
            StartupWork::Open => "App kept recovery open after open request",
        };
        let timestamp: chrono::DateTime<chrono::Utc> = completed_at.into();
        Some(format!(
            "{subject} {} at {}.",
            self.generation,
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }
    #[must_use]
    pub fn action(&self, action: StartupAction) -> StartupActionDisplay {
        let (label, a11y_label) = match action {
            StartupAction::CopyReport => ("Copy report", "Copy the complete startup report"),
            StartupAction::CheckAgain
                if self.phase == StartupPhase::Running(StartupWork::Check) =>
            {
                ("Checking…", "App is checking startup requirements")
            }
            StartupAction::CheckAgain => (
                "Check again",
                "Check existing core resources without setup or migration",
            ),
            StartupAction::OpenApp if self.phase == StartupPhase::Running(StartupWork::Open) => {
                ("Opening…", "App is revalidating and preparing to open")
            }
            StartupAction::OpenApp => (
                "Open app",
                "Revalidate and prepare core resources, then open the app",
            ),
            StartupAction::Quit => ("Quit", "Close recovery and exit without opening the app"),
            StartupAction::Details => (
                if self.details {
                    "Hide details"
                } else {
                    "Show details"
                },
                "Expand or collapse the full startup report",
            ),
        };
        let availability = if matches!(self.phase, StartupPhase::Closed | StartupPhase::Mounted) {
            StartupAvailability::Unavailable
        } else if matches!(action, StartupAction::CheckAgain | StartupAction::OpenApp) {
            if let StartupPhase::Running(work) = self.phase {
                if matches!(
                    (action, work),
                    (StartupAction::CheckAgain, StartupWork::Check)
                        | (StartupAction::OpenApp, StartupWork::Open)
                ) {
                    StartupAvailability::Working
                } else {
                    StartupAvailability::Unavailable
                }
            } else if !self.worker_available
                || (action == StartupAction::OpenApp && !self.outcome.can_open())
            {
                StartupAvailability::Unavailable
            } else {
                StartupAvailability::Available
            }
        } else {
            StartupAvailability::Available
        };
        StartupActionDisplay {
            action,
            label,
            a11y_label,
            availability,
        }
    }
}

#[must_use]
pub fn format_report(outcome: &CoreCheckOutcome) -> String {
    let timestamp: chrono::DateTime<chrono::Utc> = outcome.observed_at.into();
    let mut text = format!(
        "[{}] App recorded this startup report.\n",
        timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    for observation in &outcome.observations {
        let timestamp: chrono::DateTime<chrono::Utc> = observation.observed_at.into();
        let _ = write!(
            text,
            "\n[{}] {}\nLocation: {}\n",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            observation.description,
            observation.resource.display()
        );
    }
    for issue in &outcome.issues {
        let timestamp: chrono::DateTime<chrono::Utc> = issue.observed_at.into();
        let resource = issue.resource.as_ref().map_or_else(
            || "Location could not be determined.".into(),
            |path| format!("Location: {}", path.display()),
        );
        let consequence = match issue.severity {
            IssueSeverity::Blocked => "App has not opened normal library or show operations.",
            IssueSeverity::NeedsPreparation => {
                "App needs explicit preparation before it can use this resource."
            }
            IssueSeverity::Notice => {
                "This issue affects its related optional operation; core checks remain independent."
            }
        };
        let _ = write!(
            text,
            "\n[{}] {}\n{}\n{}\n{}\nNext: {}\n",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            subject(issue.stage),
            resource,
            issue.cause,
            consequence,
            issue.next_action
        );
    }
    if outcome.can_open() {
        text.push_str("\nCore checks permit Open app. App will revalidate before starting normal operations.\n");
    }
    text
}

fn subject(stage: StartupStage) -> &'static str {
    match stage {
        StartupStage::ConfigPath => "App could not locate its configuration",
        StartupStage::ConfigRead => "App tried to read the configuration document",
        StartupStage::ConfigField => "App checked a configuration setting",
        StartupStage::MusicInspect => "App inspected the music directory",
        StartupStage::MusicList => "App tried to list the music directory",
        StartupStage::MusicCreateProbe => "App tried to create a music-storage probe",
        StartupStage::MusicWriteProbe => "App tried to write the music-storage probe",
        StartupStage::MusicReadProbe => "App tried to read back the music-storage probe",
        StartupStage::MusicRemoveProbe => "App tried to remove the music-storage probe",
        StartupStage::MusicSetup => "App checked first-run music setup",
        StartupStage::Artists => "App tried to prepare the music download directory",
        StartupStage::Database(DbStage::Inspect) => "App inspected the SQLite database path",
        StartupStage::Database(DbStage::Open) => "App tried to open the SQLite database",
        StartupStage::Database(DbStage::Configure) => {
            "App tried to configure the SQLite connection"
        }
        StartupStage::Database(DbStage::Schema) => "App checked the SQLite schema",
        StartupStage::Database(DbStage::Read) => "App tried to read the SQLite database",
        StartupStage::Database(DbStage::WriteProbe) => "App tested writing to the SQLite database",
        StartupStage::Database(DbStage::Rollback) => "App tried to roll back the SQLite probe",
        StartupStage::Database(DbStage::Initialize) => "App tried to prepare SQLite tables",
        StartupStage::Database(DbStage::Migrate) => "App tried to upgrade the SQLite schema",
        StartupStage::Worker => "App could not run the startup worker",
        StartupStage::Window => "App could not open the desktop window",
        StartupStage::Activation => "App could not activate the existing desktop window",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::startup::StartupIssue;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn adr_0066_identical_rechecks_keep_distinct_visible_completions() {
        let finished = UNIX_EPOCH + Duration::from_secs(1_000);
        let failed_check = || {
            let mut issue = StartupIssue::new(
                StartupStage::MusicInspect,
                Some(std::path::Path::new("/fixture/music")),
                "The music path is a file, not a directory.",
                "Correct the music folder and choose Check again.",
            );
            issue.observed_at = finished;
            CoreCheckOutcome::blocked(issue)
        };
        let mut vm = StartupReportVm::new(true);
        assert_eq!(vm.feedback(), None);
        let first = vm.begin(StartupAction::CheckAgain).unwrap();
        let button = vm.action(StartupAction::CheckAgain);
        assert_eq!(button.label, "Checking…");
        assert_eq!(button.availability, StartupAvailability::Working);
        assert_eq!(button.a11y_label, "App is checking startup requirements");
        assert_eq!(
            vm.action(StartupAction::OpenApp).availability,
            StartupAvailability::Unavailable
        );
        assert!(vm.begin(StartupAction::CheckAgain).is_none());
        assert!(vm.feedback().unwrap().contains("check 1"));
        assert!(vm.complete_at(first, failed_check(), finished));
        let first_feedback = vm.feedback().unwrap();
        assert_eq!(
            first_feedback,
            "App finished check 1 at 1970-01-01 00:16:40 UTC."
        );
        assert_eq!(vm.action(StartupAction::CheckAgain).label, "Check again");
        vm.details = false;
        assert_eq!(vm.feedback().unwrap(), first_feedback);
        assert!(vm.report().starts_with(&first_feedback));

        // Identical failures in the same clock second must still acknowledge
        // each accepted click. Rendering/disclosure cannot change the receipt.
        let second = vm.begin(StartupAction::CheckAgain).unwrap();
        let running = vm.feedback().unwrap();
        assert!(!vm.complete_at(first, failed_check(), finished));
        assert_eq!(vm.feedback().unwrap(), running);
        assert!(vm.complete_at(second, failed_check(), finished));
        let second_feedback = vm.feedback().unwrap();
        assert_eq!(
            second_feedback,
            "App finished check 2 at 1970-01-01 00:16:40 UTC."
        );
        assert_ne!(first_feedback, second_feedback);
        assert_eq!(vm.feedback().unwrap(), second_feedback);
        assert!(vm.report().starts_with(&second_feedback));
        assert!(!vm.complete_at(first, failed_check(), finished));
        assert_eq!(vm.feedback().unwrap(), second_feedback);
        vm.close();
        assert_eq!(vm.feedback(), None);
        assert!(!vm.complete_at(second, failed_check(), finished));
    }

    #[test]
    fn adr_0066_worker_failure_does_not_claim_a_completed_check() {
        let mut vm = StartupReportVm::new(true);
        let generation = vm.begin(StartupAction::CheckAgain).unwrap();
        let outcome = CoreCheckOutcome::blocked(StartupIssue::new(
            StartupStage::Worker,
            None,
            "App could not run the startup worker.",
            "Copy this report and quit.",
        ));
        assert!(vm.complete_at(generation, outcome, UNIX_EPOCH));
        assert_eq!(
            vm.feedback().unwrap(),
            "App could not complete check 1 at 1970-01-01 00:00:00 UTC."
        );
        assert!(!vm.report().contains("App finished check"));
    }

    #[test]
    fn adr_0066_endpoint_credentials_are_removed_before_debug_or_reporting() {
        let issue = StartupIssue::new(StartupStage::ConfigField, None,
            "Publisher https://operator:private-password@relay.example/path?token=private-token failed.", "Check the publisher settings.");
        let outcome = CoreCheckOutcome::blocked(issue);
        for text in [format_report(&outcome), format!("{outcome:?}")] {
            for secret in ["operator", "private-password", "private-token"] {
                assert!(!text.contains(secret));
            }
            assert!(text.contains("https://relay.example/path"));
        }
    }

    #[test]
    fn adr_0066_stale_closed_and_repeated_results_cannot_mount() {
        let mut vm = StartupReportVm::new(true);
        let generation = vm.begin(StartupAction::CheckAgain).unwrap();
        assert!(vm.begin(StartupAction::CheckAgain).is_none());
        assert!(vm.begin(StartupAction::OpenApp).is_none());
        assert!(!vm.mount(generation + 1));
        assert!(vm.mount(generation));
        assert!(!vm.mount(generation));
        let mut vm = StartupReportVm::new(true);
        let generation = vm.begin(StartupAction::CheckAgain).unwrap();
        vm.close();
        assert!(!vm.mount(generation));
        assert!(!vm.accepts(generation));
    }

    #[test]
    fn adr_0066_worker_failure_leaves_copy_and_quit_available() {
        let vm = StartupReportVm::new(false);
        for action in [
            StartupAction::CopyReport,
            StartupAction::Quit,
            StartupAction::Details,
        ] {
            assert_eq!(
                vm.action(action).availability,
                StartupAvailability::Available
            );
        }
        for action in [StartupAction::CheckAgain, StartupAction::OpenApp] {
            assert_eq!(
                vm.action(action).availability,
                StartupAvailability::Unavailable
            );
        }
    }

    #[test]
    fn adr_0066_report_uses_recorded_utc_and_keeps_full_path_and_action() {
        let mut issue = StartupIssue::new(
            StartupStage::MusicInspect,
            Some(std::path::Path::new("/fixture/a very long path/music")),
            "The configured location does not exist.",
            "Mount the storage and choose Check again.",
        );
        issue.observed_at = UNIX_EPOCH + Duration::from_secs(1_000);
        let report = format_report(&CoreCheckOutcome::blocked(issue));
        assert!(report.contains("1970-01-01 00:16:40 UTC"));
        assert!(report.contains("/fixture/a very long path/music"));
        assert!(report.contains("App has not opened normal library or show operations."));
        assert!(report.contains("Next: Mount the storage and choose Check again."));
        let unresolved = CoreCheckOutcome::blocked(StartupIssue::new(
            StartupStage::ConfigPath,
            None,
            "App could not resolve its configuration path.",
            "Correct the environment and relaunch.",
        ));
        assert!(format_report(&unresolved).contains("Location could not be determined."));
    }

    #[test]
    fn adr_0066_parse_report_omits_source_excerpts_and_debug_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "musicindex_endpoint = \"https://secret-user:secret-pass@host\"\ninvalid = [\n# secret-token-canary").unwrap();
        let mut backend = crate::startup::StartupBackend::new(Some(path));
        let crate::startup::CoreResult::Checked(outcome) =
            backend.execute(crate::startup::CheckIntent::Check)
        else {
            panic!("expected failed parse");
        };
        let report = format_report(&outcome);
        assert!(report.contains("line"));
        assert!(report.contains("column"));
        for secret in ["secret-user", "secret-pass", "secret-token-canary"] {
            assert!(!report.contains(secret));
            assert!(!format!("{outcome:?}").contains(secret));
        }
    }
}
