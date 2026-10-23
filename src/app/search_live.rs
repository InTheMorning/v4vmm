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

#![warn(clippy::pedantic)]

use gpui::Context;

use crate::application::CancellationToken;
use crate::presentation::delayed_presenter::present_after;
use crate::ui::tokens::Motion;
use crate::view_models::live_run::{LiveRunGate, PauseTicket, RunGeneration};
use crate::view_models::search_results::{live_run_step, LiveRunStep, SearchRun};
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

    /// Cancels the token of the Index request in flight, and keeps `token`
    /// as the token of the new request.
    pub(super) fn replace_index_cancel(&mut self, token: CancellationToken) {
        if let Some(older) = self.index_cancel.replace(token) {
            older.cancel();
        }
    }
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
                // its page through the function that Back uses. The Library
                // detail of the earlier page thus clears. The earlier
                // entries stay in the stack for Back. The restore also
                // starts the MusicIndex search of the query.
                self.restore_content_list_nav_entry(
                    content_list_id,
                    &FrameNavigationEntry::Search(query),
                    cx,
                );
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
}
