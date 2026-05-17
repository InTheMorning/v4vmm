//! Search-results tab and origin models.

#![warn(clippy::pedantic)]

use crate::view_models::workspace::ContentFilter;

/// Stable numeric identity for one search-result row.
pub(crate) type SearchResultItemId = u64;

/// Primary tab in the search-results inspector.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SearchResultsTab {
    /// Artist result rows.
    #[default]
    Artists,
    /// Feed result rows.
    Feeds,
    /// Track result rows.
    Tracks,
}

impl SearchResultsTab {
    /// Returns the visible label for this tab.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Artists => "Artists",
            Self::Feeds => "Feeds",
            Self::Tracks => "Tracks",
        }
    }
}

/// Result origin used by content filtering and membership display.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchResultOrigin {
    /// Row came from the local library.
    Library,
    /// Row came from the remote index.
    Index,
}

impl SearchResultOrigin {
    /// Returns whether this origin is visible under the given filter.
    #[must_use]
    pub(crate) const fn matches_filter(self, filter: ContentFilter) -> bool {
        match filter {
            ContentFilter::All => true,
            ContentFilter::Library => matches!(self, Self::Library),
            ContentFilter::Index => matches!(self, Self::Index),
        }
    }
}
