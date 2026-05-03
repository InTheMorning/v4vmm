#![warn(clippy::pedantic)]
//! Discover-mode track row renderer.
//!
//! Thin screen-level glue: projects [`api::Track`] through shared release row
//! view-models, then adds screen-specific trailing actions (download button,
//! playlist popover, play button). All layout lives inside shared composites;
//! this module only wires callbacks.

use std::sync::Arc;

use gpui::{prelude::*, AnyElement, ClickEvent, ClipboardItem, Context, Image, SharedString};

use crate::api::{Feed, Track};
use crate::db;
use crate::search::{render_play_icon_button_with_id, render_track_download_button, SearchApp};
use crate::ui::composites::{
    identity_action_button, AddToPlaylistDisplay, AddToPlaylistPopover, IdentityActionKind,
    PlaylistOption, PlaylistOptionDisplay, TrackRow, TrackSurfaceElement,
};
use crate::view_models::entity_detail::EntityActionKind;
use crate::view_models::track::TrackVm;
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
use crate::views::TrackView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackRowMode {
    Discover,
}

fn playlist_options(playlists: &[db::Playlist]) -> Vec<PlaylistOption> {
    playlists
        .iter()
        .map(|playlist| {
            PlaylistOption::new(PlaylistOptionDisplay {
                id: playlist.id,
                name: SharedString::from(playlist.name.clone()),
            })
        })
        .collect()
}

#[must_use]
pub(crate) fn render_track_identity_actions(
    detail: &TrackDetailVm<'_>,
    id_prefix: &str,
) -> Vec<TrackSurfaceElement> {
    detail
        .identity_actions()
        .into_iter()
        .filter_map(|action| {
            let payload = action.payload.as_deref()?;
            let kind = match action.kind {
                EntityActionKind::OpenWebsite => IdentityActionKind::Website,
                EntityActionKind::CopyNostr => IdentityActionKind::Nostr,
                EntityActionKind::Download
                | EntityActionKind::Remove
                | EntityActionKind::AddToPlaylist
                | EntityActionKind::Play
                | EntityActionKind::CompareMetadata
                | EntityActionKind::OpenMusicBrainz
                | EntityActionKind::OpenRss => return None,
            };
            let payload_for_click = payload.to_string();
            let button = identity_action_button(
                SharedString::from(format!("{id_prefix}-{}:{payload}", kind_slug(kind))),
                kind,
            )
            .on_click(move |_, _, cx| match kind {
                IdentityActionKind::Website | IdentityActionKind::Rss => {
                    let _ = open::that(&payload_for_click);
                }
                IdentityActionKind::Nostr => {
                    cx.write_to_clipboard(ClipboardItem::new_string(payload_for_click.clone()));
                }
            });

            Some(TrackSurfaceElement::from_element(button.into_any_element()))
        })
        .collect()
}

const fn kind_slug(kind: IdentityActionKind) -> &'static str {
    match kind {
        IdentityActionKind::Website => "website",
        IdentityActionKind::Nostr => "nostr",
        IdentityActionKind::Rss => "rss",
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "staged extraction preserves the existing Discover row contract"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "caller in search.rs passes Track by move; switching to &Track cascades outside this module"
)]
pub(crate) fn render_track_row(
    track: Track,
    thumbnail: Option<Arc<Image>>,
    feed: Option<Feed>,
    is_downloaded: bool,
    is_in_flight: bool,
    feed_guid: Option<&str>,
    feed_url: Option<&str>,
    playlists: &[db::Playlist],
    mode: TrackRowMode,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    match mode {
        TrackRowMode::Discover => render_discover_track_row(
            &track,
            thumbnail,
            feed,
            is_downloaded,
            is_in_flight,
            feed_guid,
            feed_url,
            playlists,
            cx,
        ),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "shared row still needs the existing Discover inputs during rollout"
)]
fn render_discover_track_row(
    track: &Track,
    thumbnail: Option<Arc<Image>>,
    feed: Option<Feed>,
    is_downloaded: bool,
    is_in_flight: bool,
    feed_guid: Option<&str>,
    feed_url: Option<&str>,
    playlists: &[db::Playlist],
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = TrackVm::new(track);
    let guid = vm.guid();
    let title = vm.title();
    let audio_display = vm.play_audio_display();
    let play_button_id = SharedString::from(format!("track-row-play:{guid}"));
    let guid_for_click = guid.clone();
    let title_for_click = title.clone();
    let feed_guid_owned = feed_guid.map(str::to_string);
    let track_view = TrackView::from_api(track.clone());
    let row_vm = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Discover).row();

    let download_btn =
        render_track_download_button(track.clone(), feed, is_downloaded, is_in_flight, cx)
            .into_any_element();
    let play_btn =
        render_play_icon_button_with_id(play_button_id, audio_display, cx).into_any_element();
    let mut actions = vec![download_btn];

    if let Some(ref fguid) = feed_guid_owned {
        if !guid.is_empty() {
            let feed_guid_sel = fguid.clone();
            let feed_url_sel = feed_url.map(str::to_string);
            let track_guid_sel = guid.clone();
            let feed_guid_cre = feed_guid_sel.clone();
            let feed_url_cre = feed_url_sel.clone();
            let track_guid_cre = track_guid_sel.clone();
            let popover = AddToPlaylistPopover::new(AddToPlaylistDisplay {
                id: SharedString::from(format!("add-pl:{guid}")),
                playlists: playlist_options(playlists),
                trigger_label: SharedString::from("+ Playlist"),
            })
            .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
                this.add_search_track_to_playlist(
                    &feed_guid_sel,
                    feed_url_sel.as_deref(),
                    &track_guid_sel,
                    *playlist_id,
                    cx,
                );
            }))
            .on_create(cx.listener(move |this, name: &String, _window, cx| {
                this.create_playlist_and_add_discover_track(
                    name,
                    &feed_guid_cre,
                    feed_url_cre.as_deref(),
                    &track_guid_cre,
                    cx,
                );
            }));
            actions.push(popover.into_any_element());
        }
    }

    actions.push(play_btn);

    let mut row = TrackRow::from_vm(SharedString::from(format!("track-row:{guid}")), &row_vm)
        .thumbnail(thumbnail)
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.push_inspector(
                "track".into(),
                guid_for_click.clone(),
                title_for_click.clone(),
                cx,
            );
        }));

    for action in actions {
        row = row.trailing_child(action);
    }

    row.into_any_element()
}
