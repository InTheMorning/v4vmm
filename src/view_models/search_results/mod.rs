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

use crate::db::TrackRow;
use crate::view_models::workspace::{ContentFilter, FilterChipStripDisplay};

mod empty_state;
mod index_detail;
mod paged_tab;
mod results;
mod tabs;
#[cfg(test)]
mod tests;

pub(crate) use empty_state::EmptyStateDisplay;
pub(crate) use index_detail::{IndexDetailDisplay, IndexDetailKind, IndexSearchResultRows};
pub(crate) use paged_tab::SearchResultsPagedTab;
pub(crate) use results::{ArtistResultDisplay, FeedResultDisplay, TrackResultDisplay};
pub(crate) use tabs::{SearchResultItemId, SearchResultOrigin, SearchResultsTab};

use self::empty_state::empty_state_for;
use self::results::local_library_result_rows;

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
