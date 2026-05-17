//! Search-results empty-state contracts.

#![warn(clippy::pedantic)]

use crate::view_models::workspace::ContentFilter;

use super::SearchResultsTab;

/// Display contract for a content-unavailable state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EmptyStateDisplay {
    /// Primary empty-state title.
    pub(crate) title: String,
    /// Secondary empty-state explanation.
    pub(crate) secondary: String,
    /// Optional command id that clears the active filter.
    pub(crate) clear_filter_action_id: Option<&'static str>,
}

impl EmptyStateDisplay {
    /// Creates an empty-state display contract.
    #[must_use]
    pub(crate) fn new(
        title: impl Into<String>,
        secondary: impl Into<String>,
        clear_filter_action_id: Option<&'static str>,
    ) -> Self {
        Self {
            title: title.into(),
            secondary: secondary.into(),
            clear_filter_action_id,
        }
    }
}

pub(super) fn empty_state_for(
    tab: SearchResultsTab,
    filter: ContentFilter,
    query: &str,
) -> EmptyStateDisplay {
    let title = format!("No {} results", tab.label().to_lowercase());
    let secondary = match filter {
        ContentFilter::All => format!("No results matched \"{query}\"."),
        ContentFilter::Library => format!("No library results matched \"{query}\"."),
        ContentFilter::Index => format!("No index results matched \"{query}\"."),
    };
    EmptyStateDisplay::new(title, secondary, clear_filter_action_id(filter))
}

pub(super) const fn clear_filter_action_id(filter: ContentFilter) -> Option<&'static str> {
    match filter {
        ContentFilter::All => None,
        ContentFilter::Library | ContentFilter::Index => Some("search-results.clear-filter"),
    }
}
