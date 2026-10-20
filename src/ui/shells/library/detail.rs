//! Library detail-pane dispatcher.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, Entity, Image, ScrollHandle, SharedString,
};
use gpui_component::input::InputState;

use crate::db;
use crate::library::PlaylistActorState;
use crate::library::{LibraryApp, LibraryDetail};
use crate::media::cover_color::CoverColor;
use crate::ui::composites::TrackRow;
use crate::ui::shells::library::feed_detail::{render_library_feed_detail, LibraryTrackRowContext};
use crate::ui::shells::library::feed_list::render_library_feed_list;
use crate::ui::shells::library::playlist_detail::render_library_playlist_detail;
use crate::ui::shells::library::singles_group::render_library_singles_group_page;
use crate::ui::shells::library::track_detail::render_library_track_detail;
use crate::ui::style::color;
use crate::view_models::library::{LibraryChromeDisplay, LibraryViewModel, MbTrackStatus};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::named_feeds::NamedFeed;
use crate::view_models::workspace::FrameNavigationEntry;

/// The selection facts and the scroll probe of one row of a Library
/// detail page (ADR 0084 Decision 10, packet 018). `LibraryApp` builds one
/// for each row of a release page and of a singles group page, in the
/// visible order.
pub(crate) struct DetailRowSlot {
    pub(crate) selection: RowSelectionVm,
    pub(crate) probe: Option<AnyElement>,
}

/// A Library track row's page context with its detail selection facts
/// (ADR 0084 packet 018).
pub(crate) struct LibraryTrackRowPlace {
    pub(crate) context: LibraryTrackRowContext,
    pub(crate) slot: DetailRowSlot,
}

impl LibraryTrackRowContext {
    /// Joins this page context and the detail selection facts of the row.
    pub(crate) fn with_slot(self, slot: DetailRowSlot) -> LibraryTrackRowPlace {
        LibraryTrackRowPlace {
            context: self,
            slot,
        }
    }
}

/// The selection wiring of one row of a Library detail page (ADR 0084
/// Decision 10, packet 018): the checkbox, the selected fill, the focus
/// outline and the click. `open_entry` is the page that a plain click in
/// `Normal` mode opens. In `Selecting` mode, a plain click changes the
/// selection of the row.
pub(crate) fn with_detail_row_slot(
    row: TrackRow,
    slot: DetailRowSlot,
    open_entry: FrameNavigationEntry,
    cx: &mut Context<LibraryApp>,
) -> TrackRow {
    let toggle_entry = open_entry.clone();
    row.selection(slot.selection)
        .scroll_probe(slot.probe)
        .on_toggle_selection(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.toggle_library_detail_row(&toggle_entry, cx);
        }))
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.click_library_detail_row(&open_entry, event.modifiers(), cx);
        }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "render_library_detail dispatches to all detail-pane variants; \
              the `playlist_actor` argument exists to avoid GPUI re-entrancy."
)]
pub(crate) fn render_library_detail(
    detail: &LibraryDetail,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    library_vm: &LibraryViewModel,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    chrome: &LibraryChromeDisplay,
    rename_playlist_input: Entity<InputState>,
    renaming_playlist_id: Option<i64>,
    playlist_actor: Option<&PlaylistActorState>,
    playlist_rss_check: Option<&crate::runtime::PlaylistRssCheckSnapshot>,
    track_named_feeds: &[NamedFeed],
    album_cover_color: Option<CoverColor>,
    row_slots: Vec<DetailRowSlot>,
    scroll: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    match detail {
        // ADR 0084 Decision 3: `LibraryApp::render_detail_column` renders an
        // open Discover view before it reaches this dispatcher. If a later
        // path reaches this arm, it shows the empty state. The render path
        // never panics.
        LibraryDetail::None | LibraryDetail::Discover(_) => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(color::text_muted())
                    .text_center()
                    .child(SharedString::from(chrome.empty_detail_label)),
            )
            .into_any_element(),

        LibraryDetail::Artist(artist) => {
            render_library_feed_list(artist, album_thumbs, chrome, scroll, cx)
        }

        LibraryDetail::Album(album) => render_library_feed_detail(
            album,
            busy_track,
            mb_status,
            library_vm,
            album_thumbs,
            album_cover_color,
            playlists,
            row_slots,
            scroll,
            cx,
        ),

        LibraryDetail::Track(frame) => {
            render_library_track_detail(frame, playlists, chrome, track_named_feeds, scroll, cx)
        }

        // ADR 0084 Decision 6: the group page reads the current tree at
        // each render, so a download or a removal updates it in place.
        LibraryDetail::SinglesGroup(key) => render_library_singles_group_page(
            library_vm.singles_group_page(key),
            mb_status,
            playlists,
            album_thumbs,
            row_slots,
            scroll,
            cx,
        ),

        LibraryDetail::Playlist(detail) => render_library_playlist_detail(
            detail,
            album_thumbs,
            chrome,
            rename_playlist_input,
            renaming_playlist_id,
            playlist_actor,
            playlist_rss_check,
            cx,
        ),
    }
}
