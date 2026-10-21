//! Search-results empty-state contracts.

#![warn(clippy::pedantic)]

use super::failure::SearchFailureDisplay;
use super::SearchResultsTab;

/// Display contract for a content-unavailable state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EmptyStateDisplay {
    /// Primary empty-state title.
    pub(crate) title: String,
    /// Secondary empty-state explanation.
    pub(crate) secondary: String,
    /// Optional failure report; ordinary empty results have no diagnostic actions.
    pub(crate) failure: Option<SearchFailureDisplay>,
}

impl EmptyStateDisplay {
    /// Creates an empty-state display contract.
    #[must_use]
    pub(crate) fn new(title: impl Into<String>, secondary: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            secondary: secondary.into(),
            failure: None,
        }
    }
}

/// The empty state of a tab (packet 010 decision D13). It names the tab
/// and the query, for example `No releases matched "bluegrass".` It has
/// no clear-filter action, because no filter stays.
pub(super) fn empty_state_for(tab: SearchResultsTab, query: &str) -> EmptyStateDisplay {
    let noun = tab.plural_noun();
    EmptyStateDisplay::new(
        format!("No {noun}"),
        format!("No {noun} matched \"{query}\"."),
    )
}
