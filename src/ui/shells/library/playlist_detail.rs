//! Library playlist detail surface.
//!
//! Renders the right-hand pane when a playlist is selected. The shared playlist
//! shell owns the page hierarchy; this module wires Library-specific callbacks
//! back to `LibraryApp`.
//!
//! When the async runtime is enabled and a [`PagedTrackListActor`] for the
//! selected playlist has published a snapshot, the screen is rendered via
//! [`render_paged_playlist_detail`]: rows the actor has fulfilled paint as
//! `Ready`, the rest paint as `Pending` skeletons. The actor receives a
//! single `ReportVisible` per render so its prefetch policy can stay ahead
//! of scroll without the screen having to track the visible window directly.
//!
//! [`PagedTrackListActor`]: crate::application::paged_track_list::PagedTrackListActor

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{AnyElement, Context, Entity, Image};
use gpui_component::input::InputState;

#[cfg(feature = "async-runtime")]
use crate::library::PlaylistActorState;
use crate::library::{LibraryApp, LibraryAppEvent, PlaylistDetail};
use crate::ui::shells::library::thumbnail::render_album_thumb;
use crate::ui::shells::playlist::{
    click_slot, render_playlist_detail_shell, PlaylistDetailBehaviorSlots, PlaylistShellReadyRow,
    PlaylistShellRow, PlaylistTrackRowSlot,
};
use crate::view_models::library::{LibraryChromeDisplay, PlaylistDetailVm};

pub(crate) fn render_library_playlist_detail(
    detail: &PlaylistDetail,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    chrome: &LibraryChromeDisplay,
    rename_playlist_input: Entity<InputState>,
    renaming_playlist_id: Option<i64>,
    #[cfg(feature = "async-runtime")] playlist_actor: Option<&PlaylistActorState>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    #[cfg(feature = "async-runtime")]
    if let Some(state) = playlist_actor {
        if let Some(rendered) = try_render_paged(
            detail,
            state,
            album_thumbs,
            chrome,
            rename_playlist_input.clone(),
            renaming_playlist_id,
            cx,
        ) {
            return rendered;
        }
    }

    render_eager_playlist_detail(
        detail,
        album_thumbs,
        chrome,
        rename_playlist_input,
        renaming_playlist_id,
        cx,
    )
}

fn render_eager_playlist_detail(
    detail: &PlaylistDetail,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    chrome: &LibraryChromeDisplay,
    rename_playlist_input: Entity<InputState>,
    renaming_playlist_id: Option<i64>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let page = PlaylistDetailVm::new(&detail.playlist, &detail.tracks)
        .page(chrome.playlist_detail_scroll_id);
    let playlist_id = page.playlist_id();
    let playlist_name = detail.playlist.name.clone();
    let rename_placeholder = page.actions_display().rename_input_placeholder;
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
            let on_play = display.controls.play_enabled.then(|| {
                click_slot(cx.listener(move |_this, _, _, cx| {
                    cx.emit(LibraryAppEvent::PlayPlaylistAt {
                        playlist_id,
                        playlist_position: position,
                    });
                }))
            });

            let slot = PlaylistTrackRowSlot {
                thumbnail: Some(render_album_thumb(thumbnail, 24.0)),
                on_select: Some(click_slot(cx.listener(move |this, _, _, cx| {
                    this.select_track(&track_for_select, cx);
                    cx.notify();
                }))),
                on_play,
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
            PlaylistShellRow::Ready(Box::new(PlaylistShellReadyRow { display, slot }))
        })
        .collect();

    render_playlist_detail_shell(
        &page,
        PlaylistDetailBehaviorSlots {
            on_rename: Some(click_slot(cx.listener(move |this, _, window, cx| {
                this.begin_playlist_rename(
                    playlist_id,
                    playlist_name.clone(),
                    rename_placeholder,
                    window,
                    cx,
                );
            }))),
            rename_input: Some(rename_playlist_input),
            renaming: renaming_playlist_id == Some(playlist_id),
            on_submit_rename: Some(click_slot(cx.listener(move |this, _, _, cx| {
                this.submit_playlist_rename(cx);
            }))),
            on_cancel_rename: Some(click_slot(cx.listener(move |this, _, _, cx| {
                this.cancel_playlist_rename(cx);
            }))),
            on_delete: Some(click_slot(cx.listener(move |this, _, _, cx| {
                this.delete_playlist(playlist_id, cx);
            }))),
            track_rows,
        },
        cx,
    )
}

#[cfg(feature = "async-runtime")]
fn try_render_paged(
    detail: &PlaylistDetail,
    state: &PlaylistActorState,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    chrome: &LibraryChromeDisplay,
    rename_playlist_input: Entity<InputState>,
    renaming_playlist_id: Option<i64>,
    cx: &mut Context<LibraryApp>,
) -> Option<AnyElement> {
    use crate::application::paged_track_list::PagedTrackListMsg;
    use crate::view_models::paged_playlist_detail::{PagedPlaylistDetailVm, PagedPlaylistRow};

    let playlist_id = detail.playlist.id;
    if state.playlist_id != playlist_id {
        return None;
    }
    let backing = state.snapshot.clone();
    let inbox = state.handle.clone();

    let page = PlaylistDetailVm::new(&detail.playlist, &detail.tracks)
        .page(chrome.playlist_detail_scroll_id);
    let playlist_name = detail.playlist.name.clone();
    let rename_placeholder = page.actions_display().rename_input_placeholder;
    let paged = PagedPlaylistDetailVm::new(&detail.playlist, &backing);
    let total = paged.track_count();

    // Hint the actor about the visible window. Until the table delegate
    // exposes scroll position, request the first page-worth of rows so
    // the cold render hydrates quickly; subsequent renders that touch
    // higher indices will widen the request as the user scrolls.
    if total > 0 {
        let initial_window = total.min(crate::runtime::paged_list_vm::DEFAULT_PAGE_SIZE);
        let _ = inbox.try_send(PagedTrackListMsg::ReportVisible(0..initial_window));
    }

    let mut track_rows: Vec<PlaylistShellRow> = Vec::with_capacity(total);
    for position in 0..total {
        match paged.row(position) {
            PagedPlaylistRow::Pending { position } => {
                let last_position = total.saturating_sub(1);
                track_rows.push(PlaylistShellRow::Pending {
                    position,
                    last_position,
                });
                // Nudge the actor to fetch this row; the reactive watch
                // bridge will repaint when the page lands.
                let _ = inbox.try_send(PagedTrackListMsg::Read(position));
            }
            PagedPlaylistRow::Ready {
                position,
                last_position,
                track,
            } => track_rows.push(render_ready_paged_playlist_row(
                playlist_id,
                position,
                last_position,
                &track,
                album_thumbs,
                cx,
            )),
        }
    }

    Some(render_playlist_detail_shell(
        &page,
        PlaylistDetailBehaviorSlots {
            on_rename: Some(click_slot(cx.listener(move |this, _, window, cx| {
                this.begin_playlist_rename(
                    playlist_id,
                    playlist_name.clone(),
                    rename_placeholder,
                    window,
                    cx,
                );
            }))),
            rename_input: Some(rename_playlist_input),
            renaming: renaming_playlist_id == Some(playlist_id),
            on_submit_rename: Some(click_slot(cx.listener(move |this, _, _, cx| {
                this.submit_playlist_rename(cx);
            }))),
            on_cancel_rename: Some(click_slot(cx.listener(move |this, _, _, cx| {
                this.cancel_playlist_rename(cx);
            }))),
            on_delete: Some(click_slot(cx.listener(move |this, _, _, cx| {
                this.delete_playlist(playlist_id, cx);
            }))),
            track_rows,
        },
        cx,
    ))
}

#[cfg(feature = "async-runtime")]
fn render_ready_paged_playlist_row(
    playlist_id: i64,
    position: usize,
    last_position: usize,
    track: &Arc<crate::db::TrackRow>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> PlaylistShellRow {
    let track_for_select = (*track).clone();
    let position_i64 = i64::try_from(position).unwrap_or(i64::MAX);
    let prev_position_i64 = position_i64 - 1;
    let next_position_i64 = position_i64 + 1;
    let thumbnail = track
        .track_image_href
        .as_deref()
        .or(track.album_image_href.as_deref())
        .and_then(|url| album_thumbs.get(url))
        .cloned()
        .flatten();
    let display = crate::view_models::library::PlaylistTrackRowVm::new(
        track.as_ref(),
        position,
        last_position,
    )
    .display(playlist_id);
    let on_play = display.controls.play_enabled.then(|| {
        click_slot(cx.listener(move |_this, _, _, cx| {
            cx.emit(LibraryAppEvent::PlayPlaylistAt {
                playlist_id,
                playlist_position: position_i64,
            });
        }))
    });
    let slot = PlaylistTrackRowSlot {
        thumbnail: Some(render_album_thumb(thumbnail, 24.0)),
        on_select: Some(click_slot(cx.listener(move |this, _, _, cx| {
            this.select_track(&track_for_select, cx);
            cx.notify();
        }))),
        on_play,
        on_move_up: Some(click_slot(cx.listener(move |this, _, _, cx| {
            this.move_playlist_track(playlist_id, position_i64, prev_position_i64, cx);
        }))),
        on_move_down: Some(click_slot(cx.listener(move |this, _, _, cx| {
            this.move_playlist_track(playlist_id, position_i64, next_position_i64, cx);
        }))),
        on_remove: Some(click_slot(cx.listener(move |this, _, _, cx| {
            this.remove_playlist_track_at(playlist_id, position_i64, cx);
        }))),
    };
    PlaylistShellRow::Ready(Box::new(PlaylistShellReadyRow { display, slot }))
}
