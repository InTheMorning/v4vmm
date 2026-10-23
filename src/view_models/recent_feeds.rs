//! Recent feed pager view-model.
//!
//! ADR 0062 retires the separate Recent Feeds destination while keeping the
//! existing Index recency query as the default Music ordering source.
//!
//! ADR 0086 Decision 5: one page request is one chain. After a page
//! arrives, the pager loads one more page while a window of a loaded group
//! is open, at most [`EXTRA_PAGE_CAP`] extra pages. The rows of the chain
//! stay in `staged` until the chain stops. [`RecentFeedsPageVm::next_step`]
//! decides each extra page.

#![warn(clippy::pedantic)]

use crate::view_models::search_results::{FeedResultDisplay, SearchResultItemId};

/// The largest number of extra pages for one page request (ADR 0086
/// Decision 5). Five pages of 20 feeds give 100 feeds.
pub(crate) const EXTRA_PAGE_CAP: usize = 5;

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
    generation: u64,
    cursor: Option<String>,
    resume_after: usize,
}

impl RecentFeedsLoadIntent {
    /// The generation of the page request. A result of an old generation
    /// changes nothing.
    #[must_use]
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// The number of rows that the pager holds before this page: the
    /// loaded rows and the staged rows. The first row id of the page
    /// follows them.
    #[must_use]
    pub(crate) const fn resume_after(&self) -> usize {
        self.resume_after
    }

    /// Consumes the intent and returns the cursor for the request.
    #[must_use]
    pub(crate) fn into_cursor(self) -> Option<String> {
        self.cursor
    }
}

/// The next step of a page request after a page arrives (ADR 0086
/// Decision 5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RecentFeedsPageStep {
    /// Load one more page with this intent.
    LoadExtra(RecentFeedsLoadIntent),
    /// The chain stopped. The staged rows are now loaded rows.
    Finished,
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
    /// The generation of the current page request. A fresh load adds 1.
    generation: u64,
    /// The current page request adds its rows to the loaded rows.
    append: bool,
    /// The rows of the current chain. They show when the chain stops.
    staged: Vec<RecentFeedResultRow>,
    /// The number of extra pages of the current page request.
    extra_pages: usize,
    /// The detail of a failed older page, until the next page request.
    older_pages_failure: Option<String>,
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
            generation: 0,
            append: false,
            staged: Vec::new(),
            extra_pages: 0,
            older_pages_failure: None,
        }
    }

    /// Begins a fresh or append Recent Feeds page request.
    ///
    /// A fresh load adds 1 to the generation, also while a chain is in
    /// flight. The old chain then stops at its next page. An append gives
    /// `None` while a request is in flight, or when the Index has no more
    /// pages.
    #[must_use]
    pub(crate) fn begin_load(&mut self, append: bool) -> Option<RecentFeedsLoadIntent> {
        if append && (self.loading || !self.has_more) {
            return None;
        }

        if !append {
            self.generation = self.generation.wrapping_add(1);
            self.state = RecentFeedsPageState::Loading;
            self.cursor = None;
            self.has_more = false;
        }
        self.loading = true;
        self.append = append;
        self.staged.clear();
        self.extra_pages = 0;
        self.older_pages_failure = None;

        Some(RecentFeedsLoadIntent {
            generation: self.generation,
            cursor: self.cursor.clone(),
            resume_after: self.row_count(),
        })
    }

    /// Adds the rows of `batch` to the staged rows, and stores its cursor
    /// and `has_more`. Gives `false` and changes nothing when `generation`
    /// is old or no request is in flight.
    pub(crate) fn receive_page(&mut self, generation: u64, batch: RecentFeedsPageBatch) -> bool {
        if !self.accepts(generation) {
            return false;
        }
        self.staged.extend(batch.rows);
        self.cursor = batch.cursor;
        self.has_more = batch.has_more;
        true
    }

    /// Decides the next step after a page arrives (ADR 0086 Decision 5).
    ///
    /// It gives `LoadExtra` when `windows_open` is true, the Index has
    /// more pages and the request has fewer than [`EXTRA_PAGE_CAP`] extra
    /// pages. Else it moves the staged rows into the loaded rows, stops
    /// the loading state and gives `Finished`.
    pub(crate) fn next_step(&mut self, windows_open: bool) -> RecentFeedsPageStep {
        if !self.loading {
            return RecentFeedsPageStep::Finished;
        }
        if windows_open && self.has_more && self.extra_pages < EXTRA_PAGE_CAP {
            self.extra_pages += 1;
            return RecentFeedsPageStep::LoadExtra(RecentFeedsLoadIntent {
                generation: self.generation,
                cursor: self.cursor.clone(),
                resume_after: self.row_count() + self.staged.len(),
            });
        }
        self.finish_chain();
        RecentFeedsPageStep::Finished
    }

    /// Records a failed page. Gives `false` and changes nothing when
    /// `generation` is old or no request is in flight.
    ///
    /// - A failed first page of a fresh load gives `Error`.
    /// - Else the staged rows become loaded rows, and the pager keeps
    ///   `detail` as the failure of the older pages. The cursor and
    ///   `has_more` of the last page stay, so the next append requests the
    ///   failed page again.
    pub(crate) fn fail_page(
        &mut self,
        generation: u64,
        message: impl Into<String>,
        detail: impl Into<String>,
    ) -> bool {
        if !self.accepts(generation) {
            return false;
        }
        if !self.append && self.extra_pages == 0 {
            self.loading = false;
            self.staged.clear();
            self.state = RecentFeedsPageState::Error {
                message: message.into(),
                detail: detail.into(),
            };
            return true;
        }
        self.older_pages_failure = Some(detail.into());
        self.finish_chain();
        true
    }

    /// True when `generation` is the current request and a request is in
    /// flight.
    const fn accepts(&self, generation: u64) -> bool {
        self.loading && generation == self.generation
    }

    /// Moves the staged rows into the loaded rows and stops the loading
    /// state.
    fn finish_chain(&mut self) {
        self.loading = false;
        let staged = std::mem::take(&mut self.staged);
        match &mut self.state {
            RecentFeedsPageState::Loaded(rows) if self.append => rows.extend(staged),
            RecentFeedsPageState::Loaded(_)
            | RecentFeedsPageState::Loading
            | RecentFeedsPageState::Error { .. } => {
                self.state = RecentFeedsPageState::Loaded(staged);
            }
        }
    }

    /// The loaded rows, then the staged rows of the current chain.
    pub(crate) fn chain_rows(&self) -> impl Iterator<Item = &RecentFeedResultRow> {
        let loaded: &[RecentFeedResultRow] = match &self.state {
            RecentFeedsPageState::Loaded(rows) => rows,
            RecentFeedsPageState::Loading | RecentFeedsPageState::Error { .. } => &[],
        };
        loaded.iter().chain(&self.staged)
    }

    /// The number of extra pages of the current page request. Only the
    /// pager compares it with the cap.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn extra_pages(&self) -> usize {
        self.extra_pages
    }

    /// The detail of a failed older page. The next page request clears it.
    #[must_use]
    pub(crate) fn older_pages_failure(&self) -> Option<&str> {
        self.older_pages_failure.as_deref()
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

    /// Returns the current page state. It holds no staged row.
    #[must_use]
    pub(crate) const fn state(&self) -> &RecentFeedsPageState {
        &self.state
    }

    /// Runs one page request of one recorded page with no extra page, for
    /// a test.
    #[cfg(test)]
    pub(crate) fn load_recorded_page(&mut self, batch: RecentFeedsPageBatch, append: bool) {
        let intent = self
            .begin_load(append)
            .expect("the test starts a page request");
        assert!(self.receive_page(intent.generation(), batch));
        assert_eq!(self.next_step(false), RecentFeedsPageStep::Finished);
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
        let generation = first_intent.generation();

        assert_eq!(first_intent.into_cursor(), None);
        assert!(vm.is_loading());
        assert!(vm.begin_load(true).is_none());

        assert!(vm.receive_page(generation, page(&["first"], true)));
        assert_eq!(vm.next_step(false), RecentFeedsPageStep::Finished);

        let append_intent = vm
            .begin_load(true)
            .expect("append load should use the stored cursor");

        assert_eq!(append_intent.resume_after(), 1);
        assert_eq!(append_intent.into_cursor().as_deref(), Some("next"));
        assert!(vm.is_loading());
        assert_eq!(vm.row_count(), 1);
    }

    #[test]
    fn finish_append_extends_rows_and_updates_pagination() {
        let mut vm = RecentFeedsPageVm::loading();
        vm.load_recorded_page(page(&["first"], true), false);
        vm.load_recorded_page(page(&["second"], false), true);

        assert_eq!(vm.row_count(), 2);
        assert!(!vm.is_loading());
        assert!(!vm.has_more());
        assert!(vm.begin_load(true).is_none());
    }

    #[test]
    fn error_state_owns_display_copy() {
        let mut vm = RecentFeedsPageVm::loading();
        let intent = vm.begin_load(false).expect("a fresh load starts");

        assert!(vm.fail_page(
            intent.generation(),
            "Recent Feeds unavailable",
            "network failed"
        ));

        assert!(matches!(
            vm.state(),
            RecentFeedsPageState::Error { message, detail }
                if message == "Recent Feeds unavailable" && detail == "network failed"
        ));
        assert_eq!(vm.older_pages_failure(), None);
    }

    /// The pager after a fresh load whose first page holds `ids`, with
    /// the chain in flight.
    fn first_page_received(ids: &[&str], has_more: bool) -> (RecentFeedsPageVm, u64) {
        let mut vm = RecentFeedsPageVm::loading();
        let generation = vm
            .begin_load(false)
            .expect("a fresh load starts")
            .generation();
        assert!(vm.receive_page(generation, page(ids, has_more)));
        (vm, generation)
    }

    fn row_ids<'a>(rows: impl IntoIterator<Item = &'a RecentFeedResultRow>) -> Vec<String> {
        rows.into_iter().map(|(_, row)| row.id.clone()).collect()
    }

    fn loaded_ids(vm: &RecentFeedsPageVm) -> Vec<String> {
        match vm.state() {
            RecentFeedsPageState::Loaded(rows) => row_ids(rows),
            RecentFeedsPageState::Loading | RecentFeedsPageState::Error { .. } => Vec::new(),
        }
    }

    /// R86-3-01: a page with more pages and an open window gives
    /// `LoadExtra` with the same generation and the cursor of the page.
    #[test]
    fn adr_0086_pager_windows_r01_open_window_loads_an_extra_page() {
        let (mut vm, generation) = first_page_received(&["a"], true);
        let RecentFeedsPageStep::LoadExtra(intent) = vm.next_step(true) else {
            panic!("an open window loads an extra page");
        };
        assert_eq!(intent.generation(), generation);
        assert_eq!(intent.into_cursor().as_deref(), Some("next"));
    }

    /// R86-3-02: `resume_after` counts the loaded rows and the staged
    /// rows.
    #[test]
    fn adr_0086_pager_windows_r02_resume_after_counts_loaded_and_staged_rows() {
        let mut vm = RecentFeedsPageVm::loading();
        vm.load_recorded_page(page(&["a", "b"], true), false);
        let generation = vm.begin_load(true).expect("an append starts").generation();
        assert!(vm.receive_page(generation, page(&["c", "d", "e"], true)));
        let RecentFeedsPageStep::LoadExtra(intent) = vm.next_step(true) else {
            panic!("an open window loads an extra page");
        };
        assert_eq!(intent.resume_after(), 5);
    }

    /// R86-3-03: the cap stops the chain after five extra pages.
    #[test]
    fn adr_0086_pager_windows_r03_the_cap_stops_the_chain() {
        let (mut vm, generation) = first_page_received(&["p0"], true);
        for extra in 1..=EXTRA_PAGE_CAP {
            assert!(matches!(
                vm.next_step(true),
                RecentFeedsPageStep::LoadExtra(_)
            ));
            assert!(vm.receive_page(generation, page(&[&format!("p{extra}")], true)));
        }
        assert_eq!(vm.next_step(true), RecentFeedsPageStep::Finished);
        assert_eq!(vm.extra_pages(), EXTRA_PAGE_CAP);
        assert_eq!(vm.row_count(), EXTRA_PAGE_CAP + 1);
    }

    /// R86-3-04: the next page request gets a new count of extra pages.
    #[test]
    fn adr_0086_pager_windows_r04_next_request_gets_a_new_cap() {
        let (mut vm, generation) = first_page_received(&["p0"], true);
        for extra in 1..=EXTRA_PAGE_CAP {
            let _ = vm.next_step(true);
            assert!(vm.receive_page(generation, page(&[&format!("p{extra}")], true)));
        }
        assert_eq!(vm.next_step(true), RecentFeedsPageStep::Finished);

        let generation = vm.begin_load(true).expect("an append starts").generation();
        assert_eq!(vm.extra_pages(), 0);
        assert!(vm.receive_page(generation, page(&["q0"], true)));
        for extra in 1..=EXTRA_PAGE_CAP {
            assert!(matches!(
                vm.next_step(true),
                RecentFeedsPageStep::LoadExtra(_)
            ));
            assert!(vm.receive_page(generation, page(&[&format!("q{extra}")], true)));
        }
        assert_eq!(vm.next_step(true), RecentFeedsPageStep::Finished);
    }

    /// R86-3-05: with no open window, the first page finishes the chain.
    #[test]
    fn adr_0086_pager_windows_r05_closed_windows_finish_at_once() {
        let (mut vm, _) = first_page_received(&["a"], true);
        assert_eq!(vm.next_step(false), RecentFeedsPageStep::Finished);
        assert_eq!(vm.extra_pages(), 0);
        assert!(!vm.is_loading());
    }

    /// R86-3-06: with no more pages, the chain finishes.
    #[test]
    fn adr_0086_pager_windows_r06_no_more_pages_finishes() {
        let (mut vm, _) = first_page_received(&["a"], false);
        assert_eq!(vm.next_step(true), RecentFeedsPageStep::Finished);
    }

    /// R86-3-07: during the chain, `state()` holds no staged row, and
    /// `chain_rows()` gives the loaded rows, then the staged rows.
    #[test]
    fn adr_0086_pager_windows_r07_staged_rows_stay_out_of_the_state() {
        let mut vm = RecentFeedsPageVm::loading();
        vm.load_recorded_page(page(&["a"], true), false);
        let generation = vm.begin_load(true).expect("an append starts").generation();
        assert!(vm.receive_page(generation, page(&["b"], true)));
        assert!(matches!(
            vm.next_step(true),
            RecentFeedsPageStep::LoadExtra(_)
        ));
        assert!(vm.receive_page(generation, page(&["c"], true)));

        assert!(vm.is_loading());
        assert_eq!(loaded_ids(&vm), ["index-feed:a"]);
        assert_eq!(
            row_ids(vm.chain_rows()),
            ["index-feed:a", "index-feed:b", "index-feed:c"]
        );
    }

    /// R86-3-09: after `Finished`, the state holds each row of the chain
    /// in the list order.
    #[test]
    fn adr_0086_pager_windows_r09_finished_chain_shows_each_row() {
        let (mut vm, generation) = first_page_received(&["a", "b"], true);
        assert!(matches!(
            vm.next_step(true),
            RecentFeedsPageStep::LoadExtra(_)
        ));
        assert!(vm.receive_page(generation, page(&["c"], false)));
        assert_eq!(vm.next_step(true), RecentFeedsPageStep::Finished);
        assert!(!vm.is_loading());
        assert_eq!(
            loaded_ids(&vm),
            ["index-feed:a", "index-feed:b", "index-feed:c"]
        );
    }

    /// R86-3-21: a fresh load during a chain gives a new generation. A
    /// result of the old generation changes no row.
    #[test]
    fn adr_0086_pager_windows_r21_fresh_load_stops_the_old_chain() {
        let (mut vm, old) = first_page_received(&["a"], true);
        assert!(matches!(
            vm.next_step(true),
            RecentFeedsPageStep::LoadExtra(_)
        ));
        let new = vm
            .begin_load(false)
            .expect("a fresh load replaces the chain");
        assert_ne!(new.generation(), old);

        assert!(!vm.receive_page(old, page(&["late"], true)));
        assert!(!vm.fail_page(old, "Recent music unavailable", "late failure"));
        assert_eq!(vm.chain_rows().count(), 0);
        assert!(matches!(vm.state(), RecentFeedsPageState::Loading));
        assert!(vm.is_loading());
        assert_eq!(vm.older_pages_failure(), None);
    }

    /// R86-3-23: a failed extra page keeps the first page. The next
    /// append uses the cursor of that page.
    #[test]
    fn adr_0086_pager_windows_r23_failed_extra_page_keeps_the_first_page() {
        let (mut vm, generation) = first_page_received(&["a"], true);
        assert!(matches!(
            vm.next_step(true),
            RecentFeedsPageStep::LoadExtra(_)
        ));
        assert!(vm.fail_page(generation, "Recent music unavailable", "timeout"));

        assert_eq!(loaded_ids(&vm), ["index-feed:a"]);
        assert!(!vm.is_loading());
        assert!(vm.has_more());
        assert_eq!(vm.older_pages_failure(), Some("timeout"));
        let intent = vm.begin_load(true).expect("the next append starts");
        assert_eq!(intent.into_cursor().as_deref(), Some("next"));
        assert_eq!(vm.older_pages_failure(), None);
    }

    /// R86-3-28 (the pager part): a failed first page of a fresh load
    /// gives `Error`.
    #[test]
    fn adr_0086_pager_windows_r28_failed_first_page_gives_error() {
        let mut vm = RecentFeedsPageVm::loading();
        let generation = vm
            .begin_load(false)
            .expect("a fresh load starts")
            .generation();
        assert!(vm.fail_page(generation, "Recent music unavailable", "network failed"));
        assert!(matches!(
            vm.state(),
            RecentFeedsPageState::Error { message, .. } if message == "Recent music unavailable"
        ));
        assert_eq!(vm.older_pages_failure(), None);
        assert!(!vm.is_loading());
    }

    /// One page of rows with these ids. `has_more` gives the cursor
    /// "next".
    fn page(ids: &[&str], has_more: bool) -> RecentFeedsPageBatch {
        RecentFeedsPageBatch {
            rows: ids
                .iter()
                .zip(0..)
                .map(|(id, index)| (index, recent_feed_row(&format!("index-feed:{id}"), id)))
                .collect(),
            cursor: has_more.then(|| "next".to_owned()),
            has_more,
        }
    }

    fn recent_feed_row(id: &str, title: &str) -> FeedResultDisplay {
        FeedResultDisplay::new(id, title, SearchResultOrigin::Index)
    }
}
