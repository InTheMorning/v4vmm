//! Search-results display rows.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::db::TrackRow;
use crate::view_models::format::plural;
use crate::view_models::library::LibraryTrackRowVm;
use crate::views::FeedView;

use super::{SearchResultItemId, SearchResultOrigin};

/// Display-ready artist search-result row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArtistResultDisplay {
    /// Stable artist identifier from the source system.
    pub(crate) id: String,
    /// Primary row label.
    pub(crate) label: String,
    /// Secondary row text.
    pub(crate) secondary_text: String,
    /// Optional thumbnail URL or href.
    pub(crate) thumbnail_href: Option<String>,
    /// Accessibility label for the row.
    pub(crate) a11y_label: String,
    /// Source origin used for local content filtering.
    pub(crate) origin: SearchResultOrigin,
}

impl ArtistResultDisplay {
    /// Creates an artist result display with empty optional text.
    #[must_use]
    pub(crate) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        origin: SearchResultOrigin,
    ) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            a11y_label: format!("Artist: {label}"),
            label,
            secondary_text: String::new(),
            thumbnail_href: None,
            origin,
        }
    }

    /// Returns this display with secondary text attached.
    #[must_use]
    pub(crate) fn with_secondary_text(mut self, value: impl Into<String>) -> Self {
        self.secondary_text = value.into();
        self
    }

    /// Returns this display with a thumbnail href attached.
    #[must_use]
    pub(crate) fn with_thumbnail_href(mut self, value: impl Into<String>) -> Self {
        self.thumbnail_href = Some(value.into());
        self
    }
}

/// Display-ready feed search-result row.
#[derive(Clone, Debug)]
pub(crate) struct FeedResultDisplay {
    /// Stable feed identifier from the source system.
    pub(crate) id: String,
    /// Primary row label.
    pub(crate) label: String,
    /// Secondary row text.
    pub(crate) secondary_text: String,
    /// Optional thumbnail URL or href.
    pub(crate) thumbnail_href: Option<String>,
    /// Accessibility label for the row.
    pub(crate) a11y_label: String,
    /// Source origin used for local content filtering.
    pub(crate) origin: SearchResultOrigin,
    /// Optional remote feed detail captured during Index search.
    pub(crate) remote_feed: Option<FeedView>,
}

impl FeedResultDisplay {
    /// Creates a feed result display with empty optional text.
    #[must_use]
    pub(crate) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        origin: SearchResultOrigin,
    ) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            a11y_label: format!("Feed: {label}"),
            label,
            secondary_text: String::new(),
            thumbnail_href: None,
            origin,
            remote_feed: None,
        }
    }

    /// Returns this display with secondary text attached.
    #[must_use]
    pub(crate) fn with_secondary_text(mut self, value: impl Into<String>) -> Self {
        self.secondary_text = value.into();
        self
    }

    /// Returns this display with a thumbnail href attached.
    #[must_use]
    pub(crate) fn with_thumbnail_href(mut self, value: impl Into<String>) -> Self {
        self.thumbnail_href = Some(value.into());
        self
    }

    /// Returns this display with remote feed detail attached.
    #[must_use]
    pub(crate) fn with_remote_feed(mut self, feed: FeedView) -> Self {
        self.remote_feed = Some(feed);
        self
    }
}

/// Display-ready track search-result row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackResultDisplay {
    /// Stable track identifier from the source system.
    pub(crate) id: String,
    /// Primary row label.
    pub(crate) label: String,
    /// Secondary row text.
    pub(crate) secondary_text: String,
    /// Optional thumbnail URL or href.
    pub(crate) thumbnail_href: Option<String>,
    /// Accessibility label for the row.
    pub(crate) a11y_label: String,
    /// Source origin used for local content filtering.
    pub(crate) origin: SearchResultOrigin,
}

impl TrackResultDisplay {
    /// Creates a track result display with empty optional text.
    #[must_use]
    pub(crate) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        origin: SearchResultOrigin,
    ) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            a11y_label: format!("Track: {label}"),
            label,
            secondary_text: String::new(),
            thumbnail_href: None,
            origin,
        }
    }

    /// Returns this display with secondary text attached.
    #[must_use]
    pub(crate) fn with_secondary_text(mut self, value: impl Into<String>) -> Self {
        self.secondary_text = value.into();
        self
    }

    /// Returns this display with a thumbnail href attached.
    #[must_use]
    pub(crate) fn with_thumbnail_href(mut self, value: impl Into<String>) -> Self {
        self.thumbnail_href = Some(value.into());
        self
    }
}

pub(super) struct LocalLibrarySearchRows {
    pub(super) artists: Vec<(SearchResultItemId, ArtistResultDisplay)>,
    pub(super) feeds: Vec<(SearchResultItemId, FeedResultDisplay)>,
    pub(super) tracks: Vec<(SearchResultItemId, TrackResultDisplay)>,
}

pub(super) fn local_library_result_rows(tracks: &[TrackRow]) -> LocalLibrarySearchRows {
    let mut artist_rows = BTreeMap::<String, LocalArtistResult>::new();
    let mut feed_rows = BTreeMap::<i64, LocalFeedResult>::new();
    let mut track_rows = Vec::new();

    for (index, track) in tracks.iter().enumerate() {
        let track_vm = LibraryTrackRowVm::new(track, None);
        let artist_name = track_vm.display_artist();
        artist_rows
            .entry(artist_name.clone())
            .or_insert_with(|| LocalArtistResult::new(artist_name))
            .push(track);

        feed_rows
            .entry(track.feed_id)
            .or_insert_with(|| LocalFeedResult::new(track))
            .push(track);

        let id = item_id_from_i64(track.id, index);
        let track_row = TrackResultDisplay::new(
            track.id.to_string(),
            track_vm.display_title(),
            SearchResultOrigin::Library,
        )
        .with_secondary_text(track_vm.display_artist());
        track_rows.push((id, track_row));
    }

    LocalLibrarySearchRows {
        artists: artist_rows
            .into_values()
            .enumerate()
            .map(|(index, row)| row.into_display(index))
            .collect(),
        feeds: feed_rows
            .into_values()
            .map(LocalFeedResult::into_display)
            .collect(),
        tracks: track_rows,
    }
}

#[derive(Debug)]
struct LocalArtistResult {
    name: String,
    feed_ids: BTreeSet<i64>,
    track_count: usize,
}

impl LocalArtistResult {
    fn new(name: String) -> Self {
        Self {
            name,
            feed_ids: BTreeSet::new(),
            track_count: 0,
        }
    }

    fn push(&mut self, track: &TrackRow) {
        self.feed_ids.insert(track.feed_id);
        self.track_count += 1;
    }

    fn into_display(self, index: usize) -> (SearchResultItemId, ArtistResultDisplay) {
        let feed_count = self.feed_ids.len();
        let secondary = format!(
            "{} album{} - {} track{}",
            feed_count,
            plural(feed_count),
            self.track_count,
            plural(self.track_count)
        );
        let display = ArtistResultDisplay::new(
            format!("library-artist:{}", self.name),
            self.name,
            SearchResultOrigin::Library,
        )
        .with_secondary_text(secondary);

        (item_id_from_index(index), display)
    }
}

#[derive(Debug)]
struct LocalFeedResult {
    feed_id: i64,
    title: String,
    artist_name: String,
    track_count: usize,
}

impl LocalFeedResult {
    fn new(track: &TrackRow) -> Self {
        Self {
            feed_id: track.feed_id,
            title: feed_result_title(track),
            artist_name: LibraryTrackRowVm::new(track, None).display_artist(),
            track_count: 0,
        }
    }

    fn push(&mut self, _track: &TrackRow) {
        self.track_count += 1;
    }

    fn into_display(self) -> (SearchResultItemId, FeedResultDisplay) {
        let secondary = format!(
            "{} - {} track{}",
            self.artist_name,
            self.track_count,
            plural(self.track_count)
        );
        let display = FeedResultDisplay::new(
            self.feed_id.to_string(),
            self.title,
            SearchResultOrigin::Library,
        )
        .with_secondary_text(secondary);

        (item_id_from_i64(self.feed_id, 0), display)
    }
}

fn feed_result_title(track: &TrackRow) -> String {
    track
        .feed_title
        .clone()
        .or_else(|| track.album_title.clone())
        .unwrap_or_else(|| "Untitled Feed".to_string())
}

fn item_id_from_i64(value: i64, fallback_index: usize) -> SearchResultItemId {
    u64::try_from(value).unwrap_or_else(|_| item_id_from_index(fallback_index))
}

fn item_id_from_index(index: usize) -> SearchResultItemId {
    u64::try_from(index)
        .unwrap_or(SearchResultItemId::MAX)
        .saturating_add(1)
}
