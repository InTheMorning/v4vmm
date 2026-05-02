#![warn(clippy::pedantic)]
//! Discover-mode track row renderer.
//!
//! Thin screen-level glue: projects [`api::Track`] through shared release row
//! view-models, then adds screen-specific trailing actions (download button,
//! playlist popover, play button). All layout lives inside shared composites;
//! this module only wires callbacks.

use std::sync::Arc;

use gpui::{prelude::*, AnyElement, ClickEvent, Context, Image, SharedString};

use crate::api::{Feed, Track};
use crate::db;
use crate::search::{render_play_icon_button_with_id, render_track_download_button, SearchApp};
use crate::ui::composites::AddToPlaylistPopover;
use crate::ui_entity::{render_release_track_row, ReleaseTrackRowSlot};
use crate::view_models::entity_detail::{EntitySurfaceContext, SharedTrackRowVm};
use crate::view_models::track::TrackVm;
use crate::views::TrackView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackRowMode {
    Discover,
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
    let audio_url = vm.play_url();
    let play_button_id = SharedString::from(format!("track-row-play:{guid}"));
    let guid_for_click = guid.clone();
    let title_for_click = title.clone();
    let feed_guid_owned = feed_guid.map(str::to_string);
    let track_view = TrackView::from_api(track.clone());
    let row = SharedTrackRowVm::new(&track_view, EntitySurfaceContext::Discover);

    let download_btn =
        render_track_download_button(track.clone(), feed, is_downloaded, is_in_flight, cx)
            .into_any_element();
    let play_btn =
        render_play_icon_button_with_id(play_button_id, audio_url, cx).into_any_element();
    let mut actions = vec![download_btn];

    if let Some(ref fguid) = feed_guid_owned {
        if !guid.is_empty() {
            let feed_guid_sel = fguid.clone();
            let feed_url_sel = feed_url.map(str::to_string);
            let track_guid_sel = guid.clone();
            let feed_guid_cre = feed_guid_sel.clone();
            let feed_url_cre = feed_url_sel.clone();
            let track_guid_cre = track_guid_sel.clone();
            let popover = AddToPlaylistPopover::new(
                SharedString::from(format!("add-pl:{guid}")),
                playlists.to_vec(),
            )
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

    let slot = ReleaseTrackRowSlot {
        thumbnail,
        actions,
        ..ReleaseTrackRowSlot::default()
    }
    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
        this.push_inspector(
            "track".into(),
            guid_for_click.clone(),
            title_for_click.clone(),
            cx,
        );
    }));

    render_release_track_row(SharedString::from(format!("track-row:{guid}")), row, slot)
}
