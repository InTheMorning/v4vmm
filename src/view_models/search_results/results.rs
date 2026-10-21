//! Search-results display rows.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::db::TrackRow;
use crate::view_models::artist_identity::{
    name_label, ArtistNameDestination, ArtistNameKey, NameFeedMatch, NameFeedMatches,
};
use crate::view_models::artists_everything::FeedLinkClass;
use crate::view_models::collection_art::CollectionArtVm;
use crate::view_models::format::plural;
use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::library::LibraryTrackRowVm;
use crate::view_models::name_level_page::NameLevelPageVm;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::workspace::{ReleaseKey, TrackKey};
use crate::views::FeedView;

use super::paged_tab::SearchRowMergeKey;
use super::{SearchResultItemId, SearchResultOrigin, SearchResultsTab};

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
    /// The logo frame art of a Library name row: the newest images of its
    /// Library releases, the same art as its Artists list row (packet 028
    /// D3). `None` for each other row.
    pub(crate) art: Option<CollectionArtVm>,
    /// The class of a `MusicIndex` feed row by its link facts (ADR 0084
    /// packet 029). `None` for a row that gives no link fact, and for each
    /// row that is not a `MusicIndex` feed row.
    pub(crate) link_class: Option<FeedLinkClass>,
}

impl SearchRowMergeKey for ArtistResultDisplay {
    type Key = String;

    /// A feed row merges by its feed GUID, and a name row by the sort key
    /// of its name (packet 010 "The Artists tab": one feed GUID gives one
    /// row).
    fn merge_key(&self) -> Option<String> {
        match SearchResultTarget::from_row_id(SearchResultsTab::Artists, &self.id)? {
            SearchResultTarget::FeedPage(feed_guid) => Some(format!("feed:{feed_guid}")),
            SearchResultTarget::NameMatches(name) | SearchResultTarget::ArtistByName(name) => {
                Some(format!("name:{}", name_sort_key(&name)))
            }
            SearchResultTarget::Release(_)
            | SearchResultTarget::Track(_)
            | SearchResultTarget::LibraryTrack(_) => None,
        }
    }

    /// Keeps this row's own image or art. With no image and no art of its
    /// own, this row takes the image of the merged Index row (packet 010
    /// "One row for each item").
    fn adopt_missing_thumbnail_href(&mut self, other: &Self) {
        if self.thumbnail_href.is_none() && self.art.is_none() {
            self.thumbnail_href.clone_from(&other.thumbnail_href);
        }
    }
}

impl ArtistResultDisplay {
    /// Creates an artist result display with empty optional text.
    #[must_use]
    pub(crate) fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            a11y_label: format!("Artist: {label}"),
            label,
            secondary_text: String::new(),
            thumbnail_href: None,
            art: None,
            link_class: None,
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
    /// The parts of the secondary text, for example the artist and the
    /// track count. The search list joins them after the kind of the
    /// release (walk of 2026-10-21). Empty when the row gives only
    /// `secondary_text`.
    pub(crate) secondary_parts: Vec<String>,
    /// Optional thumbnail URL or href.
    pub(crate) thumbnail_href: Option<String>,
    /// Accessibility label for the row.
    pub(crate) a11y_label: String,
    /// Source origin of the row.
    pub(crate) origin: SearchResultOrigin,
    /// Optional remote feed detail captured during Index search.
    pub(crate) remote_feed: Option<FeedView>,
    /// The release key of a search row (packet 010). `None` for a row of
    /// another list.
    pub(crate) key: Option<ReleaseKey>,
    /// The kind that the row resolved from its own facts (ADR 0084
    /// Decision 5). The Library facts of the release replace it.
    pub(crate) kind: ReleaseKind,
}

impl SearchRowMergeKey for FeedResultDisplay {
    type Key = ReleaseKey;

    fn merge_key(&self) -> Option<ReleaseKey> {
        self.key.clone()
    }

    /// Keeps this row's own image. With no image of its own, this row
    /// takes the image of the merged Index row (packet 010 "One row for
    /// each item").
    fn adopt_missing_thumbnail_href(&mut self, other: &Self) {
        if self.thumbnail_href.is_none() {
            self.thumbnail_href.clone_from(&other.thumbnail_href);
        }
    }
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
            secondary_parts: Vec::new(),
            thumbnail_href: None,
            origin,
            remote_feed: None,
            key: None,
            kind: ReleaseKind::Album,
        }
    }

    /// Returns this display as a release row of the search (packet 010):
    /// with its release key, its kind and the accessibility label of a
    /// release (ADR 0084 Decision 5).
    #[must_use]
    pub(crate) fn with_release_key(mut self, key: ReleaseKey, kind: ReleaseKind) -> Self {
        self.a11y_label = format!("Release: {}", self.label);
        self.key = Some(key);
        self.kind = kind;
        self
    }

    /// Returns this display with secondary text attached.
    #[must_use]
    pub(crate) fn with_secondary_text(mut self, value: impl Into<String>) -> Self {
        self.secondary_text = value.into();
        self
    }

    /// Returns this display with the parts of its secondary text. The
    /// secondary text joins the parts with " - ". The search list joins
    /// them with " · " after the kind of the release (walk of
    /// 2026-10-21).
    #[must_use]
    pub(crate) fn with_secondary_parts(mut self, parts: Vec<String>) -> Self {
        self.secondary_text = parts.join(" - ");
        self.secondary_parts = parts;
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
    /// Source origin of the row.
    pub(crate) origin: SearchResultOrigin,
    /// The track key of a search row (packet 010). `None` for an Index
    /// track with no feed GUID: that row has no checkbox.
    pub(crate) key: Option<TrackKey>,
}

impl SearchRowMergeKey for TrackResultDisplay {
    type Key = TrackKey;

    fn merge_key(&self) -> Option<TrackKey> {
        self.key.clone()
    }

    /// Keeps this row's own image. With no image of its own, this row
    /// takes the image of the merged Index row (packet 010 "One row for
    /// each item").
    fn adopt_missing_thumbnail_href(&mut self, other: &Self) {
        if self.thumbnail_href.is_none() {
            self.thumbnail_href.clone_from(&other.thumbnail_href);
        }
    }
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
            key: None,
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

    /// Returns this display with the track key of a search row (packet
    /// 010).
    #[must_use]
    pub(crate) fn with_track_key(mut self, key: TrackKey) -> Self {
        self.key = Some(key);
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
pub(crate) fn artist_feed_result_display(feed_guid: &str, title: &str) -> ArtistResultDisplay {
    let mut display =
        ArtistResultDisplay::new(format!("{ARTIST_FEED_RESULT_PREFIX}{feed_guid}"), title);
    display.a11y_label = format!("Open the artist page {title}");
    display
}

/// The id prefix of a search row of a `MusicIndex` feed that
/// `GET /v1/search?type=publisher` found (packet 010 "The Artists tab").
/// It opens the feed page of the feed GUID after the prefix.
pub(crate) const INDEX_FEED_PAGE_RESULT_PREFIX: &str = "index-feed-page:";

/// The id prefix of an Index track row with a feed GUID. The row id is
/// `index-track:<feed>:<track>`. A feed GUID has no colon, thus the first
/// colon after the prefix ends the feed GUID.
const INDEX_TRACK_RESULT_PREFIX: &str = "index-track:";

/// The id prefix of an Index track row with no feed GUID (packet 028 D1).
/// All the text after the prefix is the track GUID, thus a track GUID with
/// a colon, for example `urn:uuid:x`, opens its own key.
const INDEX_TRACK_GUID_RESULT_PREFIX: &str = "index-track-guid:";

/// The row id of an Index track hit (packet 028 D1). Each kind of row has
/// its own prefix, thus no colon in a GUID can move the split.
#[must_use]
pub(crate) fn index_track_row_id(feed_guid: Option<&str>, track_guid: &str) -> String {
    feed_guid.map_or_else(
        || format!("{INDEX_TRACK_GUID_RESULT_PREFIX}{track_guid}"),
        |feed_guid| format!("{INDEX_TRACK_RESULT_PREFIX}{feed_guid}:{track_guid}"),
    )
}

/// The page that a search row opens (packet 010 "The clicks"). The view
/// model parses each row id one time, here. The app layer matches the
/// target and parses no id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SearchResultTarget {
    /// A release page. A Library release row gives
    /// [`ReleaseKey::LibraryFeed`], and the app layer opens the release
    /// page by the key of its album.
    Release(ReleaseKey),
    /// The track page of an Index track row.
    Track(TrackKey),
    /// The track page of a Library track row, by its Library track id.
    LibraryTrack(i64),
    /// The feed page of the feed GUID: the row of a Library artist feed,
    /// or a `MusicIndex` feed row (flow map finding 6).
    FeedPage(String),
    /// The name row of a `MusicIndex` name. Packet 017 opens the name
    /// page, or the feed page of the one artist feed of the name.
    NameMatches(String),
    /// The name row of a Library name. It opens the same pages as
    /// [`Self::NameMatches`].
    ArtistByName(String),
}

impl SearchResultTarget {
    /// The target of the row `id` of the tab `tab`. `None` for an id of
    /// another form.
    #[must_use]
    pub(crate) fn from_row_id(tab: SearchResultsTab, id: &str) -> Option<Self> {
        match tab {
            SearchResultsTab::Releases => id.strip_prefix("index-feed:").map_or_else(
                || {
                    id.parse::<i64>()
                        .ok()
                        .map(|feed_id| Self::Release(ReleaseKey::LibraryFeed(feed_id)))
                },
                |feed_guid| Some(Self::Release(ReleaseKey::from_index_feed_guid(feed_guid))),
            ),
            SearchResultsTab::Tracks => {
                if let Some(track_guid) = id.strip_prefix(INDEX_TRACK_GUID_RESULT_PREFIX) {
                    return Some(Self::Track(TrackKey::from_index_track_target(
                        None, track_guid,
                    )));
                }
                if let Some(target) = id.strip_prefix(INDEX_TRACK_RESULT_PREFIX) {
                    let (feed_guid, track_guid) = target.split_once(':')?;
                    return Some(Self::Track(TrackKey::from_index_track_target(
                        Some(feed_guid),
                        track_guid,
                    )));
                }
                id.strip_prefix("library-track:")
                    .unwrap_or(id)
                    .parse::<i64>()
                    .ok()
                    .map(Self::LibraryTrack)
            }
            SearchResultsTab::Artists => {
                if let Some(feed_guid) = id
                    .strip_prefix(ARTIST_FEED_RESULT_PREFIX)
                    .or_else(|| id.strip_prefix(INDEX_FEED_PAGE_RESULT_PREFIX))
                {
                    return Some(Self::FeedPage(feed_guid.to_owned()));
                }
                if let Some(name) = id.strip_prefix("index-artist:") {
                    return Some(Self::NameMatches(name.to_owned()));
                }
                id.strip_prefix("library-artist:")
                    .map(|name| Self::ArtistByName(name.to_owned()))
            }
        }
    }

    /// The page of an Artists tab target (ADR 0084 Decision 7, packet 017
    /// decision D6). A feed row opens its feed page. A name row opens the
    /// feed page of the one feed that its name matches, else its name
    /// level page. `None` for a release target and a track target.
    #[must_use]
    pub(crate) fn artist_destination(
        &self,
        matches: &NameFeedMatches,
    ) -> Option<ArtistNameDestination> {
        match self {
            Self::FeedPage(feed_guid) => Some(ArtistNameDestination::FeedPage(feed_guid.clone())),
            Self::NameMatches(name) | Self::ArtistByName(name) => {
                Some(matches.destination(ArtistNameKey::from_text(name)))
            }
            Self::Release(_) | Self::Track(_) | Self::LibraryTrack(_) => None,
        }
    }
}

/// The Library facts that the artist rows of a search read (ADR 0084
/// Decision 7, packet 017 decisions D6 and D7). The app layer reads them
/// from the Library tree when the search starts.
#[derive(Clone, Debug, Default)]
pub(crate) struct ArtistSearchFacts {
    /// The match of each name to the artist feeds of the Library.
    pub(crate) matches: NameFeedMatches,
    /// The logo frame art of each name key with one Library release or
    /// more of the name level (packet 028 D3). The name row of the search
    /// shows this art, the same art as its Artists list row.
    pub(crate) name_level_art: BTreeMap<String, CollectionArtVm>,
    /// The channel image of each Library artist feed, by feed GUID (ADR
    /// 0084 Decision 12). The feed row of this name shows it, the same
    /// image that the Artists list shows.
    pub(crate) feed_images: BTreeMap<String, String>,
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
        let mut track_row = TrackResultDisplay::new(
            track.id.to_string(),
            track_vm.display_title(),
            SearchResultOrigin::Library,
        )
        .with_secondary_text(track_vm.display_artist())
        .with_track_key(track.track_key());
        if let Some(thumbnail_href) = library_track_thumbnail_href(track) {
            track_row = track_row.with_thumbnail_href(thumbnail_href);
        }
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
                    let mut display = artist_feed_result_display(&feed_guid, &title);
                    if let Some(image) = artist_facts.feed_images.get(&feed_guid) {
                        display = display.with_thumbnail_href(image.clone());
                    }
                    displays.push(display);
                }
            }
            NameFeedMatch::Ambiguous | NameFeedMatch::NoFeed => {
                if let Some(art) = artist_facts.name_level_art.get(&key) {
                    let mut display = ArtistResultDisplay::new(
                        format!("library-artist:{name}"),
                        NameLevelPageVm::title_text_for_name(&name),
                    );
                    display.a11y_label.clone_from(&display.label);
                    display.art = Some(art.clone());
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
    key: ReleaseKey,
    title: String,
    artist_name: String,
    track_count: usize,
    /// The first album image across the tracks of the feed (ADR 0084
    /// Decision 12). The release row prefers it. This is the same order
    /// that the Releases list of the Library uses.
    album_image_href: Option<String>,
    /// The first track image across the tracks of the feed. The release
    /// row shows it only when the feed has no album image.
    track_image_href: Option<String>,
}

impl LocalFeedResult {
    fn new(track: &TrackRow) -> Self {
        // The key of the album of the track (ADR 0084 Decision 4): the
        // feed GUID, else the Library feed id.
        let key = track
            .feed_guid
            .clone()
            .map_or(ReleaseKey::LibraryFeed(track.feed_id), ReleaseKey::FeedGuid);
        Self {
            feed_id: track.feed_id,
            key,
            title: feed_result_title(track),
            artist_name: LibraryTrackRowVm::new(track, None).display_artist(),
            track_count: 0,
            album_image_href: None,
            track_image_href: None,
        }
    }

    /// Counts one more track of the feed. It keeps the first album image
    /// and the first track image that a track of the feed carries.
    fn push(&mut self, track: &TrackRow) {
        self.track_count += 1;
        if self.album_image_href.is_none() {
            self.album_image_href.clone_from(&track.album_image_href);
        }
        if self.track_image_href.is_none() {
            self.track_image_href.clone_from(&track.track_image_href);
        }
    }

    fn into_display(self) -> (SearchResultItemId, FeedResultDisplay) {
        let secondary = vec![
            self.artist_name,
            format!("{} track{}", self.track_count, plural(self.track_count)),
        ];
        let thumbnail_href = self.album_image_href.or(self.track_image_href);
        let mut display = FeedResultDisplay::new(
            self.feed_id.to_string(),
            self.title,
            SearchResultOrigin::Library,
        )
        .with_secondary_parts(secondary)
        .with_release_key(self.key, ReleaseKind::resolve(None, self.track_count));
        if let Some(thumbnail_href) = thumbnail_href {
            display = display.with_thumbnail_href(thumbnail_href);
        }

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

/// The thumbnail of a Library track row: the track's own image, else the
/// album image of its feed. This keeps the lookup order that
/// `ContentListRowDisplay::from_track_with_album_image` uses for the track
/// list of the Library, in `src/view_models/library.rs`.
fn library_track_thumbnail_href(track: &TrackRow) -> Option<String> {
    track
        .track_image_href
        .clone()
        .or_else(|| track.album_image_href.clone())
}

fn item_id_from_i64(value: i64, fallback_index: usize) -> SearchResultItemId {
    u64::try_from(value).unwrap_or_else(|_| item_id_from_index(fallback_index))
}

fn item_id_from_index(index: usize) -> SearchResultItemId {
    u64::try_from(index)
        .unwrap_or(SearchResultItemId::MAX)
        .saturating_add(1)
}
