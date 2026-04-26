//! Canonical now-playing assembly for Phase 2 playback sessions.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{db, track_identity};

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
    let (track_id, position) = db::playlist_track_at(conn, playlist_id, playlist_position)?
        .with_context(|| {
            format!("playlist {playlist_id} has no track at position {playlist_position}")
        })?;
    let source = track_identity::local_track_identity(conn, track_id)?;
    let session = preview_session_row(session_id, track_id, Some(playlist_id), Some(position));
    update_from_parts(&session, source)
}

pub fn set_track(conn: &Connection, track_id: i64, session_id: &str) -> Result<NowPlayingUpdate> {
    set_track_with_source(conn, track_id, None, None, session_id)
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

pub fn stop(conn: &Connection, session_id: &str) -> Result<db::PlaybackSessionRow> {
    db::stop_playback_session(conn, session_id)
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
