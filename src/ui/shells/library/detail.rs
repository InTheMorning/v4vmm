//! Library detail-pane dispatcher.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, Image, SharedString};

use crate::db;
use crate::library::{LibraryApp, LibraryDetail};
use crate::ui::shells::library::feed_detail::render_library_feed_detail;
use crate::ui::shells::library::feed_list::render_library_feed_list;
use crate::ui::shells::library::playlist_detail::render_library_playlist_detail;
use crate::ui::shells::library::track_detail::render_library_track_detail;
use crate::ui::style::color;
use crate::view_models::library::{LibraryChromeDisplay, MbTrackStatus};

pub(crate) fn render_library_detail(
    detail: &LibraryDetail,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    chrome: &LibraryChromeDisplay,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    match detail {
        LibraryDetail::None => div()
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

        LibraryDetail::Artist(artist) => render_library_feed_list(artist, album_thumbs, chrome, cx),

        LibraryDetail::Album(album) => {
            render_library_feed_detail(album, busy_track, mb_status, album_thumbs, playlists, cx)
        }

        LibraryDetail::Track(frame) => render_library_track_detail(frame, playlists, chrome, cx),

        LibraryDetail::Playlist(detail) => {
            render_library_playlist_detail(detail, album_thumbs, chrome, cx)
        }
    }
}
