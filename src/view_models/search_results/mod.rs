//! Search-results view models of the Music list column.
//!
//! ADR 0084 Decision 2: the toolbar search fills the list column, and a
//! result opens in the detail column. Packet 010 gives the view three tabs,
//! one row for each item with the state word of each Library item, and the
//! selection model of packets 018 and 019 on the Releases tab and the
//! Tracks tab. Each tab keeps its rows in one ADR 0041 [`PagedListVm`]
//! window.
#![warn(clippy::pedantic)]

use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};
use crate::view_models::detail_selection::{
    detail_header_action, search_header_action, search_release_rows, search_track_rows,
    DetailPageRows, DetailRowClick, DetailRowKey, DetailSelection,
};
use crate::view_models::list_selection::{FocusStep, SelectionRow};
use crate::view_models::live_run::RunGeneration;
use crate::view_models::selection_action_bar::SelectionActionBarVm;
use crate::view_models::window_groups::WindowGroupTileVm;
use crate::view_models::workspace::{FrameHeaderActionDisplay, FrameNavigationEntry};

mod empty_state;
mod failure;
mod index_detail;
mod index_result_note;
mod paged_tab;
mod results;
mod rows;
mod saved_search;
mod saved_search_groups;
mod search_run;
mod tabs;
#[cfg(test)]
mod tests;

pub(crate) use empty_state::EmptyStateDisplay;
pub(crate) use failure::{SearchFailureAction, SearchFailureAvailability};
pub(crate) use index_detail::{IndexDetailDisplay, IndexDetailKind, IndexSearchResultRows};
pub(crate) use index_result_note::{IndexResultDiagnostic, IndexResultStopReason};
pub(crate) use paged_tab::SearchResultsPagedTab;
pub(crate) use results::{
    artist_feed_result_display, index_track_row_id, ArtistResultDisplay, ArtistSearchFacts,
    FeedResultDisplay, SearchResultTarget, TrackResultDisplay, INDEX_FEED_PAGE_RESULT_PREFIX,
};
pub(crate) use rows::{
    ReleaseKindPlacement, SearchLibraryLookup, SearchLibraryRelease, SearchLibraryTrack,
    SearchReleaseWindowFacts, SearchResultRowVm, SearchRowBadge,
};
pub(crate) use saved_search::{
    delete_saved_search_confirmation, location_after_saved_search_delete, save_search_action,
    saved_search_label, saved_search_restore, saved_search_result_is_current,
    saved_search_row_a11y_label, SavedSearch, SavedSearchDeleteConfirmation, SavedSearchId,
    SavedSearchPage, SavedSearchRestore, SearchPageActionDisplay, SearchPageActionStyle,
};
pub(crate) use saved_search_groups::{saved_search_window_page, SearchResultItemVm};
pub(crate) use search_run::{
    library_part_runs, live_run_step, IndexSearchPlan, IndexSearchRequest, IndexSkip,
    LibrarySearchRows, LiveRunStep, SearchRun, INDEX_SKIP_ROWS_TEXT, SCOPE_CONFLICT_TEXT,
    SCOPE_CONFLICT_TITLE,
};
pub(crate) use tabs::{SearchResultItemId, SearchResultOrigin, SearchResultsTab};

use self::empty_state::empty_state_for;
use self::paged_tab::SearchRowMergeKey;
use self::results::local_library_result_rows;
use self::saved_search_groups::{
    saved_search_window_items, window_group_selection_row, SavedSearchWindowItem,
};

/// The count of loading rows while the Index part of a run loads (ADR 0087
/// packet 007 decision D2). The rows show after the rows of the active tab.
pub(crate) const INDEX_PENDING_ROW_COUNT: usize = 3;

/// GPUI-free page contract for the search results in the list column.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one independent fact of the page; ADR 0086 packet 005 names the two partial flags"
)]
pub(crate) struct SearchResultsInspectorPageVm {
    query: String,
    tab: SearchResultsTab,
    releases: SearchResultsPagedTab<FeedResultDisplay>,
    artists: SearchResultsPagedTab<ArtistResultDisplay>,
    tracks: SearchResultsPagedTab<TrackResultDisplay>,
    index_loading: bool,
    index_error: Option<EmptyStateDisplay>,
    /// The cause of an Index part that sends no request (ADR 0087 Decision
    /// 5, packet 002). It is not an Index error.
    index_skip: Option<IndexSkip>,
    /// The note under the rows of a skipped Index part, when the Index
    /// cannot apply a term of the query.
    index_skip_notice: Option<EmptyStateDisplay>,
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
    /// The Index feed rows are not all the feed rows of the query (ADR 0086
    /// packet 005 decision D3).
    feed_rows_partial: bool,
    /// The Library track search reached its limit, so the Library rows are
    /// not all the Library rows of the query (ADR 0086 packet 005 decision
    /// D3).
    library_rows_partial: bool,
    /// The generation of the newest run of the page (ADR 0087 Decision 4,
    /// packet 003). A result of an older run does not change the page.
    run_generation: RunGeneration,
    /// `true` when the page shows the Index rows or the Index error of its
    /// run (packet 007 decision D6).
    index_result_shown: bool,
    /// The Index result of this query that the page did not show, and its
    /// note (packet 007 decision D6).
    index_result_stop: Option<(IndexResultDiagnostic, EmptyStateDisplay)>,
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
            index_skip: None,
            index_skip_notice: None,
            empty_state: None,
            tab_was_user_selected: false,
            entry,
            selection,
            status_text: None,
            feed_rows_partial: false,
            library_rows_partial: false,
            run_generation: RunGeneration::default(),
            index_result_shown: false,
            index_result_stop: None,
        };
        page.refresh_empty_state();
        page
    }

    /// Creates a search-results page from the Library rows of one run
    /// (ADR 0087 packet 002). The Releases tab and the Artists tab read
    /// `rows.releases`. The Tracks tab reads `rows.tracks`. `artist_facts`
    /// gives the match of each artist name (ADR 0084 Decision 7, packet
    /// 017).
    #[must_use]
    pub(crate) fn from_library_search(
        text: impl Into<String>,
        rows: &LibrarySearchRows,
        artist_facts: &ArtistSearchFacts,
    ) -> Self {
        let mut page = Self::new(text);
        let local_rows = local_library_result_rows(&rows.releases, &rows.tracks, artist_facts);
        page.artists = SearchResultsPagedTab::ready_library(local_rows.artists);
        page.releases = SearchResultsPagedTab::ready_library(local_rows.feeds);
        page.tracks = SearchResultsPagedTab::ready_library(local_rows.tracks);
        page.refresh_empty_state();
        page
    }

    /// Returns this page with the state of its Library rows: `partial` is
    /// true when the Library track search reached its limit (ADR 0086
    /// packet 005 decision D3).
    #[must_use]
    pub(crate) fn with_library_rows_partial(mut self, partial: bool) -> Self {
        self.library_rows_partial = partial;
        self
    }

    /// Returns this page with the run generation `generation` (packet 003).
    #[must_use]
    pub(crate) fn with_run_generation(mut self, generation: RunGeneration) -> Self {
        self.run_generation = generation;
        self
    }

    /// The generation of the newest run of the page.
    #[must_use]
    pub(crate) const fn run_generation(&self) -> RunGeneration {
        self.run_generation
    }

    /// `true` when this page can replace `current`: `current` is `None`, or
    /// this generation is equal to or greater than the generation of
    /// `current` (ADR 0087 invariant "A result of an older run never
    /// replaces the result of a newer run").
    #[must_use]
    pub(crate) fn is_newer_than(&self, current: Option<&Self>) -> bool {
        current.is_none_or(|current| self.run_generation >= current.run_generation)
    }

    /// Records that the app did not apply the Index result of the run
    /// `generation` for `query`, because of `reason` (packet 007 decision
    /// D6). The page records the note only when `query` is the page query
    /// and the page shows no Index result. Gives `true` when the page
    /// changed.
    pub(crate) fn record_index_result_stop(
        &mut self,
        query: &str,
        generation: RunGeneration,
        reason: IndexResultStopReason,
    ) -> bool {
        if self.query != query || self.index_result_shown {
            return false;
        }
        let diagnostic = IndexResultDiagnostic {
            query: self.query.clone(),
            run_generation: generation,
            page_generation: self.run_generation,
            reason,
        };
        let display = diagnostic.display();
        self.index_result_stop = Some((diagnostic, display));
        self.refresh_empty_state();
        true
    }

    /// The Index result that the page did not show, or `None` (packet 007
    /// decision D6). Production code reads the note through
    /// [`Self::index_notice`] and [`Self::empty_state`].
    #[cfg(test)]
    #[must_use]
    pub(crate) fn index_result_diagnostic(&self) -> Option<&IndexResultDiagnostic> {
        self.index_result_stop
            .as_ref()
            .map(|(diagnostic, _)| diagnostic)
    }

    /// The count of loading rows after the rows of the active tab (packet
    /// 007 decision D2). It is [`INDEX_PENDING_ROW_COUNT`] while the Index
    /// part loads, else 0. The rows of the active tab do not change it.
    #[must_use]
    pub(crate) const fn index_pending_row_count(&self) -> usize {
        if self.index_loading {
            INDEX_PENDING_ROW_COUNT
        } else {
            0
        }
    }

    /// Starts the Index part of the run `generation`. An older generation
    /// gives `false` and changes nothing. Else the page takes the
    /// generation, marks the Index part as loading and gives `true`.
    pub(crate) fn begin_index_run(&mut self, generation: RunGeneration) -> bool {
        if generation < self.run_generation {
            return false;
        }
        self.run_generation = generation;
        self.mark_index_loading();
        true
    }

    /// Applies the Index rows of the run `generation` for `query`. A
    /// generation other than the page generation gives `false` and changes
    /// nothing. Else the rule of `replace_index_results_for_query` applies.
    pub(crate) fn replace_index_results_for_run(
        &mut self,
        generation: RunGeneration,
        query: &str,
        rows: IndexSearchResultRows,
    ) -> bool {
        if generation != self.run_generation {
            return false;
        }
        self.replace_index_results_for_query(query, rows)
    }

    /// Records the Index error of the run `generation` for `query`. A
    /// generation other than the page generation, or a different query,
    /// gives `false` and changes nothing.
    pub(crate) fn set_index_error_for_run(
        &mut self,
        generation: RunGeneration,
        query: &str,
        error: &crate::application::CommandError,
        endpoint: &str,
        observed_at: std::time::SystemTime,
    ) -> bool {
        if generation != self.run_generation || self.query != query {
            return false;
        }
        self.set_index_error(error, endpoint, observed_at);
        true
    }

    /// Tells if the page holds each release row of the query (ADR 0086
    /// packet 005 "The open state"). The Index request is complete with no
    /// error, the feed response gave no later page, and the Library track
    /// search did not reach its limit. Then no later page can change a
    /// window group, so each group is closed.
    #[must_use]
    pub(crate) fn release_rows_complete(&self) -> bool {
        !self.index_loading
            && self.index_error.is_none()
            && !self.feed_rows_partial
            && !self.library_rows_partial
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
        self.index_result_shown = false;
        self.index_error = None;
        self.index_skip = None;
        self.index_skip_notice = None;
        self.refresh_empty_state();
    }

    /// Records that the Index part sends no request, and its cause (ADR
    /// 0087 Decision 5, packet 002 decision D17). A skip is not an Index
    /// error, so the page shows no loading rows and no error.
    pub(crate) fn set_index_skipped(&mut self, skip: &IndexSkip) {
        self.index_loading = false;
        self.index_error = None;
        self.index_skip_notice = match skip {
            IndexSkip::Unsupported(terms) => Some(EmptyStateDisplay::new(
                terms.note_title(),
                INDEX_SKIP_ROWS_TEXT,
            )),
            IndexSkip::NoIndexPart | IndexSkip::LibraryOnly | IndexSkip::Conflict => None,
        };
        self.index_skip = Some(skip.clone());
        self.refresh_empty_state();
    }

    /// Returns whether the async remote Index search is pending. The
    /// renderer reads [`Self::index_pending_row_count`] (packet 007
    /// decision D2).
    #[cfg(test)]
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
        self.index_result_shown = true;
        self.index_result_stop = None;
        self.artists.replace_index_rows(rows.artists);
        self.releases.replace_index_rows(rows.feeds);
        self.tracks.replace_index_rows(rows.tracks);
        self.feed_rows_partial = rows.feed_rows_partial;
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
        self.index_result_shown = true;
        self.index_result_stop = None;
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
    /// 2026-10-21, packet 028 D5). It shows when the tab has rows, for
    /// example Library rows, and the Index part failed: `MusicIndex` did
    /// not accept the query, or `MusicIndex` was not available. A tab with
    /// no rows shows the same note as its empty state.
    #[must_use]
    ///
    /// A skipped Index part shows the note of ADR 0087 Decision 5 when the
    /// Index cannot apply a term of the query.
    ///
    /// The note of an Index result that the page did not show comes first
    /// (packet 007 decision D6). It also shows after the loading rows of a
    /// tab with no rows. When the page shows it as the empty state, this
    /// function does not give it again.
    pub(crate) fn index_notice(&self) -> Option<&EmptyStateDisplay> {
        if let Some((_, note)) = &self.index_result_stop {
            if self.empty_state.is_none() {
                return Some(note);
            }
        }
        if self.is_empty(self.tab) {
            return None;
        }
        self.index_error
            .as_ref()
            .or(self.index_skip_notice.as_ref())
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

    /// The visible items of the active tab (ADR 0086 Decision 6). The
    /// Releases tab of a saved search page gives one group item for two or
    /// more singles of one artist in one window, at the place of its
    /// anchor. Each other tab and the toolbar search give one row item for
    /// each row. `lookup` gives the Library facts at this render.
    #[must_use]
    pub(crate) fn visible_items(
        &self,
        lookup: &dyn SearchLibraryLookup,
    ) -> Vec<SearchResultItemVm> {
        if !self.groups_release_rows() {
            return self
                .visible_rows(lookup)
                .into_iter()
                .map(SearchResultItemVm::Row)
                .collect();
        }
        let rows = self.releases.rows();
        let placement = self.release_kind_placement();
        saved_search_window_items(&rows, lookup, self.release_rows_complete())
            .into_iter()
            .map(|item| match item {
                SavedSearchWindowItem::Plain(position) => SearchResultItemVm::Row(
                    rows::release_row_vm(&rows[position], lookup, &self.selection, placement),
                ),
                SavedSearchWindowItem::Group { mut tile, .. } => {
                    tile.selection = self
                        .selection
                        .tile_selection(&window_group_selection_row(&tile), &tile.title);
                    SearchResultItemVm::WindowGroup(tile)
                }
            })
            .collect()
    }

    /// The group tile whose anchor row is `row_id` (ADR 0086 packet 005).
    /// A click and Enter on that row open the window group page of the
    /// tile. `None` for each other row.
    #[must_use]
    pub(crate) fn window_group_for_row(
        &self,
        row_id: &str,
        lookup: &dyn SearchLibraryLookup,
    ) -> Option<WindowGroupTileVm> {
        if !self.groups_release_rows() {
            return None;
        }
        saved_search_window_items(&self.releases.rows(), lookup, self.release_rows_complete())
            .into_iter()
            .find_map(|item| match item {
                SavedSearchWindowItem::Group { tile, .. } if tile.open_row_id == row_id => {
                    Some(*tile)
                }
                SavedSearchWindowItem::Group { .. } | SavedSearchWindowItem::Plain(_) => None,
            })
    }

    /// Tells if the active tab groups its rows: only the Releases tab of a
    /// saved search page (ADR 0086 packet 005 decision D1).
    fn groups_release_rows(&self) -> bool {
        self.tab == SearchResultsTab::Releases
            && matches!(self.entry, FrameNavigationEntry::SavedSearch { .. })
    }

    /// The selectable rows of the Releases tab of a saved search page with
    /// one group or more (ADR 0086 packet 004 decision D11). `rows` holds
    /// each shown release and each member of each group, in item order.
    /// `tiles` holds one selection row for each item with a key. `None`
    /// when no group shows.
    fn grouped_release_rows(&self, lookup: &dyn SearchLibraryLookup) -> Option<DetailPageRows> {
        if !self.groups_release_rows() {
            return None;
        }
        let rows = self.releases.rows();
        let items = saved_search_window_items(&rows, lookup, true);
        if !items
            .iter()
            .any(|item| matches!(item, SavedSearchWindowItem::Group { .. }))
        {
            return None;
        }
        let mut ordered = Vec::new();
        let mut tiles = Vec::new();
        for item in items {
            match item {
                SavedSearchWindowItem::Plain(position) => {
                    let row = &rows[position];
                    if let Some(key) = row.merge_key() {
                        tiles.push(SelectionRow::of_one(DetailRowKey::Release(key)));
                    }
                    ordered.push(row.clone());
                }
                SavedSearchWindowItem::Group { tile, members } => {
                    tiles.push(window_group_selection_row(&tile));
                    ordered.extend(members.into_iter().map(|position| rows[position].clone()));
                }
            }
        }
        Some(DetailPageRows::GroupedReleases {
            rows: search_release_rows(&ordered, lookup),
            tiles,
        })
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
        if let Some(grouped) = self.grouped_release_rows(lookup) {
            return Some(grouped);
        }
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

    /// A click on the checkbox of the row `key`. The checkbox of a group
    /// row changes the selection of each member of the group (ADR 0086
    /// packet 004 decision D12).
    pub(crate) fn toggle_row(&mut self, key: &DetailRowKey, lookup: &dyn SearchLibraryLookup) {
        let group = self.grouped_release_rows(lookup).and_then(|rows| {
            rows.selection_rows()
                .into_iter()
                .find(|row| &row.id == key && row.members.len() > 1)
        });
        match group {
            Some(row) => self.selection.toggle_selection_row(&row),
            None => self.selection.toggle_row(key),
        }
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

    /// The index of the item with the keyboard focus among all the items
    /// of the active tab, or `None` when no item has the focus (packet 028
    /// D2, ADR 0086 packet 005). The items are the children of the scroll
    /// container in this order, thus the app layer scrolls to this index
    /// after a focus move.
    #[must_use]
    pub(crate) fn focused_row_index(&self, lookup: &dyn SearchLibraryLookup) -> Option<usize> {
        let focused = self.selection.focused()?;
        if self.groups_release_rows() {
            return self
                .visible_items(lookup)
                .iter()
                .position(|item| item.key().as_ref() == Some(focused));
        }
        match self.tab {
            SearchResultsTab::Releases => self.releases.rows().iter().position(|row| {
                row.merge_key()
                    .is_some_and(|key| DetailRowKey::Release(key) == *focused)
            }),
            SearchResultsTab::Tracks => self.tracks.rows().iter().position(|row| {
                row.merge_key()
                    .is_some_and(|key| DetailRowKey::Track(key) == *focused)
            }),
            SearchResultsTab::Artists => None,
        }
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
        if let Some(error) = &self.index_error {
            self.empty_state = Some(error.clone());
            return;
        }
        // Packet 007 decision D6: a page with no rows tells why it shows
        // no Index rows.
        if let Some((_, note)) = &self.index_result_stop {
            self.empty_state = Some(note.clone());
            return;
        }
        let empty = empty_state_for(self.tab, &self.query);
        self.empty_state = Some(match &self.index_skip {
            Some(IndexSkip::Unsupported(terms)) => {
                EmptyStateDisplay::new(terms.note_title(), empty.secondary)
            }
            Some(IndexSkip::Conflict) => {
                EmptyStateDisplay::new(SCOPE_CONFLICT_TITLE, SCOPE_CONFLICT_TEXT)
            }
            Some(IndexSkip::NoIndexPart | IndexSkip::LibraryOnly) | None => empty,
        });
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
    /// Creates a search-results page whose three tabs read the same Library
    /// tracks.
    #[must_use]
    pub(crate) fn from_local_library_tracks(
        query: impl Into<String>,
        tracks: &[crate::db::TrackRow],
        artist_facts: &ArtistSearchFacts,
    ) -> Self {
        let rows = LibrarySearchRows {
            releases: tracks.to_vec(),
            tracks: tracks.to_vec(),
            matched_count: tracks.len(),
        };
        Self::from_library_search(query, &rows, artist_facts)
    }

    /// The cause of a skipped Index part, or `None`.
    #[must_use]
    pub(crate) fn index_skip(&self) -> Option<&IndexSkip> {
        self.index_skip.as_ref()
    }

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
