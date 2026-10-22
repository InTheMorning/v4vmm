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

use gpui::{
    point, px, AnyElement, App, Context, FocusHandle, Modifiers, Pixels, ScrollHandle, Window,
};

use super::{LibraryApp, LibraryDetail};
use crate::app::keyboard::primary_modifier_held;
use crate::db::PlaylistEntryKey;
use crate::ui::composites::music_column_scroll::{key_scroll, ScrollReveal};
use crate::ui::shells::library::detail::DetailRowSlot;
use crate::ui::tokens::Size;
use crate::view_models::detail_selection::{
    discover_release_rows, discover_row_key, library_release_track_rows, playlist_entry_rows,
    singles_group_release_rows, DetailPageRows, DetailRowClick, DetailRowKey, DetailSelection,
    LibraryReleaseFacts,
};
use crate::view_models::library::{ContentListSelectionTarget, PlaylistDetailVm};
use crate::view_models::list_selection::{FocusStep, RowSelectionVm};
use crate::view_models::workspace::{
    DetailScrollTarget, DiscoverView, FrameNavigationEntry, NavigationLocationId,
    NavigationPathStart, ReleaseKey,
};

/// The scroll binding of the detail column to the open history entry (ADR
/// 0084 Decision 9, amended on 2026-10-21).
///
/// A move to another history entry stores the offset of the entry that
/// closes and sets the offset of the new entry. GPUI keeps an offset in
/// the length of the page, so a shorter page shows its end. A page that
/// loads after its first draw gets the stored offset when it becomes long
/// enough, until the operator scrolls it.
#[derive(Debug, Default)]
pub(crate) struct DetailScrollBinding {
    bound: Option<NavigationLocationId>,
    /// The restored offset that the page cannot hold yet.
    pending: Option<Pixels>,
}

impl DetailScrollBinding {
    /// Binds `handle` to `target`. Gives the history entry that closed and
    /// its offset, in whole pixels, when the entry changes.
    pub(crate) fn bind(
        &mut self,
        handle: &ScrollHandle,
        target: DetailScrollTarget,
        entry_changed: bool,
    ) -> Option<(NavigationLocationId, u32)> {
        if entry_changed || self.bound != Some(target.location) {
            let closed = self.bound.map(|location| {
                // A pending offset is the one the closing page could not
                // hold yet. Store it, not the shorter offset that
                // happened to show (review of 2026-10-21).
                let offset = self.pending.unwrap_or_else(|| -handle.offset().y);
                (location, whole_pixels(offset))
            });
            let offset = whole_pixels_to_offset(target.offset);
            handle.set_offset(point(px(0.0), -offset));
            self.bound = Some(target.location);
            self.pending = (target.offset > 0).then_some(offset);
            return closed;
        }
        if let Some(offset) = self.pending {
            let current = -handle.offset().y;
            let max = handle.max_offset().y;
            if (current - offset.min(max)).abs() > px(1.0) {
                // The operator scrolled the page.
                self.pending = None;
            } else if max >= offset {
                self.pending = None;
            } else {
                handle.set_offset(point(px(0.0), -offset));
            }
        }
        None
    }

    /// Clears the pending offset (review of 2026-10-21). The operator
    /// interacted with the detail column: a scroll, a click or a key. A
    /// page that grows later, for example through the description
    /// toggle, must not jump to the offset the page could not hold when
    /// it opened.
    pub(crate) fn clear_pending(&mut self) {
        self.pending = None;
    }
}

/// The stored offset `offset` as a GPUI length.
#[expect(
    clippy::cast_precision_loss,
    reason = "an f32 holds each whole pixel below 2^24 exactly"
)]
fn whole_pixels_to_offset(offset: u32) -> Pixels {
    px(offset as f32)
}

/// The offset `offset` in whole pixels. A negative offset gives 0.
fn whole_pixels(offset: Pixels) -> u32 {
    let value = f32::from(offset).round();
    if value <= 0.0 {
        0
    } else {
        // The value is positive and below the length of a page.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a positive scroll offset fits in u32"
        )]
        let whole = value as u32;
        whole
    }
}

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
        // A page that opens from Discover keeps `LibraryDetail::Discover`.
        // Thus the open page entry decides the handle, not the detail kind.
        if self.discover_is_open() {
            self.recent_music_scroll.clone()
        } else {
            self.detail_column_scroll.clone()
        }
    }

    /// The request to keep the focused detail row in view. The Discover
    /// view uses its own request, on the scroll handle of the content list.
    pub(crate) fn detail_scroll_reveal(&self) -> &ScrollReveal {
        if self.discover_is_open() {
            &self.discover_scroll_reveal
        } else {
            &self.detail_scroll_reveal
        }
    }

    /// Tells if the open page is the Discover "New releases" view (packet
    /// 020). A release page or a feed page that opens from Discover keeps
    /// `LibraryDetail::Discover`, so this also reads the page entry of the
    /// detail selection. The top app binds that entry at each render.
    fn discover_is_open(&self) -> bool {
        matches!(
            (&self.detail, self.detail_selection.entry()),
            (
                LibraryDetail::Discover(DiscoverView::NewReleases),
                Some(FrameNavigationEntry::DiscoverView(
                    DiscoverView::NewReleases
                ))
            )
        )
    }

    /// The scroll probe of a detail row: `Some` only for the row with the
    /// keyboard focus, so the column can keep that row in view.
    pub(crate) fn detail_row_probe(&self, selection: &RowSelectionVm) -> Option<AnyElement> {
        selection
            .focused
            .then(|| self.detail_scroll_reveal().probe())
    }

    /// The Library facts of the release `key`.
    pub(crate) fn library_release_facts(&self, key: &ReleaseKey) -> Option<LibraryReleaseFacts> {
        self.vm.library_release_facts(key)
    }

    /// A page change (ADR 0084 packet 018): clears the detail selection and
    /// sets `Normal`. The scroll offset changes at the next
    /// [`Self::bind_detail_selection`], which first stores the offset of
    /// the page that closes.
    pub(crate) fn reset_detail_selection(&mut self, entry: &FrameNavigationEntry) {
        self.detail_selection.reset(Some(entry.clone()));
    }

    /// Binds the detail selection and the detail column offset to the open
    /// history entry. A move to another page clears the selection, the
    /// same as [`Self::reset_detail_selection`]. A move to another history
    /// entry shows the page at `target` (ADR 0084 Decision 9, amended on
    /// 2026-10-21). A new open gives the top. Back and Forward give the
    /// stored offset. The caller stores the returned offset of the entry
    /// that closed.
    pub(crate) fn bind_detail_selection(
        &mut self,
        entry: &FrameNavigationEntry,
        target: DetailScrollTarget,
    ) -> Option<(NavigationLocationId, u32)> {
        let entry_changed = self.detail_selection.bind(entry);
        self.detail_scroll_binding
            .bind(&self.detail_column_scroll, target, entry_changed)
    }

    /// Clears the pending restored offset of the detail column (review of
    /// 2026-10-21). Call this from each click and key entry point of the
    /// detail column, and from a click that can grow the open page, for
    /// example the description toggle. A page the operator already acted
    /// on must not later jump to an offset it could not hold when it
    /// opened.
    pub(crate) fn clear_pending_detail_scroll(&mut self) {
        self.detail_scroll_binding.clear_pending();
    }

    /// The selectable rows of a detail page that this entity renders: a
    /// Library release page, a singles group page, the Discover "New
    /// releases" view and a playlist page (packet 025). `None` for each
    /// other page.
    pub(crate) fn library_detail_page_rows(&self) -> Option<DetailPageRows> {
        match &self.detail {
            LibraryDetail::Album(album) => {
                Some(DetailPageRows::Tracks(library_release_track_rows(
                    album.release_track_rows(),
                    self.vm.busy_items().release_downloading(album),
                )))
            }
            LibraryDetail::SinglesGroup(key) => {
                let page = self.vm.singles_group_page(key);
                Some(DetailPageRows::Releases(singles_group_release_rows(
                    &page.rows,
                    |key| self.vm.library_release_facts(key),
                )))
            }
            LibraryDetail::Discover(DiscoverView::NewReleases) if self.discover_is_open() => {
                Some(DetailPageRows::Releases(discover_release_rows(
                    self.vm.content_list_page().visible_rows(),
                    |key| self.vm.library_release_facts(key),
                )))
            }
            LibraryDetail::Playlist(detail) => {
                let active = self.vm.release_download_active_track_ids();
                let vm = PlaylistDetailVm::new(&detail.playlist, &detail.tracks);
                Some(DetailPageRows::PlaylistEntries {
                    playlist_id: detail.playlist.id,
                    rows: playlist_entry_rows(&vm.visible_tracks(), |track| {
                        active.contains(&track.id)
                    }),
                })
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
        self.clear_pending_detail_scroll();
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

    /// A click on the body of a Discover row or tile (packet 020). A plain
    /// click in `Normal` mode opens the page of the row, the same as
    /// before. A row with no key opens its page in each mode.
    pub(crate) fn click_discover_row(
        &mut self,
        row_id: &str,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        let key = self.discover_row_key_of(row_id);
        let open = match (key, self.library_detail_page_rows()) {
            (Some(key), Some(rows)) => self.click_detail_row(&key, modifiers, &rows, cx),
            _ => true,
        };
        if open {
            self.open_content_list_row(row_id, cx);
        }
    }

    /// A click on the checkbox of a Discover row or tile (packet 020).
    pub(crate) fn toggle_discover_row(&mut self, row_id: &str, cx: &mut Context<Self>) {
        if let Some(key) = self.discover_row_key_of(row_id) {
            self.toggle_detail_row(&key, cx);
        }
    }

    fn discover_row_key_of(&self, row_id: &str) -> Option<DetailRowKey> {
        self.vm
            .content_list_page()
            .visible_rows()
            .into_iter()
            .find(|row| row.id == row_id)
            .and_then(discover_row_key)
    }

    /// A click on the body of a content row or tile (packet 020). The
    /// list column keeps its own selection methods. A Discover click goes
    /// to the detail selection of packet 018.
    pub(crate) fn click_content_row(
        &mut self,
        target: ContentListSelectionTarget,
        row_id: &str,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        match target {
            ContentListSelectionTarget::ListColumn => {
                self.click_content_list_row(row_id, modifiers, cx);
            }
            ContentListSelectionTarget::DetailColumn => {
                self.click_discover_row(row_id, modifiers, cx);
            }
        }
    }

    /// A click on the checkbox of a content row or tile (packet 020).
    pub(crate) fn toggle_content_row(
        &mut self,
        target: ContentListSelectionTarget,
        row_id: &str,
        cx: &mut Context<Self>,
    ) {
        match target {
            ContentListSelectionTarget::ListColumn => {
                self.toggle_content_list_row_selection(row_id, cx);
            }
            ContentListSelectionTarget::DetailColumn => self.toggle_discover_row(row_id, cx),
        }
    }

    /// A click on the body of a playlist page row (packet 025). Gives
    /// `true` when the caller must open the track page: a plain click in
    /// `Normal` mode.
    pub(crate) fn click_playlist_entry(
        &mut self,
        entry: PlaylistEntryKey,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(rows) = self.library_detail_page_rows() else {
            return true;
        };
        self.click_detail_row(&DetailRowKey::PlaylistEntry(entry), modifiers, &rows, cx)
    }

    /// A click on the checkbox of a playlist page row (packet 025).
    pub(crate) fn toggle_playlist_entry(
        &mut self,
        entry: PlaylistEntryKey,
        cx: &mut Context<Self>,
    ) {
        self.toggle_detail_row(&DetailRowKey::PlaylistEntry(entry), cx);
    }

    /// Opens the track page of the playlist entry `entry`: Enter on a
    /// playlist page (packet 025).
    pub(crate) fn open_playlist_entry(&mut self, entry: PlaylistEntryKey, cx: &mut Context<Self>) {
        let track = self
            .library_detail_page_rows()
            .and_then(|rows| rows.playlist_entry(&entry).map(|row| row.track.clone()));
        if let Some(track) = track {
            self.open_page(
                FrameNavigationEntry::Track(track),
                NavigationPathStart::CurrentPath,
                cx,
            );
        }
    }

    /// A click on the checkbox of a detail row.
    pub(crate) fn toggle_detail_row(&mut self, key: &DetailRowKey, cx: &mut Context<Self>) {
        self.detail_selection.toggle_row(key);
        self.clear_pending_detail_scroll();
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
        self.clear_pending_detail_scroll();
        match rows.filter(|rows| rows.has_selectable_rows()) {
            Some(rows) => {
                if self
                    .detail_selection
                    .move_focus(step, &rows.selection_rows())
                    .is_some()
                {
                    self.detail_scroll_reveal().request();
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
        self.clear_pending_detail_scroll();
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
        self.clear_pending_detail_scroll();
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
            self.clear_pending_detail_scroll();
            cx.notify();
        }
        ended
    }
}

#[cfg(test)]
mod tests {
    use gpui::{point, px, ScrollHandle};

    use super::DetailScrollBinding;
    use crate::view_models::workspace::{
        FrameNavigationEntry, FrameNavigationState, MusicDestination, TrackKey,
    };

    fn track(id: i64) -> FrameNavigationEntry {
        FrameNavigationEntry::Track(TrackKey::from_index_track_target(None, id.to_string()))
    }

    /// Binds the open entry of `nav` and stores the offset of the entry
    /// that closed, as the top app does at each render.
    fn bind(
        binding: &mut DetailScrollBinding,
        handle: &ScrollHandle,
        nav: &mut FrameNavigationState,
        entry_changed: bool,
    ) {
        if let Some((location, offset)) =
            binding.bind(handle, nav.detail_scroll_target(), entry_changed)
        {
            nav.record_detail_scroll(location, offset);
        }
    }

    fn offset(handle: &ScrollHandle) -> gpui::Pixels {
        -handle.offset().y
    }

    /// R30-11 (ADR 0084 Decision 9, amended on 2026-10-21): a second page
    /// of the same type opens at the top. The same entry again does not
    /// change the offset. Back and Forward show the stored offset of the
    /// history entry. A new open of a page that is in the history shows it
    /// at the top.
    #[test]
    fn adr_0084_music_open_defects_r30_11_offset_for_each_history_entry() {
        let handle = ScrollHandle::new();
        let mut binding = DetailScrollBinding::default();
        let mut nav =
            FrameNavigationState::new(FrameNavigationEntry::Destination(MusicDestination::Tracks));
        nav.push(track(1));
        bind(&mut binding, &handle, &mut nav, true);
        assert_eq!(offset(&handle), px(0.0));

        // The operator scrolls the first track page. The same entry again
        // keeps the offset.
        handle.set_offset(point(px(0.0), px(-400.0)));
        bind(&mut binding, &handle, &mut nav, false);
        assert_eq!(offset(&handle), px(400.0));

        // A second track page opens at the top.
        nav.push(track(2));
        bind(&mut binding, &handle, &mut nav, true);
        assert_eq!(offset(&handle), px(0.0));
        handle.set_offset(point(px(0.0), px(-150.0)));

        // Back shows the first page at its stored offset.
        nav.go_back().unwrap();
        bind(&mut binding, &handle, &mut nav, true);
        assert_eq!(offset(&handle), px(400.0));

        // Forward shows the second page at its stored offset.
        nav.go_forward().unwrap();
        bind(&mut binding, &handle, &mut nav, true);
        assert_eq!(offset(&handle), px(150.0));

        // A new open of the first page shows it at the top.
        nav.push(track(1));
        bind(&mut binding, &handle, &mut nav, true);
        assert_eq!(offset(&handle), px(0.0));
    }

    /// Review of 2026-10-21, P5 point 2: a page can close while its
    /// restored offset is still pending, because the page cannot hold it
    /// yet. The close must store the pending offset, not the shorter
    /// offset that happened to show.
    #[test]
    fn adr_0084_music_open_defects_review_closing_a_pending_offset_keeps_it_full() {
        let handle = ScrollHandle::new();
        let mut binding = DetailScrollBinding::default();
        let mut nav =
            FrameNavigationState::new(FrameNavigationEntry::Destination(MusicDestination::Tracks));
        nav.push(track(1));
        bind(&mut binding, &handle, &mut nav, true);
        handle.set_offset(point(px(0.0), px(-400.0)));
        bind(&mut binding, &handle, &mut nav, false);

        nav.push(track(2));
        bind(&mut binding, &handle, &mut nav, true);

        // Back shows track 1 at its stored offset of 400. The page is
        // short today, so it can only show 120 (the real element clamps
        // the handle to the length of its content).
        nav.go_back().unwrap();
        bind(&mut binding, &handle, &mut nav, true);
        handle.set_offset(point(px(0.0), px(-120.0)));

        // Track 1 closes again while its offset of 400 is still pending.
        nav.push(track(2));
        bind(&mut binding, &handle, &mut nav, true);
        nav.go_back().unwrap();
        bind(&mut binding, &handle, &mut nav, true);
        assert_eq!(
            offset(&handle),
            px(400.0),
            "a pending offset must survive the close of its page"
        );
    }

    /// Review of 2026-10-21, P5 point 3: an interaction with the detail
    /// column, for example a click on the description toggle, clears the
    /// pending offset. A page that closes after that interaction stores
    /// where it actually showed, not the offset it never reached.
    #[test]
    fn adr_0084_music_open_defects_review_an_interaction_clears_the_pending_offset() {
        let handle = ScrollHandle::new();
        let mut binding = DetailScrollBinding::default();
        let mut nav =
            FrameNavigationState::new(FrameNavigationEntry::Destination(MusicDestination::Tracks));
        nav.push(track(1));
        bind(&mut binding, &handle, &mut nav, true);
        handle.set_offset(point(px(0.0), px(-400.0)));
        bind(&mut binding, &handle, &mut nav, false);

        nav.push(track(2));
        bind(&mut binding, &handle, &mut nav, true);

        // Back shows track 1's stored offset of 400, but the page is
        // short today and can only show 120.
        nav.go_back().unwrap();
        bind(&mut binding, &handle, &mut nav, true);
        handle.set_offset(point(px(0.0), px(-120.0)));

        // The operator clicks the description toggle: an interaction
        // with the detail column, with no change of entry.
        binding.clear_pending();

        // Track 1 closes. The pending offset is gone, so the close
        // stores where the page actually showed, not the unmet 400.
        nav.push(track(2));
        bind(&mut binding, &handle, &mut nav, true);
        nav.go_back().unwrap();
        bind(&mut binding, &handle, &mut nav, true);
        assert_eq!(
            offset(&handle),
            px(120.0),
            "a cleared pending offset must not jump the page later"
        );
    }
}
