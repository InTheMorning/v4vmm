//! The selection of the search list in the Music list column (ADR 0084
//! Decision 10, packet 010).
//!
//! The search page view model holds the one selection of the search list.
//! The top app composes the list: the keyboard focus, the "Select" button,
//! the clicks and the action bar. The actions use the functions that the
//! detail column also uses, with the rows and the selection of the search
//! list.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, Context, Modifiers, SharedString, Window};

use super::keyboard::primary_modifier_held;
use super::TopApp;
use crate::library::playlist_options;
use crate::ui::composites::{SelectionActionBar, SelectionActionBarPlaylists};
use crate::view_models::detail_selection::{DetailPageRows, DetailRowClick, DetailRowKey};
use crate::view_models::list_selection::FocusStep;
use crate::view_models::search_results::{SearchResultsInspectorPageVm, SearchResultsTab};
use crate::view_models::selection_action_bar::{SelectionActionBarVm, SelectionActionKind};

/// The kind of a row click from its modifier keys, by the rule of the
/// detail column (packet 018).
fn row_click(modifiers: Modifiers) -> DetailRowClick {
    if primary_modifier_held(modifiers) {
        DetailRowClick::Toggle
    } else if modifiers.shift {
        DetailRowClick::Extend
    } else {
        DetailRowClick::Plain
    }
}

impl TopApp {
    /// A click on the body of a search row. A row with a key changes the
    /// selection by the rules of packet 018, and a plain click in `Normal`
    /// mode opens its page. A row with no key opens its page in `Normal`
    /// mode only.
    pub(super) fn click_search_result_row(
        &mut self,
        tab: SearchResultsTab,
        result_id: &str,
        key: Option<DetailRowKey>,
        modifiers: Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.search_list_focus, cx);
        let open = {
            let library = self.library.read(cx);
            let lookup = library.search_lookup();
            match (key, self.search_results_detail.as_mut()) {
                (Some(key), Some(detail)) => detail.click_row(&key, row_click(modifiers), &lookup),
                (None, Some(detail)) => !detail.selection().mode().is_selecting(),
                (_, None) => false,
            }
        };
        if open {
            self.handle_search_result_selected(tab, result_id, cx);
        }
        cx.notify();
    }

    /// A click on the checkbox of a search row.
    pub(super) fn toggle_search_result_row(
        &mut self,
        key: &DetailRowKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.search_list_focus, cx);
        if let Some(detail) = self.search_results_detail.as_mut() {
            detail.toggle_row(key);
        }
        cx.notify();
    }

    /// The "Select" and "Done" button of the search list.
    pub(super) fn toggle_search_selection_mode(&mut self, cx: &mut Context<Self>) {
        if let Some(detail) = self.search_results_detail.as_mut() {
            detail.toggle_mode();
        }
        cx.notify();
    }

    /// The selectable rows of the search list, with the Library facts of
    /// this moment. `None` on the Artists tab.
    fn search_selection_rows(&self, cx: &gpui::App) -> Option<DetailPageRows> {
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        self.search_results_detail.as_ref()?.selection_rows(&lookup)
    }

    /// The search list with its keyboard focus and its action bar. A
    /// click in the list gives the list the keyboard focus. The bar shows
    /// at the bottom of the list column, in selection mode only.
    pub(super) fn render_search_list_frame(
        &self,
        content: AnyElement,
        bar: Option<AnyElement>,
    ) -> AnyElement {
        div()
            .id("music-search-list")
            .track_focus(&self.search_list_focus)
            .size_full()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(content),
            )
            .children(bar)
            .into_any_element()
    }

    /// The bar of the search list: the release bar or the track bar of
    /// packet 018.
    pub(super) fn render_search_action_bar(
        &mut self,
        bar: SelectionActionBarVm,
        rows: &DetailPageRows,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let choices = rows.add_to_playlist_choices(self.library.read(cx).playlists());
        let playlists = SelectionActionBarPlaylists {
            playlists: playlist_options(&choices),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add the selection"),
        };
        let download = cx.listener(|this, (): &(), window, cx| {
            this.run_search_selection_action(SelectionActionKind::Download, window, cx);
        });
        let remove = cx.listener(|this, (): &(), window, cx| {
            this.run_search_selection_action(SelectionActionKind::Remove, window, cx);
        });
        let clear = cx.listener(|this, (): &(), window, cx| {
            this.run_search_selection_action(SelectionActionKind::Clear, window, cx);
        });
        SelectionActionBar::new(bar, playlists)
            .on_download(move |window, cx| download(&(), window, cx))
            .on_remove(move |window, cx| remove(&(), window, cx))
            .on_clear(move |window, cx| clear(&(), window, cx))
            .on_add_to_playlist(cx.listener(|this, playlist_id: &i64, _window, cx| {
                let selection = this
                    .search_results_detail
                    .as_ref()
                    .map(|detail| detail.selection().clone());
                if let (Some(rows), Some(selection)) = (this.search_selection_rows(cx), selection) {
                    this.add_selection_to_playlist(&rows, &selection, *playlist_id, cx);
                }
            }))
            .on_create_playlist(cx.listener(|this, name: &String, _window, cx| {
                let selection = this
                    .search_results_detail
                    .as_ref()
                    .map(|detail| detail.selection().clone());
                if let (Some(rows), Some(selection)) = (this.search_selection_rows(cx), selection) {
                    this.create_playlist_and_add_selection(&rows, &selection, name, cx);
                }
            }))
            .into_any_element()
    }

    /// Runs "Download", "Remove…" or "Clear" of the search bar. The
    /// selection stays after each action.
    fn run_search_selection_action(
        &mut self,
        kind: SelectionActionKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rows) = self.search_selection_rows(cx) else {
            return;
        };
        let Some(selection) = self
            .search_results_detail
            .as_ref()
            .map(|detail| detail.selection().clone())
        else {
            return;
        };
        match kind {
            SelectionActionKind::Download => self.download_selection(&rows, &selection, cx),
            SelectionActionKind::Remove => {
                self.request_selection_removal(&rows, &selection, window, cx);
            }
            SelectionActionKind::Clear => {
                if let Some(detail) = self.search_results_detail.as_mut() {
                    detail.clear_rows();
                }
            }
            // The search list is no playlist page. "Add to playlist" opens
            // its own popover.
            SelectionActionKind::AddToPlaylist
            | SelectionActionKind::RemoveFromPlaylist
            | SelectionActionKind::MoveToTop
            | SelectionActionKind::MoveToBottom => {}
        }
        cx.notify();
    }

    /// Tells if the search list is mounted and carries the keyboard focus.
    fn search_list_has_focus(&self, window: &Window, cx: &gpui::App) -> bool {
        self.search_results_detail.is_some() && self.search_list_focus.contains_focused(window, cx)
    }

    /// Up, Down, Page Up, Page Down, Home and End, for the search list.
    pub(super) fn move_search_selection_focus(
        &mut self,
        step: FocusStep,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if !self.search_list_has_focus(window, cx) {
            return;
        }
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        if let Some(detail) = self.search_results_detail.as_mut() {
            detail.move_focus(step, &lookup);
        }
        cx.notify();
    }

    /// Space, for the search list.
    pub(super) fn toggle_focused_search_row(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.search_list_has_focus(window, cx) {
            return;
        }
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        if let Some(detail) = self.search_results_detail.as_mut() {
            detail.toggle_focused(&lookup);
        }
        cx.notify();
    }

    /// Ctrl+A, for the search list.
    pub(super) fn select_all_search_rows(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.search_list_has_focus(window, cx) {
            return;
        }
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        if let Some(detail) = self.search_results_detail.as_mut() {
            detail.select_all(&lookup);
        }
        cx.notify();
    }

    /// Enter, for the search list: opens the page of the row with the
    /// keyboard focus. Gives `true` when it opened a page.
    pub(super) fn open_focused_search_row(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.search_list_has_focus(window, cx) {
            return false;
        }
        let Some((tab, id)) = self.focused_search_row_id(cx) else {
            return false;
        };
        self.handle_search_result_selected(tab, &id, cx);
        true
    }

    /// The tab and the row id of the row with the keyboard focus.
    fn focused_search_row_id(&self, cx: &gpui::App) -> Option<(SearchResultsTab, String)> {
        let detail = self.search_results_detail.as_ref()?;
        let focused = detail.selection().focused()?.clone();
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        detail
            .visible_rows(&lookup)
            .into_iter()
            .find(|row| row.key.as_ref() == Some(&focused))
            .map(|row| (detail.tab(), row.id))
    }

    /// Escape, for the search list. Gives `false` outside selection mode,
    /// so the caller keeps the Back action of Escape.
    pub(super) fn end_search_selecting(&mut self, window: &Window, cx: &mut Context<Self>) -> bool {
        if !self.search_list_has_focus(window, cx) {
            return false;
        }
        let ended = self
            .search_results_detail
            .as_mut()
            .is_some_and(SearchResultsInspectorPageVm::end_selecting);
        if ended {
            cx.notify();
        }
        ended
    }
}
