//! Library playlist detail surface.
//!
//! Renders the right-hand pane when a playlist is selected. The shared playlist
//! shell owns the page hierarchy; this module wires Library-specific callbacks
//! back to `LibraryApp`.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{AnyElement, Context, Image};

use crate::library::{LibraryApp, LibraryAppEvent, PlaylistDetail};
use crate::ui::shells::library::thumbnail::render_album_thumb;
use crate::ui::shells::playlist::{
    click_slot, render_playlist_detail_shell, PlaylistDetailBehaviorSlots, PlaylistShellRow,
    PlaylistTrackRowSlot,
};
use crate::view_models::library::{LibraryChromeDisplay, PlaylistDetailVm};

pub(crate) fn render_library_playlist_detail(
    detail: &PlaylistDetail,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    chrome: &LibraryChromeDisplay,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let page = PlaylistDetailVm::new(&detail.playlist, &detail.tracks)
        .page(chrome.playlist_detail_scroll_id);
    let playlist_id = page.playlist_id();
    let track_rows = page
        .track_rows()
        .into_iter()
        .map(|row| {
            let track_for_select = row.track().clone();
            let position = row.position();
            let thumbnail = row
                .thumb_url()
                .and_then(|url| album_thumbs.get(url))
                .cloned()
                .flatten();
            let display = row.display(playlist_id);

            let slot = PlaylistTrackRowSlot {
                thumbnail: Some(render_album_thumb(thumbnail, 24.0)),
                on_select: Some(click_slot(cx.listener(move |this, _, _, cx| {
                    this.select_track(&track_for_select, cx);
                    cx.notify();
                }))),
                on_play: Some(click_slot(cx.listener(move |_this, _, _, cx| {
                    cx.emit(LibraryAppEvent::PlayPlaylistAt {
                        playlist_id,
                        playlist_position: position,
                    });
                }))),
                on_move_up: Some(click_slot(cx.listener(move |this, _, _, cx| {
                    this.move_playlist_track(playlist_id, position, position - 1, cx);
                }))),
                on_move_down: Some(click_slot(cx.listener(move |this, _, _, cx| {
                    this.move_playlist_track(playlist_id, position, position + 1, cx);
                }))),
                on_remove: Some(click_slot(cx.listener(move |this, _, _, cx| {
                    this.remove_playlist_track_at(playlist_id, position, cx);
                }))),
            };
            PlaylistShellRow::Ready { display, slot }
        })
        .collect();

    render_playlist_detail_shell(
        &page,
        PlaylistDetailBehaviorSlots {
            on_rename: Some(click_slot(cx.listener(move |_this, _, _, cx| {
                // TODO Stage 3: implement inline rename modal/input
                cx.notify();
            }))),
            on_delete: Some(click_slot(cx.listener(move |this, _, _, cx| {
                this.delete_playlist(playlist_id, cx);
            }))),
            track_rows,
        },
    )
}
