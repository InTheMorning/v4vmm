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
    identity_action_button, AddToPlaylistDisplay, AddToPlaylistPopover,
    IdentityActionButtonDisplay, IdentityActionKind, PlaylistOption, PlaylistOptionDisplay,
    TrackDetailSurface, TrackRow, TrackSurfaceElement,
};
use crate::view_models::entity_detail::{
    EntityActionVm, IdentityActionDisplay, IdentityActionDisplayKind,
};
use crate::view_models::playlist_option_displays;
use crate::view_models::track::{TrackRowControlsDisplay, TrackVm};
use crate::view_models::track_detail::{
    TrackDetailLoadState, TrackDetailPageVm, TrackDetailSection, TrackDetailSurfaceContext,
    TrackDetailVm,
};
use crate::views::TrackView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackRowMode {
    Discover,
}

#[derive(Default)]
pub(crate) struct TrackDetailBehaviorSlots {
    pub hero_image: Option<Arc<Image>>,
    pub load_state: Option<TrackDetailLoadState>,
    pub primary_actions: Vec<TrackSurfaceElement>,
    pub external_links: Vec<TrackSurfaceElement>,
    pub description_panel: Option<TrackSurfaceElement>,
    pub sections: Vec<TrackDetailSection>,
    pub section_elements: Vec<TrackSurfaceElement>,
    pub advanced_panels: Vec<TrackSurfaceElement>,
}

fn playlist_options(playlists: &[db::Playlist]) -> Vec<PlaylistOption> {
    playlist_option_displays(playlists)
        .into_iter()
        .map(|option| {
            PlaylistOption::new(PlaylistOptionDisplay {
                id: option.id,
                name: SharedString::from(option.name),
                a11y_label: SharedString::from(option.a11y_label),
            })
        })
        .collect()
}

#[must_use]
pub(crate) fn render_track_page_identity_actions(
    page: &TrackDetailPageVm<'_>,
) -> Vec<TrackSurfaceElement> {
    render_track_identity_actions(page.identity_actions(), page.identity_action_prefix())
}

fn render_track_identity_actions(
    actions: Vec<EntityActionVm>,
    identity_action_prefix: &str,
) -> Vec<TrackSurfaceElement> {
    actions
        .into_iter()
        .filter_map(|action| {
            let display = action.identity_display(identity_action_prefix)?;
            let IdentityActionDisplay {
                id,
                kind,
                payload,
                a11y_label,
            } = display;
            let kind = match kind {
                IdentityActionDisplayKind::Website => IdentityActionKind::Website,
                IdentityActionDisplayKind::Nostr => IdentityActionKind::Nostr,
                IdentityActionDisplayKind::Rss => IdentityActionKind::Rss,
            };
            let payload_for_click = payload;
            let button = identity_action_button(IdentityActionButtonDisplay {
                id: SharedString::from(id),
                kind,
                a11y_label: SharedString::from(a11y_label),
            })
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

pub(crate) fn build_track_detail_surface(
    page: &TrackDetailPageVm<'_>,
    slots: TrackDetailBehaviorSlots,
) -> TrackDetailSurface {
    let mut surface = TrackDetailSurface::new(page.detail()).image(slots.hero_image);

    if let Some(load_state) = slots.load_state {
        surface = surface.load_state(load_state);
    }

    if !slots.primary_actions.is_empty() {
        surface = surface.primary_actions(slots.primary_actions);
    }

    if !slots.external_links.is_empty() {
        surface = surface.external_links(slots.external_links);
    }

    if let Some(description_panel) = slots.description_panel {
        surface = surface.description_panel(description_panel);
    }

    if !slots.sections.is_empty() {
        surface = surface.sections(slots.sections);
    }

    if !slots.section_elements.is_empty() {
        surface = surface.section_elements(slots.section_elements);
    }

    if !slots.advanced_panels.is_empty() {
        surface = surface.advanced_panels(slots.advanced_panels);
    }

    surface
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
    let TrackRowControlsDisplay {
        row_id,
        play_button_id,
        playlist_popover_id,
        playlist_trigger_label,
    } = vm.row_controls_display();
    let audio_display = vm.play_audio_display();
    let guid_for_click = guid.clone();
    let title_for_click = title.clone();
    let feed_guid_owned = feed_guid.map(str::to_string);
    let track_view = TrackView::from_api(track.clone());
    let row_vm = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Discover).row();

    let download_btn =
        render_track_download_button(track.clone(), feed, is_downloaded, is_in_flight, cx)
            .into_any_element();
    let play_btn =
        render_play_icon_button_with_id(SharedString::from(play_button_id), audio_display, cx)
            .into_any_element();
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
                id: SharedString::from(playlist_popover_id),
                playlists: playlist_options(playlists),
                trigger_label: SharedString::from(playlist_trigger_label),
                trigger_a11y_label: SharedString::from("Add track to playlist"),
                new_playlist_a11y_label: SharedString::from("Create a new playlist"),
                back_a11y_label: SharedString::from("Back to playlist choices"),
                create_a11y_label: SharedString::from("Create playlist and add track"),
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

    let mut row = TrackRow::from_vm(SharedString::from(row_id), &row_vm)
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
