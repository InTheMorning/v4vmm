//! The Music detail column selection of `LibraryApp` (ADR 0084 Decision
//! 10, packet 018).
//!
//! `LibraryApp` holds the one detail selection, its mode, the focus handle
//! and the scroll handle of the detail column. The pure rules are in
//! `src/view_models/detail_selection.rs`. The top app gives the rows of a
//! page that it renders itself: the Index release page and the feed page.
//! Each key method acts only while the detail column carries the keyboard
//! focus.

#![warn(clippy::pedantic)]

use gpui::{AnyElement, App, Context, FocusHandle, Modifiers, ScrollHandle, Window};

use super::{LibraryApp, LibraryDetail};
use crate::app::keyboard::primary_modifier_held;
use crate::ui::composites::music_column_scroll::{key_scroll, ScrollReveal};
use crate::ui::shells::library::detail::DetailRowSlot;
use crate::ui::tokens::Size;
use crate::view_models::detail_selection::{
    library_release_track_rows, singles_group_release_rows, DetailPageRows, DetailRowClick,
    DetailRowKey, DetailSelection, LibraryReleaseFacts,
};
use crate::view_models::list_selection::{FocusStep, RowSelectionVm};
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart, ReleaseKey};

impl LibraryApp {
    /// The focus handle of the Music detail column.
    pub(crate) fn detail_column_focus(&self) -> &FocusHandle {
        &self.detail_column_focus
    }

    /// The detail column selection.
    pub(crate) fn detail_selection(&self) -> &DetailSelection {
        &self.detail_selection
    }

    /// The held scroll handle of the open detail page. The Discover view
    /// keeps the handle of the content list, as before this packet.
    pub(crate) fn detail_scroll_handle(&self) -> ScrollHandle {
        match self.detail {
            LibraryDetail::Discover(_) => self.recent_music_scroll.clone(),
            _ => self.detail_column_scroll.clone(),
        }
    }

    /// The request to keep the focused detail row in view.
    pub(crate) fn detail_scroll_reveal(&self) -> &ScrollReveal {
        &self.detail_scroll_reveal
    }

    /// The scroll probe of a detail row: `Some` only for the row with the
    /// keyboard focus, so the column can keep that row in view.
    pub(crate) fn detail_row_probe(&self, selection: &RowSelectionVm) -> Option<AnyElement> {
        selection.focused.then(|| self.detail_scroll_reveal.probe())
    }

    /// The Library facts of the release `key`.
    pub(crate) fn library_release_facts(&self, key: &ReleaseKey) -> Option<LibraryReleaseFacts> {
        self.vm.library_release_facts(key)
    }

    /// A page change (ADR 0084 packet 018): clears the detail selection,
    /// sets `Normal`, and scrolls the detail column to the top.
    pub(crate) fn reset_detail_selection(&mut self, entry: &FrameNavigationEntry) {
        self.detail_selection.reset(Some(entry.clone()));
        self.detail_column_scroll.set_offset(gpui::Point::default());
    }

    /// Binds the detail selection to the open page `entry`. A move to
    /// another page clears it, the same as [`Self::reset_detail_selection`].
    pub(crate) fn bind_detail_selection(&mut self, entry: &FrameNavigationEntry) {
        if self.detail_selection.bind(entry) {
            self.detail_column_scroll.set_offset(gpui::Point::default());
        }
    }

    /// The selectable rows of a detail page that this entity renders: a
    /// Library release page and a singles group page. `None` for each
    /// other page.
    pub(crate) fn library_detail_page_rows(&self) -> Option<DetailPageRows> {
        match &self.detail {
            LibraryDetail::Album(album) => Some(DetailPageRows::Tracks(
                library_release_track_rows(album.release_track_rows()),
            )),
            LibraryDetail::SinglesGroup(key) => {
                let page = self.vm.singles_group_page(key);
                Some(DetailPageRows::Releases(singles_group_release_rows(
                    &page.rows,
                    |key| self.vm.library_release_facts(key),
                )))
            }
            _ => None,
        }
    }

    /// A click on the body of a detail row. Gives `true` when the caller
    /// must open the page of the row: a plain click in `Normal` mode.
    pub(crate) fn click_detail_row(
        &mut self,
        key: &DetailRowKey,
        modifiers: Modifiers,
        rows: &DetailPageRows,
        cx: &mut Context<Self>,
    ) -> bool {
        let click = if primary_modifier_held(modifiers) {
            DetailRowClick::Toggle
        } else if modifiers.shift {
            DetailRowClick::Extend
        } else {
            DetailRowClick::Plain
        };
        let open = self
            .detail_selection
            .click(key, click, &rows.selection_rows());
        cx.notify();
        open
    }

    /// A click on the body of a row of a Library detail page. The row key
    /// comes from `open_entry`, the page that a plain click in `Normal`
    /// mode opens.
    pub(crate) fn click_library_detail_row(
        &mut self,
        open_entry: &FrameNavigationEntry,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        let (Some(key), Some(rows)) = (
            DetailRowKey::for_open_entry(open_entry),
            self.library_detail_page_rows(),
        ) else {
            return;
        };
        if self.click_detail_row(&key, modifiers, &rows, cx) {
            self.open_page(open_entry.clone(), NavigationPathStart::CurrentPath, cx);
        }
    }

    /// A click on the checkbox of a detail row.
    pub(crate) fn toggle_detail_row(&mut self, key: &DetailRowKey, cx: &mut Context<Self>) {
        self.detail_selection.toggle_row(key);
        cx.notify();
    }

    /// A click on the checkbox of a row of a Library detail page.
    pub(crate) fn toggle_library_detail_row(
        &mut self,
        open_entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        if let Some(key) = DetailRowKey::for_open_entry(open_entry) {
            self.toggle_detail_row(&key, cx);
        }
    }

    /// The selection facts and the scroll probe of each row of a Library
    /// detail page, in the visible order. Empty for each other page.
    pub(crate) fn library_detail_row_slots(&self) -> Vec<DetailRowSlot> {
        self.library_detail_page_rows()
            .map(|rows| {
                self.detail_selection
                    .row_selections(&rows)
                    .into_iter()
                    .map(|selection| DetailRowSlot {
                        probe: self.detail_row_probe(&selection),
                        selection,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The "Select" and "Done" button of the detail column header.
    pub(crate) fn toggle_detail_selection_mode(&mut self, cx: &mut Context<Self>) {
        self.detail_selection.toggle_mode();
        cx.notify();
    }

    /// "Clear" of the detail bar: clears the rows and keeps the mode.
    pub(crate) fn clear_detail_rows(&mut self, cx: &mut Context<Self>) {
        self.detail_selection.clear_rows();
        cx.notify();
    }

    /// Up, Down, Page Up, Page Down, Home and End in the detail column.
    /// On a page with selectable rows, they move the row focus and keep
    /// the focused row in view. On each other page, they scroll the page.
    pub(crate) fn move_detail_focus(
        &mut self,
        step: FocusStep,
        rows: Option<&DetailPageRows>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if !self.detail_column_focus.contains_focused(window, cx) {
            return;
        }
        match rows.filter(|rows| rows.has_selectable_rows()) {
            Some(rows) => {
                if self
                    .detail_selection
                    .move_focus(step, &rows.selection_rows())
                    .is_some()
                {
                    self.detail_scroll_reveal.request();
                }
            }
            None => key_scroll(
                &self.detail_scroll_handle(),
                step,
                Size::KeyScrollStep.scaled(cx),
            ),
        }
        cx.notify();
    }

    /// Space in the detail column: starts selection mode and changes the
    /// row with the keyboard focus.
    pub(crate) fn toggle_focused_detail_row(
        &mut self,
        rows: Option<&DetailPageRows>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if !self.detail_column_focus.contains_focused(window, cx) {
            return;
        }
        let Some(rows) = rows.filter(|rows| rows.has_selectable_rows()) else {
            return;
        };
        self.detail_selection.toggle_focused(&rows.selection_rows());
        cx.notify();
    }

    /// Ctrl+A in the detail column: starts selection mode and selects each
    /// row.
    pub(crate) fn select_all_detail_rows(
        &mut self,
        rows: Option<&DetailPageRows>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if !self.detail_column_focus.contains_focused(window, cx) {
            return;
        }
        let Some(rows) = rows.filter(|rows| rows.has_selectable_rows()) else {
            return;
        };
        self.detail_selection.select_all(&rows.selection_rows());
        cx.notify();
    }

    /// Enter in the detail column: the row with the keyboard focus, when
    /// the detail column carries the focus.
    pub(crate) fn focused_detail_row(&self, window: &Window, cx: &App) -> Option<DetailRowKey> {
        if !self.detail_column_focus.contains_focused(window, cx) {
            return None;
        }
        self.detail_selection.focused().cloned()
    }

    /// Escape in the detail column: ends selection mode and clears the
    /// selection. Gives `false` outside selection mode, so the caller
    /// keeps the Back action of Escape.
    pub(crate) fn end_detail_selecting(&mut self, window: &Window, cx: &mut Context<Self>) -> bool {
        if !self.detail_column_focus.contains_focused(window, cx) {
            return false;
        }
        let ended = self.detail_selection.end_selecting();
        if ended {
            cx.notify();
        }
        ended
    }
}
