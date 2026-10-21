//! Search-results tab and origin models.

#![warn(clippy::pedantic)]

/// Stable numeric identity for one search-result row.
pub(crate) type SearchResultItemId = u64;

/// One tab of the search results in the list column (ADR 0084 Decision 2,
/// packet 010 decision D1). The tabs come in the order of [`Self::ALL`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SearchResultsTab {
    /// Release result rows. ADR 0084 Decision 5: the curator sees
    /// "Release" in place of "Feed".
    #[default]
    Releases,
    /// Artist result rows: the feed rows and the name rows.
    Artists,
    /// Track result rows.
    Tracks,
}

impl SearchResultsTab {
    /// Each tab, in the visible order.
    pub(crate) const ALL: [Self; 3] = [Self::Releases, Self::Artists, Self::Tracks];

    /// Returns the visible label for this tab. A label shows no count.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Releases => "Releases",
            Self::Artists => "Artists",
            Self::Tracks => "Tracks",
        }
    }

    /// The noun of the empty state of this tab, for example "releases".
    #[must_use]
    pub(crate) const fn plural_noun(self) -> &'static str {
        match self {
            Self::Releases => "releases",
            Self::Artists => "artists",
            Self::Tracks => "tracks",
        }
    }
}

/// The source that found a search row. The row of a Library item and the
/// row of an Index item of one key merge into one row (packet 010).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchResultOrigin {
    /// Row came from the local library.
    Library,
    /// Row came from the remote index.
    Index,
}
