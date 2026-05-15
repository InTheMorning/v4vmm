//! Search-results inspector view-models (ADR 0047 Phase B).
//!
//! This module defines the GPUI-free contract for the future
//! `SearchResultsInspector` detail frame. It owns tab state, local
//! content-filter state, display-ready result rows, empty-state copy, and
//! windowed result lists backed by ADR 0041 [`PagedListVm`] instances.

#![cfg(feature = "async-runtime")]
#![warn(clippy::pedantic)]
#![expect(
    dead_code,
    reason = "ADR 0047 Task 004 lands the VM contract before UI routing consumes it"
)]

use crate::runtime::paged_list_vm::PagedListVm;
use crate::view_models::workspace::{ContentFilter, FilterChipStripDisplay};

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
#[derive(Clone, Debug, Eq, PartialEq)]
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
            library: PagedListVm::new(library),
            index: PagedListVm::new(index),
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

/// GPUI-free page contract for the search-results inspector.
#[derive(Debug)]
pub(crate) struct SearchResultsInspectorPageVm {
    query: String,
    tab: SearchResultsTab,
    filter: ContentFilter,
    artists: SearchResultsPagedTab<ArtistResultDisplay>,
    feeds: SearchResultsPagedTab<FeedResultDisplay>,
    tracks: SearchResultsPagedTab<TrackResultDisplay>,
    empty_state: Option<EmptyStateDisplay>,
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
            empty_state: None,
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

    /// Returns the query represented by this page.
    #[must_use]
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    /// Returns the selected tab.
    #[must_use]
    pub(crate) const fn tab(&self) -> SearchResultsTab {
        self.tab
    }

    /// Sets the selected tab without changing the content filter.
    pub(crate) fn set_tab(&mut self, tab: SearchResultsTab) {
        self.tab = tab;
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
        self.empty_state = self
            .is_empty(self.tab, self.filter)
            .then(|| empty_state_for(self.tab, self.filter, &self.query));
    }
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
    use crate::runtime::paged_list_vm::RowSlot;
    use crate::view_models::workspace::ContentFilter;

    use super::{
        ArtistResultDisplay, FeedResultDisplay, SearchResultOrigin, SearchResultsInspectorPageVm,
        SearchResultsPagedTab, SearchResultsTab, TrackResultDisplay,
    };

    fn artist(id: SearchResultItemId, label: &str) -> ArtistResultDisplay {
        ArtistResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
    }

    fn feed(id: SearchResultItemId, label: &str) -> FeedResultDisplay {
        FeedResultDisplay::new(id.to_string(), label, SearchResultOrigin::Library)
    }

    fn track(id: SearchResultItemId, label: &str) -> TrackResultDisplay {
        TrackResultDisplay::new(id.to_string(), label, SearchResultOrigin::Index)
    }

    use super::SearchResultItemId;

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
}
