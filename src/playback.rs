//! Canonical now-playing assembly for Phase 2 playback sessions.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::playback_driver::DriverStatus;
use crate::{db, playlist_service, track_identity};

pub const DEFAULT_SESSION_ID: &str = "default";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NowPlayingUpdate {
    pub session_id: String,
    pub sequence: u64,
    pub started_at: DateTime<Utc>,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub feed_guid: String,
    pub item_guid: String,
    pub local_track_id: i64,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub image: Option<String>,
    pub value_block: serde_json::Value,
    pub raw_extra_json: serde_json::Value,
}

pub fn now_playing_update(conn: &Connection, session_id: &str) -> Result<Option<NowPlayingUpdate>> {
    let Some(session) = db::playback_session(conn, session_id)? else {
        return Ok(None);
    };
    if session.state == "stopped" {
        return Ok(None);
    }
    let source = track_identity::local_track_identity(conn, session.local_track_id)?;
    Ok(Some(update_from_parts(&session, source)?))
}

pub fn dry_run_playlist(
    conn: &Connection,
    playlist_id: i64,
    session_id: &str,
) -> Result<NowPlayingUpdate> {
    dry_run_playlist_at(conn, playlist_id, 0, session_id)
}

pub fn dry_run_playlist_at(
    conn: &Connection,
    playlist_id: i64,
    playlist_position: i64,
    session_id: &str,
) -> Result<NowPlayingUpdate> {
    anyhow::ensure!(
        playlist_position >= 0,
        "playlist position cannot be negative"
    );
    let selection = playlist_service::select_track_at(conn, playlist_id, playlist_position)?;
    let session = preview_session_row(
        session_id,
        selection.track_id,
        Some(selection.playlist_id),
        Some(selection.position),
    );
    update_from_parts(&session, selection.identity)
}

pub fn play_playlist_at(
    conn: &Connection,
    playlist_id: i64,
    playlist_position: i64,
    session_id: &str,
) -> Result<NowPlayingUpdate> {
    anyhow::ensure!(
        playlist_position >= 0,
        "playlist position cannot be negative"
    );
    let selection = playlist_service::select_track_at(conn, playlist_id, playlist_position)?;
    set_track_with_source(
        conn,
        selection.track_id,
        Some(selection.playlist_id),
        Some(selection.position),
        session_id,
    )
}

pub fn set_track(conn: &Connection, track_id: i64, session_id: &str) -> Result<NowPlayingUpdate> {
    set_track_with_source(conn, track_id, None, None, session_id)
}

pub fn skip_next(conn: &Connection, session_id: &str) -> Result<NowPlayingUpdate> {
    skip_by(conn, session_id, 1)
}

pub fn skip_previous(conn: &Connection, session_id: &str) -> Result<NowPlayingUpdate> {
    skip_by(conn, session_id, -1)
}

pub fn update_position(
    conn: &Connection,
    position_ms: u64,
    session_id: &str,
) -> Result<NowPlayingUpdate> {
    let session = db::update_playback_session_position(conn, session_id, position_ms)?;
    let source = track_identity::local_track_identity(conn, session.local_track_id)?;
    update_from_parts(&session, source)
}

pub fn update_paused(
    conn: &Connection,
    paused: bool,
    session_id: &str,
) -> Result<NowPlayingUpdate> {
    let session = db::update_playback_session_paused(conn, session_id, paused)?;
    let source = track_identity::local_track_identity(conn, session.local_track_id)?;
    update_from_parts(&session, source)
}

pub fn reconcile_driver_status(
    conn: &Connection,
    status: &DriverStatus,
    session_id: &str,
) -> Result<Option<NowPlayingUpdate>> {
    if let Some(error) = &status.error {
        anyhow::bail!("playback driver error: {error}");
    }
    if status.eof {
        return skip_next(conn, session_id).map(Some);
    }
    let Some(session) = db::reconcile_playback_session_driver_status(
        conn,
        session_id,
        status.position_ms,
        status.paused,
    )?
    else {
        return Ok(None);
    };
    if session.state == "stopped" {
        return Ok(None);
    }
    let source = track_identity::local_track_identity(conn, session.local_track_id)?;
    update_from_parts(&session, source).map(Some)
}

pub fn stop(conn: &Connection, session_id: &str) -> Result<db::PlaybackSessionRow> {
    db::stop_playback_session(conn, session_id)
}

fn skip_by(conn: &Connection, session_id: &str, delta: i64) -> Result<NowPlayingUpdate> {
    let session = db::playback_session(conn, session_id)?
        .with_context(|| format!("no playback session {session_id:?}"))?;
    anyhow::ensure!(
        session.state != "stopped",
        "playback session {session_id:?} is stopped"
    );
    let playlist_id = session
        .playlist_id
        .context("current playback session is not tied to a playlist")?;
    let playlist_position = session
        .playlist_position
        .context("current playback session has no playlist position")?;
    let selection =
        playlist_service::select_playable_track_after(conn, playlist_id, playlist_position, delta)?;
    set_track_with_source(
        conn,
        selection.track_id,
        Some(selection.playlist_id),
        Some(selection.position),
        session_id,
    )
}

fn set_track_with_source(
    conn: &Connection,
    track_id: i64,
    playlist_id: Option<i64>,
    playlist_position: Option<i64>,
    session_id: &str,
) -> Result<NowPlayingUpdate> {
    let source = track_identity::local_track_identity(conn, track_id)?;
    let started_at = DateTime::<Utc>::from(std::time::SystemTime::now()).to_rfc3339();
    let session = db::set_playback_session_track(
        conn,
        session_id,
        track_id,
        playlist_id,
        playlist_position,
        &started_at,
        0,
    )?;
    update_from_parts(&session, source)
}

fn preview_session_row(
    session_id: &str,
    local_track_id: i64,
    playlist_id: Option<i64>,
    playlist_position: Option<i64>,
) -> db::PlaybackSessionRow {
    db::PlaybackSessionRow {
        session_id: session_id.to_string(),
        sequence: 0,
        local_track_id,
        playlist_id,
        playlist_position,
        started_at: DateTime::<Utc>::from(std::time::SystemTime::now()).to_rfc3339(),
        position_ms: 0,
        state: "playing".to_string(),
    }
}

fn update_from_parts(
    session: &db::PlaybackSessionRow,
    source: track_identity::TrackIdentity,
) -> Result<NowPlayingUpdate> {
    let started_at = DateTime::parse_from_rfc3339(&session.started_at)
        .with_context(|| format!("parse started_at for session {}", session.session_id))?
        .with_timezone(&Utc);
    let value_block = source.value_block();

    Ok(NowPlayingUpdate {
        session_id: session.session_id.clone(),
        sequence: session.sequence,
        started_at,
        position_ms: session.position_ms,
        duration_ms: source.duration_ms,
        feed_guid: source.feed_guid,
        item_guid: source.item_guid,
        local_track_id: source.local_track_id,
        title: source.title,
        artist: source.artist,
        album: source.album,
        image: source.image,
        value_block,
        raw_extra_json: source.raw_extra_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_feed(conn: &Connection, feed_guid: Option<&str>) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (
                 feed_url, feed_guid, title, album_image_href, podcast_value_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "https://example.test/feed.xml",
                feed_guid,
                "Feed Title",
                "https://example.test/feed.png",
                r#"{"feed":"value"}"#
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(conn: &Connection, feed_id: i64, item_guid: &str, path: &str) -> Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, album_title,
                 duration_seconds, item_value_json, extra_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                feed_id,
                item_guid,
                format!("Track {item_guid}"),
                "Artist Name",
                "Album Title",
                123_i64,
                r#"{"item":"value"}"#,
                r#"{"source":"rss"}"#
            ],
        )?;
        let track_id = conn.last_insert_rowid();
        db::mark_track_downloaded(conn, track_id, std::path::Path::new(path), None)?;
        Ok(track_id)
    }

    fn create_playlist_track(conn: &Connection, feed_guid: Option<&str>) -> Result<(i64, i64)> {
        let feed_id = create_feed(conn, feed_guid)?;
        let track_id = create_track(conn, feed_id, "item-guid", "/tmp/track.mp3")?;
        let playlist_id = db::playlist_create(conn, "Phase 2")?;
        db::playlist_append(conn, playlist_id, track_id)?;
        Ok((playlist_id, track_id))
    }

    #[test]
    fn dry_run_playlist_builds_now_playing_update() -> Result<()> {
        let conn = setup_test_db()?;
        let (playlist_id, track_id) = create_playlist_track(&conn, Some("feed-guid"))?;

        let update = dry_run_playlist(&conn, playlist_id, DEFAULT_SESSION_ID)?;

        assert_eq!(update.session_id, DEFAULT_SESSION_ID);
        assert_eq!(update.sequence, 0);
        assert_eq!(update.local_track_id, track_id);
        assert_eq!(update.feed_guid, "feed-guid");
        assert_eq!(update.item_guid, "item-guid");
        assert_eq!(update.duration_ms, Some(123_000));
        assert_eq!(update.value_block["item"], "value");
        assert_eq!(update.raw_extra_json["source"], "rss");
        assert!(
            now_playing_update(&conn, DEFAULT_SESSION_ID)?.is_none(),
            "dry-run should not persist a playback session"
        );

        Ok(())
    }

    #[test]
    fn dry_run_playlist_can_preview_zero_based_position() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"))?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        let second_track_id = create_track(&conn, feed_id, "second-guid", "/tmp/second.mp3")?;
        let playlist_id = db::playlist_create(&conn, "Phase 2")?;
        db::playlist_append(&conn, playlist_id, first_track_id)?;
        db::playlist_append(&conn, playlist_id, second_track_id)?;

        let update = dry_run_playlist_at(&conn, playlist_id, 1, DEFAULT_SESSION_ID)?;

        assert_eq!(update.local_track_id, second_track_id);
        assert_eq!(update.item_guid, "second-guid");

        Ok(())
    }

    #[test]
    fn play_playlist_persists_now_playing_session() -> Result<()> {
        let conn = setup_test_db()?;
        let (playlist_id, track_id) = create_playlist_track(&conn, Some("feed-guid"))?;

        let update = play_playlist_at(&conn, playlist_id, 0, DEFAULT_SESSION_ID)?;
        let current = now_playing_update(&conn, DEFAULT_SESSION_ID)?.expect("current session");

        assert_eq!(update.sequence, 1);
        assert_eq!(update.local_track_id, track_id);
        assert_eq!(current.local_track_id, track_id);
        assert_eq!(current.sequence, 1);

        Ok(())
    }

    #[test]
    fn skip_next_and_previous_move_within_playlist() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"))?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        let second_track_id = create_track(&conn, feed_id, "second-guid", "/tmp/second.mp3")?;
        let playlist_id = db::playlist_create(&conn, "Phase 2")?;
        db::playlist_append(&conn, playlist_id, first_track_id)?;
        db::playlist_append(&conn, playlist_id, second_track_id)?;

        let first = play_playlist_at(&conn, playlist_id, 0, DEFAULT_SESSION_ID)?;
        let second = skip_next(&conn, DEFAULT_SESSION_ID)?;
        let previous = skip_previous(&conn, DEFAULT_SESSION_ID)?;

        assert_eq!(first.local_track_id, first_track_id);
        assert_eq!(second.local_track_id, second_track_id);
        assert_eq!(second.sequence, 2);
        assert_eq!(previous.local_track_id, first_track_id);
        assert_eq!(previous.sequence, 3);

        Ok(())
    }

    #[test]
    fn skip_next_and_previous_ignore_unavailable_playlist_rows() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"))?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        let removed_track_id = create_track(&conn, feed_id, "removed-guid", "/tmp/removed.mp3")?;
        let third_track_id = create_track(&conn, feed_id, "third-guid", "/tmp/third.mp3")?;
        let playlist_id = db::playlist_create(&conn, "Phase 2")?;
        db::playlist_append(&conn, playlist_id, first_track_id)?;
        db::playlist_append(&conn, playlist_id, removed_track_id)?;
        db::playlist_append(&conn, playlist_id, third_track_id)?;
        db::set_track_in_library(&conn, removed_track_id, false)?;

        let first = play_playlist_at(&conn, playlist_id, 0, DEFAULT_SESSION_ID)?;
        let third = skip_next(&conn, DEFAULT_SESSION_ID)?;
        let previous = skip_previous(&conn, DEFAULT_SESSION_ID)?;

        assert_eq!(first.local_track_id, first_track_id);
        assert_eq!(third.local_track_id, third_track_id);
        assert_eq!(previous.local_track_id, first_track_id);

        Ok(())
    }

    #[test]
    fn current_now_playing_reads_persisted_session_and_increments_sequence() -> Result<()> {
        let conn = setup_test_db()?;
        let (_, track_id) = create_playlist_track(&conn, Some("feed-guid"))?;

        let first = set_track(&conn, track_id, DEFAULT_SESSION_ID)?;
        let second = update_position(&conn, 42_000, DEFAULT_SESSION_ID)?;
        let current = now_playing_update(&conn, DEFAULT_SESSION_ID)?.expect("current session");

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(current.sequence, 2);
        assert_eq!(current.position_ms, 42_000);
        assert_eq!(current.feed_guid, "feed-guid");

        Ok(())
    }

    #[test]
    fn paused_session_still_emits_now_playing() -> Result<()> {
        let conn = setup_test_db()?;
        let (_, track_id) = create_playlist_track(&conn, Some("feed-guid"))?;

        set_track(&conn, track_id, DEFAULT_SESSION_ID)?;
        let paused = update_paused(&conn, true, DEFAULT_SESSION_ID)?;
        let row = db::playback_session(&conn, DEFAULT_SESSION_ID)?.expect("session");
        let current = now_playing_update(&conn, DEFAULT_SESSION_ID)?.expect("current session");

        assert_eq!(paused.sequence, 2);
        assert_eq!(row.state, "paused");
        assert_eq!(current.local_track_id, track_id);

        Ok(())
    }

    #[test]
    fn reconcile_driver_status_updates_position_and_pause_state() -> Result<()> {
        let conn = setup_test_db()?;
        let (_, track_id) = create_playlist_track(&conn, Some("feed-guid"))?;

        set_track(&conn, track_id, DEFAULT_SESSION_ID)?;
        let status = DriverStatus {
            position_ms: 7_000,
            paused: true,
            eof: false,
            error: None,
        };
        let update =
            reconcile_driver_status(&conn, &status, DEFAULT_SESSION_ID)?.expect("reconciled");
        let row = db::playback_session(&conn, DEFAULT_SESSION_ID)?.expect("session");

        assert_eq!(update.position_ms, 7_000);
        assert_eq!(row.state, "paused");
        assert_eq!(row.sequence, 2);

        Ok(())
    }

    #[test]
    fn reconcile_eof_uses_playlist_advance_path() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"))?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        let second_track_id = create_track(&conn, feed_id, "second-guid", "/tmp/second.mp3")?;
        let playlist_id = db::playlist_create(&conn, "Phase 2")?;
        db::playlist_append(&conn, playlist_id, first_track_id)?;
        db::playlist_append(&conn, playlist_id, second_track_id)?;

        play_playlist_at(&conn, playlist_id, 0, DEFAULT_SESSION_ID)?;
        let status = DriverStatus {
            eof: true,
            ..DriverStatus::default()
        };
        let update = reconcile_driver_status(&conn, &status, DEFAULT_SESSION_ID)?.expect("next");

        assert_eq!(update.local_track_id, second_track_id);
        assert_eq!(update.sequence, 2);

        Ok(())
    }

    #[test]
    fn missing_feed_guid_is_rejected() -> Result<()> {
        let conn = setup_test_db()?;
        let (playlist_id, _) = create_playlist_track(&conn, None)?;

        let result = dry_run_playlist(&conn, playlist_id, DEFAULT_SESSION_ID);

        assert!(result.is_err(), "missing feed GUID should not be inferred");
        assert!(
            now_playing_update(&conn, DEFAULT_SESSION_ID)?.is_none(),
            "failed dry-run should not persist a playback session"
        );

        Ok(())
    }

    #[test]
    fn set_track_rejects_invalid_source_before_persisting() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, None)?;
        let track_id = create_track(&conn, feed_id, "item-guid", "/tmp/track.mp3")?;

        let result = set_track(&conn, track_id, DEFAULT_SESSION_ID);

        assert!(result.is_err(), "missing feed GUID should not be inferred");
        assert!(
            now_playing_update(&conn, DEFAULT_SESSION_ID)?.is_none(),
            "failed set-track should not persist a playback session"
        );

        Ok(())
    }

    #[test]
    fn stop_hides_current_now_playing_but_increments_sequence() -> Result<()> {
        let conn = setup_test_db()?;
        let (_, track_id) = create_playlist_track(&conn, Some("feed-guid"))?;

        let playing = set_track(&conn, track_id, DEFAULT_SESSION_ID)?;
        let stopped = stop(&conn, DEFAULT_SESSION_ID)?;
        let current = now_playing_update(&conn, DEFAULT_SESSION_ID)?;

        assert_eq!(playing.sequence, 1);
        assert_eq!(stopped.sequence, 2);
        assert_eq!(stopped.state, "stopped");
        assert!(
            current.is_none(),
            "stopped sessions should not emit now-playing JSON"
        );

        Ok(())
    }
}
