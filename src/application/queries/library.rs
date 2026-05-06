//! Library local query family.

use rusqlite::Connection;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::errors::command::CommandError;
use crate::{db, library_service};

impl ApplicationQueryService {
    /// Lists cached local tracks that are not currently in the library.
    ///
    /// # Errors
    ///
    /// Returns an error when local cached-track state cannot be read.
    pub fn cached_tracks(&self, conn: &Connection) -> Result<Vec<db::TrackRow>, CommandError> {
        library_service::cached_tracks(conn).map_err(|error| query_error(&error))
    }
}

fn query_error(error: &anyhow::Error) -> CommandError {
    CommandError::Query(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    #[test]
    fn library_queries_return_cached_tracks() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id)?;
        library_service::mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;
        library_service::set_track_in_library(&conn, track_id, false)?;

        let rows = ApplicationQueryService::new().cached_tracks(&conn)?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, track_id);

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

    fn create_track(conn: &Connection, feed_id: i64) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![feed_id, "item-guid", "Track Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }
}
