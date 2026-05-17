//! Search-results inspector view-models.
//!
//! ADR 0048 mounts `SearchResultsInspector` in the workspace `ContentList` frame.
//! This module owns tab state, local content-filter state, display-ready result
//! rows, empty-state copy, and windowed result lists backed by ADR 0041
//! [`PagedListVm`] instances.

#![cfg(feature = "async-runtime")]
#![warn(clippy::pedantic)]
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ADR 0048 routing consumes these VM contracts from GPUI renderers"
    )
)]

use std::collections::{BTreeMap, BTreeSet};

use crate::db::TrackRow;
use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};
use crate::view_models::format::plural;
use crate::view_models::library::LibraryTrackRowVm;
use crate::view_models::workspace::{ContentFilter, FilterChipStripDisplay};
use crate::views::FeedView;

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

/// Paged result windows for one result category.
///
/// Each content filter owns an independent ADR 0041 window. This lets a
/// frame keep tab state and filter state independent while avoiding an eager
/// filtered `Vec<Row>` projection in the view-model.
#[derive(Debug)]
pub(crate) struct SearchResultsPagedTab<Row> {
    all: PagedListVm<SearchResultItemId, Row>,
    library: PagedListVm<SearchResultItemId, Row>,
    index: PagedListVm<SearchResultItemId, Row>,
    library_ids: Vec<SearchResultItemId>,
    index_ids: Vec<SearchResultItemId>,
}

impl<Row> SearchResultsPagedTab<Row> {
    /// Creates empty paged windows for all filters.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), Vec::new())
    }

    /// Creates paged windows from precomputed identity indexes.
    #[must_use]
    pub(crate) fn new(
        all: Vec<SearchResultItemId>,
        library: Vec<SearchResultItemId>,
        index: Vec<SearchResultItemId>,
    ) -> Self {
        Self {
            all: PagedListVm::new(all),
            library: PagedListVm::new(library.clone()),
            index: PagedListVm::new(index.clone()),
            library_ids: library,
            index_ids: index,
        }
    }

    /// Returns the paged window for a filter.
    #[must_use]
    pub(crate) const fn window(
        &self,
        filter: ContentFilter,
    ) -> &PagedListVm<SearchResultItemId, Row> {
        match filter {
            ContentFilter::All => &self.all,
            ContentFilter::Library => &self.library,
            ContentFilter::Index => &self.index,
        }
    }

    /// Returns the mutable paged window for a filter.
    pub(crate) const fn window_mut(
        &mut self,
        filter: ContentFilter,
    ) -> &mut PagedListVm<SearchResultItemId, Row> {
        match filter {
            ContentFilter::All => &mut self.all,
            ContentFilter::Library => &mut self.library,
            ContentFilter::Index => &mut self.index,
        }
    }

    /// Returns whether the filtered window is empty.
    #[must_use]
    pub(crate) fn is_empty(&self, filter: ContentFilter) -> bool {
        self.window(filter).total() == 0
    }
}

impl<Row: Clone> SearchResultsPagedTab<Row> {
    /// Creates a tab whose local-library rows are already loaded.
    #[must_use]
    pub(crate) fn ready_library(rows: Vec<(SearchResultItemId, Row)>) -> Self {
        let ids = rows.iter().map(|(id, _row)| *id).collect::<Vec<_>>();
        let mut tab = Self::new(ids.clone(), ids, Vec::new());
        tab.all.fulfill_page(0, rows.clone());
        tab.library.fulfill_page(0, rows);
        tab
    }

    /// Replaces the remote-index rows while preserving loaded library rows.
    pub(crate) fn replace_index_rows(&mut self, rows: Vec<(SearchResultItemId, Row)>) {
        let library_rows = self.cached_library_rows();
        self.index_ids = rows.iter().map(|(id, _row)| *id).collect();
        self.index.replace_index(self.index_ids.clone());
        self.index.fulfill_page(0, rows.clone());

        self.all.replace_index(self.all_ids());
        self.all
            .fulfill_page(0, library_rows.into_iter().chain(rows));
    }

    fn all_ids(&self) -> Vec<SearchResultItemId> {
        self.library_ids
            .iter()
            .chain(&self.index_ids)
            .copied()
            .collect()
    }

    fn cached_library_rows(&self) -> Vec<(SearchResultItemId, Row)> {
        self.library_ids
            .iter()
            .enumerate()
            .filter_map(|(index, id)| match self.library.peek_row(index) {
                RowSlot::Ready(row) => Some((*id, row.as_ref().clone())),
                RowSlot::Pending(_) => None,
            })
            .collect()
    }

    fn cached_row_matching(
        &self,
        filter: ContentFilter,
        predicate: impl Fn(&Row) -> bool,
    ) -> Option<Row> {
        let window = self.window(filter);
        (0..window.total()).find_map(|index| match window.peek_row(index) {
            RowSlot::Ready(row) if predicate(row.as_ref()) => Some(row.as_ref().clone()),
            RowSlot::Ready(_) | RowSlot::Pending(_) => None,
        })
    }
}

/// Display-ready rows returned by an async remote Index search.
#[derive(Clone, Debug, Default)]
pub(crate) struct IndexSearchResultRows {
    /// Remote artist matches.
    pub(crate) artists: Vec<(SearchResultItemId, ArtistResultDisplay)>,
    /// Remote feed matches.
    pub(crate) feeds: Vec<(SearchResultItemId, FeedResultDisplay)>,
    /// Remote track matches.
    pub(crate) tracks: Vec<(SearchResultItemId, TrackResultDisplay)>,
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
}

impl IndexDetailDisplay {
    fn new(
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
        }
    }

    fn feed(row: &FeedResultDisplay, fallback_id: &str) -> Self {
        let mut display = Self::new(
            IndexDetailKind::Feed,
            fallback_id,
            row.label.clone(),
            row.secondary_text.clone(),
        );
        display.feed.clone_from(&row.remote_feed);
        display
    }

    fn track(row: &TrackResultDisplay, fallback_id: &str) -> Self {
        Self::new(
            IndexDetailKind::Track,
            fallback_id,
            row.label.clone(),
            row.secondary_text.clone(),
        )
    }
}

/// GPUI-free page contract for the search-results inspector.
#[derive(Debug)]
pub(crate) struct SearchResultsInspectorPageVm {
    query: String,
    tab: SearchResultsTab,
    filter: ContentFilter,
    artists: SearchResultsPagedTab<ArtistResultDisplay>,
    feeds: SearchResultsPagedTab<FeedResultDisplay>,
    tracks: SearchResultsPagedTab<TrackResultDisplay>,
    index_loading: bool,
    index_error: Option<EmptyStateDisplay>,
    empty_state: Option<EmptyStateDisplay>,
    tab_was_user_selected: bool,
}

impl SearchResultsInspectorPageVm {
    /// Creates an empty search-results page for a query.
    #[must_use]
    pub(crate) fn new(query: impl Into<String>) -> Self {
        let mut page = Self {
            query: query.into(),
            tab: SearchResultsTab::default(),
            filter: ContentFilter::default(),
            artists: SearchResultsPagedTab::empty(),
            feeds: SearchResultsPagedTab::empty(),
            tracks: SearchResultsPagedTab::empty(),
            index_loading: false,
            index_error: None,
            empty_state: None,
            tab_was_user_selected: false,
        };
        page.refresh_empty_state();
        page
    }

    /// Returns this page with artist result windows attached.
    #[must_use]
    pub(crate) fn with_artists(
        mut self,
        artists: SearchResultsPagedTab<ArtistResultDisplay>,
    ) -> Self {
        self.artists = artists;
        self.refresh_empty_state();
        self
    }

    /// Returns this page with feed result windows attached.
    #[must_use]
    pub(crate) fn with_feeds(mut self, feeds: SearchResultsPagedTab<FeedResultDisplay>) -> Self {
        self.feeds = feeds;
        self.refresh_empty_state();
        self
    }

    /// Returns this page with track result windows attached.
    #[must_use]
    pub(crate) fn with_tracks(mut self, tracks: SearchResultsPagedTab<TrackResultDisplay>) -> Self {
        self.tracks = tracks;
        self.refresh_empty_state();
        self
    }

    /// Creates a search-results page from local library track matches.
    #[must_use]
    pub(crate) fn from_local_library_tracks(query: impl Into<String>, tracks: &[TrackRow]) -> Self {
        Self::new(query).with_local_library_tracks(tracks)
    }

    /// Returns this page with local library result rows attached.
    #[must_use]
    pub(crate) fn with_local_library_tracks(mut self, tracks: &[TrackRow]) -> Self {
        let local_rows = local_library_result_rows(tracks);
        self.artists = SearchResultsPagedTab::ready_library(local_rows.artists);
        self.feeds = SearchResultsPagedTab::ready_library(local_rows.feeds);
        self.tracks = SearchResultsPagedTab::ready_library(local_rows.tracks);
        self.refresh_empty_state();
        self
    }

    /// Marks the async remote Index search as pending.
    pub(crate) fn mark_index_loading(&mut self) {
        self.index_loading = true;
        self.index_error = None;
        self.refresh_empty_state();
    }

    /// Returns whether the async remote Index search is pending.
    #[must_use]
    pub(crate) const fn is_index_loading(&self) -> bool {
        self.index_loading
    }

    /// Replaces remote Index results and merges them into All windows.
    pub(crate) fn replace_index_results(&mut self, rows: IndexSearchResultRows) {
        self.index_loading = false;
        self.index_error = None;
        self.artists.replace_index_rows(rows.artists);
        self.feeds.replace_index_rows(rows.feeds);
        self.tracks.replace_index_rows(rows.tracks);
        self.select_first_populated_tab_if_automatic();
        self.refresh_empty_state();
    }

    /// Marks the async remote Index search as failed.
    pub(crate) fn set_index_error(
        &mut self,
        title: impl Into<String>,
        secondary: impl Into<String>,
    ) {
        self.index_loading = false;
        self.index_error = Some(EmptyStateDisplay::new(title, secondary, None));
        self.refresh_empty_state();
    }

    /// Returns the query represented by this page.
    #[must_use]
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    /// Updates the query represented by this page.
    pub(crate) fn set_query(&mut self, query: String) {
        self.query = query;
        self.refresh_empty_state();
    }

    /// Clears the query represented by this page.
    pub(crate) fn clear_query(&mut self) {
        self.set_query(String::new());
    }

    /// Returns the selected tab.
    #[must_use]
    pub(crate) const fn tab(&self) -> SearchResultsTab {
        self.tab
    }

    /// Sets the selected tab without changing the content filter.
    pub(crate) fn set_tab(&mut self, tab: SearchResultsTab) {
        self.tab = tab;
        self.tab_was_user_selected = true;
        self.refresh_empty_state();
    }

    /// Returns the selected content filter.
    #[must_use]
    pub(crate) const fn filter(&self) -> ContentFilter {
        self.filter
    }

    /// Returns the frame-chrome filter display for this inspector.
    #[must_use]
    pub(crate) fn filter_chip_strip(&self) -> FilterChipStripDisplay {
        FilterChipStripDisplay::default_for_search_inspector(self.filter, true)
    }

    /// Sets the content filter without changing the selected tab.
    pub(crate) fn set_filter(&mut self, filter: ContentFilter) {
        self.filter = filter;
        self.refresh_empty_state();
    }

    /// Returns the display label for an Index feed activation id.
    #[must_use]
    pub(crate) fn index_feed_label(&self, activation_id: &str) -> Option<String> {
        self.feeds
            .cached_row_matching(ContentFilter::All, |row| row.id == activation_id)
            .map(|row| row.label)
    }

    /// Returns the display label for an Index track activation id.
    #[must_use]
    pub(crate) fn index_track_label(&self, activation_id: &str) -> Option<String> {
        self.tracks
            .cached_row_matching(ContentFilter::All, |row| row.id == activation_id)
            .map(|row| row.label)
    }

    /// Projects a remote Index feed detail page from a result row or fallback nav data.
    #[must_use]
    pub(crate) fn index_feed_detail(
        &self,
        activation_id: &str,
        fallback_id: &str,
        fallback_label: &str,
    ) -> IndexDetailDisplay {
        self.feeds
            .cached_row_matching(ContentFilter::All, |row| row.id == activation_id)
            .map_or_else(
                || {
                    IndexDetailDisplay::new(
                        IndexDetailKind::Feed,
                        fallback_id,
                        fallback_label,
                        "MusicIndex feed",
                    )
                },
                |row| IndexDetailDisplay::feed(&row, fallback_id),
            )
    }

    /// Projects a remote Index track detail page from a result row or fallback nav data.
    #[must_use]
    pub(crate) fn index_track_detail(
        &self,
        activation_id: &str,
        fallback_id: &str,
        fallback_label: &str,
    ) -> IndexDetailDisplay {
        self.tracks
            .cached_row_matching(ContentFilter::All, |row| row.id == activation_id)
            .map_or_else(
                || {
                    IndexDetailDisplay::new(
                        IndexDetailKind::Track,
                        fallback_id,
                        fallback_label,
                        "MusicIndex track",
                    )
                },
                |row| IndexDetailDisplay::track(&row, fallback_id),
            )
    }

    /// Returns whether a tab/filter combination has no rows.
    #[must_use]
    pub(crate) fn is_empty(&self, tab: SearchResultsTab, filter: ContentFilter) -> bool {
        match tab {
            SearchResultsTab::Artists => self.artists.is_empty(filter),
            SearchResultsTab::Feeds => self.feeds.is_empty(filter),
            SearchResultsTab::Tracks => self.tracks.is_empty(filter),
        }
    }

    /// Returns the empty state for the active tab/filter, if any.
    #[must_use]
    pub(crate) fn empty_state(&self) -> Option<&EmptyStateDisplay> {
        self.empty_state.as_ref()
    }

    /// Returns the empty state for an explicit tab/filter scope.
    #[must_use]
    pub(crate) fn empty_state_for_scope(
        &self,
        tab: SearchResultsTab,
        filter: ContentFilter,
    ) -> Option<EmptyStateDisplay> {
        if self.index_loading
            && matches!(filter, ContentFilter::All | ContentFilter::Index)
            && self.is_empty(tab, filter)
        {
            return None;
        }

        if matches!(filter, ContentFilter::All | ContentFilter::Index) && self.is_empty(tab, filter)
        {
            if let Some(error) = self.index_error.as_ref() {
                return Some(error.clone());
            }
        }

        self.is_empty(tab, filter)
            .then(|| empty_state_for(tab, filter, &self.query))
    }

    /// Returns artist result windows.
    #[must_use]
    pub(crate) const fn artists(&self) -> &SearchResultsPagedTab<ArtistResultDisplay> {
        &self.artists
    }

    /// Returns mutable artist result windows.
    pub(crate) const fn artists_mut(&mut self) -> &mut SearchResultsPagedTab<ArtistResultDisplay> {
        &mut self.artists
    }

    /// Returns feed result windows.
    #[must_use]
    pub(crate) const fn feeds(&self) -> &SearchResultsPagedTab<FeedResultDisplay> {
        &self.feeds
    }

    /// Returns mutable feed result windows.
    pub(crate) const fn feeds_mut(&mut self) -> &mut SearchResultsPagedTab<FeedResultDisplay> {
        &mut self.feeds
    }

    /// Returns track result windows.
    #[must_use]
    pub(crate) const fn tracks(&self) -> &SearchResultsPagedTab<TrackResultDisplay> {
        &self.tracks
    }

    /// Returns mutable track result windows.
    pub(crate) const fn tracks_mut(&mut self) -> &mut SearchResultsPagedTab<TrackResultDisplay> {
        &mut self.tracks
    }

    fn refresh_empty_state(&mut self) {
        if self.index_loading
            && matches!(self.filter, ContentFilter::All | ContentFilter::Index)
            && self.is_empty(self.tab, self.filter)
        {
            self.empty_state = None;
            return;
        }

        if matches!(self.filter, ContentFilter::All | ContentFilter::Index)
            && self.is_empty(self.tab, self.filter)
        {
            if let Some(error) = self.index_error.as_ref() {
                self.empty_state = Some(error.clone());
                return;
            }
        }

        self.empty_state = self
            .is_empty(self.tab, self.filter)
            .then(|| empty_state_for(self.tab, self.filter, &self.query));
    }

    fn select_first_populated_tab_if_automatic(&mut self) {
        if self.tab_was_user_selected || !self.is_empty(self.tab, self.filter) {
            return;
        }

        if let Some(tab) = [
            SearchResultsTab::Artists,
            SearchResultsTab::Feeds,
            SearchResultsTab::Tracks,
        ]
        .into_iter()
        .find(|tab| !self.is_empty(*tab, self.filter))
        {
            self.tab = tab;
        }
    }
}

struct LocalLibrarySearchRows {
    artists: Vec<(SearchResultItemId, ArtistResultDisplay)>,
    feeds: Vec<(SearchResultItemId, FeedResultDisplay)>,
    tracks: Vec<(SearchResultItemId, TrackResultDisplay)>,
}

fn local_library_result_rows(tracks: &[TrackRow]) -> LocalLibrarySearchRows {
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

fn empty_state_for(tab: SearchResultsTab, filter: ContentFilter, query: &str) -> EmptyStateDisplay {
    let title = format!("No {} results", tab.label().to_lowercase());
    let secondary = match filter {
        ContentFilter::All => format!("No results matched \"{query}\"."),
        ContentFilter::Library => format!("No library results matched \"{query}\"."),
        ContentFilter::Index => format!("No index results matched \"{query}\"."),
    };
    EmptyStateDisplay::new(title, secondary, clear_filter_action_id(filter))
}

const fn clear_filter_action_id(filter: ContentFilter) -> Option<&'static str> {
    match filter {
        ContentFilter::All => None,
        ContentFilter::Library | ContentFilter::Index => Some("search-results.clear-filter"),
    }
}

#[cfg(test)]
mod tests {
    use crate::db::TrackRow;
    use crate::runtime::paged_list_vm::RowSlot;
    use crate::view_models::workspace::ContentFilter;
    use crate::views::{FeedView, TrackView};

    use super::{
        ArtistResultDisplay, FeedResultDisplay, IndexDetailKind, IndexSearchResultRows,
        SearchResultOrigin, SearchResultsInspectorPageVm, SearchResultsPagedTab, SearchResultsTab,
        TrackResultDisplay,
    };

    fn artist(id: SearchResultItemId, label: &str) -> ArtistResultDisplay {
        ArtistResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
    }

    fn feed(id: SearchResultItemId, label: &str) -> FeedResultDisplay {
        FeedResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
    }

    fn track(id: SearchResultItemId, label: &str) -> TrackResultDisplay {
        TrackResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
    }

    use super::SearchResultItemId;

    fn local_track(id: i64, feed_id: i64, feed_title: &str, title: &str, artist: &str) -> TrackRow {
        TrackRow {
            id,
            feed_id,
            track_title: Some(title.to_string()),
            artist_name: Some(artist.to_string()),
            album_artist_name: Some(artist.to_string()),
            feed_title: Some(feed_title.to_string()),
            is_in_library: true,
            ..TrackRow::default()
        }
    }

    #[test]
    fn tab_and_filter_state_are_independent() {
        let mut vm = SearchResultsInspectorPageVm::new("jazz")
            .with_artists(SearchResultsPagedTab::new(vec![1], Vec::new(), vec![1]))
            .with_feeds(SearchResultsPagedTab::new(vec![2], vec![2], Vec::new()));

        vm.set_filter(ContentFilter::Library);
        vm.set_tab(SearchResultsTab::Feeds);

        assert_eq!(vm.tab(), SearchResultsTab::Feeds);
        assert_eq!(vm.filter(), ContentFilter::Library);
        assert!(
            !vm.is_empty(SearchResultsTab::Feeds, ContentFilter::Library),
            "tab switch must not reset the selected content filter"
        );
    }

    #[test]
    fn per_tab_paged_windows_operate_independently() {
        let mut vm = SearchResultsInspectorPageVm::new("ambient")
            .with_artists(SearchResultsPagedTab::new(
                vec![10, 11],
                Vec::new(),
                vec![10, 11],
            ))
            .with_feeds(SearchResultsPagedTab::new(vec![20], vec![20], Vec::new()))
            .with_tracks(SearchResultsPagedTab::new(vec![30], Vec::new(), vec![30]));

        let artist_window = vm.artists_mut().window_mut(ContentFilter::All);
        assert!(matches!(artist_window.row(0), RowSlot::Pending(_)));
        let artist_requests = artist_window.drain_requests();
        assert_eq!(artist_requests.len(), 1);
        artist_window.fulfill_page(0, [(10, artist(10, "A")), (11, artist(11, "B"))]);

        assert!(
            vm.feeds_mut()
                .window_mut(ContentFilter::All)
                .drain_requests()
                .is_empty(),
            "reading the artist window must not enqueue feed requests"
        );
        assert!(
            vm.tracks_mut()
                .window_mut(ContentFilter::All)
                .drain_requests()
                .is_empty(),
            "reading the artist window must not enqueue track requests"
        );
        assert!(matches!(
            vm.artists().window(ContentFilter::All).peek_row(1),
            RowSlot::Ready(_)
        ));
    }

    #[test]
    fn empty_state_tracks_active_tab_and_filter() {
        let mut vm = SearchResultsInspectorPageVm::new("noise")
            .with_artists(SearchResultsPagedTab::new(vec![1], Vec::new(), vec![1]))
            .with_feeds(SearchResultsPagedTab::new(vec![2], vec![2], Vec::new()));

        vm.set_tab(SearchResultsTab::Artists);
        vm.set_filter(ContentFilter::Library);
        let empty = vm.empty_state().expect("library artists should be empty");
        assert_eq!(empty.title, "No artists results");
        assert_eq!(
            empty.clear_filter_action_id,
            Some("search-results.clear-filter")
        );

        vm.set_filter(ContentFilter::Index);
        assert!(
            vm.empty_state().is_none(),
            "index artists should have one result"
        );

        vm.set_tab(SearchResultsTab::Feeds);
        assert!(
            vm.empty_state().is_some(),
            "index feeds should be empty for the active filter"
        );
    }

    #[test]
    fn result_display_builders_project_accessible_labels() {
        let artist = ArtistResultDisplay::new("a1", "Alice", SearchResultOrigin::Index)
            .with_secondary_text("3 feeds")
            .with_thumbnail_href("https://example.test/a.png");
        let feed = feed(7, "Morning Show");
        let track = track(9, "Theme");

        assert_eq!(artist.a11y_label, "Artist: Alice");
        assert_eq!(artist.secondary_text, "3 feeds");
        assert_eq!(
            artist.thumbnail_href.as_deref(),
            Some("https://example.test/a.png")
        );
        assert_eq!(feed.a11y_label, "Feed: Morning Show");
        assert_eq!(track.a11y_label, "Track: Theme");
        assert!(SearchResultOrigin::Index.matches_filter(ContentFilter::All));
        assert!(!SearchResultOrigin::Index.matches_filter(ContentFilter::Library));
    }

    #[test]
    fn local_library_tracks_populate_ready_artist_feed_and_track_results() {
        let rows = [
            local_track(10, 1, "The Heycitizen Experience", "Opening", "HeyCitizen"),
            local_track(
                11,
                2,
                "HeyCitizen's Lo-Fi Hip-Hop Beats",
                "Side B",
                "HeyCitizen",
            ),
        ];

        let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks("heycitizen", &rows);

        assert!(
            vm.empty_state().is_none(),
            "default Artists/All tab should not show empty state when local artist rows exist"
        );
        assert_eq!(vm.artists().window(ContentFilter::All).total(), 1);
        assert_eq!(vm.artists().window(ContentFilter::Library).total(), 1);
        assert_eq!(vm.artists().window(ContentFilter::Index).total(), 0);
        assert_eq!(vm.feeds().window(ContentFilter::All).total(), 2);
        assert_eq!(vm.tracks().window(ContentFilter::All).total(), 2);

        let RowSlot::Ready(artist) = vm.artists().window(ContentFilter::All).peek_row(0) else {
            panic!("local artist row should be preloaded");
        };
        assert_eq!(artist.label, "HeyCitizen");
        assert_eq!(artist.secondary_text, "2 albums - 2 tracks");

        vm.set_tab(SearchResultsTab::Tracks);
        let RowSlot::Ready(track) = vm.tracks().window(ContentFilter::All).peek_row(0) else {
            panic!("local track row should be preloaded");
        };
        assert_eq!(track.label, "Opening");
        assert_eq!(track.secondary_text, "HeyCitizen");
    }

    #[test]
    fn index_loading_suppresses_empty_state_for_index_and_all() {
        let mut vm = SearchResultsInspectorPageVm::new("ambient");

        vm.mark_index_loading();

        assert!(vm.is_index_loading());
        assert!(
            vm.empty_state().is_none(),
            "All filter should keep pending remote search visually pending"
        );

        vm.set_filter(ContentFilter::Index);
        assert!(
            vm.empty_state().is_none(),
            "Index filter should keep pending remote search visually pending"
        );

        vm.set_filter(ContentFilter::Library);
        assert!(
            vm.empty_state().is_some(),
            "Library filter is not waiting on remote Index results"
        );
    }

    #[test]
    fn index_rows_populate_index_and_all_but_not_library() {
        let local_rows = [local_track(
            10,
            1,
            "Local Feed",
            "Local Track",
            "Local Artist",
        )];
        let mut vm = SearchResultsInspectorPageVm::from_local_library_tracks("mix", &local_rows);

        vm.replace_index_results(IndexSearchResultRows {
            artists: vec![(101, artist(101, "Remote Artist"))],
            feeds: vec![(201, feed(201, "Remote Feed"))],
            tracks: vec![(301, track(301, "Remote Track"))],
        });

        assert!(!vm.is_index_loading());
        assert_eq!(vm.artists().window(ContentFilter::Index).total(), 1);
        assert_eq!(vm.artists().window(ContentFilter::All).total(), 2);
        assert_eq!(vm.artists().window(ContentFilter::Library).total(), 1);
        assert_eq!(vm.feeds().window(ContentFilter::Index).total(), 1);
        assert_eq!(vm.feeds().window(ContentFilter::All).total(), 2);
        assert_eq!(vm.feeds().window(ContentFilter::Library).total(), 1);
        assert_eq!(vm.tracks().window(ContentFilter::Index).total(), 1);
        assert_eq!(vm.tracks().window(ContentFilter::All).total(), 2);
        assert_eq!(vm.tracks().window(ContentFilter::Library).total(), 1);

        let RowSlot::Ready(local_artist) = vm.artists().window(ContentFilter::All).peek_row(0)
        else {
            panic!("local All artist row should stay cached after remote rows arrive");
        };
        assert_eq!(local_artist.label, "Local Artist");

        let RowSlot::Ready(remote_artist) = vm.artists().window(ContentFilter::All).peek_row(1)
        else {
            panic!("remote All artist row should be cached after replacement");
        };
        assert_eq!(remote_artist.label, "Remote Artist");

        let RowSlot::Ready(remote_feed) = vm.feeds().window(ContentFilter::All).peek_row(1) else {
            panic!("remote All feed row should be cached after replacement");
        };
        assert_eq!(remote_feed.label, "Remote Feed");

        let RowSlot::Ready(remote_track) = vm.tracks().window(ContentFilter::All).peek_row(1)
        else {
            panic!("remote All track row should be cached after replacement");
        };
        assert_eq!(remote_track.label, "Remote Track");
    }

    #[test]
    fn index_results_auto_select_first_populated_tab_until_user_selects_tab() {
        let mut vm = SearchResultsInspectorPageVm::new("delta");

        vm.replace_index_results(IndexSearchResultRows {
            artists: Vec::new(),
            feeds: vec![(201, feed(201, "Remote Feed"))],
            tracks: Vec::new(),
        });

        assert_eq!(
            vm.tab(),
            SearchResultsTab::Feeds,
            "automatic search landing should move off an empty default tab"
        );
        assert!(
            vm.empty_state().is_none(),
            "populated remote feed rows should be visible after auto-tab selection"
        );

        vm.set_tab(SearchResultsTab::Artists);
        vm.replace_index_results(IndexSearchResultRows {
            artists: Vec::new(),
            feeds: vec![(202, feed(202, "Second Feed"))],
            tracks: Vec::new(),
        });

        assert_eq!(
            vm.tab(),
            SearchResultsTab::Artists,
            "explicit user tab selection must not be overwritten by remote refresh"
        );
    }

    #[test]
    fn index_detail_projection_uses_cached_result_rows() {
        let mut vm = SearchResultsInspectorPageVm::new("delta");
        vm.replace_index_results(IndexSearchResultRows {
            artists: Vec::new(),
            feeds: vec![(
                201,
                FeedResultDisplay::new(
                    "index-feed:feed-guid",
                    "Remote Feed",
                    SearchResultOrigin::Index,
                )
                .with_secondary_text("Remote Artist - 6 tracks")
                .with_remote_feed(FeedView {
                    title: Some("Remote Feed".to_string()),
                    tracks: vec![TrackView {
                        title: Some("Remote Track".to_string()),
                        ..TrackView::default()
                    }],
                    ..FeedView::default()
                }),
            )],
            tracks: vec![(
                301,
                TrackResultDisplay::new(
                    "index-track:feed-guid:track-guid",
                    "Remote Track",
                    SearchResultOrigin::Index,
                )
                .with_secondary_text("Remote Artist"),
            )],
        });

        assert_eq!(
            vm.index_feed_label("index-feed:feed-guid").as_deref(),
            Some("Remote Feed")
        );
        let feed = vm.index_feed_detail("index-feed:feed-guid", "feed-guid", "feed-guid");
        assert_eq!(feed.kind, IndexDetailKind::Feed);
        assert_eq!(feed.title, "Remote Feed");
        assert_eq!(feed.secondary_text, "Remote Artist - 6 tracks");
        assert!(
            feed.feed.is_some(),
            "Index feed detail should preserve rich remote feed projection when search fetched it"
        );
        assert_eq!(
            feed.feed.as_ref().map(|feed| feed.tracks.len()),
            Some(1),
            "Index feed detail should preserve remote track rows for release-detail rendering"
        );

        assert_eq!(
            vm.index_track_label("index-track:feed-guid:track-guid")
                .as_deref(),
            Some("Remote Track")
        );
        let track = vm.index_track_detail(
            "index-track:feed-guid:track-guid",
            "feed-guid:track-guid",
            "track-guid",
        );
        assert_eq!(track.kind, IndexDetailKind::Track);
        assert_eq!(track.title, "Remote Track");
        assert_eq!(track.secondary_text, "Remote Artist");
    }

    #[test]
    fn scoped_empty_state_does_not_mutate_root_tab_or_filter() {
        let mut vm = SearchResultsInspectorPageVm::new("delta");
        vm.set_tab(SearchResultsTab::Artists);
        vm.set_filter(ContentFilter::All);

        let empty = vm
            .empty_state_for_scope(SearchResultsTab::Feeds, ContentFilter::Index)
            .expect("scoped feeds/index render should compute its own empty state");

        assert_eq!(empty.title, "No feeds results");
        assert_eq!(vm.tab(), SearchResultsTab::Artists);
        assert_eq!(vm.filter(), ContentFilter::All);
    }

    #[test]
    fn index_error_surfaces_for_index_when_no_index_rows_exist() {
        let mut vm = SearchResultsInspectorPageVm::new("field recordings");

        vm.mark_index_loading();
        vm.set_filter(ContentFilter::Index);
        vm.set_index_error("Index unavailable", "Try again later.");

        let empty = vm
            .empty_state()
            .expect("index error should surface as empty-state display");
        assert_eq!(empty.title, "Index unavailable");
        assert_eq!(empty.secondary, "Try again later.");
        assert_eq!(empty.clear_filter_action_id, None);
    }

    #[test]
    fn filter_chip_strip_uses_search_inspector_contract() {
        let mut vm = SearchResultsInspectorPageVm::new("beats");
        vm.set_filter(ContentFilter::Index);

        let strip = vm.filter_chip_strip();

        assert_eq!(strip.id, "workspace-search-inspector-filter");
        assert_eq!(strip.selected, ContentFilter::Index);
        assert!(
            strip.narrow_collapse_to_pulldown,
            "search inspector filters should collapse in narrow detail frames"
        );
    }

    #[test]
    fn query_update_refreshes_empty_state_copy() {
        let mut vm = SearchResultsInspectorPageVm::new("old query");

        vm.set_query("new query".to_string());

        assert_eq!(vm.query(), "new query");
        assert_eq!(
            vm.empty_state()
                .expect("empty inspector should expose empty state")
                .secondary,
            "No results matched \"new query\"."
        );
    }

    #[test]
    fn clear_query_refreshes_empty_state_copy() {
        let mut vm = SearchResultsInspectorPageVm::new("old query");

        vm.clear_query();

        assert_eq!(vm.query(), "");
        assert_eq!(
            vm.empty_state()
                .expect("empty inspector should expose empty state")
                .secondary,
            "No results matched \"\"."
        );
    }
}
