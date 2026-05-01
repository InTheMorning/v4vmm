//! Playback local query family.

use rusqlite::Connection;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::errors::command::CommandError;
use crate::{db, playback};

/// Local now-playing snapshot for presentation adapters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlaybackSnapshot {
    active: bool,
    paused: bool,
    title: Option<String>,
}

impl PlaybackSnapshot {
    /// Creates an inactive playback snapshot.
    #[must_use]
    pub const fn inactive() -> Self {
        Self {
            active: false,
            paused: false,
            title: None,
        }
    }

    /// Creates an active playback snapshot.
    #[must_use]
    pub fn active(paused: bool, title: impl Into<String>) -> Self {
        Self {
            active: true,
            paused,
            title: Some(title.into()),
        }
    }

    /// Returns whether playback is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Returns whether playback is paused.
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    /// Returns the current title when playback is active.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl ApplicationQueryService {
    /// Reads the local now-playing snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the local playback session cannot be read.
    pub fn playback_snapshot(
        &self,
        conn: &Connection,
        session_id: &str,
    ) -> Result<PlaybackSnapshot, CommandError> {
        let Some(session) = db::playback_session(conn, session_id).map_err(query_error)? else {
            return Ok(PlaybackSnapshot::inactive());
        };
        if session.state == "stopped" {
            return Ok(PlaybackSnapshot::inactive());
        }
        let title = playback::now_playing_update(conn, session_id)
            .map_err(query_error)?
            .map_or_else(|| "Current track".to_string(), |update| update.title);
        Ok(PlaybackSnapshot::active(session.state == "paused", title))
    }
}

fn query_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::Query(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_feed(conn: &Connection) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(conn: &Connection, feed_id: i64) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, album_title,
                 duration_seconds, item_value_json, extra_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                feed_id,
                "item-guid",
                "Track Title",
                "Artist Name",
                "Album Title",
                123_i64,
                r#"{"item":"value"}"#,
                r#"{"source":"rss"}"#
            ],
        )?;
        let track_id = conn.last_insert_rowid();
        db::mark_track_downloaded(conn, track_id, std::path::Path::new("/tmp/track.mp3"), None)?;
        Ok(track_id)
    }

    #[test]
    fn playback_snapshot_reads_local_now_playing_state() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id)?;
        playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        playback::update_paused(&conn, true, playback::DEFAULT_SESSION_ID)?;

        let snapshot = ApplicationQueryService::new()
            .playback_snapshot(&conn, playback::DEFAULT_SESSION_ID)?;

        assert!(snapshot.is_active());
        assert!(snapshot.is_paused());
        assert_eq!(snapshot.title(), Some("Track Title"));
        Ok(())
    }
}
