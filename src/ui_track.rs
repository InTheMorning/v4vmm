use std::sync::Arc;

use gpui::{div, prelude::*, px, AnyElement, ClickEvent, Context, Image, SharedString};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{Sizable, Size};

use crate::api::{Feed, Track};
use crate::db;
use crate::search::{
    fmt_dur, render_play_icon_button_with_id, render_row_playlist_popup,
    render_track_download_button, track_play_url, track_title, SearchApp,
};
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
    open_guid: Option<&str>,
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
            open_guid,
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
    open_guid: Option<&str>,
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
    let popup_open = feed_guid.is_some()
        && !guid.is_empty()
        && open_guid.map(|s| s == guid.as_str()).unwrap_or(false);

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

    if feed_guid_owned.is_some() && !guid.is_empty() {
        let guid_for_toggle = guid.clone();
        row = row.child(
            Button::new(SharedString::from(format!("track-row-add:{guid}")))
                .label("+")
                .ghost()
                .with_size(Size::XSmall)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_add_to_playlist_track_guid(&guid_for_toggle);
                    cx.notify();
                })),
        );
    }

    row = row.child(render_play_icon_button_with_id(
        play_button_id,
        audio_url,
        cx,
    ));

    if popup_open {
        let feed_guid_str = feed_guid_owned.clone().unwrap_or_default();
        let track_guid_str = guid.clone();
        let feed_url_str = feed_url.map(str::to_string);
        let popup = render_row_playlist_popup(
            &feed_guid_str,
            feed_url_str.as_deref(),
            &track_guid_str,
            playlists,
            cx,
        );
        return div()
            .flex()
            .flex_col()
            .child(row)
            .child(popup)
            .into_any_element();
    }

    row.into_any_element()
}
