//! Recent feed pager view-model.
//!
//! ADR 0062 retires the separate Recent Feeds destination while keeping the
//! existing Index recency query as the default Music ordering source.

#![warn(clippy::pedantic)]

use crate::view_models::search_results::{FeedResultDisplay, SearchResultItemId};

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

/// Current load state for the recent feed pager.
#[derive(Clone, Debug)]
pub(crate) enum RecentFeedsPageState {
    /// A request is in flight.
    Loading,
    /// Recent feed rows loaded from the remote Index.
    Loaded(Vec<RecentFeedResultRow>),
    /// The remote Index returned an error.
    Error { message: String, detail: String },
}

/// GPUI-free page contract for the recent feed pager.
#[derive(Clone, Debug)]
pub(crate) struct RecentFeedsPageVm {
    state: RecentFeedsPageState,
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
            cursor: None,
            has_more: false,
            loading: false,
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::search_results::SearchResultOrigin;

    #[test]
    fn loading_state_is_initial() {
        let vm = RecentFeedsPageVm::loading();

        assert!(matches!(vm.state(), RecentFeedsPageState::Loading));
        assert!(!vm.is_loading());
        assert!(!vm.has_more());
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
