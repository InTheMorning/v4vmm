//! Library playlist detail surface.
//!
//! Renders the right-hand pane when a playlist is selected. The shared playlist
//! shell owns the page hierarchy; this module wires Library-specific callbacks
//! back to `LibraryApp`.
//!
//! When a [`PagedTrackListActor`] for the selected playlist has published a
//! snapshot, the screen is rendered via the paged path: rows the actor has
//! fulfilled paint as `Ready`, the rest paint as `Pending` skeletons. The
//! actor receives a single `ReportVisible` per render so its prefetch policy
//! can stay ahead of scroll without the screen having to track the visible
//! window directly.
//!
//! ADR 0084 Decision 10, packet 025: each row carries the selection facts of
//! the detail column, by position. A body click goes through the detail
//! click of packet 018, and the page scrolls with the held detail column
//! handle.
//!
//! ADR 0083 task 007: each row has a "⋯" menu and the same right-click
//! menu. The right-click menu gives the focus back to the detail column.
//!
//! [`PagedTrackListActor`]: crate::application::paged_track_list::PagedTrackListActor

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, FocusHandle, Image, ScrollHandle, Window,
};
use gpui_component::input::InputState;

use crate::application::capability::ExecutionUnavailable;
use crate::db::{self, PlaylistEntryKey, TrackRow};
use crate::library::PlaylistActorState;
use crate::library::{LibraryApp, LibraryAppEvent, PlaylistDetail};
use crate::runtime::PlaylistRssCheckSnapshot;
use crate::ui::composites::list_row::RowMenu;
use crate::ui::composites::{
    CollectionArtImage, PlaylistOption, PlaylistOptionDisplay, TrackRowMenuHandlers,
};
use crate::ui::shells::library::detail::DetailRowSlot;
use crate::ui::shells::library::thumbnail::render_album_thumb;
use crate::ui::shells::playlist::{
    click_slot, command_slot, render_playlist_detail_shell, reorder_slot,
    PlaylistDetailBehaviorSlots, PlaylistShellReadyRow, PlaylistShellRow, PlaylistTrackRowSlot,
};
use crate::view_models::library::{
    playlist_row_add_choices, LibraryChromeDisplay, PlaylistDetailVm, PlaylistTrackRowVm,
    TrackRowWork,
};
use crate::view_models::playlist_detail::PlaylistDetailPageVm;
use crate::view_models::playlist_rss_check::removed_from_feed_row;
use crate::view_models::workspace::{FrameNavigationEntry, NavigationPathStart};

/// The Library state that the playlist page reads (ADR 0076, ADR 0084
/// packet 025).
pub(crate) struct LibraryPlaylistPage<'a> {
    pub(crate) detail: &'a PlaylistDetail,
    pub(crate) album_thumbs: &'a BTreeMap<String, Option<Arc<Image>>>,
    /// The header art in the logo frame (ADR 0084 Decision 12).
    pub(crate) header_art: Option<CollectionArtImage>,
    pub(crate) chrome: &'a LibraryChromeDisplay,
    pub(crate) rename_playlist_input: Entity<InputState>,
    pub(crate) renaming_playlist_id: Option<i64>,
    pub(crate) playlist_actor: Option<&'a PlaylistActorState>,
    pub(crate) playlist_rss_check: Option<&'a PlaylistRssCheckSnapshot>,
    /// The selection facts of each entry, in position order.
    pub(crate) row_slots: Vec<DetailRowSlot>,
    /// The held scroll handle of the detail column.
    pub(crate) scroll: &'a ScrollHandle,
    /// Each playlist, for the "Add to playlist" level of the row menu.
    pub(crate) playlists: &'a [db::Playlist],
    /// The work in progress on each Library track that has work (ADR 0083
    /// task 007).
    pub(crate) track_works: BTreeMap<i64, TrackRowWork>,
    /// The focus that the right-click menu of a row gives back (decision
    /// D15).
    pub(crate) detail_focus: &'a FocusHandle,
}

/// The facts that each row of one render shares.
struct PlaylistRowContext<'a> {
    playlist_id: i64,
    last_position: usize,
    album_thumbs: &'a BTreeMap<String, Option<Arc<Image>>>,
    playback_availability: Result<(), ExecutionUnavailable>,
    playlist_rss_check: Option<&'a PlaylistRssCheckSnapshot>,
    menu: &'a PlaylistRowMenuContext<'a>,
}

/// The facts of the row menus that each row of one render shares (ADR
/// 0083 task 007).
struct PlaylistRowMenuContext<'a> {
    /// The playlist choices, with no open playlist (decision D8).
    playlists: Rc<Vec<PlaylistOption>>,
    track_works: &'a BTreeMap<i64, TrackRowWork>,
    detail_focus: FocusHandle,
}

pub(crate) fn render_library_playlist_detail(
    input: LibraryPlaylistPage<'_>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let LibraryPlaylistPage {
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
        playlists,
        track_works,
        detail_focus,
    } = input;
    let playlist_id = detail.playlist.id;
    let menu = PlaylistRowMenuContext {
        playlists: Rc::new(
            playlist_row_add_choices(playlists, playlist_id)
                .into_iter()
                .map(|option| {
                    PlaylistOption::new(PlaylistOptionDisplay {
                        id: option.id,
                        name: option.name.into(),
                        a11y_label: option.a11y_label.into(),
                    })
                })
                .collect(),
        ),
        track_works: &track_works,
        detail_focus: detail_focus.clone(),
    };
    let page = PlaylistDetailVm::new(&detail.playlist, &detail.tracks)
        .with_rss_check(playlist_rss_check)
        .page(chrome.playlist_detail_scroll_id);
    let mut slots: Vec<Option<DetailRowSlot>> = row_slots.into_iter().map(Some).collect();
    let mut take_slot = |position: usize| slots.get_mut(position).and_then(Option::take);

    let paged_state = playlist_actor.filter(|state| state.playlist_id == playlist_id);
    let track_rows = if let Some(state) = paged_state {
        render_paged_rows(
            detail,
            state,
            album_thumbs,
            chrome,
            playlist_rss_check,
            &menu,
            &mut take_slot,
            cx,
        )
    } else {
        let context = PlaylistRowContext {
            playlist_id,
            last_position: detail.tracks.len().saturating_sub(1),
            album_thumbs,
            playback_availability: chrome.playback_availability,
            playlist_rss_check,
            menu: &menu,
        };
        page.track_rows()
            .iter()
            .map(|row| {
                let position = usize::try_from(row.position()).unwrap_or(usize::MAX);
                render_ready_playlist_row(&context, position, row.track(), take_slot(position), cx)
            })
            .collect()
    };

    let mut slots = behavior_slots(
        detail,
        &page,
        rename_playlist_input,
        renaming_playlist_id,
        track_rows,
        cx,
    );
    slots.header_art = header_art;
    render_playlist_detail_shell(&page, slots, scroll, cx)
}

/// The page actions of the playlist page. The eager path and the paged
/// path use the same slots.
fn behavior_slots(
    detail: &PlaylistDetail,
    page: &PlaylistDetailPageVm<'_>,
    rename_playlist_input: Entity<InputState>,
    renaming_playlist_id: Option<i64>,
    track_rows: Vec<PlaylistShellRow>,
    cx: &mut Context<LibraryApp>,
) -> PlaylistDetailBehaviorSlots {
    let playlist_id = detail.playlist.id;
    let playlist_name = detail.playlist.name.clone();
    let rename_placeholder = page.actions_display().rename_input_placeholder;
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
        on_check_rss: Some(check_rss_slot(playlist_id, cx)),
        on_download_added_track: Some(download_added_track_slot(cx)),
        on_reorder: Some(reorder_slot(cx.listener(
            move |this, positions: &(i64, i64), _, cx| {
                let (from, to) = *positions;
                this.move_playlist_track(playlist_id, from, to, cx);
            },
        ))),
        track_rows,
        header_art: None,
    }
}

/// The "Check RSS" slot: the button starts the check of this playlist
/// (ADR 0076 Decision 2). The shell renders the typed availability.
fn check_rss_slot(
    playlist_id: i64,
    cx: &mut Context<LibraryApp>,
) -> crate::ui::shells::playlist::PlaylistClickHandler {
    click_slot(cx.listener(move |this, _, _, cx| {
        this.check_playlist_rss(playlist_id, crate::runtime::RssCheckTrigger::Button, cx);
    }))
}

/// The download slot of a `track_added` difference: the existing Library
/// track download (ADR 0076 Decision 6).
fn download_added_track_slot(
    cx: &mut Context<LibraryApp>,
) -> crate::ui::shells::playlist::PlaylistTrackIdHandler {
    let entity = cx.entity().downgrade();
    std::rc::Rc::new(move |track_id, _window, cx| {
        if let Some(entity) = entity.upgrade() {
            entity.update(cx, |this, cx| this.download_library_tracks(&[track_id], cx));
        }
    })
}

/// ADR 0076 Decision 7: "Remove from all playlists" opens its confirmation.
fn remove_from_all_playlists_slot(
    track_id: i64,
    cx: &mut Context<LibraryApp>,
) -> crate::ui::shells::playlist::PlaylistClickHandler {
    click_slot(cx.listener(move |this, _, window, cx| {
        this.request_remove_track_from_all_playlists(track_id, window, cx);
    }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "the paged path takes each value that the eager path reads"
)]
fn render_paged_rows(
    detail: &PlaylistDetail,
    state: &PlaylistActorState,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    chrome: &LibraryChromeDisplay,
    playlist_rss_check: Option<&PlaylistRssCheckSnapshot>,
    menu: &PlaylistRowMenuContext<'_>,
    take_slot: &mut impl FnMut(usize) -> Option<DetailRowSlot>,
    cx: &mut Context<LibraryApp>,
) -> Vec<PlaylistShellRow> {
    use crate::application::paged_track_list::PagedTrackListMsg;
    use crate::view_models::paged_playlist_detail::{PagedPlaylistDetailVm, PagedPlaylistRow};

    let inbox = state.handle.clone();
    let paged = PagedPlaylistDetailVm::new(&detail.playlist, &state.snapshot);
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
                let probe = take_slot(position).and_then(|slot| slot.probe);
                track_rows.push(PlaylistShellRow::Pending {
                    position,
                    last_position: total.saturating_sub(1),
                    probe,
                });
                // Nudge the actor to fetch this row; the reactive watch
                // bridge will repaint when the page lands.
                let _ = inbox.try_send(PagedTrackListMsg::Read(position));
            }
            PagedPlaylistRow::Ready {
                position,
                last_position,
                track,
            } => {
                let context = PlaylistRowContext {
                    playlist_id: detail.playlist.id,
                    last_position,
                    album_thumbs,
                    playback_availability: chrome.playback_availability,
                    playlist_rss_check,
                    menu,
                };
                let slot = take_slot(position);
                track_rows.push(render_ready_playlist_row(
                    &context, position, &track, slot, cx,
                ));
            }
        }
    }
    track_rows
}

fn render_ready_playlist_row(
    context: &PlaylistRowContext<'_>,
    position: usize,
    track: &TrackRow,
    selection_slot: Option<DetailRowSlot>,
    cx: &mut Context<LibraryApp>,
) -> PlaylistShellRow {
    let playlist_id = context.playlist_id;
    let track_key_for_select = track.track_key();
    let position_i64 = i64::try_from(position).unwrap_or(i64::MAX);
    let thumbnail = track
        .track_image_href
        .as_deref()
        .or(track.album_image_href.as_deref())
        .and_then(|url| context.album_thumbs.get(url))
        .cloned()
        .flatten();
    let (selection, scroll_probe) = selection_slot
        .map(|slot| (Some(slot.selection), slot.probe))
        .unwrap_or_default();
    let work = context
        .menu
        .track_works
        .get(&track.id)
        .copied()
        .unwrap_or_default();
    let mut display = PlaylistTrackRowVm::new(track, position, context.last_position)
        .with_work(work)
        .display_with_playback(playlist_id, context.playback_availability);
    if let Some(selection) = selection {
        display = display.with_selection(selection);
    }
    display.removed_from_feed = removed_from_feed_row(
        context.playlist_rss_check,
        playlist_id,
        track.id,
        display.position,
        &display.title,
    );
    let original_track_id = track.id;
    let entry = PlaylistEntryKey {
        position: position_i64,
        track_id: original_track_id,
    };
    let on_play = display.controls.play_enabled.then(|| {
        click_slot(cx.listener(move |_this, _, _, cx| {
            cx.emit(LibraryAppEvent::PlayPlaylistAt {
                playlist_id,
                playlist_position: position_i64,
                track_id: original_track_id,
            });
        }))
    });
    let slot = PlaylistTrackRowSlot {
        thumbnail: Some(render_album_thumb(thumbnail, 24.0, cx)),
        // ADR 0084 packet 025: a body click goes through the detail click
        // of packet 018. A plain click in `Normal` opens the track page.
        on_select: Some(click_slot(cx.listener(
            move |this, event: &ClickEvent, _, cx| {
                if this.click_playlist_entry(entry, event.modifiers(), cx) {
                    this.open_page(
                        FrameNavigationEntry::Track(track_key_for_select.clone()),
                        NavigationPathStart::CurrentPath,
                        cx,
                    );
                }
            },
        ))),
        on_play,
        menu: display.controls.menu.clone().map(|menu| RowMenu {
            menu,
            playlists: Rc::clone(&context.menu.playlists),
            handlers: playlist_row_menu_handlers(
                (playlist_id, position_i64),
                original_track_id,
                cx,
            ),
            return_focus: context.menu.detail_focus.clone(),
        }),
        on_remove: Some(command_slot(cx.listener(move |this, (), _, cx| {
            this.remove_playlist_track_at(playlist_id, position_i64, cx);
        }))),
        on_remove_from_all_playlists: Some(remove_from_all_playlists_slot(original_track_id, cx)),
        on_toggle_selection: Some(click_slot(cx.listener(move |this, _, _, cx| {
            this.toggle_playlist_entry(entry, cx);
        }))),
        scroll_probe,
    };
    PlaylistShellRow::Ready(Box::new(PlaylistShellReadyRow { display, slot }))
}

/// The handlers of the row menu of the playlist entry `entry` (ADR 0083
/// task 007, "The commands"). Each item goes to
/// `LibraryApp::run_playlist_row_menu_action`. "Add to playlist" adds the
/// track to the chosen playlist.
fn playlist_row_menu_handlers(
    entry: (i64, i64),
    track_id: i64,
    cx: &mut Context<LibraryApp>,
) -> TrackRowMenuHandlers {
    let entity = cx.entity().downgrade();
    let action = {
        let entity = entity.clone();
        Rc::new(move |item, window: &mut Window, cx: &mut App| {
            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |this, cx| {
                    this.run_playlist_row_menu_action(entry, track_id, item, window, cx);
                });
            }
        })
    };
    let select_playlist = {
        let entity = entity.clone();
        Rc::new(move |playlist_id: &i64, _: &mut Window, cx: &mut App| {
            if let Some(entity) = entity.upgrade() {
                entity.update(cx, |this, cx| {
                    this.add_track_to_playlist(track_id, *playlist_id, cx);
                });
            }
        })
    };
    let create_playlist = Rc::new(move |name: &String, _: &mut Window, cx: &mut App| {
        if let Some(entity) = entity.upgrade() {
            entity.update(cx, |this, cx| {
                this.create_playlist_and_add_track(name, track_id, cx);
            });
        }
    });
    TrackRowMenuHandlers {
        action,
        select_playlist,
        create_playlist,
    }
}
