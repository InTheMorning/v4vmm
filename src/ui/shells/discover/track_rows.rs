//! Discover feed track-list row assembly.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{AnyElement, Context, Image};

use crate::api::{Feed, Track};
use crate::db;
use crate::library_service;
use crate::search::{FeedTrackListContext, SearchApp};
use crate::ui::shells::track;
use crate::view_models::search::TrackRowActionVm;

#[expect(
    clippy::needless_pass_by_value,
    reason = "feed is cloned into per-row action closures while rows are consumed"
)]
pub(crate) fn render_track_list_rows(
    tracks: Vec<Track>,
    feed: Option<Feed>,
    feed_context: Option<FeedTrackListContext<'_>>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> Vec<AnyElement> {
    let downloaded: Vec<bool> = {
        let db = app.conn.lock().ok();
        tracks
            .iter()
            .map(|track| {
                db.as_ref()
                    .and_then(|db| {
                        library_service::track_is_in_library_by_match(
                            db,
                            track
                                .feed_url
                                .as_deref()
                                .or_else(|| feed.as_ref().and_then(|f| f.feed_url.as_deref())),
                            track.track_guid.as_deref(),
                            track.enclosure_url.as_deref(),
                        )
                        .ok()
                    })
                    .unwrap_or(false)
            })
            .collect()
    };
    let (feed_guid, feed_url, playlists) = match feed_context {
        Some((g, url, pls)) => (Some(g.to_string()), url.map(str::to_string), pls.to_vec()),
        None => (None, None, Vec::new()),
    };

    tracks
        .into_iter()
        .zip(downloaded)
        .map(|(track, is_downloaded)| {
            let key = TrackRowActionVm::new(&track, is_downloaded, false).key();
            let is_in_flight = app.vm.is_track_operation_in_flight(&key);
            let thumb = app.thumbnail_for_url(track.image_url.as_deref(), cx);
            render_track_row(
                track,
                thumb,
                feed.clone(),
                is_downloaded,
                is_in_flight,
                feed_guid.as_deref(),
                feed_url.as_deref(),
                &playlists,
                cx,
            )
        })
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "stage 4 keeps the legacy Discover row wrapper stable while delegating"
)]
fn render_track_row(
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
    track::render_track_row(
        track,
        thumbnail,
        feed,
        is_downloaded,
        is_in_flight,
        feed_guid,
        feed_url,
        playlists,
        track::TrackRowMode::Discover,
        cx,
    )
}
