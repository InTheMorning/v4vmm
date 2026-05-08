//! Library feed (album) detail surface.
//!
//! Renders the right-hand pane when a Library feed is selected. Mutations and
//! selected-entity state stay on `LibraryApp`; this module only builds shell
//! slots and routes callbacks back through `cx.listener(...)`.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, ClipboardItem, Context, Image, SharedString, Styled};

use crate::db::{self, TrackRow};
use crate::library::{playlist_options, LibraryApp};
use crate::ui::composites::{
    action_button, identity_action_button, ActionButtonDisplay, AddToPlaylistDisplay,
    AddToPlaylistPopover, IdentityActionButtonDisplay, IdentityActionKind, ReleaseSurfaceElement,
    StatusRole, TrackRow as TrackRowComposite,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button as UiButton;
use crate::ui::shells::entity::{
    render_contributor_panel, render_feed_identity_actions, render_release_detail_shell,
    ContributorRowSlot, ReleaseDetailBehaviorSlots,
};
use crate::ui::style::{color, spacing, typography};
use crate::view_models::entity_detail::{
    ContributorIdentityActionDisplay, ContributorIdentityActionKind, ContributorRowVm,
    EntityActionKind, EntityActionTone, EntityActionVm, EntitySurfaceContext, ReleaseDetailVm,
};
use crate::view_models::library::{
    AlbumNode, LibraryAlbumDetailVm, LibraryTrackRowDisplay, LibraryTrackRowVm, MbStatusKind,
    MbTrackStatus,
};
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
use crate::views::{FeedView, TrackView};

#[expect(
    clippy::too_many_lines,
    reason = "lifted legacy feed-detail surface stays intact during Task 007 decomposition"
)]
pub(crate) fn render_library_feed_detail(
    album: &AlbumNode,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let feed_row = db::FeedRow {
        id: album.feed_id.unwrap_or(0),
        feed_url: album.feed_url_for_detail(),
        feed_guid: album.feed_guid.clone(),
        title: Some(album.name.clone()),
        description: None,
        album_image_href: album.image_href.clone(),
        is_subscribed: false,
    };
    let feed_view = FeedView::from_local_with_identity(
        feed_row,
        album
            .tracks
            .clone()
            .into_iter()
            .map(TrackView::from_local)
            .collect(),
        album.identity_facts.clone(),
    );

    let thumb_image = album
        .image_href
        .as_ref()
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(Clone::clone);

    let track_rows: Vec<ReleaseSurfaceElement> = album
        .tracks
        .iter()
        .map(|track| {
            ReleaseSurfaceElement::from_element(render_library_track_row(
                track,
                mb_status,
                busy_track,
                album_thumbs,
                playlists,
                cx,
            ))
        })
        .collect();

    let vm = LibraryAlbumDetailVm::new(&feed_view, &album.tracks, mb_status);

    let album_for_mb = album.clone();
    let feed_id = album.feed_id;
    let mut buttons = div().flex().flex_row().items_center().gap(spacing::SM);
    if let Some(fid) = feed_id {
        let remove_action = vm.primary_action_vm(fid, false);
        let remove_a11y = remove_action.a11y_label();
        let remove_label = remove_action.label;
        buttons = buttons.child(
            action_button(
                ActionButtonDisplay {
                    label: SharedString::from(remove_label),
                    a11y_label: SharedString::from(remove_a11y),
                },
                cx,
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.unsubscribe_feed(fid, window, cx);
                cx.notify();
            })),
        );
    }
    let musicbrainz_action = vm.musicbrainz_action_vm();
    buttons = buttons.child(
        action_button(
            ActionButtonDisplay {
                label: SharedString::from(musicbrainz_action.label),
                a11y_label: SharedString::from(musicbrainz_action.a11y_label),
            },
            cx,
        )
        .disabled(musicbrainz_action.disabled)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.musicbrainz_feed(album_for_mb.clone(), cx);
        })),
    );
    if let Some(fid) = feed_id {
        let playlist_display = vm
            .playlist_display(fid)
            .expect("library feed playlist action should render for local feeds");
        buttons = buttons.child(
            AddToPlaylistPopover::new(AddToPlaylistDisplay {
                id: SharedString::from(playlist_display.popover_id),
                playlists: playlist_options(playlists),
                trigger_label: SharedString::from(playlist_display.trigger_label),
                trigger_a11y_label: SharedString::from("Add feed to playlist"),
                new_playlist_a11y_label: SharedString::from("Create a new playlist"),
                back_a11y_label: SharedString::from("Back to playlist choices"),
                create_a11y_label: SharedString::from("Create playlist and add feed"),
            })
            .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
                this.add_album_to_playlist(fid, *playlist_id, cx);
            }))
            .on_create(cx.listener(move |this, name: &String, _window, cx| {
                this.create_playlist_and_add_album(name, fid, cx);
            })),
        );
    }

    let projection = ReleaseDetailVm::new(&feed_view, EntitySurfaceContext::Library);
    let page = projection.page();
    let mut slots = ReleaseDetailBehaviorSlots {
        hero_image: thumb_image,
        primary_actions: vec![ReleaseSurfaceElement::from_element(
            buttons.into_any_element(),
        )],
        identity_actions: render_feed_identity_actions(&page),
        track_rows: Some(track_rows),
        ..ReleaseDetailBehaviorSlots::default()
    };
    if let Some(panel) = render_library_contributors_panel(&projection, album_thumbs) {
        slots
            .after_section
            .push(ReleaseSurfaceElement::from_element(panel));
    }
    render_release_detail_shell(&page, slots)
}

fn render_library_contributors_panel(
    projection: &ReleaseDetailVm<'_>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
) -> Option<AnyElement> {
    let display = projection.contributor_panel_display();
    render_contributor_panel(
        display.id,
        display.title,
        projection.contributors(),
        |contributor| {
            let thumbnail = contributor
                .image_url()
                .and_then(|url| album_thumbs.get(url))
                .and_then(Clone::clone);
            ContributorRowSlot {
                thumbnail,
                actions: library_contributor_identity_actions(contributor),
            }
        },
    )
}

fn library_contributor_identity_actions(
    contributor: &ContributorRowVm<'_>,
) -> Vec<ReleaseSurfaceElement> {
    contributor
        .identity_actions()
        .into_iter()
        .map(|action| {
            let ContributorIdentityActionDisplay {
                id,
                kind,
                target,
                a11y_label,
            } = action;
            let target_for_click = target;
            let a11y_label = SharedString::from(a11y_label);
            match kind {
                ContributorIdentityActionKind::Website => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Website,
                        a11y_label,
                    })
                    .on_click(move |_, _, _| {
                        let _ = open::that(&target_for_click);
                    })
                    .into_any_element()
                }
                ContributorIdentityActionKind::Nostr => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Nostr,
                        a11y_label,
                    })
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(target_for_click.clone()));
                    })
                    .into_any_element()
                }
            }
        })
        .map(ReleaseSurfaceElement::from_element)
        .collect()
}

fn render_library_track_row(
    track: &TrackRow,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    busy_track: Option<i64>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let track_for_click = track.clone();
    let track_for_select = track.clone();
    let track_id = track.id;
    let is_busy = busy_track == Some(track_id);
    let vm = LibraryTrackRowVm::new(track, mb_status.get(&track_id));
    let EntityActionVm {
        kind,
        label,
        enabled,
        tone,
        ..
    } = vm.primary_action_vm(is_busy);
    let LibraryTrackRowDisplay {
        row_id,
        toggle_button_id,
    } = vm.row_display();
    let in_library = kind == EntityActionKind::Remove;
    let mb_text = vm.mb_status_text();
    let mb_kind = vm.mb_status_kind();
    let thumbnail = track
        .track_image_href
        .as_ref()
        .or(track.album_image_href.as_ref())
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(Clone::clone);
    let primary_style = match tone {
        EntityActionTone::DestructiveQuiet => ControlStyle::DestructiveRowAction,
        _ => ControlStyle::RowAction,
    };

    let toggle_button = UiButton::styled(SharedString::from(toggle_button_id), primary_style)
        .label(label)
        .disabled(!enabled)
        .on_click(cx.listener(move |this, _, window, cx| {
            if in_library {
                this.remove_track(track_id, window, cx);
            } else {
                this.subscribe_track(track_for_click.clone(), cx);
            }
            cx.notify();
        }));
    let mut actions = vec![toggle_button.into_any_element()];

    if let Some(text) = mb_text {
        let status_color = match mb_kind {
            Some(MbStatusKind::Success) => StatusRole::Success.color(cx),
            Some(MbStatusKind::Danger) => StatusRole::Danger.color(cx),
            Some(MbStatusKind::Warning) => StatusRole::Warning.color(cx),
            _ => color::text_muted(),
        };
        actions.push(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(status_color)
                .child(SharedString::from(text))
                .into_any_element(),
        );
    }

    let playlist_display = vm.playlist_display();
    actions.push(
        AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(playlist_display.popover_id),
            playlists: playlist_options(playlists),
            trigger_label: SharedString::from(playlist_display.trigger_label),
            trigger_a11y_label: SharedString::from("Add track to playlist"),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add track"),
        })
        .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
            this.add_track_to_playlist(track_id, *playlist_id, cx);
        }))
        .on_create(cx.listener(move |this, name: &String, _window, cx| {
            this.create_playlist_and_add_track(name, track_id, cx);
        }))
        .into_any_element(),
    );

    let track_view = TrackView::from_local(track.clone());
    let row_vm = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library).row();
    let mut row = TrackRowComposite::from_vm(SharedString::from(row_id), &row_vm)
        .thumbnail(thumbnail)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_track(&track_for_select, cx);
            cx.notify();
        }));

    for action in actions {
        row = row.trailing_child(action);
    }

    row.into_any_element()
}
