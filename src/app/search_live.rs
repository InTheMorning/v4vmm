//! The live runs of the Music search (ADR 0087 Decision 4, packet 003).
//!
//! The toolbar search runs while the curator types. Each change of the
//! field text arms a pause of `Motion::SearchPause`. When the pause of the
//! last change ends, the search runs against the Library and the Index.
//! Enter runs it at once. Each run of a search page slot takes a new
//! generation, and the app drops a result of an older generation (ADR 0087
//! invariant "A result of an older run never replaces the result of a newer
//! run"). A newer Index run cancels the token of the older Index request.
//!
//! The timer is `present_after`, so this module calls no `cx.spawn` (ADR
//! 0040). A live run does not move the focus, so the caret stays in the
//! field.
//!
//! Packet 007: a toolbar run keeps one generation from its start to its
//! Index result. [`index_result_step`] decides if an Index result changes
//! the page, and gives the reason of each result that it stops.

#![warn(clippy::pedantic)]

use gpui::Context;

use crate::application::CancellationToken;
use crate::presentation::delayed_presenter::present_after;
use crate::ui::tokens::Motion;
use crate::view_models::live_run::{LiveRunGate, PauseTicket, RunGeneration};
use crate::view_models::search_results::{
    live_run_step, IndexResultStopReason, LiveRunStep, SearchResultsInspectorPageVm, SearchRun,
};
use crate::view_models::workspace::{FrameNavigationEntry, FrameNavigationState};

use super::search_selection::SearchPageSlot;
use super::{AppTab, TopApp};

/// The pause, the run generation and the Index request in flight of one
/// search page slot.
#[derive(Debug, Default)]
pub(super) struct SearchSlotRuns {
    /// The pause and the run generation of the slot.
    pub(super) gate: LiveRunGate,
    /// The cancellation token of the Index request in flight.
    index_cancel: Option<CancellationToken>,
}

impl SearchSlotRuns {
    /// Starts a new run of the slot and gives its generation. The result of
    /// an older run is no longer current, so the function also cancels the
    /// token of the Index request in flight (packet 003 decision D12).
    pub(super) fn next_run(&mut self) -> RunGeneration {
        if let Some(older) = self.index_cancel.take() {
            older.cancel();
        }
        self.gate.next_run()
    }

    /// `true` when `generation` is the current run of the slot (ADR 0087
    /// Decision 4).
    #[must_use]
    pub(super) fn is_current_run(&self, generation: RunGeneration) -> bool {
        self.gate.is_current(generation)
    }

    /// Cancels the token of the Index request in flight, and keeps `token`
    /// as the token of the new request.
    pub(super) fn replace_index_cancel(&mut self, token: CancellationToken) {
        if let Some(older) = self.index_cancel.replace(token) {
            older.cancel();
        }
    }
}

/// The step of one Index result (ADR 0087 Decision 4, packet 007 decision
/// D4). Only `Apply` and `MountPage` change the page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum IndexResultStep {
    /// The result applies to the mounted page.
    Apply,
    /// No page is mounted. The app builds the page, then applies the
    /// result.
    MountPage,
    /// The result is of a run that is not the current run of the slot.
    DropOlderRun,
    /// The navigation entry or the page is of another query.
    DropOtherQuery,
    /// The page shows another run of the same query.
    DropPageGeneration,
}

impl IndexResultStep {
    /// The typed reason of a stopped result for the page note of packet
    /// 007 decision D6. A result of another query gives no note, because
    /// the page of that query is not mounted.
    #[must_use]
    pub(super) const fn stop_reason(self) -> Option<IndexResultStopReason> {
        match self {
            Self::DropOlderRun => Some(IndexResultStopReason::OlderRun),
            Self::DropPageGeneration => Some(IndexResultStopReason::PageGeneration),
            Self::Apply | Self::MountPage | Self::DropOtherQuery => None,
        }
    }
}

/// Decides the step of the Index result of the run `generation` for
/// `query` (packet 007 decision D4). `current_run` tells if the run is the
/// current run of the slot. `entry_matches` tells if the navigation entry
/// of the slot is of `query`. `page` is the mounted page of the slot. The
/// first true row of the table of packet 007 gives the step.
#[must_use]
pub(super) fn index_result_step(
    current_run: bool,
    entry_matches: bool,
    page: Option<&SearchResultsInspectorPageVm>,
    generation: RunGeneration,
    query: &str,
) -> IndexResultStep {
    if !current_run {
        return IndexResultStep::DropOlderRun;
    }
    if !entry_matches {
        return IndexResultStep::DropOtherQuery;
    }
    let Some(page) = page else {
        return IndexResultStep::MountPage;
    };
    if page.query() != query {
        return IndexResultStep::DropOtherQuery;
    }
    if page.run_generation() != generation {
        return IndexResultStep::DropPageGeneration;
    }
    IndexResultStep::Apply
}

impl TopApp {
    /// The runs of the search page `slot`.
    pub(super) fn search_runs_mut(&mut self, slot: SearchPageSlot) -> &mut SearchSlotRuns {
        match slot {
            SearchPageSlot::ToolbarSearch => &mut self.toolbar_search_runs,
            SearchPageSlot::SavedSearch => &mut self.saved_search_runs,
        }
    }

    /// The run of `text` for the search page `slot`, with a new generation
    /// of that slot. Each run of a search page comes from this function.
    pub(super) fn next_search_run(&mut self, slot: SearchPageSlot, text: &str) -> SearchRun {
        let generation = self.search_runs_mut(slot).next_run();
        SearchRun::new(text).with_generation(generation)
    }

    /// Arms the pause after a change of the toolbar field text. Outside the
    /// Music tab, typing arms no pause (packet 003 decision D8).
    pub(super) fn start_search_pause(&mut self, cx: &mut Context<Self>) {
        if self.tab != AppTab::Music {
            return;
        }
        let ticket = self.toolbar_search_runs.gate.text_changed();
        present_after(Motion::SearchPause.duration(), cx, move |this, cx| {
            this.end_search_pause(ticket, cx);
        });
    }

    /// Runs the search when the pause of `ticket` ends and `ticket` is the
    /// ticket of the last change. Each other ticket does nothing.
    pub(super) fn end_search_pause(&mut self, ticket: PauseTicket, cx: &mut Context<Self>) {
        if !self.toolbar_search_runs.gate.pause_ended(ticket) {
            return;
        }
        let text = self.global_search_input.read(cx).value().to_string();
        let mounted_query = self
            .content_list_frame_id()
            .and_then(|content_list_id| self.workspace_layout.frame_nav(content_list_id))
            .and_then(FrameNavigationState::active_search_query);
        let step = live_run_step(&text, mounted_query, self.tab == AppTab::Music);
        match step {
            LiveRunStep::Run(run) => {
                let generation = self.toolbar_search_runs.next_run();
                self.show_toolbar_search_run(&run.with_generation(generation), cx);
            }
            LiveRunStep::NotRunnable | LiveRunStep::SameQuery | LiveRunStep::NotMusic => {}
        }
    }

    /// Shows the toolbar search page of `run` in the one navigation stack.
    /// A current `Search` page changes its text in place. Each other page
    /// starts a new path with the `Search` page (ADR 0084 Decision 9). The
    /// function takes no `Window` and moves no focus (packet 003 decision
    /// D9).
    pub(super) fn show_toolbar_search_run(&mut self, run: &SearchRun, cx: &mut Context<Self>) {
        let query = run.text().to_owned();
        match self
            .workspace_layout
            .open_search_results_in_content_list(query.clone())
        {
            Ok(content_list_id) => {
                let page = self
                    .search_results_detail_for_run(run, cx)
                    .with_run_generation(run.generation());
                if page.is_newer_than(self.search_results_detail.as_ref()) {
                    self.search_results_detail = Some(page);
                }
                // Packet 028 review: a new query starts at the top of its list.
                self.scroll_search_page_to_top(SearchPageSlot::ToolbarSearch);
                // ADR 0084 Decision 9, walk of 2026-10-21: a search restores
                // its page through the steps that Back uses. The Library
                // detail of the earlier page thus clears. The earlier
                // entries stay in the stack for Back.
                self.restore_content_list_page(
                    content_list_id,
                    &FrameNavigationEntry::Search(query),
                    cx,
                );
                // Packet 007 decision D3: the Index part keeps the
                // generation of this run.
                self.start_index_search_for_run(SearchPageSlot::ToolbarSearch, run, cx);
            }
            Err(e) => {
                self.settings_status = format!("Error opening search results: {e}");
                cx.notify();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::TestAppContext;
    use gpui_component::input::{InputEvent, InputState};

    use super::*;
    use crate::view_models::search_results::{
        FeedResultDisplay, IndexSearchResultRows, SearchResultOrigin, SearchResultsTab,
    };

    /// R87-3-14: `set_value` emits no `Change`. `replace` and `clean` each
    /// emit one `Change`. Thus a restored text arms no pause.
    #[gpui::test]
    fn adr_0087_live_results_r87_3_14_set_value_emits_no_change(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (input, cx) = cx.add_window_view(InputState::new);
        let changes = Rc::new(Cell::new(0));
        cx.update(|_, cx| {
            let changes = changes.clone();
            cx.subscribe(&input, move |_, event: &InputEvent, _| {
                if matches!(event, InputEvent::Change) {
                    changes.set(changes.get() + 1);
                }
            })
            .detach();
        });

        cx.update(|window, cx| {
            input.update(cx, |input, cx| input.set_value("haleen", window, cx));
        });
        cx.run_until_parked();
        assert_eq!(changes.get(), 0, "set_value must emit no Change");

        cx.update(|window, cx| {
            input.update(cx, |input, cx| input.replace(" dri", window, cx));
        });
        cx.run_until_parked();
        assert_eq!(changes.get(), 1, "replace must emit one Change");

        cx.update(|window, cx| {
            input.update(cx, |input, cx| input.clean(window, cx));
        });
        cx.run_until_parked();
        assert_eq!(changes.get(), 2, "clean must emit one Change");
    }

    /// R87-3-15: a new Index token cancels the token in flight. The
    /// generations of a slot increase.
    #[test]
    fn adr_0087_live_results_r87_3_15_new_token_cancels_the_older_token() {
        let mut runs = SearchSlotRuns::default();
        let a = CancellationToken::new();
        let b = CancellationToken::new();
        runs.replace_index_cancel(a.clone());
        runs.replace_index_cancel(b.clone());
        assert!(a.is_cancelled());
        assert!(!b.is_cancelled());

        let first = runs.next_run();
        assert!(b.is_cancelled(), "a new run cancels the request in flight");
        let second = runs.next_run();
        assert!(first < second);
        assert!(runs.gate.is_current(second));
    }

    /// The generations 1 to `count` of one slot.
    fn generations(count: usize) -> Vec<RunGeneration> {
        let mut runs = SearchSlotRuns::default();
        (0..count).map(|_| runs.next_run()).collect()
    }

    /// The page of `query` with the run generation `generation`.
    fn page(query: &str, generation: RunGeneration) -> SearchResultsInspectorPageVm {
        SearchResultsInspectorPageVm::new(query).with_run_generation(generation)
    }

    /// The Index rows of one release.
    fn index_rows() -> IndexSearchResultRows {
        IndexSearchResultRows {
            feeds: vec![(
                2_000_000_000,
                FeedResultDisplay::new(
                    "index-feed:g-fittest",
                    "SURVIVAL OF THE FITTEST",
                    SearchResultOrigin::Index,
                ),
            )],
            ..Default::default()
        }
    }

    /// R87-7-03: each row of the table of packet 007 gives its step.
    #[test]
    fn adr_0087_live_index_rows_r87_7_03_each_row_of_the_table() {
        let generation = generations(2);
        let mounted = page("survival", generation[1]);
        let step =
            |current, entry, page, run, query| index_result_step(current, entry, page, run, query);
        assert_eq!(
            step(false, true, Some(&mounted), generation[1], "survival"),
            IndexResultStep::DropOlderRun
        );
        assert_eq!(
            step(true, false, Some(&mounted), generation[1], "survival"),
            IndexResultStep::DropOtherQuery
        );
        assert_eq!(
            step(true, true, None, generation[1], "survival"),
            IndexResultStep::MountPage
        );
        assert_eq!(
            step(true, true, Some(&mounted), generation[1], "surviva"),
            IndexResultStep::DropOtherQuery
        );
        assert_eq!(
            step(true, true, Some(&mounted), generation[0], "survival"),
            IndexResultStep::DropPageGeneration
        );
        assert_eq!(
            step(true, true, Some(&mounted), generation[1], "survival"),
            IndexResultStep::Apply
        );
    }

    /// Packet 007 decision D6: each step that stops a result of the page
    /// query gives its typed reason. The other steps give none.
    #[test]
    fn adr_0087_live_index_rows_d6_each_step_gives_its_reason() {
        assert_eq!(
            IndexResultStep::DropOlderRun.stop_reason(),
            Some(IndexResultStopReason::OlderRun)
        );
        assert_eq!(
            IndexResultStep::DropPageGeneration.stop_reason(),
            Some(IndexResultStopReason::PageGeneration)
        );
        for step in [
            IndexResultStep::Apply,
            IndexResultStep::MountPage,
            IndexResultStep::DropOtherQuery,
        ] {
            assert_eq!(step.stop_reason(), None, "{step:?}");
        }
    }

    /// R87-7-04: a page of `survival` and generation 2 applies only the
    /// result of generation 2 and `survival`.
    #[test]
    fn adr_0087_live_index_rows_r87_7_04_page_generation_and_query() {
        let generation = generations(2);
        let mounted = page("survival", generation[1]);
        assert_eq!(
            index_result_step(true, true, Some(&mounted), generation[1], "survival"),
            IndexResultStep::Apply
        );
        assert_eq!(
            index_result_step(true, true, Some(&mounted), generation[0], "survival"),
            IndexResultStep::DropPageGeneration
        );
        assert_eq!(
            index_result_step(true, true, Some(&mounted), generation[1], "surviva"),
            IndexResultStep::DropOtherQuery
        );
    }

    /// The live sequence of R87-7-05: the pause ends, the run takes G, the
    /// page and the Index request keep G, and the result of G applies.
    fn live_run_applies(runs: &mut SearchSlotRuns) -> (RunGeneration, CancellationToken) {
        let ticket = runs.gate.text_changed();
        assert!(runs.gate.pause_ended(ticket));
        let g = runs.next_run();
        let mut mounted = page("survival", g);
        assert!(mounted.begin_index_run(g));
        let t = CancellationToken::new();
        runs.replace_index_cancel(t.clone());

        assert!(runs.gate.is_current(g));
        assert!(!t.is_cancelled());
        assert_eq!(
            index_result_step(runs.is_current_run(g), true, Some(&mounted), g, "survival"),
            IndexResultStep::Apply
        );
        assert!(mounted.replace_index_results_for_run(g, "survival", index_rows()));
        assert!(!mounted.is_empty(SearchResultsTab::Releases));
        assert_eq!(mounted.index_pending_row_count(), 0);
        (g, t)
    }

    /// R87-7-05: a live run keeps one generation from its start to its
    /// Index result, and the result applies.
    #[test]
    fn adr_0087_live_index_rows_r87_7_05_live_run_applies_its_result() {
        let mut runs = SearchSlotRuns::default();
        live_run_applies(&mut runs);
    }

    /// R87-7-06: an Enter run of the same text after the live run takes H
    /// and cancels T. The result of G stops, and the result of H applies.
    #[test]
    fn adr_0087_live_index_rows_r87_7_06_enter_after_the_live_run() {
        let mut runs = SearchSlotRuns::default();
        let (g, t) = live_run_applies(&mut runs);
        let h = runs.next_run();
        assert!(t.is_cancelled());
        let mut mounted = page("survival", h);
        assert!(mounted.begin_index_run(h));
        assert_eq!(
            index_result_step(runs.is_current_run(g), true, Some(&mounted), g, "survival"),
            IndexResultStep::DropOlderRun
        );
        assert_eq!(
            index_result_step(runs.is_current_run(h), true, Some(&mounted), h, "survival"),
            IndexResultStep::Apply
        );
    }

    /// R87-7-07: the run of `haleen` mounts its page over the page of
    /// `hal`. The result of `haleen` applies, and the result of `hal`
    /// stops.
    #[test]
    fn adr_0087_live_index_rows_r87_7_07_newer_query_replaces_the_page() {
        let mut runs = SearchSlotRuns::default();
        let first = runs.next_run();
        let hal = page("hal", first);
        let second = runs.next_run();
        let haleen = page("haleen", second);
        assert!(haleen.is_newer_than(Some(&hal)));
        assert_eq!(
            index_result_step(
                runs.is_current_run(second),
                true,
                Some(&haleen),
                second,
                "haleen"
            ),
            IndexResultStep::Apply
        );
        assert_eq!(
            index_result_step(
                runs.is_current_run(first),
                false,
                Some(&haleen),
                first,
                "hal"
            ),
            IndexResultStep::DropOlderRun
        );
    }
}
