//! Queue/Now Playing frame adapter.
//!
//! This module bridges application playback/session state into the ADR 0046
//! Queue frame display contract. Toolbar rendering stays in `playback_bar.rs`;
//! the detailed queue and output controls are projected here.

use gpui::{App, ClickEvent, Context, Entity, Window};

use crate::ui::shells::queue_now_playing::{
    render_queue_now_playing, QueueNowPlayingShell, QueueNowPlayingSlots,
};
use crate::view_models::queue_now_playing::{
    LiveValueDeviceDisplay, QueueNowPlayingPageVm, QueueTrackInput, TransportState, VolumeDisplay,
};
use crate::{db, playback};

use super::TopApp;

pub(super) fn build_queue_now_playing_frame(
    app: &TopApp,
    cx: &mut Context<TopApp>,
) -> QueueNowPlayingShell {
    let entity = cx.entity();
    render_queue_now_playing(
        queue_now_playing_vm(app),
        QueueNowPlayingSlots::new()
            .on_skip_previous(queue_transport_action(
                entity.clone(),
                TopApp::skip_playback_previous,
            ))
            .on_play_pause(queue_transport_action(
                entity.clone(),
                TopApp::toggle_playback_paused,
            ))
            .on_skip_next(queue_transport_action(entity, TopApp::skip_playback_next)),
    )
}

type QueueTransportAction = fn(&mut TopApp, &mut Context<TopApp>);

fn queue_transport_action(
    entity: Entity<TopApp>,
    action: QueueTransportAction,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        entity.update(cx, action);
    }
}

fn queue_now_playing_vm(app: &TopApp) -> QueueNowPlayingPageVm {
    let conn = app.conn.lock().expect("lock db");
    let session = db::playback_session(&conn, playback::DEFAULT_SESSION_ID)
        .ok()
        .flatten()
        .filter(|session| session.state != "stopped");
    let transport_state = session.as_ref().map_or(TransportState::Stopped, |session| {
        if session.state == "paused" {
            TransportState::Paused
        } else {
            TransportState::Playing
        }
    });
    let queue = session
        .as_ref()
        .map_or_else(QueueProjection::default, |session| {
            queue_tracks_for_session(app, &conn, session)
        });

    QueueNowPlayingPageVm::builder()
        .tracks(queue.tracks)
        .transport_state(transport_state)
        .skip_availability(queue.can_skip_previous, queue.can_skip_next)
        .live_value(LiveValueDeviceDisplay::unavailable())
        .volume(VolumeDisplay::new(1.0, true))
        .build()
}

#[derive(Default)]
struct QueueProjection {
    tracks: Vec<QueueTrackInput>,
    can_skip_previous: bool,
    can_skip_next: bool,
}

fn queue_tracks_for_session(
    app: &TopApp,
    conn: &rusqlite::Connection,
    session: &db::PlaybackSessionRow,
) -> QueueProjection {
    if let Some(playlist_id) = session.playlist_id {
        let rows = app
            .application_services
            .query_service()
            .playlist_tracks(conn, playlist_id)
            .unwrap_or_default();
        return playlist_queue_projection(rows, session.local_track_id);
    }

    db::track_row_by_id(conn, session.local_track_id)
        .ok()
        .flatten()
        .map_or_else(QueueProjection::default, |row| QueueProjection {
            tracks: vec![queue_track_input(row, session.local_track_id)],
            can_skip_previous: false,
            can_skip_next: false,
        })
}

fn playlist_queue_projection(rows: Vec<db::TrackRow>, active_track_id: i64) -> QueueProjection {
    let active_index = rows.iter().position(|row| row.id == active_track_id);
    let can_skip_previous =
        active_index.is_some_and(|index| rows[..index].iter().rev().any(queue_track_can_play));
    let can_skip_next = active_index.is_some_and(|index| {
        rows[index.saturating_add(1)..]
            .iter()
            .any(queue_track_can_play)
    });
    QueueProjection {
        tracks: rows
            .into_iter()
            .map(|row| queue_track_input(row, active_track_id))
            .collect(),
        can_skip_previous,
        can_skip_next,
    }
}

fn queue_track_can_play(row: &db::TrackRow) -> bool {
    row.is_in_library && row.local_path.is_some()
}

fn queue_track_input(row: db::TrackRow, active_track_id: i64) -> QueueTrackInput {
    QueueTrackInput {
        id: row.id,
        title: row.track_title.or(row.feed_title),
        artist: row.artist_name.or(row.album_artist_name),
        duration_seconds: row.duration_seconds,
        now_playing: row.id == active_track_id,
    }
}
