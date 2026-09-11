//! Session-maintenance intent, availability and recorded reports (ADR 0066).

use std::fmt::Write;
use std::time::SystemTime;

use super::StartupAvailability;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionAction {
    EndSession,
    RetryDrain,
    CopyReport,
    Quit,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SessionActionDisplay {
    pub(crate) action: SessionAction,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) availability: StartupAvailability,
}

#[derive(Debug)]
pub(crate) struct SessionReportVm {
    pub(crate) generation: u64,
    pub(crate) working: bool,
    pub(crate) report: String,
}

impl SessionReportVm {
    pub(crate) const TITLE: &'static str = "App session";
    pub(crate) const EXPLANATION: &'static str = "End this app session to open recovery and maintenance. App commands and built-in playback will stop. Unsaved Settings edits will be discarded. External publisher and encoder services will keep running.";
    pub(crate) const WAITING: &'static str = "App is finishing its work and releasing its database connections. Maintenance cannot start until they are released.";

    pub(crate) fn new(generation: u64) -> Self {
        let mut vm = Self {
            generation,
            working: true,
            report: String::new(),
        };
        vm.record("App started ending this session. New commands are blocked; admitted work is still tracked.");
        vm
    }

    pub(crate) fn generation_label(generation: u64) -> String {
        format!("Current app session: {generation}")
    }

    pub(crate) fn retain_previous(&mut self, report: &str) {
        if !report.is_empty() {
            self.report.push_str("\nPrevious recorded report:\n");
            self.report.push_str(report);
            self.report.push('\n');
        }
    }

    pub(crate) fn entry(available: bool) -> SessionActionDisplay {
        SessionActionDisplay {
            action: SessionAction::EndSession,
            label: "End app session",
            a11y_label:
                "Stop this app's work and built-in playback, then open recovery and maintenance",
            availability: if available {
                StartupAvailability::Available
            } else {
                StartupAvailability::Unavailable
            },
        }
    }

    pub(crate) fn action(&self, action: SessionAction) -> SessionActionDisplay {
        let (label, a11y_label) = match action {
            SessionAction::EndSession => return Self::entry(false),
            SessionAction::RetryDrain => (
                "Retry drain",
                "Wait again for this app session to finish and release its resources",
            ),
            SessionAction::CopyReport => (
                "Copy report",
                "Copy the complete session-maintenance report",
            ),
            SessionAction::Quit => (
                "Quit",
                "Close the app; outstanding work must still finish before the process exits",
            ),
        };
        SessionActionDisplay {
            action,
            label,
            a11y_label,
            availability: if action == SessionAction::RetryDrain && self.working {
                StartupAvailability::Working
            } else {
                StartupAvailability::Available
            },
        }
    }

    pub(crate) fn failed(&mut self, remaining: &[String]) {
        self.working = false;
        self.record(&format!("App could not finish ending this session. Maintenance is not available. Let the named work finish, then choose Retry drain.\nRemaining: {}", remaining.join("; ")));
    }

    pub(crate) fn released(&mut self) {
        self.working = false;
        self.record("App finished its work, stopped built-in playback and closed its configured database connections. Recovery is available. Ending the app session sent no stop command to external publisher or encoder services.");
    }

    pub(crate) fn resumed(&mut self, generation: u64) {
        self.record(&format!("App opened fresh session {generation} after verifying configuration, music storage and SQLite."));
    }

    fn record(&mut self, message: &str) {
        let at: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
        let _ = writeln!(
            self.report,
            "[{}] Session {}: {}",
            at.format("%Y-%m-%d %H:%M:%S UTC"),
            self.generation,
            message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_session_actions_explain_playback_and_keep_retry_report_available() {
        let entry = SessionReportVm::entry(true);
        assert_eq!(entry.action, SessionAction::EndSession);
        assert_eq!(entry.availability, StartupAvailability::Available);
        assert!(entry.a11y_label.contains("built-in playback"));
        assert!(SessionReportVm::EXPLANATION.contains("Unsaved Settings edits"));
        assert!(SessionReportVm::EXPLANATION.contains("External publisher and encoder"));
        let mut vm = SessionReportVm::new(7);
        assert_eq!(
            vm.action(SessionAction::RetryDrain).availability,
            StartupAvailability::Working
        );
        vm.failed(&["Held fixture command: 1".into()]);
        assert!(vm.report.contains("Maintenance is not available"));
        assert!(vm.report.contains("Held fixture command: 1"));
        for action in [
            SessionAction::RetryDrain,
            SessionAction::CopyReport,
            SessionAction::Quit,
        ] {
            assert_eq!(
                vm.action(action).availability,
                StartupAvailability::Available
            );
        }
        let original = vm.report.clone();
        assert_eq!(
            SessionReportVm::generation_label(vm.generation),
            "Current app session: 7"
        );
        assert_eq!(vm.action(SessionAction::CopyReport).label, "Copy report");
        assert_eq!(vm.report, original);
        vm.released();
        vm.resumed(8);
        assert!(vm.report.starts_with(&original));
        assert!(vm.report.contains("Recovery is available"));
        assert!(vm.report.contains("opened fresh session 8"));
    }
}
