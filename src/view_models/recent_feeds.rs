//! Recent Feeds page view-model.
//!
//! ADR 0048 keeps visible discovery surfaces inside the workspace
//! `ContentList` frame. This module owns the GPUI-free state for the
//! first-class Recent Feeds route while reusing Index feed result rows and
//! detail projection types.

#![warn(clippy::pedantic)]

use crate::view_models::search_results::{
    FeedResultDisplay, IndexDetailDisplay, SearchResultItemId,
};
use crate::view_models::workspace::ContentFilter;

/// Display-ready row for one Recent Feeds item.
pub(crate) type RecentFeedResultRow = (SearchResultItemId, FeedResultDisplay);

/// One page of Recent Feeds results plus server pagination state.
#[derive(Clone, Debug)]
pub(crate) struct RecentFeedsPageBatch {
    /// Rows returned for this page.
    pub(crate) rows: Vec<RecentFeedResultRow>,
    /// Cursor to request the next page, when present.
    pub(crate) cursor: Option<String>,
    /// Whether the Index reports more pages.
    pub(crate) has_more: bool,
}

/// Request intent emitted by the Recent Feeds VM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecentFeedsLoadIntent {
    cursor: Option<String>,
}

impl RecentFeedsLoadIntent {
    /// Consumes the intent and returns the cursor for the request.
    #[must_use]
    pub(crate) fn into_cursor(self) -> Option<String> {
        self.cursor
    }
}

/// Presentation mode for the Recent Feeds route.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum RecentFeedsViewMode {
    /// Tiled artwork-first browser.
    #[default]
    Tiles,
    /// Compact row list.
    List,
}

impl RecentFeedsViewMode {
    /// Returns the visible segment label.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Tiles => "Tiles",
            Self::List => "List",
        }
    }

    /// Returns the stable segment id suffix.
    #[must_use]
    pub(crate) const fn id_suffix(self) -> &'static str {
        match self {
            Self::Tiles => "tiles",
            Self::List => "list",
        }
    }

    /// Returns the accessibility label for the segment.
    #[must_use]
    pub(crate) const fn a11y_label(self) -> &'static str {
        match self {
            Self::Tiles => "Show Recent Feeds as tiles",
            Self::List => "Show Recent Feeds as a list",
        }
    }
}

/// Current load state for the Recent Feeds route.
#[derive(Clone, Debug)]
pub(crate) enum RecentFeedsPageState {
    /// A request is in flight.
    Loading,
    /// Recent feed rows loaded from the remote Index.
    Loaded(Vec<RecentFeedResultRow>),
    /// The remote Index returned an error.
    Error { message: String, detail: String },
}

/// GPUI-free page contract for Recent Feeds.
#[derive(Clone, Debug)]
pub(crate) struct RecentFeedsPageVm {
    state: RecentFeedsPageState,
    view_mode: RecentFeedsViewMode,
    cursor: Option<String>,
    has_more: bool,
    loading: bool,
}

impl RecentFeedsPageVm {
    /// Creates a loading Recent Feeds page.
    #[must_use]
    pub(crate) const fn loading() -> Self {
        Self {
            state: RecentFeedsPageState::Loading,
            view_mode: RecentFeedsViewMode::Tiles,
            cursor: None,
            has_more: false,
            loading: false,
        }
    }

    /// Returns this page with a specific presentation mode.
    #[must_use]
    pub(crate) const fn with_view_mode(mut self, view_mode: RecentFeedsViewMode) -> Self {
        self.view_mode = view_mode;
        self
    }

    /// Begins a fresh or append Recent Feeds load.
    #[must_use]
    pub(crate) fn begin_load(&mut self, append: bool) -> Option<RecentFeedsLoadIntent> {
        if self.loading || append && !self.has_more {
            return None;
        }

        self.loading = true;
        let cursor = if append { self.cursor.clone() } else { None };
        if !append {
            self.state = RecentFeedsPageState::Loading;
            self.cursor = None;
            self.has_more = false;
        }

        Some(RecentFeedsLoadIntent { cursor })
    }

    /// Finishes a fresh or append Recent Feeds load.
    pub(crate) fn finish_load(&mut self, batch: RecentFeedsPageBatch, append: bool) {
        self.loading = false;
        self.cursor = batch.cursor;
        self.has_more = batch.has_more;

        if append {
            match &mut self.state {
                RecentFeedsPageState::Loaded(rows) => rows.extend(batch.rows),
                RecentFeedsPageState::Loading | RecentFeedsPageState::Error { .. } => {
                    self.state = RecentFeedsPageState::Loaded(batch.rows);
                }
            }
        } else {
            self.state = RecentFeedsPageState::Loaded(batch.rows);
        }
    }

    /// Replaces the page with loaded recent feed rows.
    #[cfg(test)]
    pub(crate) fn replace_feeds(&mut self, rows: Vec<RecentFeedResultRow>) {
        self.finish_load(
            RecentFeedsPageBatch {
                rows,
                cursor: None,
                has_more: false,
            },
            false,
        );
    }

    /// Marks the page as failed.
    pub(crate) fn fail_load(
        &mut self,
        message: impl Into<String>,
        detail: impl Into<String>,
        append: bool,
    ) {
        self.loading = false;
        if append && matches!(self.state, RecentFeedsPageState::Loaded(_)) {
            return;
        }

        self.state = RecentFeedsPageState::Error {
            message: message.into(),
            detail: detail.into(),
        };
    }

    /// Returns whether a request is currently in flight.
    #[must_use]
    pub(crate) const fn is_loading(&self) -> bool {
        self.loading
    }

    /// Returns whether the server has another Recent Feeds page.
    #[must_use]
    pub(crate) const fn has_more(&self) -> bool {
        self.has_more
    }

    /// Returns the number of currently loaded rows.
    #[must_use]
    pub(crate) fn row_count(&self) -> usize {
        match &self.state {
            RecentFeedsPageState::Loaded(rows) => rows.len(),
            RecentFeedsPageState::Loading | RecentFeedsPageState::Error { .. } => 0,
        }
    }

    /// Returns the current page state.
    #[must_use]
    pub(crate) const fn state(&self) -> &RecentFeedsPageState {
        &self.state
    }

    /// Returns the selected presentation mode.
    #[must_use]
    pub(crate) const fn view_mode(&self) -> RecentFeedsViewMode {
        self.view_mode
    }

    /// Sets the selected presentation mode.
    pub(crate) fn set_view_mode(&mut self, view_mode: RecentFeedsViewMode) {
        self.view_mode = view_mode;
    }

    /// Returns the display label for an Index feed activation id.
    #[must_use]
    pub(crate) fn index_feed_label(&self, activation_id: &str) -> Option<String> {
        self.feed_row_matching(|row| row.id == activation_id)
            .map(|row| row.label.clone())
    }

    /// Returns row thumbnail URLs keyed by activation id.
    #[must_use]
    pub(crate) fn feed_thumbnail_sources(&self) -> Vec<(String, String)> {
        let RecentFeedsPageState::Loaded(rows) = &self.state else {
            return Vec::new();
        };

        rows.iter()
            .filter_map(|(_id, row)| {
                row.thumbnail_href
                    .as_ref()
                    .map(|href| (row.id.clone(), href.clone()))
            })
            .collect()
    }

    /// Projects a remote Index feed detail page from a Recent Feeds row.
    #[must_use]
    pub(crate) fn index_feed_detail(
        &self,
        activation_id: &str,
        fallback_id: &str,
        fallback_label: &str,
    ) -> IndexDetailDisplay {
        IndexDetailDisplay::feed_or_fallback(
            self.feed_row_matching(|row| row.id == activation_id),
            fallback_id,
            fallback_label,
        )
    }

    fn feed_row_matching(
        &self,
        predicate: impl Fn(&FeedResultDisplay) -> bool,
    ) -> Option<&FeedResultDisplay> {
        let RecentFeedsPageState::Loaded(rows) = &self.state else {
            return None;
        };
        rows.iter()
            .map(|(_id, row)| row)
            .find(|row| row.origin.matches_filter(ContentFilter::Index) && predicate(row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::search_results::SearchResultOrigin;

    #[test]
    fn loading_state_is_initial() {
        let vm = RecentFeedsPageVm::loading();

        assert!(matches!(vm.state(), RecentFeedsPageState::Loading));
        assert_eq!(vm.view_mode(), RecentFeedsViewMode::Tiles);
        assert!(!vm.is_loading());
        assert!(!vm.has_more());
    }

    #[test]
    fn view_mode_defaults_to_tiles_and_can_switch_to_list() {
        let mut vm = RecentFeedsPageVm::loading();

        vm.set_view_mode(RecentFeedsViewMode::List);

        assert_eq!(vm.view_mode(), RecentFeedsViewMode::List);
        assert_eq!(RecentFeedsViewMode::Tiles.label(), "Tiles");
        assert_eq!(RecentFeedsViewMode::List.id_suffix(), "list");
        assert_eq!(
            RecentFeedsViewMode::Tiles.a11y_label(),
            "Show Recent Feeds as tiles"
        );
    }

    #[test]
    fn loaded_rows_project_index_feed_detail() {
        let row = FeedResultDisplay::new(
            "index-feed:feed-guid",
            "Recent Album",
            SearchResultOrigin::Index,
        )
        .with_secondary_text("Recent Artist");
        let mut vm = RecentFeedsPageVm::loading();
        vm.replace_feeds(vec![(7, row)]);

        let detail = vm.index_feed_detail("index-feed:feed-guid", "feed-guid", "Fallback");

        assert_eq!(detail.title, "Recent Album");
        assert_eq!(detail.secondary_text, "Recent Artist");
        assert_eq!(
            vm.index_feed_label("index-feed:feed-guid"),
            Some("Recent Album".to_string())
        );
        assert!(
            vm.feed_thumbnail_sources().is_empty(),
            "rows without thumbnail hrefs should not request images"
        );
    }

    #[test]
    fn loaded_rows_project_thumbnail_sources() {
        let row = FeedResultDisplay::new(
            "index-feed:feed-guid",
            "Recent Album",
            SearchResultOrigin::Index,
        )
        .with_thumbnail_href("https://example.test/art.jpg");
        let mut vm = RecentFeedsPageVm::loading();
        vm.replace_feeds(vec![(7, row)]);

        assert_eq!(
            vm.feed_thumbnail_sources(),
            vec![(
                "index-feed:feed-guid".to_string(),
                "https://example.test/art.jpg".to_string()
            )]
        );
    }

    #[test]
    fn load_intent_tracks_append_cursor_and_in_flight_guard() {
        let mut vm = RecentFeedsPageVm::loading();
        let first_intent = vm
            .begin_load(false)
            .expect("fresh load should start from an idle VM");

        assert_eq!(first_intent.into_cursor(), None);
        assert!(vm.is_loading());
        assert!(vm.begin_load(true).is_none());

        vm.finish_load(
            RecentFeedsPageBatch {
                rows: vec![(1, recent_feed_row("index-feed:first", "First"))],
                cursor: Some("next".into()),
                has_more: true,
            },
            false,
        );

        let append_intent = vm
            .begin_load(true)
            .expect("append load should use the stored cursor");

        assert_eq!(append_intent.into_cursor().as_deref(), Some("next"));
        assert!(vm.is_loading());
        assert_eq!(vm.row_count(), 1);
    }

    #[test]
    fn finish_append_extends_rows_and_updates_pagination() {
        let mut vm = RecentFeedsPageVm::loading();
        vm.finish_load(
            RecentFeedsPageBatch {
                rows: vec![(1, recent_feed_row("index-feed:first", "First"))],
                cursor: Some("next".into()),
                has_more: true,
            },
            false,
        );
        assert!(vm.begin_load(true).is_some());

        vm.finish_load(
            RecentFeedsPageBatch {
                rows: vec![(2, recent_feed_row("index-feed:second", "Second"))],
                cursor: None,
                has_more: false,
            },
            true,
        );

        assert_eq!(vm.row_count(), 2);
        assert!(!vm.is_loading());
        assert!(!vm.has_more());
        assert!(vm.begin_load(true).is_none());
    }

    #[test]
    fn error_state_owns_display_copy() {
        let mut vm = RecentFeedsPageVm::loading();

        vm.fail_load("Recent Feeds unavailable", "network failed", false);

        assert!(matches!(
            vm.state(),
            RecentFeedsPageState::Error { message, detail }
                if message == "Recent Feeds unavailable" && detail == "network failed"
        ));
    }

    fn recent_feed_row(id: &str, title: &str) -> FeedResultDisplay {
        FeedResultDisplay::new(id, title, SearchResultOrigin::Index)
    }
}
