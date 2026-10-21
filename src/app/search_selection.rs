//! The selection of the search list in the Music list column (ADR 0084
//! Decision 10, packet 010).
//!
//! The search page view model holds the one selection of the search list.
//! The top app composes the list: the keyboard focus, the "Select" button,
//! the clicks and the action bar. The actions use the functions that the
//! detail column also uses, with the rows and the selection of the search
//! list. The saved-search page of the detail column (packet 012) uses the
//! same wiring through [`SearchPageSlot::SavedSearch`].

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, Context, Modifiers, SharedString, Window};

use super::keyboard::primary_modifier_held;
use super::TopApp;
use crate::library::playlist_options;
use crate::ui::composites::{SelectionActionBar, SelectionActionBarPlaylists};
use crate::view_models::detail_selection::{DetailPageRows, DetailRowClick, DetailRowKey};
use crate::view_models::list_selection::FocusStep;
use crate::view_models::search_results::{
    SavedSearchPage, SearchResultsInspectorPageVm, SearchResultsTab,
};
use crate::view_models::selection_action_bar::{SelectionActionBarVm, SelectionActionKind};

/// The search page that a click or a key acts on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SearchPageSlot {
    /// The toolbar search in the list column (packet 010).
    ToolbarSearch,
    /// The saved-search page in the detail column (packet 012).
    SavedSearch,
}

impl SearchPageSlot {
    const fn element_id(self) -> &'static str {
        match self {
            Self::ToolbarSearch => "music-search-list",
            Self::SavedSearch => "music-saved-search-page",
        }
    }
}

/// One click on the body of a search row.
pub(super) struct SearchRowClick {
    pub(super) tab: SearchResultsTab,
    pub(super) result_id: String,
    pub(super) key: Option<DetailRowKey>,
    pub(super) modifiers: Modifiers,
}

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
    /// The view model of the search page `slot`.
    pub(super) fn search_page(
        &self,
        slot: SearchPageSlot,
    ) -> Option<&SearchResultsInspectorPageVm> {
        match slot {
            SearchPageSlot::ToolbarSearch => self.search_results_detail.as_ref(),
            SearchPageSlot::SavedSearch => self
                .saved_search_page
                .as_ref()
                .and_then(SavedSearchPage::results),
        }
    }

    /// The mutable view model of the search page `slot`.
    pub(super) fn search_page_mut(
        &mut self,
        slot: SearchPageSlot,
    ) -> Option<&mut SearchResultsInspectorPageVm> {
        match slot {
            SearchPageSlot::ToolbarSearch => self.search_results_detail.as_mut(),
            SearchPageSlot::SavedSearch => self
                .saved_search_page
                .as_mut()
                .and_then(SavedSearchPage::results_mut),
        }
    }

    const fn search_page_focus(&self, slot: SearchPageSlot) -> &gpui::FocusHandle {
        match slot {
            SearchPageSlot::ToolbarSearch => &self.search_list_focus,
            SearchPageSlot::SavedSearch => &self.saved_search_focus,
        }
    }

    /// The search page that carries the keyboard focus, when one does.
    fn focused_search_page(&self, window: &Window, cx: &gpui::App) -> Option<SearchPageSlot> {
        [SearchPageSlot::ToolbarSearch, SearchPageSlot::SavedSearch]
            .into_iter()
            .find(|slot| {
                self.search_page(*slot).is_some()
                    && self.search_page_focus(*slot).contains_focused(window, cx)
            })
    }

    /// A click on the body of a search row. A row with a key changes the
    /// selection by the rules of packet 018, and a plain click in `Normal`
    /// mode opens its page. A row with no key opens its page in `Normal`
    /// mode only.
    pub(super) fn click_search_result_row(
        &mut self,
        slot: SearchPageSlot,
        click: SearchRowClick,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SearchRowClick {
            tab,
            result_id,
            key,
            modifiers,
        } = click;
        window.focus(&self.search_page_focus(slot).clone(), cx);
        let open = {
            let library = self.library.read(cx);
            let lookup = library.search_lookup();
            let page = page_mut(
                slot,
                &mut self.search_results_detail,
                &mut self.saved_search_page,
            );
            match (key, page) {
                (Some(key), Some(detail)) => detail.click_row(&key, row_click(modifiers), &lookup),
                (None, Some(detail)) => !detail.selection().mode().is_selecting(),
                (_, None) => false,
            }
        };
        if open {
            self.handle_search_result_selected(slot, tab, &result_id, cx);
        }
        cx.notify();
    }

    /// A click on the checkbox of a search row.
    pub(super) fn toggle_search_result_row(
        &mut self,
        slot: SearchPageSlot,
        key: &DetailRowKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.search_page_focus(slot).clone(), cx);
        if let Some(detail) = self.search_page_mut(slot) {
            detail.toggle_row(key);
        }
        cx.notify();
    }

    /// The "Select" and "Done" button of a search page.
    pub(super) fn toggle_search_selection_mode(
        &mut self,
        slot: SearchPageSlot,
        cx: &mut Context<Self>,
    ) {
        if let Some(detail) = self.search_page_mut(slot) {
            detail.toggle_mode();
        }
        cx.notify();
    }

    /// The selectable rows of the search list, with the Library facts of
    /// this moment. `None` on the Artists tab.
    fn search_selection_rows(
        &self,
        slot: SearchPageSlot,
        cx: &gpui::App,
    ) -> Option<DetailPageRows> {
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        self.search_page(slot)?.selection_rows(&lookup)
    }

    /// The search list with its keyboard focus and its action bar. A
    /// click in the list gives the list the keyboard focus. The bar shows
    /// at the bottom of the list column, in selection mode only.
    pub(super) fn render_search_list_frame(
        &self,
        slot: SearchPageSlot,
        content: AnyElement,
        bar: Option<AnyElement>,
    ) -> AnyElement {
        div()
            .id(slot.element_id())
            .track_focus(self.search_page_focus(slot))
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
        slot: SearchPageSlot,
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
        let download = cx.listener(move |this, (): &(), window, cx| {
            this.run_search_selection_action(slot, SelectionActionKind::Download, window, cx);
        });
        let remove = cx.listener(move |this, (): &(), window, cx| {
            this.run_search_selection_action(slot, SelectionActionKind::Remove, window, cx);
        });
        let clear = cx.listener(move |this, (): &(), window, cx| {
            this.run_search_selection_action(slot, SelectionActionKind::Clear, window, cx);
        });
        SelectionActionBar::new(bar, playlists)
            .on_download(move |window, cx| download(&(), window, cx))
            .on_remove(move |window, cx| remove(&(), window, cx))
            .on_clear(move |window, cx| clear(&(), window, cx))
            .on_add_to_playlist(cx.listener(move |this, playlist_id: &i64, _window, cx| {
                let selection = this
                    .search_page(slot)
                    .map(|detail| detail.selection().clone());
                if let (Some(rows), Some(selection)) =
                    (this.search_selection_rows(slot, cx), selection)
                {
                    this.add_selection_to_playlist(&rows, &selection, *playlist_id, cx);
                }
            }))
            .on_create_playlist(cx.listener(move |this, name: &String, _window, cx| {
                let selection = this
                    .search_page(slot)
                    .map(|detail| detail.selection().clone());
                if let (Some(rows), Some(selection)) =
                    (this.search_selection_rows(slot, cx), selection)
                {
                    this.create_playlist_and_add_selection(&rows, &selection, name, cx);
                }
            }))
            .into_any_element()
    }

    /// Runs "Download", "Remove…" or "Clear" of the search bar. The
    /// selection stays after each action.
    fn run_search_selection_action(
        &mut self,
        slot: SearchPageSlot,
        kind: SelectionActionKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rows) = self.search_selection_rows(slot, cx) else {
            return;
        };
        let Some(selection) = self
            .search_page(slot)
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
                if let Some(detail) = self.search_page_mut(slot) {
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

    /// Up, Down, Page Up, Page Down, Home and End, for the search list.
    pub(super) fn move_search_selection_focus(
        &mut self,
        step: FocusStep,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self.focused_search_page(window, cx) else {
            return;
        };
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        if let Some(detail) = page_mut(
            slot,
            &mut self.search_results_detail,
            &mut self.saved_search_page,
        ) {
            detail.move_focus(step, &lookup);
        }
        cx.notify();
    }

    /// Space, for the search list.
    pub(super) fn toggle_focused_search_row(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some(slot) = self.focused_search_page(window, cx) else {
            return;
        };
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        if let Some(detail) = page_mut(
            slot,
            &mut self.search_results_detail,
            &mut self.saved_search_page,
        ) {
            detail.toggle_focused(&lookup);
        }
        cx.notify();
    }

    /// Ctrl+A, for the search list.
    pub(super) fn select_all_search_rows(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some(slot) = self.focused_search_page(window, cx) else {
            return;
        };
        let library = self.library.read(cx);
        let lookup = library.search_lookup();
        if let Some(detail) = page_mut(
            slot,
            &mut self.search_results_detail,
            &mut self.saved_search_page,
        ) {
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
        let Some(slot) = self.focused_search_page(window, cx) else {
            return false;
        };
        let Some((tab, id)) = self.focused_search_row_id(slot, cx) else {
            return false;
        };
        self.handle_search_result_selected(slot, tab, &id, cx);
        true
    }

    /// The tab and the row id of the row with the keyboard focus.
    fn focused_search_row_id(
        &self,
        slot: SearchPageSlot,
        cx: &gpui::App,
    ) -> Option<(SearchResultsTab, String)> {
        let detail = self.search_page(slot)?;
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
        let Some(slot) = self.focused_search_page(window, cx) else {
            return false;
        };
        let ended = self
            .search_page_mut(slot)
            .is_some_and(SearchResultsInspectorPageVm::end_selecting);
        if ended {
            cx.notify();
        }
        ended
    }
}

/// The mutable view model of the search page `slot`, from the two fields.
/// A caller that holds a read of the Library entity uses this, so the
/// borrow of the two fields stays apart from that read.
fn page_mut<'a>(
    slot: SearchPageSlot,
    toolbar: &'a mut Option<SearchResultsInspectorPageVm>,
    saved: &'a mut Option<SavedSearchPage>,
) -> Option<&'a mut SearchResultsInspectorPageVm> {
    match slot {
        SearchPageSlot::ToolbarSearch => toolbar.as_mut(),
        SearchPageSlot::SavedSearch => saved.as_mut().and_then(SavedSearchPage::results_mut),
    }
}
