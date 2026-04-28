//! Long-running playback owner for live driver mode.
//!
//! One-shot CLI commands keep using `playback.rs` directly. This owner is for
//! processes that can hold a driver, poll it, and reconcile observed state back
//! into the canonical playback session.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::playback_driver::{DriverStatus, PlaybackDriver};
use crate::{db, playback, playlist_service, track_identity};

#[derive(Clone, Debug)]
pub enum PollOutcome {
    NoSession,
    Reconciled(Option<playback::NowPlayingUpdate>),
    Advanced(playback::NowPlayingUpdate),
}

#[derive(Debug)]
pub struct PlaybackOwner<D> {
    driver: D,
    session_id: String,
    eof_armed: bool,
    loaded_track_id: Option<i64>,
}

impl<D: PlaybackDriver> PlaybackOwner<D> {
    pub fn new(driver: D, session_id: impl Into<String>) -> Self {
        Self {
            driver,
            session_id: session_id.into(),
            eof_armed: true,
            loaded_track_id: None,
        }
    }

    pub fn driver(&self) -> &D {
        &self.driver
    }

    pub fn load_track_path(
        &mut self,
        conn: &Connection,
        track_id: i64,
        path: &Path,
        start_ms: u64,
    ) -> Result<playback::NowPlayingUpdate> {
        self.driver.load(path, start_ms)?;
        self.eof_armed = true;
        self.loaded_track_id = Some(track_id);
        let update = playback::set_track(conn, track_id, &self.session_id)?;
        if start_ms == 0 {
            return Ok(update);
        }
        playback::update_position(conn, start_ms, &self.session_id)
    }

    pub fn play_playlist_at(
        &mut self,
        conn: &Connection,
        playlist_id: i64,
        playlist_position: i64,
    ) -> Result<playback::NowPlayingUpdate> {
        let selection = playlist_service::select_track_at(conn, playlist_id, playlist_position)?;
        self.driver
            .load(Path::new(&selection.identity.local_path), 0)?;
        self.eof_armed = true;
        self.loaded_track_id = Some(selection.track_id);
        playback::play_playlist_at(conn, playlist_id, playlist_position, &self.session_id)
    }

    pub fn load_current_session(&mut self, conn: &Connection) -> Result<Option<DriverStatus>> {
        let Some(session) = db::playback_session(conn, &self.session_id)? else {
            return Ok(None);
        };
        if session.state == "stopped" {
            self.loaded_track_id = None;
            return Ok(None);
        }
        let identity = track_identity::local_track_identity(conn, session.local_track_id)?;
        self.driver
            .load(Path::new(&identity.local_path), session.position_ms)?;
        if session.state == "paused" {
            self.driver.pause(true)?;
        }
        self.eof_armed = true;
        self.loaded_track_id = Some(session.local_track_id);
        self.driver.poll().map(Some)
    }

    pub fn seek(
        &mut self,
        conn: &Connection,
        position_ms: u64,
    ) -> Result<playback::NowPlayingUpdate> {
        self.driver.seek(position_ms)?;
        playback::update_position(conn, position_ms, &self.session_id)
    }

    pub fn pause(&mut self, conn: &Connection, paused: bool) -> Result<playback::NowPlayingUpdate> {
        self.driver.pause(paused)?;
        playback::update_paused(conn, paused, &self.session_id)
    }

    pub fn stop(&mut self, conn: &Connection) -> Result<db::PlaybackSessionRow> {
        self.driver.stop()?;
        self.eof_armed = false;
        self.loaded_track_id = None;
        playback::stop(conn, &self.session_id)
    }

    pub fn poll(&mut self, conn: &Connection) -> Result<PollOutcome> {
        let Some(session) = db::playback_session(conn, &self.session_id)? else {
            if self.loaded_track_id.take().is_some() {
                self.driver.stop()?;
            }
            return Ok(PollOutcome::NoSession);
        };
        if session.state == "stopped" {
            if self.loaded_track_id.take().is_some() {
                self.driver.stop()?;
            }
            return Ok(PollOutcome::Reconciled(None));
        }
        if self.loaded_track_id != Some(session.local_track_id) {
            let identity = track_identity::local_track_identity(conn, session.local_track_id)?;
            self.driver
                .load(Path::new(&identity.local_path), session.position_ms)?;
            if session.state == "paused" {
                self.driver.pause(true)?;
            }
            self.loaded_track_id = Some(session.local_track_id);
            return playback::now_playing_update(conn, &self.session_id)
                .map(PollOutcome::Reconciled);
        }
        let status = self.driver.poll()?;
        if let Some(error) = &status.error {
            anyhow::bail!("playback driver error: {error}");
        }
        if status.eof {
            if !self.eof_armed {
                return Ok(PollOutcome::Reconciled(None));
            }
            self.eof_armed = false;
            let update = playback::skip_next(conn, &self.session_id)
                .context("advance playlist after driver EOF")?;
            let identity = track_identity::local_track_identity(conn, update.local_track_id)?;
            self.driver.load(Path::new(&identity.local_path), 0)?;
            self.loaded_track_id = Some(update.local_track_id);
            self.eof_armed = true;
            return Ok(PollOutcome::Advanced(update));
        }
        self.eof_armed = true;
        let update = playback::reconcile_driver_status(conn, &status, &self.session_id)?;
        Ok(PollOutcome::Reconciled(update))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::Result;

    use super::*;
    use crate::playback_driver::NullDriver;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_feed(conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (
                 feed_url, feed_guid, title, album_image_href, podcast_value_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "https://example.test/feed.xml",
                "feed-guid",
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

    #[test]
    fn owner_seek_updates_driver_and_session_immediately() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "item-guid", "/tmp/track.mp3")?;
        playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        let mut owner = PlaybackOwner::new(NullDriver::new(), playback::DEFAULT_SESSION_ID);
        owner.load_current_session(&conn)?;

        let update = owner.seek(&conn, 12_000)?;
        let snap = owner.driver().snapshot();

        assert_eq!(update.position_ms, 12_000);
        assert_eq!(snap.position_ms, 12_000);
        Ok(())
    }

    #[test]
    fn owner_pause_persists_paused_state() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "item-guid", "/tmp/track.mp3")?;
        playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        let mut owner = PlaybackOwner::new(NullDriver::new(), playback::DEFAULT_SESSION_ID);
        owner.load_current_session(&conn)?;

        owner.pause(&conn, true)?;
        let row = db::playback_session(&conn, playback::DEFAULT_SESSION_ID)?.expect("session");

        assert_eq!(row.state, "paused");
        assert!(owner.driver().snapshot().paused);
        Ok(())
    }

    #[test]
    fn owner_play_playlist_at_loads_driver_and_preserves_playlist_context() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let first_track_id = create_track(&conn, feed_id, "first-guid", "/tmp/first.mp3")?;
        let second_track_id = create_track(&conn, feed_id, "second-guid", "/tmp/second.mp3")?;
        let playlist_id = db::playlist_create(&conn, "Phase 2")?;
        db::playlist_append(&conn, playlist_id, first_track_id)?;
        db::playlist_append(&conn, playlist_id, second_track_id)?;
        let mut owner = PlaybackOwner::new(NullDriver::new(), playback::DEFAULT_SESSION_ID);

        let update = owner.play_playlist_at(&conn, playlist_id, 1)?;
        let row = db::playback_session(&conn, playback::DEFAULT_SESSION_ID)?.expect("session");
        let snap = owner.driver().snapshot();

        assert_eq!(update.local_track_id, second_track_id);
        assert_eq!(row.playlist_id, Some(playlist_id));
        assert_eq!(row.playlist_position, Some(1));
        assert_eq!(snap.loaded_path, Some(PathBuf::from("/tmp/second.mp3")));
        Ok(())
    }

    #[test]
    fn owner_load_current_uses_stored_path_and_position() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "item-guid", "/tmp/track.mp3")?;
        playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        playback::update_position(&conn, 9_000, playback::DEFAULT_SESSION_ID)?;
        let mut owner = PlaybackOwner::new(NullDriver::new(), playback::DEFAULT_SESSION_ID);

        owner.load_current_session(&conn)?;
        let snap = owner.driver().snapshot();

        assert_eq!(snap.loaded_path, Some(PathBuf::from("/tmp/track.mp3")));
        assert_eq!(snap.position_ms, 9_000);
        Ok(())
    }

    #[test]
    fn owner_poll_loads_new_active_session_before_reconciling() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "item-guid", "/tmp/track.mp3")?;
        playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        playback::update_position(&conn, 5_000, playback::DEFAULT_SESSION_ID)?;
        let mut owner = PlaybackOwner::new(NullDriver::new(), playback::DEFAULT_SESSION_ID);

        let outcome = owner.poll(&conn)?;
        let snap = owner.driver().snapshot();

        assert!(matches!(outcome, PollOutcome::Reconciled(Some(_))));
        assert_eq!(snap.loaded_path, Some(PathBuf::from("/tmp/track.mp3")));
        assert_eq!(snap.position_ms, 5_000);
        Ok(())
    }

    #[test]
    fn owner_poll_stops_driver_when_session_is_stopped() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "item-guid", "/tmp/track.mp3")?;
        playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        let mut owner = PlaybackOwner::new(NullDriver::new(), playback::DEFAULT_SESSION_ID);
        owner.load_current_session(&conn)?;
        playback::stop(&conn, playback::DEFAULT_SESSION_ID)?;

        let outcome = owner.poll(&conn)?;

        assert!(matches!(outcome, PollOutcome::Reconciled(None)));
        assert!(owner.driver().snapshot().stopped);
        Ok(())
    }
}
