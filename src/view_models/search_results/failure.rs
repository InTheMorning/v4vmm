//! Readable Index failures and diagnostic disclosure (ADR 0066).

#![warn(clippy::pedantic)]

use std::time::SystemTime;

use crate::application::CommandError;
use crate::diagnostics::{endpoint_for_report, redact_endpoint_details};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchFailureAction {
    ToggleDetails,
    CopyReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchFailureAvailability {
    Available,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchFailureActionDisplay {
    pub action: SearchFailureAction,
    pub label: &'static str,
    pub a11y_label: &'static str,
    pub availability: SearchFailureAvailability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchFailureDisplay {
    pub summary: String,
    report: String,
    expanded: bool,
}

impl SearchFailureDisplay {
    pub(super) fn new(error: &CommandError, endpoint: &str, observed_at: SystemTime) -> Self {
        let (summary, next) = match error {
            CommandError::Unavailable(_) => (
                "App could not start the MusicIndex search because its background runtime is unavailable. No search request was sent.",
                "Open Background tools in Settings and choose Check again, then repeat your search.",
            ),
            CommandError::Cancelled => (
                "App cancelled the MusicIndex search before it completed.",
                "Run the search again when you are ready.",
            ),
            _ => (
                "App could not get search results from MusicIndex.",
                "Check your connection and the MusicIndex endpoint in Settings, then run the search again.",
            ),
        };
        let summary = format!("{summary}\nYour local library remains available.\n{next}");
        // Parse the configured address before displaying it; rejected values may
        // contain secrets. All report URLs share startup's redaction policy.
        let endpoint = endpoint_for_report(endpoint);
        let at: chrono::DateTime<chrono::Utc> = observed_at.into();
        let report = format!(
            "[{}] {summary}\nConfigured MusicIndex endpoint: {endpoint}\n\nTechnical details\n{}",
            at.format("%Y-%m-%d %H:%M:%S UTC"),
            redact_endpoint_details(&error.to_string()),
        );
        Self {
            summary,
            report,
            expanded: false,
        }
    }

    pub(crate) fn action(&self, action: SearchFailureAction) -> SearchFailureActionDisplay {
        let (label, a11y_label) = match action {
            SearchFailureAction::ToggleDetails if self.expanded => {
                ("Hide details", "Hide the MusicIndex search failure report")
            }
            SearchFailureAction::ToggleDetails => {
                ("Show details", "Show the MusicIndex search failure report")
            }
            SearchFailureAction::CopyReport => (
                "Copy report",
                "Copy the complete MusicIndex search failure report",
            ),
        };
        SearchFailureActionDisplay {
            action,
            label,
            a11y_label,
            availability: if self.report.is_empty() {
                SearchFailureAvailability::Unavailable
            } else {
                SearchFailureAvailability::Available
            },
        }
    }

    pub(crate) fn visible_report(&self) -> Option<&str> {
        self.expanded.then_some(self.report.as_str())
    }

    pub(super) fn activate(&mut self, action: SearchFailureAction) -> Option<String> {
        if self.action(action).availability != SearchFailureAvailability::Available {
            return None;
        }
        match action {
            SearchFailureAction::ToggleDetails => {
                self.expanded = !self.expanded;
                None
            }
            SearchFailureAction::CopyReport => Some(self.report.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::capability::ExecutionUnavailable;

    #[test]
    fn adr_0066_search_failure_reports_the_dependency_without_inventing_a_network_answer() {
        let unavailable = SearchFailureDisplay::new(
            &CommandError::Unavailable(ExecutionUnavailable::RUNTIME),
            "http://127.0.0.1:9",
            SystemTime::UNIX_EPOCH,
        );
        assert!(unavailable.summary.contains("No search request was sent."));
        assert!(unavailable.summary.contains("Background tools in Settings"));
        let query = SearchFailureDisplay::new(
            &CommandError::Query("response could not be decoded".into()),
            "http://127.0.0.1:9",
            SystemTime::UNIX_EPOCH,
        );
        assert!(query.summary.contains("could not get search results"));
        assert!(!query.summary.contains("background runtime"));
        assert!(!query.summary.contains("unreachable"));
        assert!(!query.summary.contains("response could not be decoded"));
        assert!(query.report.contains("response could not be decoded"));
        let cancelled = SearchFailureDisplay::new(
            &CommandError::Cancelled,
            "http://127.0.0.1:9",
            SystemTime::UNIX_EPOCH,
        );
        assert!(cancelled.summary.contains("cancelled"));
        assert!(!cancelled.summary.contains("Check your connection"));
    }

    #[test]
    fn adr_0066_search_report_copy_is_complete_recorded_and_safe_before_disclosure() {
        let long = "diagnostic ".repeat(1000);
        let mut failure = SearchFailureDisplay::new(
            &CommandError::Query(format!("feed search failed: error for url (https://user:password@example.org/v1/search?token=secret); track search failed: {long}END")),
            "https://user:password@example.org/base?token=secret#private",
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10),
        );
        assert!(failure.visible_report().is_none());
        for intent in [
            SearchFailureAction::ToggleDetails,
            SearchFailureAction::CopyReport,
        ] {
            let action = failure.action(intent);
            assert_eq!(action.availability, SearchFailureAvailability::Available);
            assert!(!action.a11y_label.is_empty());
        }
        let report = failure.activate(SearchFailureAction::CopyReport).unwrap();
        assert!(report.starts_with("[1970-01-01 00:00:10 UTC]"));
        assert!(report.contains("https://example.org/base"));
        assert!(report.contains("https://example.org/v1/search"));
        assert!(report.ends_with(&format!("{long}END")));
        for secret in ["password", "token=secret", "user:", "#private"] {
            assert!(!report.contains(secret));
            assert!(!format!("{failure:?}").contains(secret));
        }
        failure.activate(SearchFailureAction::ToggleDetails);
        assert_eq!(failure.visible_report(), Some(report.as_str()));
        assert_eq!(
            failure.action(SearchFailureAction::ToggleDetails).label,
            "Hide details"
        );
        failure.activate(SearchFailureAction::ToggleDetails);
        assert!(failure.visible_report().is_none());
        assert_eq!(
            failure.activate(SearchFailureAction::CopyReport),
            Some(report)
        );
    }
}
