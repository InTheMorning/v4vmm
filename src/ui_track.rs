use std::sync::Arc;

use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, Image, SharedString};

use crate::api::{Feed, Track};
use crate::db;
use crate::search::{
    fmt_dur, render_play_icon_button_with_id, render_track_download_button, track_play_url,
    track_title, SearchApp,
};
use crate::ui::playlist_popover::AddToPlaylistPopover;
use crate::ui::theme::{color, radius, spacing, typography};
use crate::ui_common::{render_thumb, truncated};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackRowMode {
    Discover,
}

#[expect(
    clippy::too_many_arguments,
    reason = "staged extraction preserves the existing Discover row contract"
)]
pub(crate) fn render_track_row(
    track: Track,
    thumbnail: Option<Arc<Image>>,
    feed: Option<Feed>,
    is_downloaded: bool,
    is_in_flight: bool,
    feed_guid: Option<&str>,
    feed_url: Option<&str>,
    _open_guid: Option<&str>,
    playlists: &[db::Playlist],
    mode: TrackRowMode,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    match mode {
        TrackRowMode::Discover => render_discover_track_row(
            track,
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
    track: Track,
    thumbnail: Option<Arc<Image>>,
    feed: Option<Feed>,
    is_downloaded: bool,
    is_in_flight: bool,
    feed_guid: Option<&str>,
    feed_url: Option<&str>,
    playlists: &[db::Playlist],
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let guid = track.track_guid.clone().unwrap_or_default();
    let title = track_title(&track);
    let track_number = track.track_number;
    let duration_secs = track.duration_secs;
    let audio_url = track_play_url(&track);
    let play_button_id = SharedString::from(format!("track-row-play:{guid}"));
    let guid_for_click = guid.clone();
    let title_for_click = title.clone();
    let feed_guid_owned = feed_guid.map(str::to_string);

    let mut row = div()
        .id(SharedString::from(format!("track-row:{guid}")))
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .px(spacing::XS)
        .py(spacing::XS)
        .rounded(radius::SM)
        .child(
            div()
                .id(SharedString::from(format!("track-row-open:{guid}")))
                .flex_1()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::SM)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.push_inspector(
                        "track".into(),
                        guid_for_click.clone(),
                        title_for_click.clone(),
                        cx,
                    );
                }))
                .child(
                    div()
                        .w(px(24.0))
                        .text_right()
                        .text_color(color::text_muted())
                        .text_size(typography::SIZE_MICRO)
                        .child(track_number.map_or_else(|| "·".into(), |n| n.to_string())),
                )
                .child(render_thumb(thumbnail.clone(), "track", 28.0, false))
                .child(truncated(title).flex_1())
                .when(duration_secs.is_some(), |el| {
                    el.child(
                        div()
                            .text_color(color::text_muted())
                            .text_size(typography::SIZE_MICRO)
                            .child(SharedString::from(fmt_dur(
                                duration_secs.unwrap_or_default(),
                            ))),
                    )
                }),
        );

    row = row.child(render_track_download_button(
        track.clone(),
        feed,
        is_downloaded,
        is_in_flight,
        cx,
    ));

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
            row = row.child(popover);
        }
    }

    row = row.child(render_play_icon_button_with_id(
        play_button_id,
        audio_url,
        cx,
    ));

    row.into_any_element()
}
