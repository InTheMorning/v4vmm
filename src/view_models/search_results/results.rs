//! Search-results display rows.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::db::TrackRow;
use crate::view_models::artist_identity::{
    name_label, ArtistNameDestination, ArtistNameKey, NameFeedMatch, NameFeedMatches,
};
use crate::view_models::format::plural;
use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::library::LibraryTrackRowVm;
use crate::view_models::name_level_page::NameLevelPageVm;
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
#[derive(Clone, Debug)]
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

/// The id prefix of a search row that opens the feed page of a Library
/// artist feed (ADR 0084 Decision 7, packet 017 decision D6).
pub(crate) const ARTIST_FEED_RESULT_PREFIX: &str = "artist-feed:";

/// The row of a Library artist feed in place of the name rows that match
/// it (ADR 0084 Decision 7, packet 017 decision D6). Its label is the feed
/// title. It opens the feed page of `feed_guid`.
#[must_use]
pub(crate) fn artist_feed_result_display(
    feed_guid: &str,
    title: &str,
    origin: SearchResultOrigin,
) -> ArtistResultDisplay {
    let mut display = ArtistResultDisplay::new(
        format!("{ARTIST_FEED_RESULT_PREFIX}{feed_guid}"),
        title,
        origin,
    );
    display.a11y_label = format!("Open the artist page {title}");
    display
}

/// The page that a search artist row opens (ADR 0084 Decision 7, packet
/// 017 decision D6). A feed row opens its feed page. A name row, from the
/// Library or from `MusicIndex`, opens the feed page of the one feed that
/// its name matches, else its name level page. `None` for an id of another
/// form.
#[must_use]
pub(crate) fn artist_result_destination(
    result_id: &str,
    matches: &NameFeedMatches,
) -> Option<ArtistNameDestination> {
    if let Some(feed_guid) = result_id.strip_prefix(ARTIST_FEED_RESULT_PREFIX) {
        return Some(ArtistNameDestination::FeedPage(feed_guid.to_owned()));
    }
    let name = result_id
        .strip_prefix("index-artist:")
        .or_else(|| result_id.strip_prefix("library-artist:"))?;
    Some(matches.destination(ArtistNameKey::from_text(name)))
}

/// The Library facts that the artist rows of a search read (ADR 0084
/// Decision 7, packet 017 decisions D6 and D7). The app layer reads them
/// from the Library tree when the search starts.
#[derive(Clone, Debug, Default)]
pub(crate) struct ArtistSearchFacts {
    /// The match of each name to the artist feeds of the Library.
    pub(crate) matches: NameFeedMatches,
    /// Each name key with one Library release or more of the name level.
    pub(crate) name_level_keys: BTreeSet<String>,
}

pub(super) struct LocalLibrarySearchRows {
    pub(super) artists: Vec<(SearchResultItemId, ArtistResultDisplay)>,
    pub(super) feeds: Vec<(SearchResultItemId, FeedResultDisplay)>,
    pub(super) tracks: Vec<(SearchResultItemId, TrackResultDisplay)>,
}

pub(super) fn local_library_result_rows(
    tracks: &[TrackRow],
    artist_facts: &ArtistSearchFacts,
) -> LocalLibrarySearchRows {
    let mut artist_rows = BTreeMap::<String, LocalArtistResult>::new();
    let mut feed_rows = BTreeMap::<i64, LocalFeedResult>::new();
    let mut track_rows = Vec::new();

    for (index, track) in tracks.iter().enumerate() {
        let track_vm = LibraryTrackRowVm::new(track, None);
        let artist_name = track_vm.display_artist();
        artist_rows
            .entry(name_sort_key(&artist_name))
            .or_default()
            .push(artist_name);

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
        artists: local_artist_rows(artist_rows, artist_facts),
        feeds: feed_rows
            .into_values()
            .map(LocalFeedResult::into_display)
            .collect(),
        tracks: track_rows,
    }
}

/// The raw artist texts of the matched Library tracks of one name key.
#[derive(Debug, Default)]
struct LocalArtistResult {
    texts: Vec<String>,
}

impl LocalArtistResult {
    fn push(&mut self, text: String) {
        self.texts.push(text);
    }

    fn label(&self) -> Option<String> {
        name_label(self.texts.iter().map(String::as_str))
    }
}

/// The Library artist rows of a search (ADR 0084 Decision 7, packet 017
/// decisions D6 and D7). A name that matches exactly one Library artist
/// feed gives the row of that feed, one time for each feed. Each other name
/// gives its name row only when it has a Library release of the name
/// level. A name row has the label "Releases that name <name>", an equal
/// accessibility label, and no count.
fn local_artist_rows(
    artist_rows: BTreeMap<String, LocalArtistResult>,
    artist_facts: &ArtistSearchFacts,
) -> Vec<(SearchResultItemId, ArtistResultDisplay)> {
    let mut feeds_shown = BTreeSet::new();
    let mut displays = Vec::new();
    for (key, result) in artist_rows {
        let Some(name) = result.label() else {
            continue;
        };
        match artist_facts.matches.get(&key) {
            NameFeedMatch::MatchedFeed(feed_guid) => {
                if feeds_shown.insert(feed_guid.clone()) {
                    let title = artist_facts.matches.feed_title(&feed_guid);
                    displays.push(artist_feed_result_display(
                        &feed_guid,
                        &title,
                        SearchResultOrigin::Library,
                    ));
                }
            }
            NameFeedMatch::Ambiguous | NameFeedMatch::NoFeed => {
                if artist_facts.name_level_keys.contains(&key) {
                    let mut display = ArtistResultDisplay::new(
                        format!("library-artist:{name}"),
                        NameLevelPageVm::title_text_for_name(&name),
                        SearchResultOrigin::Library,
                    );
                    display.a11y_label.clone_from(&display.label);
                    displays.push(display);
                }
            }
        }
    }
    displays
        .into_iter()
        .enumerate()
        .map(|(index, display)| (item_id_from_index(index), display))
        .collect()
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
