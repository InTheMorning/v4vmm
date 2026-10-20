//! Library detail-pane dispatcher.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, Entity, Image, ScrollHandle, SharedString};
use gpui_component::input::InputState;

use crate::db;
use crate::library::PlaylistActorState;
use crate::library::{LibraryApp, LibraryDetail};
use crate::media::cover_color::CoverColor;
use crate::ui::composites::CollectionArtImage;
use crate::ui::shells::library::feed_detail::render_library_feed_detail;
use crate::ui::shells::library::feed_list::render_library_feed_list;
use crate::ui::shells::library::playlist_detail::{
    render_library_playlist_detail, LibraryPlaylistPage,
};
use crate::ui::shells::library::singles_group::render_library_singles_group_page;
use crate::ui::shells::library::track_detail::render_library_track_detail;
use crate::ui::style::color;
use crate::view_models::library::{LibraryChromeDisplay, LibraryViewModel, MbTrackStatus};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::named_feeds::NamedFeed;

/// The selection facts and the scroll probe of one row of a Library
/// detail page (ADR 0084 Decision 10, packet 018). `LibraryApp` builds one
/// for each row of a release page, of a singles group page and of a
/// playlist page (packet 025), in the visible order.
pub(crate) struct DetailRowSlot {
    pub(crate) selection: RowSelectionVm,
    pub(crate) probe: Option<AnyElement>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "render_library_detail dispatches to all detail-pane variants; \
              the `playlist_actor` argument exists to avoid GPUI re-entrancy."
)]
pub(crate) fn render_library_detail(
    detail: &LibraryDetail,
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
    header_art: Option<CollectionArtImage>,
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
            mb_status,
            library_vm,
            album_thumbs,
            album_cover_color,
            row_slots,
            scroll,
            cx,
        ),

        LibraryDetail::Track(frame) => {
            // The orchestrator correction of 2026-10-20: the view model
            // tells if a download of the release of the track is active.
            let release_download_active = library_vm.track_release_download_active(frame.track.id);
            render_library_track_detail(
                frame,
                playlists,
                chrome,
                track_named_feeds,
                release_download_active,
                scroll,
                cx,
            )
        }

        // ADR 0084 Decision 6: the group page reads the current tree at
        // each render, so a download or a removal updates it in place.
        LibraryDetail::SinglesGroup(key) => render_library_singles_group_page(
            library_vm.singles_group_page(key),
            header_art,
            mb_status,
            playlists,
            album_thumbs,
            row_slots,
            scroll,
            cx,
        ),

        LibraryDetail::Playlist(detail) => render_library_playlist_detail(
            LibraryPlaylistPage {
                detail,
                album_thumbs,
                header_art,
                chrome,
                rename_playlist_input,
                renaming_playlist_id,
                playlist_actor,
                playlist_rss_check,
                row_slots,
                scroll,
            },
            cx,
        ),
    }
}
