//! Search screen view-model projections.
//!
//! These projections keep Discover/Search result display contracts out of
//! renderers, while remaining GPUI-free. The screen owns event wiring,
//! thumbnails, focus, and selection; this module owns the text and image
//! fields that result and inspector rows need to render.

#![warn(clippy::pedantic)]

mod actions;
mod common;
mod controls;
mod feed_detail;
mod lazy;
mod recent;
mod results;
mod track;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::fmt::Write as _;

use crate::api::{self, Artist, EntityDetail, Feed, Track};
use crate::application::library_removal::{LibraryRemovalPlan, LibraryRemovalTarget};
use crate::db;
use crate::view_models::library_removal::{
    LibraryRemovalConfirmationDisplay, LibraryRemovalConfirmationState,
};
use crate::view_models::workspace::ContentFilter;
use crate::view_models::SplitPaneState;

#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "root re-export preserves the view_models::search import surface after decomposition"
    )
)]
pub(crate) use actions::{
    ActionRowVm, PlaylistAppendIntent, PlaylistAppendOutcome, SearchInspectorPlaylistDisplay,
    SearchSubscriptionCommand,
};
pub(crate) use controls::{
    IndexControlsVisibility, InspectorChromeDisplay, SearchPaneDisplay, SearchRenderSnapshot,
    SearchResultSection, SearchResultSectionDisplay, SearchStatusSnapshot,
    SearchTypeFilterOptionDisplay, TYPE_FILTER_LEN, TYPE_FILTER_OPTIONS,
};
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "root re-export preserves the view_models::search import surface after decomposition"
    )
)]
pub(crate) use feed_detail::{PaymentRouteGroupDisplay, PaymentRouteVm, PublisherInspectorVm};
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "root re-export preserves the view_models::search import surface after decomposition"
    )
)]
pub(crate) use lazy::{DeferredPanelDisplay, DeferredPanelKind, LazyPanel, LazyPanelToggle};
pub(crate) use recent::{
    PodrollSectionDisplay, RecentFeedTileVm, RecentFeedsDisplay, RecentFeedsSnapshot,
    SearchFeedListSectionDisplay,
};
pub use recent::{RecentFeedTileDisplay, RecentFeedTileOpenTarget};
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "root re-export preserves the view_models::search import surface after decomposition"
    )
)]
pub(crate) use results::{
    artist_rows_from_result_rows, feed_display_title, normalized_search_query,
    search_result_type_is_visible, ResultRow, ResultRowDisplay, ResultRowRenderItem, ResultRowVm,
    SearchLibraryMembership, SearchLibraryMembershipDisplay, SearchResultSource,
};
use results::{entity_key, source_entity_key};
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "root re-export preserves the view_models::search import surface after decomposition"
    )
)]
pub(crate) use track::{
    TrackFeedLinkDisplay, TrackInspectorHeaderVm, TrackRowActionVm, TrackRowDownloadDisplay,
};

const DEFAULT_SPLIT_PANE_WIDTH: f32 = 360.0;

/// Source of the currently-pushed inspector frame. Used by the screen
/// to colour the back-button affordance and to decide which list the
/// "Back to results" target maps to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorOrigin {
    Recents,
    Search,
}

/// Stateful screen view-model for the Discover (search) tab.
///
/// Mirror of `view_models::library::LibraryViewModel`. Owns the
/// pure-data UI state for the search screen so `SearchApp` shrinks to
/// event wiring and `Render` glue. Per ADR 0023 this struct must
/// remain GPUI-free; fields requiring `gpui::Image`,
/// `gpui::FocusHandle`, or `Entity<_>` stay in `SearchApp`.
///
/// Subsequent commits will move snapshots (`results`,
/// `recent_feeds`, `playlists`).
#[expect(
    clippy::struct_excessive_bools,
    reason = "screen UI flags belong together; splitting them by lint count would obscure the model"
)]
#[derive(Clone, Debug)]
pub(crate) struct SearchViewModel {
    // Search filter / toggle.
    /// Current segmented filter index (`All`, `Artist`, `Feed`,
    /// `Track`). The screen owns the label/value tables; the VM owns
    /// the index and clamps it to the table length on `set_type_filter`.
    pub(crate) type_filter: usize,
    /// Whether the fuzzy-search toggle is on.
    pub(crate) fuzzy_search: bool,
    // Selection / inspector origin.
    /// Selection key — `"<entity_type>:<entity_id>"`. The screen
    /// resolves the key back to an `InspectorFrame` from its loaded
    /// rows.
    pub(crate) selected_key: Option<String>,
    /// Origin of the active inspector frame, if any.
    pub(crate) inspector_origin: Option<InspectorOrigin>,
    // Search-results pane state.
    pub(crate) loading: bool,
    pub(crate) status: String,
    pub(crate) active_query: Option<String>,
    pub(crate) active_filter: ContentFilter,
    pub(crate) cursor: Option<String>,
    pub(crate) has_more: bool,
    in_flight_tracks: HashSet<String>,
    library_removal: LibraryRemovalConfirmationState,
    pending_library_removal_origin: Option<SearchRemovalOrigin>,
    // Recents pane state.
    pub(crate) recent_loading: bool,
    pub(crate) recent_status: String,
    pub(crate) recent_loaded_once: bool,
    pub(crate) recent_cursor: Option<String>,
    pub(crate) recent_has_more: bool,
    // Layout / drag state.
    split_pane: SplitPaneState,
    // Loaded snapshots — owned here so the screen can become a thin
    // Render impl. None of these carry GPUI types.
    pub(crate) results: Vec<ResultRow>,
    pub(crate) library_results: Vec<ResultRow>,
    pub(crate) recent_feeds: Vec<Feed>,
    pub(crate) playlists: Vec<db::Playlist>,
}

/// Pure command intent for fetching the next recent-feed page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecentFeedLoadIntent {
    cursor: Option<String>,
}

impl RecentFeedLoadIntent {
    #[must_use]
    pub(crate) fn into_cursor(self) -> Option<String> {
        self.cursor
    }
}

/// Pure command intent for fetching a Discover/Search result page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchLoadIntent {
    type_filter: usize,
    cursor: Option<String>,
    fuzzy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SearchRemovalOrigin {
    Row {
        key: String,
    },
    Inspector {
        entity_type: String,
        entity_id: String,
        command: SearchSubscriptionCommand,
    },
}

impl SearchLoadIntent {
    #[must_use]
    pub(crate) fn type_filter(&self) -> usize {
        self.type_filter
    }

    #[must_use]
    pub(crate) fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    #[must_use]
    pub(crate) fn fuzzy(&self) -> bool {
        self.fuzzy
    }
}

/// One fetched Discover/Search result page.
#[derive(Clone, Debug)]
pub(crate) struct SearchBatch {
    pub(crate) rows: Vec<ResultRow>,
    pub(crate) has_more: bool,
    pub(crate) cursor: Option<String>,
}

/// Result-row identity needed by the screen to open the inspector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResultNavigationTarget {
    source: SearchResultSource,
    entity_type: String,
    entity_id: String,
    feed_guid: Option<String>,
    title: String,
}

impl ResultNavigationTarget {
    #[must_use]
    fn from_row(row: &ResultRow) -> Self {
        Self {
            source: row.source,
            entity_type: row.entity_type.clone(),
            entity_id: row.entity_id.clone(),
            feed_guid: row.feed_guid.clone(),
            title: row.inspector_title(),
        }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (SearchResultSource, String, String, Option<String>, String) {
        (
            self.source,
            self.entity_type,
            self.entity_id,
            self.feed_guid,
            self.title,
        )
    }
}

impl SearchViewModel {
    /// Construct a view-model with `SearchApp::new` defaults: `All`
    /// filter, fuzzy search on, no selection, no inspector frame, no
    /// active operation, both panes idle.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            type_filter: 0,
            fuzzy_search: true,
            selected_key: None,
            inspector_origin: None,
            loading: false,
            status: String::new(),
            active_query: None,
            active_filter: ContentFilter::Index,
            cursor: None,
            has_more: false,
            in_flight_tracks: HashSet::new(),
            library_removal: LibraryRemovalConfirmationState::new(),
            pending_library_removal_origin: None,
            recent_loading: false,
            recent_status: String::new(),
            recent_loaded_once: false,
            recent_cursor: None,
            recent_has_more: false,
            split_pane: SplitPaneState::new(DEFAULT_SPLIT_PANE_WIDTH),
            results: Vec::new(),
            library_results: Vec::new(),
            recent_feeds: Vec::new(),
            playlists: Vec::new(),
        }
    }

    /// Set the segmented filter index. Out-of-range values are
    /// silently ignored — the segmented control owns the range.
    pub(crate) fn set_type_filter(&mut self, index: usize) {
        if index < TYPE_FILTER_LEN {
            self.type_filter = index;
        }
    }

    pub(crate) fn set_type_filter_if_changed(&mut self, index: usize) -> bool {
        let before = self.type_filter;
        self.set_type_filter(index);
        self.type_filter != before
    }

    /// Flip the fuzzy-search toggle.
    pub(crate) fn toggle_fuzzy_search(&mut self) {
        self.fuzzy_search = !self.fuzzy_search;
    }

    /// Set the `inspector_origin` to `Search` — call when pushing a
    /// frame from the main results list.
    pub(crate) fn mark_inspector_from_search(&mut self) {
        self.inspector_origin = Some(InspectorOrigin::Search);
    }

    /// Set the `inspector_origin` to `Recents` — call when pushing a
    /// frame from the "Recent feeds" panel.
    pub(crate) fn mark_inspector_from_recents(&mut self) {
        self.inspector_origin = Some(InspectorOrigin::Recents);
    }

    /// Drop the inspector-origin marker — call when popping the last
    /// frame off the inspector stack.
    pub(crate) fn clear_inspector_origin(&mut self) {
        self.inspector_origin = None;
    }

    /// Replace the selection key.
    pub(crate) fn select(&mut self, key: impl Into<String>) {
        self.selected_key = Some(key.into());
    }

    pub(crate) fn select_result_from_source(
        &mut self,
        source: SearchResultSource,
        entity_type: &str,
        entity_id: &str,
        feed_guid: Option<&str>,
    ) {
        self.select(source_entity_key(source, entity_type, entity_id, feed_guid));
        self.mark_inspector_from_search();
    }

    #[cfg(test)]
    pub(crate) fn select_result(&mut self, entity_type: &str, entity_id: &str) {
        self.select(entity_key(entity_type, entity_id));
        self.mark_inspector_from_search();
    }

    pub(crate) fn select_recent_feed(&mut self, feed_guid: &str) {
        self.select(entity_key("feed", feed_guid));
        self.mark_inspector_from_recents();
    }

    /// Clear the selection.
    pub(crate) fn clear_selection(&mut self) {
        self.selected_key = None;
    }

    #[must_use]
    pub(crate) fn previous_result_target(&self) -> Option<ResultNavigationTarget> {
        let rows = self.navigation_rows();
        if rows.is_empty() {
            return None;
        }
        let next_idx = match self.selected_result_index() {
            Some(idx) if idx > 0 => idx - 1,
            _ => 0,
        };
        rows.get(next_idx).map(ResultNavigationTarget::from_row)
    }

    #[must_use]
    pub(crate) fn next_result_target(&self) -> Option<ResultNavigationTarget> {
        let rows = self.navigation_rows();
        if rows.is_empty() {
            return None;
        }
        let next_idx = match self.selected_result_index() {
            Some(idx) if idx + 1 < rows.len() => idx + 1,
            Some(idx) => idx,
            None => 0,
        };
        rows.get(next_idx).map(ResultNavigationTarget::from_row)
    }

    fn selected_result_index(&self) -> Option<usize> {
        let current_key = self.selected_key.as_deref()?;
        self.navigation_rows()
            .iter()
            .position(|row| row.key() == current_key)
    }

    fn navigation_rows(&self) -> Vec<ResultRow> {
        self.result_sections()
            .into_iter()
            .flat_map(|section| section.rows)
            .collect()
    }

    #[must_use]
    pub(crate) fn result_sections(&self) -> Vec<SearchResultSection> {
        let mut sections = Vec::new();
        match self.active_filter {
            ContentFilter::All => {
                sections.push(SearchResultSection {
                    display: SearchResultSectionDisplay {
                        id: "search-section-library",
                        heading: "Library",
                        empty_label: "No Library results",
                    },
                    rows: self.filtered_result_rows(&self.library_results),
                    show_empty: true,
                });
                sections.push(SearchResultSection {
                    display: SearchResultSectionDisplay {
                        id: "search-section-index",
                        heading: "Index",
                        empty_label: "No Index results",
                    },
                    rows: self.filtered_result_rows(&self.results),
                    show_empty: true,
                });
            }
            ContentFilter::Library => {
                sections.push(SearchResultSection {
                    display: SearchResultSectionDisplay {
                        id: "search-section-library",
                        heading: "Library",
                        empty_label: "No Library results",
                    },
                    rows: self.filtered_result_rows(&self.library_results),
                    show_empty: false,
                });
            }
            ContentFilter::Index => {
                sections.push(SearchResultSection {
                    display: SearchResultSectionDisplay {
                        id: "search-section-index",
                        heading: "Index",
                        empty_label: "No Index results",
                    },
                    rows: self.filtered_result_rows(&self.results),
                    show_empty: false,
                });
            }
        }
        sections
    }

    fn filtered_result_rows(&self, rows: &[ResultRow]) -> Vec<ResultRow> {
        let type_filter = Self::type_filter_value(self.type_filter);
        rows.iter()
            .filter(|row| type_filter.is_none_or(|kind| row.entity_type == kind))
            .cloned()
            .collect()
    }

    #[must_use]
    pub(crate) fn render_snapshot(
        &self,
        inspector_stack_empty: bool,
        input_is_empty: bool,
    ) -> SearchRenderSnapshot {
        let sections = self.result_sections();
        let empty = sections.iter().all(|section| section.rows.is_empty());
        let show_recents_root =
            inspector_stack_empty && self.inspector_origin.is_none() && empty && input_is_empty;
        let show_recents_command = !show_recents_root;
        SearchRenderSnapshot {
            status: SearchStatusSnapshot::from_text(&self.status),
            pane_display: SearchPaneDisplay::new(self.fuzzy_search),
            sections,
            selected_key: self.selected_key.clone(),
            type_filter: self.type_filter,
            index_controls: if self.active_filter == ContentFilter::Library {
                IndexControlsVisibility::Hidden
            } else {
                IndexControlsVisibility::Visible
            },
            show_recents_root,
            show_recents_command,
            loading: self.loading,
            empty,
            has_more: self.has_more,
            fuzzy_search: self.fuzzy_search,
        }
    }

    #[must_use]
    pub(crate) fn recent_feeds_snapshot(&self) -> RecentFeedsSnapshot {
        RecentFeedsSnapshot {
            display: RecentFeedsDisplay::VALUE,
            feeds: self.recent_feeds.clone(),
            status: self.recent_status.clone(),
            has_more: self.recent_has_more,
            loading: self.recent_loading,
            empty: self.recent_feeds.is_empty(),
        }
    }

    #[must_use]
    pub(crate) const fn recents_root_title() -> &'static str {
        RecentFeedsDisplay::VALUE.heading
    }

    #[must_use]
    pub(crate) fn inspector_title_display(
        show_recents_root: bool,
        frame_title: Option<&str>,
    ) -> String {
        match (show_recents_root, frame_title) {
            (true, None) => Self::recents_root_title().to_string(),
            (_, Some(title)) => title.to_string(),
            _ => String::new(),
        }
    }

    #[must_use]
    pub(crate) const fn inspector_chrome_display() -> InspectorChromeDisplay {
        InspectorChromeDisplay::VALUE
    }

    #[must_use]
    pub(crate) fn inspector_loading_message(title: &str) -> String {
        format!("Loading {title}...")
    }

    #[must_use]
    pub(crate) fn inspector_error_message(error: impl std::fmt::Display) -> String {
        format!("Error: {error}")
    }

    #[must_use]
    pub(crate) fn podroll_section_display(entity_id: &str) -> PodrollSectionDisplay {
        PodrollSectionDisplay::new(entity_id)
    }

    #[must_use]
    pub(crate) const fn deferred_panel_display(kind: DeferredPanelKind) -> DeferredPanelDisplay {
        DeferredPanelDisplay::for_kind(kind)
    }

    #[must_use]
    pub(crate) fn deferred_panel_empty_line(label: &str) -> String {
        label.to_string()
    }

    #[must_use]
    pub(crate) fn feed_inspector_tracks(feed: &Feed) -> Vec<Track> {
        feed_detail::feed_inspector_tracks(feed)
    }

    #[must_use]
    pub(crate) const fn type_filter_options() -> &'static [SearchTypeFilterOptionDisplay] {
        &TYPE_FILTER_OPTIONS
    }

    #[must_use]
    pub(crate) fn type_filter_value(index: usize) -> Option<&'static str> {
        TYPE_FILTER_OPTIONS
            .get(index)
            .and_then(|option| option.value)
    }

    #[must_use]
    pub(crate) const fn feed_list_section_display() -> SearchFeedListSectionDisplay {
        SearchFeedListSectionDisplay { heading: "Feeds" }
    }

    /// Reset pure search state after the `MusicIndex` endpoint changes.
    pub(crate) fn reset_for_endpoint_change(&mut self) {
        self.results.clear();
        self.library_results.clear();
        self.loading = false;
        self.status = "MusicIndex endpoint updated".into();
        self.active_query = None;
        self.active_filter = ContentFilter::Index;
        self.cursor = None;
        self.has_more = false;
        self.clear_selection();
        self.clear_inspector_origin();
        self.library_removal.cancel();
        self.pending_library_removal_origin = None;
        self.recent_feeds.clear();
        self.recent_cursor = None;
        self.recent_has_more = false;
        self.recent_loaded_once = false;
        self.recent_status.clear();
    }

    /// Return Discover to its recent-feeds root presentation.
    ///
    /// The screen clears the GPUI input before calling this. The VM owns the
    /// pure pane state transition so the recents affordance cannot drift into
    /// renderer-only behavior.
    pub(crate) fn return_to_recent_feeds(&mut self) -> bool {
        self.loading = false;
        self.status.clear();
        self.active_query = None;
        self.active_filter = ContentFilter::Index;
        self.cursor = None;
        self.has_more = false;
        self.results.clear();
        self.library_results.clear();
        self.clear_selection();
        self.clear_inspector_origin();
        !self.recent_loaded_once && !self.recent_loading
    }

    #[must_use]
    pub(crate) fn begin_recent_feed_load(&mut self, append: bool) -> Option<RecentFeedLoadIntent> {
        if self.recent_loading {
            return None;
        }
        self.recent_loading = true;
        if !append {
            self.recent_feeds.clear();
            self.recent_cursor = None;
            self.recent_has_more = false;
        }
        self.recent_status = if append {
            "Loading more recent feeds...".into()
        } else {
            "Loading recent feeds...".into()
        };
        Some(RecentFeedLoadIntent {
            cursor: if append {
                self.recent_cursor.clone()
            } else {
                None
            },
        })
    }

    pub(crate) fn finish_recent_feed_load(&mut self, response: api::RecentFeedsResponse) {
        self.recent_loading = false;
        self.recent_loaded_once = true;
        self.recent_feeds.extend(response.data);
        self.recent_cursor = response.pagination.cursor;
        self.recent_has_more = response.pagination.has_more;
        self.recent_status.clear();
    }

    pub(crate) fn fail_recent_feed_load(&mut self, error: impl std::fmt::Display) {
        self.recent_loading = false;
        self.recent_loaded_once = true;
        self.recent_status = format!("Error: {error}");
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn begin_search_load(&mut self, append: bool) -> Option<SearchLoadIntent> {
        let query = self.active_query.clone().unwrap_or_default();
        self.begin_global_search_load(query, ContentFilter::Index, append)
    }

    pub(crate) fn begin_global_search_load(
        &mut self,
        query: impl Into<String>,
        filter: ContentFilter,
        append: bool,
    ) -> Option<SearchLoadIntent> {
        if self.loading {
            return None;
        }
        self.loading = true;
        self.status = if append {
            "Loading more...".into()
        } else if filter == ContentFilter::Library {
            "Searching Library...".into()
        } else {
            "Searching...".into()
        };
        self.active_filter = filter;

        if !append {
            self.active_query = Some(query.into());
            self.results.clear();
            self.library_results.clear();
            self.cursor = None;
            self.has_more = false;
            self.clear_selection();
            self.clear_inspector_origin();
        }

        Some(SearchLoadIntent {
            type_filter: self.type_filter,
            cursor: if append { self.cursor.clone() } else { None },
            fuzzy: self.fuzzy_search,
        })
    }

    #[cfg(test)]
    pub(crate) fn finish_search_load(&mut self, batch: SearchBatch, append: bool) {
        self.finish_global_search_load(Vec::new(), Some(batch), ContentFilter::Index, append);
    }

    pub(crate) fn finish_global_search_load(
        &mut self,
        library_rows: Vec<ResultRow>,
        index_batch: Option<SearchBatch>,
        filter: ContentFilter,
        append: bool,
    ) {
        if !append {
            self.library_results = if matches!(filter, ContentFilter::All | ContentFilter::Library)
            {
                library_rows
            } else {
                Vec::new()
            };
        }

        let batch = index_batch.unwrap_or(SearchBatch {
            rows: Vec::new(),
            has_more: false,
            cursor: None,
        });

        if !append && batch.rows.is_empty() {
            self.results.clear();
            self.loading = false;
            self.has_more = false;
            self.cursor = None;
            self.update_search_status();
            return;
        }

        if append {
            let mut seen: HashSet<(String, String)> = self
                .results
                .iter()
                .map(|row| (row.entity_type.clone(), row.entity_id.clone()))
                .collect();
            for row in batch.rows {
                let key = (row.entity_type.clone(), row.entity_id.clone());
                if seen.insert(key) {
                    self.results.push(row);
                }
            }
        } else {
            self.results.extend(batch.rows);
        }
        self.cursor = batch.cursor;
        self.has_more = batch.has_more;
        self.loading = false;

        self.update_search_status();
    }

    fn update_search_status(&mut self) {
        let total = self.filtered_result_rows(&self.results).len()
            + self.filtered_result_rows(&self.library_results).len();
        if total == 0 {
            self.status.clear();
        } else {
            self.status = format!(
                "{total} result{}{}",
                if total == 1 { "" } else { "s" },
                if self.has_more { "+" } else { "" }
            );
        }
    }

    pub(crate) fn fail_search_load(&mut self, error: impl std::fmt::Display) {
        self.loading = false;
        self.status = format!("Error: {error}");
    }

    pub(crate) fn merge_artist_result_detail(&mut self, entity_id: &str, detail: &Artist) {
        for row in &mut self.results {
            if row.entity_type == "artist" && row.entity_id == entity_id {
                let Some(EntityDetail::Artist(artist)) = row.detail.as_mut() else {
                    continue;
                };
                artist.track_count = detail.track_count;
                artist.feed_count = detail.feed_count;
                if detail.image_url.is_some() {
                    artist.image_url.clone_from(&detail.image_url);
                }
            }
        }
    }

    pub(crate) fn replace_playlists(&mut self, playlists: Vec<db::Playlist>) {
        self.playlists = playlists;
    }

    #[must_use]
    pub(crate) fn playlists_snapshot(&self) -> Vec<db::Playlist> {
        self.playlists.clone()
    }

    pub(crate) fn fail_playlist_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading playlists: {error:#}");
    }

    pub(crate) fn fail_feed_subscription(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error subscribing feed: {error:#}");
    }

    pub(crate) fn fail_feed_tracks_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading feed tracks: {error:#}");
    }

    pub(crate) fn set_feed_has_no_tracks(&mut self) {
        self.status = "Feed has no tracks".into();
    }

    pub(crate) fn fail_playlist_create(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Create playlist: {error:#}");
    }

    pub(crate) fn set_track_not_in_library(&mut self) {
        self.status = "Track not in local library".into();
    }

    #[must_use]
    pub(crate) fn begin_playlist_append(
        &mut self,
        playlist_id: i64,
        track_ids: Vec<i64>,
    ) -> Option<PlaylistAppendIntent> {
        if track_ids.is_empty() {
            return None;
        }
        let playlist_name = self
            .playlists
            .iter()
            .find(|playlist| playlist.id == playlist_id)
            .map(|playlist| playlist.name.clone())
            .unwrap_or_default();
        self.status = format!(
            "Downloading {} track{}...",
            track_ids.len(),
            if track_ids.len() == 1 { "" } else { "s" }
        );
        Some(PlaylistAppendIntent {
            playlist_id,
            track_ids,
            playlist_name,
        })
    }

    pub(crate) fn finish_playlist_append(
        &mut self,
        intent: &PlaylistAppendIntent,
        outcome: PlaylistAppendOutcome,
    ) {
        let mut message = format!(
            "Added {} of {} to {}",
            outcome.appended,
            intent.total_tracks(),
            intent.playlist_name()
        );
        if outcome.downloaded > 0 {
            write!(&mut message, " (downloaded {})", outcome.downloaded)
                .expect("writing to a String cannot fail");
        }
        if outcome.failed > 0 {
            write!(&mut message, "; {} failed", outcome.failed)
                .expect("writing to a String cannot fail");
        }
        self.status = message;
    }

    pub(crate) fn fail_playlist_append(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error adding to playlist: {error:#}");
    }

    #[must_use]
    pub(crate) fn confirm_library_removal_from(
        &mut self,
        plan: LibraryRemovalPlan,
        origin: SearchRemovalOrigin,
    ) -> bool {
        if self.library_removal.confirm_or_defer(plan) {
            self.pending_library_removal_origin = None;
            true
        } else {
            self.pending_library_removal_origin = Some(origin);
            false
        }
    }

    #[must_use]
    pub(crate) fn pending_library_removal_confirmation(
        &self,
    ) -> Option<LibraryRemovalConfirmationDisplay> {
        self.library_removal.pending_display()
    }

    pub(crate) fn cancel_pending_library_removal(&mut self) {
        self.library_removal.cancel();
        self.pending_library_removal_origin = None;
    }

    pub(crate) fn take_pending_library_removal(
        &mut self,
    ) -> Option<(LibraryRemovalTarget, SearchRemovalOrigin)> {
        let target = self.library_removal.take_pending_target()?;
        let origin = self.pending_library_removal_origin.take()?;
        Some((target, origin))
    }

    #[must_use]
    pub(crate) fn begin_track_operation(&mut self, key: impl Into<String>) -> bool {
        let key = key.into();
        if key.is_empty() {
            return false;
        }
        self.in_flight_tracks.insert(key)
    }

    #[must_use]
    pub(crate) fn is_track_operation_in_flight(&self, key: &str) -> bool {
        !key.is_empty() && self.in_flight_tracks.contains(key)
    }

    pub(crate) fn finish_track_download(&mut self, key: &str, message: impl Into<String>) {
        self.in_flight_tracks.remove(key);
        self.status = message.into();
    }

    pub(crate) fn fail_track_download(&mut self, key: &str, error: impl std::fmt::Display) {
        self.in_flight_tracks.remove(key);
        self.status = format!("Download error: {error:#}");
    }

    pub(crate) fn finish_track_remove(&mut self, key: &str, message: impl Into<String>) {
        self.in_flight_tracks.remove(key);
        self.status = message.into();
    }

    pub(crate) fn fail_track_remove(&mut self, key: &str, error: impl std::fmt::Display) {
        self.in_flight_tracks.remove(key);
        self.status = format!("Remove error: {error:#}");
    }

    #[must_use]
    pub(crate) fn is_resizing(&self) -> bool {
        self.split_pane.is_resizing()
    }

    #[must_use]
    pub(crate) fn split_pane_width(&self) -> f32 {
        self.split_pane.leading_width()
    }

    pub(crate) fn begin_resize(&mut self) {
        self.split_pane.begin_resize();
    }

    pub(crate) fn end_resize(&mut self) {
        self.split_pane.end_resize();
    }

    pub(crate) fn resize_split_pane(
        &mut self,
        requested_width: f32,
        min_width: f32,
        max_width: f32,
    ) {
        self.split_pane
            .resize_to(requested_width, min_width, max_width);
    }
}
