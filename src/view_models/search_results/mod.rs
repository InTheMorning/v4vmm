//! Search-results view models of the Music list column.
//!
//! ADR 0084 Decision 2: the toolbar search fills the list column, and a
//! result opens in the detail column. Packet 010 gives the view three tabs,
//! one row for each item with the state word of each Library item, and the
//! selection model of packets 018 and 019 on the Releases tab and the
//! Tracks tab. Each tab keeps its rows in one ADR 0041 [`PagedListVm`]
//! window.
#![warn(clippy::pedantic)]

use crate::db::TrackRow;
use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};
use crate::view_models::detail_selection::{
    detail_header_action, search_header_action, search_release_rows, search_track_rows,
    DetailPageRows, DetailRowClick, DetailRowKey, DetailSelection,
};
use crate::view_models::list_selection::FocusStep;
use crate::view_models::selection_action_bar::SelectionActionBarVm;
use crate::view_models::workspace::{FrameHeaderActionDisplay, FrameNavigationEntry};

mod empty_state;
mod failure;
mod index_detail;
mod paged_tab;
mod results;
mod rows;
mod saved_search;
mod tabs;
#[cfg(test)]
mod tests;

pub(crate) use empty_state::EmptyStateDisplay;
pub(crate) use failure::{SearchFailureAction, SearchFailureAvailability};
pub(crate) use index_detail::{IndexDetailDisplay, IndexDetailKind, IndexSearchResultRows};
pub(crate) use paged_tab::SearchResultsPagedTab;
pub(crate) use results::{
    artist_feed_result_display, ArtistResultDisplay, ArtistSearchFacts, FeedResultDisplay,
    SearchResultTarget, TrackResultDisplay, INDEX_FEED_PAGE_RESULT_PREFIX,
};
pub(crate) use rows::{
    ReleaseKindPlacement, SearchLibraryLookup, SearchLibraryRelease, SearchLibraryTrack,
    SearchResultRowVm, SearchRowBadge,
};
pub(crate) use saved_search::{
    delete_saved_search_confirmation, location_after_saved_search_delete, save_search_action,
    saved_search_label, saved_search_restore, saved_search_result_is_current,
    saved_search_row_a11y_label, SavedSearch, SavedSearchDeleteConfirmation, SavedSearchId,
    SavedSearchPage, SavedSearchRestore, SearchPageActionDisplay, SearchPageActionStyle,
};
pub(crate) use tabs::{SearchResultItemId, SearchResultOrigin, SearchResultsTab};

use self::empty_state::empty_state_for;
use self::paged_tab::SearchRowMergeKey;
use self::results::local_library_result_rows;

/// GPUI-free page contract for the search results in the list column.
#[derive(Debug)]
pub(crate) struct SearchResultsInspectorPageVm {
    query: String,
    tab: SearchResultsTab,
    releases: SearchResultsPagedTab<FeedResultDisplay>,
    artists: SearchResultsPagedTab<ArtistResultDisplay>,
    tracks: SearchResultsPagedTab<TrackResultDisplay>,
    index_loading: bool,
    index_error: Option<EmptyStateDisplay>,
    empty_state: Option<EmptyStateDisplay>,
    tab_was_user_selected: bool,
    /// The page entry of this view model: `FrameNavigationEntry::Search`
    /// for the toolbar search, or `FrameNavigationEntry::SavedSearch` for
    /// the saved-search page of the detail column (packet 012).
    entry: FrameNavigationEntry,
    /// The one selection of the page (packet 010 "The selection"), bound
    /// to `entry`.
    selection: DetailSelection,
    /// The status text of the view, for example a save failure (packet
    /// 012). `None` when the view has no status to tell.
    status_text: Option<String>,
}

impl SearchResultsInspectorPageVm {
    /// Creates an empty search-results page for a query.
    #[must_use]
    pub(crate) fn new(query: impl Into<String>) -> Self {
        let query = query.into();
        let entry = FrameNavigationEntry::Search(query.clone());
        let mut selection = DetailSelection::new();
        selection.reset(Some(entry.clone()));
        let mut page = Self {
            query,
            tab: SearchResultsTab::default(),
            releases: SearchResultsPagedTab::empty(),
            artists: SearchResultsPagedTab::empty(),
            tracks: SearchResultsPagedTab::empty(),
            index_loading: false,
            index_error: None,
            empty_state: None,
            tab_was_user_selected: false,
            entry,
            selection,
            status_text: None,
        };
        page.refresh_empty_state();
        page
    }

    /// Creates a search-results page from local library track matches.
    /// `artist_facts` gives the match of each artist name (ADR 0084
    /// Decision 7, packet 017).
    #[must_use]
    pub(crate) fn from_local_library_tracks(
        query: impl Into<String>,
        tracks: &[TrackRow],
        artist_facts: &ArtistSearchFacts,
    ) -> Self {
        Self::new(query).with_local_library_tracks(tracks, artist_facts)
    }

    /// Returns this page with local library result rows attached.
    #[must_use]
    pub(crate) fn with_local_library_tracks(
        mut self,
        tracks: &[TrackRow],
        artist_facts: &ArtistSearchFacts,
    ) -> Self {
        let local_rows = local_library_result_rows(tracks, artist_facts);
        self.artists = SearchResultsPagedTab::ready_library(local_rows.artists);
        self.releases = SearchResultsPagedTab::ready_library(local_rows.feeds);
        self.tracks = SearchResultsPagedTab::ready_library(local_rows.tracks);
        self.refresh_empty_state();
        self
    }

    /// Returns this page as the page of the saved search `search` (packet
    /// 012). The page opens on the start tab of the save. The tab counts as
    /// a tab that the operator selected, so the arrival of the Index rows
    /// does not change it. The selection belongs to the saved-search entry.
    #[must_use]
    pub(crate) fn into_saved_search(mut self, search: &SavedSearch) -> Self {
        self.entry = search.entry();
        self.selection.reset(Some(self.entry.clone()));
        self.tab = search.start_tab;
        self.tab_was_user_selected = true;
        self.refresh_empty_state();
        self
    }

    /// "Save search", or "Saved" when `saved` holds the query (packet 012).
    #[must_use]
    pub(crate) fn save_action(&self, saved: &[SavedSearch]) -> SearchPageActionDisplay {
        save_search_action(&self.query, saved)
    }

    /// Records the failure of "Save search" (packet 012). The status text
    /// names the query and the database error.
    pub(crate) fn set_save_failure(&mut self, error: &str) {
        self.status_text = Some(format!(
            "The app could not save the search \"{}\". Database error: {error}",
            self.query
        ));
    }

    /// Records the failure of "Delete…" of a saved search (packet 012). The
    /// status text names the query and the database error.
    pub(crate) fn set_delete_failure(&mut self, error: &str) {
        self.status_text = Some(format!(
            "The app could not delete the saved search \"{}\". Database error: {error}",
            self.query
        ));
    }

    /// Removes the status text, for example after a successful save.
    pub(crate) fn clear_status(&mut self) {
        self.status_text = None;
    }

    /// The status text of the view, or `None`.
    #[must_use]
    pub(crate) fn status_text(&self) -> Option<&str> {
        self.status_text.as_deref()
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

    /// Merges the remote Index rows after the Library rows (packet 010
    /// "One row for each item"). A selected row that stays visible stays
    /// selected, and the focus stays.
    pub(crate) fn replace_index_results(&mut self, rows: IndexSearchResultRows) {
        self.index_loading = false;
        self.index_error = None;
        self.artists.replace_index_rows(rows.artists);
        self.releases.replace_index_rows(rows.feeds);
        self.tracks.replace_index_rows(rows.tracks);
        self.select_first_populated_tab_if_automatic();
        let members = self.visible_members();
        self.selection.retain_visible(&members);
        self.refresh_empty_state();
    }

    /// Applies the Index rows of a request for `query` (packet 010 "The
    /// stale results"). A result for another query changes neither the
    /// rows nor the selection. Gives `true` when the rows changed.
    pub(crate) fn replace_index_results_for_query(
        &mut self,
        query: &str,
        rows: IndexSearchResultRows,
    ) -> bool {
        if self.query != query {
            return false;
        }
        self.replace_index_results(rows);
        true
    }

    /// Marks the async remote Index search as failed.
    pub(crate) fn set_index_error(
        &mut self,
        error: &crate::application::CommandError,
        endpoint: &str,
        observed_at: std::time::SystemTime,
    ) {
        self.index_loading = false;
        // Walk of 2026-10-21: an HTTP `400` reply is a query error, not an
        // availability error.
        let (title, failure) =
            if matches!(error, crate::application::CommandError::QueryRejected(_)) {
                (
                    failure::QUERY_REJECTED_TITLE,
                    failure::SearchFailureDisplay::query_rejected(
                        &self.query,
                        error,
                        endpoint,
                        observed_at,
                    ),
                )
            } else {
                (
                    "MusicIndex search unavailable",
                    failure::SearchFailureDisplay::new(error, endpoint, observed_at),
                )
            };
        let mut empty = EmptyStateDisplay::new(title, failure.summary.clone());
        empty.failure = Some(failure);
        self.index_error = Some(empty);
        self.refresh_empty_state();
    }

    /// Applies disclosure state or returns the exact report for clipboard delivery.
    pub(crate) fn activate_failure_action(
        &mut self,
        action: SearchFailureAction,
    ) -> Option<String> {
        let report = self
            .index_error
            .as_mut()?
            .failure
            .as_mut()?
            .activate(action);
        self.refresh_empty_state();
        report
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

    /// Selects a tab. A change of tab clears the selection and sets
    /// `Normal` (packet 010 decision D8).
    pub(crate) fn set_tab(&mut self, tab: SearchResultsTab) {
        if tab != self.tab {
            self.selection.reset(Some(self.entry.clone()));
        }
        self.tab = tab;
        self.tab_was_user_selected = true;
        self.refresh_empty_state();
    }

    /// The label of the row `id` of the tab `tab`.
    #[must_use]
    pub(crate) fn row_label(&self, tab: SearchResultsTab, id: &str) -> Option<String> {
        match tab {
            SearchResultsTab::Releases => self
                .releases
                .cached_row_matching(|row| row.id == id)
                .map(|row| row.label),
            SearchResultsTab::Artists => self
                .artists
                .cached_row_matching(|row| row.id == id)
                .map(|row| row.label),
            SearchResultsTab::Tracks => self
                .tracks
                .cached_row_matching(|row| row.id == id)
                .map(|row| row.label),
        }
    }

    /// Returns whether a tab has no rows.
    #[must_use]
    pub(crate) fn is_empty(&self, tab: SearchResultsTab) -> bool {
        match tab {
            SearchResultsTab::Releases => self.releases.is_empty(),
            SearchResultsTab::Artists => self.artists.is_empty(),
            SearchResultsTab::Tracks => self.tracks.is_empty(),
        }
    }

    /// Returns the empty state for the active tab, if any.
    #[must_use]
    pub(crate) fn empty_state(&self) -> Option<&EmptyStateDisplay> {
        self.empty_state.as_ref()
    }

    /// The note of the Index part under the rows of the active tab (walk of
    /// 2026-10-21). It shows when the tab has rows, for example Library
    /// rows, and `MusicIndex` did not accept the query. A tab with no rows
    /// shows the same note as its empty state.
    #[must_use]
    pub(crate) fn index_notice(&self) -> Option<&EmptyStateDisplay> {
        if self.is_empty(self.tab) {
            return None;
        }
        self.index_error.as_ref().filter(|notice| {
            notice
                .failure
                .as_ref()
                .is_some_and(failure::SearchFailureDisplay::is_query_rejected)
        })
    }

    /// Returns the visible row thumbnail hrefs of the active tab.
    #[must_use]
    pub(crate) fn thumbnail_hrefs(&self) -> Vec<String> {
        match self.tab {
            SearchResultsTab::Releases => {
                visible_thumbnail_hrefs(self.releases.window(), |row| row.thumbnail_href.as_deref())
            }
            SearchResultsTab::Artists => {
                visible_thumbnail_hrefs(self.artists.window(), |row| row.thumbnail_href.as_deref())
            }
            SearchResultsTab::Tracks => {
                visible_thumbnail_hrefs(self.tracks.window(), |row| row.thumbnail_href.as_deref())
            }
        }
    }

    /// The visible rows of the active tab, with the state word, the badge
    /// and the selection facts of each row. `lookup` gives the Library
    /// facts at this render.
    #[must_use]
    pub(crate) fn visible_rows(&self, lookup: &dyn SearchLibraryLookup) -> Vec<SearchResultRowVm> {
        match self.tab {
            SearchResultsTab::Releases => self
                .releases
                .rows()
                .iter()
                .map(|row| {
                    rows::release_row_vm(
                        row,
                        lookup,
                        &self.selection,
                        self.release_kind_placement(),
                    )
                })
                .collect(),
            SearchResultsTab::Artists => self
                .artists
                .rows()
                .iter()
                .map(|row| rows::artist_row_vm(row, lookup))
                .collect(),
            SearchResultsTab::Tracks => self
                .tracks
                .rows()
                .iter()
                .map(|row| rows::track_row_vm(row, lookup, &self.selection))
                .collect(),
        }
    }

    /// The place of the kind of a release row (walk of 2026-10-21). The
    /// toolbar search in the list column puts the kind in the second line.
    /// The saved-search page in the detail column keeps the kind badge.
    #[must_use]
    pub(crate) const fn release_kind_placement(&self) -> ReleaseKindPlacement {
        match self.entry {
            FrameNavigationEntry::SavedSearch { .. } => ReleaseKindPlacement::Badge,
            _ => ReleaseKindPlacement::Subtitle,
        }
    }

    /// The selectable rows of the active tab (packet 010 "The
    /// selection"). The Artists tab has no selection model, so it gives
    /// `None`.
    #[must_use]
    pub(crate) fn selection_rows(
        &self,
        lookup: &dyn SearchLibraryLookup,
    ) -> Option<DetailPageRows> {
        match self.tab {
            SearchResultsTab::Releases => Some(DetailPageRows::Releases(search_release_rows(
                &self.releases.rows(),
                lookup,
            ))),
            SearchResultsTab::Tracks => Some(DetailPageRows::Tracks(search_track_rows(
                &self.tracks.rows(),
                lookup,
            ))),
            SearchResultsTab::Artists => None,
        }
    }

    /// The selection of the search list.
    #[must_use]
    pub(crate) const fn selection(&self) -> &DetailSelection {
        &self.selection
    }

    /// The "Select" or "Done" button of the column header. It shows only
    /// on the Releases tab and the Tracks tab, and only when the tab has a
    /// row with a key. The toolbar search gives the id of the list column.
    /// The saved-search page gives the id of the detail column (packet
    /// 012).
    #[must_use]
    pub(crate) fn header_action(
        &self,
        lookup: &dyn SearchLibraryLookup,
    ) -> Option<FrameHeaderActionDisplay> {
        let rows = self.selection_rows(lookup);
        match self.entry {
            FrameNavigationEntry::SavedSearch { .. } => {
                detail_header_action(rows.as_ref(), self.selection.mode())
            }
            _ => search_header_action(rows.as_ref(), self.selection.mode()),
        }
    }

    /// The action bar of packet 018, or `None` outside selection mode.
    #[must_use]
    pub(crate) fn action_bar(
        &self,
        lookup: &dyn SearchLibraryLookup,
    ) -> Option<SelectionActionBarVm> {
        self.selection_rows(lookup)?.action_bar(&self.selection)
    }

    /// A click on the body of the row with the selection key `key`. Gives
    /// `true` when the caller must open the page of the row: a plain click
    /// in `Normal` mode.
    pub(crate) fn click_row(
        &mut self,
        key: &DetailRowKey,
        click: DetailRowClick,
        lookup: &dyn SearchLibraryLookup,
    ) -> bool {
        let visible = self
            .selection_rows(lookup)
            .map(|rows| rows.selection_rows())
            .unwrap_or_default();
        self.selection.click(key, click, &visible)
    }

    /// A click on the checkbox of the row `key`.
    pub(crate) fn toggle_row(&mut self, key: &DetailRowKey) {
        self.selection.toggle_row(key);
    }

    /// The "Select" and "Done" button.
    pub(crate) fn toggle_mode(&mut self) {
        self.selection.toggle_mode();
    }

    /// "Clear" of the bar: clears the rows and keeps the mode.
    pub(crate) fn clear_rows(&mut self) {
        self.selection.clear_rows();
    }

    /// Space: changes the selection of the focused row.
    pub(crate) fn toggle_focused(&mut self, lookup: &dyn SearchLibraryLookup) {
        if let Some(rows) = self.selectable_rows(lookup) {
            self.selection.toggle_focused(&rows.selection_rows());
        }
    }

    /// Ctrl+A: selects each row of the active tab.
    pub(crate) fn select_all(&mut self, lookup: &dyn SearchLibraryLookup) {
        if let Some(rows) = self.selectable_rows(lookup) {
            self.selection.select_all(&rows.selection_rows());
        }
    }

    /// Up, Down, Page Up, Page Down, Home and End: moves the row focus.
    pub(crate) fn move_focus(
        &mut self,
        step: FocusStep,
        lookup: &dyn SearchLibraryLookup,
    ) -> Option<DetailRowKey> {
        let rows = self.selectable_rows(lookup)?;
        self.selection.move_focus(step, &rows.selection_rows())
    }

    /// Escape: ends selection mode. Gives `false` in `Normal`.
    pub(crate) fn end_selecting(&mut self) -> bool {
        self.selection.end_selecting()
    }

    fn selectable_rows(&self, lookup: &dyn SearchLibraryLookup) -> Option<DetailPageRows> {
        self.selection_rows(lookup)
            .filter(DetailPageRows::has_selectable_rows)
    }

    /// The selection key of each visible row of the active tab. The keys
    /// need no Library fact.
    fn visible_members(&self) -> Vec<DetailRowKey> {
        match self.tab {
            SearchResultsTab::Releases => self
                .releases
                .rows()
                .into_iter()
                .filter_map(|row| row.merge_key().map(DetailRowKey::Release))
                .collect(),
            SearchResultsTab::Tracks => self
                .tracks
                .rows()
                .into_iter()
                .filter_map(|row| row.merge_key().map(DetailRowKey::Track))
                .collect(),
            SearchResultsTab::Artists => Vec::new(),
        }
    }

    fn refresh_empty_state(&mut self) {
        if !self.is_empty(self.tab) {
            self.empty_state = None;
            return;
        }
        if self.index_loading {
            self.empty_state = None;
            return;
        }
        self.empty_state = Some(
            self.index_error
                .clone()
                .unwrap_or_else(|| empty_state_for(self.tab, &self.query)),
        );
    }

    fn select_first_populated_tab_if_automatic(&mut self) {
        if self.tab_was_user_selected || !self.is_empty(self.tab) {
            return;
        }
        if let Some(tab) = SearchResultsTab::ALL
            .into_iter()
            .find(|tab| !self.is_empty(*tab))
        {
            self.tab = tab;
        }
    }
}

/// The tab rows have no caller outside the tests of this module:
/// production code reads the rows through [`SearchResultsInspectorPageVm::visible_rows`].
#[cfg(test)]
impl SearchResultsInspectorPageVm {
    /// Returns the release rows.
    #[must_use]
    pub(crate) const fn releases(&self) -> &SearchResultsPagedTab<FeedResultDisplay> {
        &self.releases
    }

    /// Returns the artist rows.
    #[must_use]
    pub(crate) const fn artists(&self) -> &SearchResultsPagedTab<ArtistResultDisplay> {
        &self.artists
    }

    /// Returns the track rows.
    #[must_use]
    pub(crate) const fn tracks(&self) -> &SearchResultsPagedTab<TrackResultDisplay> {
        &self.tracks
    }
}

fn visible_thumbnail_hrefs<Row>(
    window: &PagedListVm<SearchResultItemId, Row>,
    thumbnail_href: impl Fn(&Row) -> Option<&str>,
) -> Vec<String> {
    (0..window.total().min(window.page_size()))
        .filter_map(|index| match window.peek_row(index) {
            RowSlot::Ready(row) => thumbnail_href(row.as_ref()).map(str::to_string),
            RowSlot::Pending(_) => None,
        })
        .collect()
}
