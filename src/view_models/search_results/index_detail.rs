//! Search-results remote detail projections.

#![warn(clippy::pedantic)]

use crate::views::{FeedView, TrackView};

use super::{FeedResultDisplay, TrackResultDisplay};

/// Display-ready rows returned by an async remote Index search.
#[derive(Clone, Debug, Default)]
pub(crate) struct IndexSearchResultRows {
    /// Remote artist matches.
    pub(crate) artists: Vec<(super::SearchResultItemId, super::ArtistResultDisplay)>,
    /// Remote feed matches.
    pub(crate) feeds: Vec<(super::SearchResultItemId, FeedResultDisplay)>,
    /// Remote track matches.
    pub(crate) tracks: Vec<(super::SearchResultItemId, TrackResultDisplay)>,
}

/// Remote Index detail kind rendered from a search-result drill-down.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IndexDetailKind {
    /// Remote feed detail.
    Feed,
    /// Remote track detail.
    Track,
}

impl IndexDetailKind {
    /// Returns the visible entity label for this detail kind.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Feed => "feed",
            Self::Track => "track",
        }
    }
}

/// Display contract for a remote Index drill-down page.
#[derive(Clone, Debug)]
pub(crate) struct IndexDetailDisplay {
    /// Remote entity kind.
    pub(crate) kind: IndexDetailKind,
    /// Stable remote id shown as source metadata.
    pub(crate) id: String,
    /// Primary detail title.
    pub(crate) title: String,
    /// Secondary detail text.
    pub(crate) secondary_text: String,
    /// Optional rich remote feed detail for Index feed drill-down.
    pub(crate) feed: Option<FeedView>,
    /// Optional rich remote track detail for Index track drill-down.
    pub(crate) track: Option<TrackView>,
}

impl IndexDetailDisplay {
    pub(super) fn new(
        kind: IndexDetailKind,
        id: impl Into<String>,
        title: impl Into<String>,
        secondary_text: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            id: id.into(),
            title: title.into(),
            secondary_text: secondary_text.into(),
            feed: None,
            track: None,
        }
    }

    pub(super) fn feed(row: &FeedResultDisplay, fallback_id: &str) -> Self {
        let mut display = Self::new(
            IndexDetailKind::Feed,
            fallback_id,
            row.label.clone(),
            row.secondary_text.clone(),
        );
        display.feed.clone_from(&row.remote_feed);
        display
    }

    pub(super) fn track(row: &TrackResultDisplay, fallback_id: &str) -> Self {
        let mut display = Self::new(
            IndexDetailKind::Track,
            fallback_id,
            row.label.clone(),
            row.secondary_text.clone(),
        );
        display.track.clone_from(&row.remote_track);
        display
    }

    pub(crate) fn feed_or_fallback(
        row: Option<&FeedResultDisplay>,
        fallback_id: &str,
        fallback_label: &str,
    ) -> Self {
        row.map_or_else(
            || {
                Self::new(
                    IndexDetailKind::Feed,
                    fallback_id,
                    fallback_label,
                    "MusicIndex feed",
                )
            },
            |row| Self::feed(row, fallback_id),
        )
    }

    pub(crate) fn track_or_fallback(
        row: Option<&TrackResultDisplay>,
        fallback_id: &str,
        fallback_label: &str,
    ) -> Self {
        row.map_or_else(
            || {
                Self::new(
                    IndexDetailKind::Track,
                    fallback_id,
                    fallback_label,
                    "MusicIndex track",
                )
            },
            |row| Self::track(row, fallback_id),
        )
    }
}
