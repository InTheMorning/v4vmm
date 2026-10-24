//! The note of an Index result that the search page did not show (ADR 0087
//! Decision 4, packet 007 decision D6).
//!
//! The app drops an Index result of an older run, or of a run that is not
//! the run of the page. When the page of that query has no Index result,
//! the page gives one note. The note names the query, the run of the
//! result, the run of the page and the reason. The note is display state of
//! the page. The app writes no log line for it.

#![warn(clippy::pedantic)]

use crate::view_models::live_run::RunGeneration;

use super::EmptyStateDisplay;

/// The reason why the app did not apply an Index result to the page of its
/// query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IndexResultStopReason {
    /// A newer run of the search page slot started after this run.
    OlderRun,
    /// The run of the result is the current run, but the page shows the
    /// rows of another run.
    PageGeneration,
}

/// One Index result that the page of its query did not show.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndexResultDiagnostic {
    /// The query of the result and of the page.
    pub(crate) query: String,
    /// The run of the result.
    pub(crate) run_generation: RunGeneration,
    /// The run of the page when the result came.
    pub(crate) page_generation: RunGeneration,
    /// The step that stopped the result.
    pub(crate) reason: IndexResultStopReason,
}

impl IndexResultDiagnostic {
    /// The note text for the operator: the subject, the action and the
    /// reason.
    #[must_use]
    pub(crate) fn display(&self) -> EmptyStateDisplay {
        let Self {
            query,
            run_generation,
            page_generation,
            reason,
        } = self;
        let secondary = match reason {
            IndexResultStopReason::OlderRun => format!(
                "The result came from search run {run_generation}, but a newer search run \
                 started. The page shows run {page_generation}."
            ),
            IndexResultStopReason::PageGeneration => format!(
                "The result came from search run {run_generation}, and the page shows run \
                 {page_generation}."
            ),
        };
        EmptyStateDisplay::new(
            format!("MusicIndex rows for \"{query}\" did not show"),
            secondary,
        )
    }
}
