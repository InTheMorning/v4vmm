//! Playlist local query family.

use rusqlite::Connection;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::errors::command::CommandError;
use crate::{db, playlist_service};

impl ApplicationQueryService {
    /// Lists local playlists.
    ///
    /// # Errors
    ///
    /// Returns an error when local playlist storage cannot be read.
    pub fn playlists(&self, conn: &Connection) -> Result<Vec<db::Playlist>, CommandError> {
        playlist_service::list(conn).map_err(|error| query_error(&error))
    }

    /// Lists tracks in a local playlist.
    ///
    /// # Errors
    ///
    /// Returns an error when local playlist storage cannot be read.
    pub fn playlist_tracks(
        &self,
        conn: &Connection,
        playlist_id: i64,
    ) -> Result<Vec<db::TrackRow>, CommandError> {
        playlist_service::tracks(conn, playlist_id).map_err(|error| query_error(&error))
    }
}

fn query_error(error: &anyhow::Error) -> CommandError {
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

    #[test]
    fn playlist_queries_return_local_snapshots() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let service = ApplicationQueryService::new();
        let playlist_id = playlist_service::create(&conn, "Focus")?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id, "first")?;
        playlist_service::append_track(&conn, playlist_id, track_id)?;

        let playlists = service.playlists(&conn)?;
        let tracks = service.playlist_tracks(&conn, playlist_id)?;

        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].id, playlist_id);
        assert_eq!(playlists[0].name, "Focus");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, track_id);

        Ok(())
    }

    fn create_feed(conn: &Connection) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(conn: &Connection, feed_id: i64, item_guid: &str) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, is_in_library)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![feed_id, item_guid, format!("Track {item_guid}")],
        )?;
        Ok(conn.last_insert_rowid())
    }
}
