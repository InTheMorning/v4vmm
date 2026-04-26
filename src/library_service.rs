//! Non-UI library and local-file state service boundary.

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use crate::db;

pub fn library_tracks(conn: &Connection) -> Result<Vec<db::TrackRow>> {
    db::library_tracks(conn)
}

pub fn tracks_for_feed(conn: &Connection, feed_id: i64) -> Result<Vec<db::TrackRow>> {
    db::library_tracks_for_feed(conn, feed_id)
}

pub fn cached_tracks(conn: &Connection) -> Result<Vec<db::TrackRow>> {
    db::cached_tracks(conn)
}

pub fn set_track_in_library(conn: &Connection, track_id: i64, in_library: bool) -> Result<()> {
    db::set_track_in_library(conn, track_id, in_library)
}

pub fn set_track_in_library_by_match(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
    in_library: bool,
) -> Result<bool> {
    db::set_track_in_library_by_match(conn, feed_url, item_guid, enclosure_url, in_library)
}

pub fn track_is_in_library_by_match(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<bool> {
    db::track_is_in_library_by_match(conn, feed_url, item_guid, enclosure_url)
}

pub fn mark_track_downloaded(
    conn: &Connection,
    track_id: i64,
    path: &Path,
    file_size_bytes: Option<i64>,
) -> Result<()> {
    db::mark_track_downloaded(conn, track_id, path, file_size_bytes)
}

pub fn mark_track_downloaded_by_match(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
    path: &Path,
    file_size_bytes: Option<i64>,
) -> Result<bool> {
    db::mark_track_downloaded_by_match(
        conn,
        feed_url,
        item_guid,
        enclosure_url,
        path,
        file_size_bytes,
    )
}

pub fn delete_local_file(conn: &Connection, local_file_path: &str) -> Result<()> {
    db::delete_local_file(conn, local_file_path)
}

pub fn find_track_id(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<Option<i64>> {
    db::find_track_id(conn, feed_url, item_guid, enclosure_url)
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

    fn create_feed(conn: &Connection, feed_url: &str) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![feed_url, "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(
        conn: &Connection,
        feed_id: i64,
        item_guid: &str,
        enclosure_url: &str,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, enclosure_url, track_title, artist_name
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![feed_id, item_guid, enclosure_url, "Track Title", "Artist"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn mark_downloaded_sets_library_state_and_local_file() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "https://example.test/feed.xml")?;
        let track_id = create_track(
            &conn,
            feed_id,
            "item-guid",
            "https://example.test/audio.mp3",
        )?;

        mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            Some(123),
        )?;

        let rows = library_tracks(&conn)?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, track_id);
        assert_eq!(rows[0].local_path.as_deref(), Some("/tmp/track.mp3"));

        Ok(())
    }

    #[test]
    fn cached_tracks_returns_downloaded_tracks_not_in_library() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "https://example.test/feed.xml")?;
        let track_id = create_track(
            &conn,
            feed_id,
            "item-guid",
            "https://example.test/audio.mp3",
        )?;
        mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;
        set_track_in_library(&conn, track_id, false)?;

        let rows = cached_tracks(&conn)?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, track_id);

        Ok(())
    }

    #[test]
    fn match_helpers_use_feed_scoped_identity_first() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_one = create_feed(&conn, "https://example.test/one.xml")?;
        let feed_two = create_feed(&conn, "https://example.test/two.xml")?;
        let first_track_id =
            create_track(&conn, feed_one, "first-guid", "https://cdn.test/shared.mp3")?;
        let second_track_id = create_track(
            &conn,
            feed_two,
            "second-guid",
            "https://cdn.test/shared.mp3",
        )?;

        let matched = find_track_id(
            &conn,
            Some("https://example.test/two.xml"),
            Some("second-guid"),
            Some("https://cdn.test/shared.mp3"),
        )?;

        assert_eq!(matched, Some(second_track_id));
        assert_ne!(matched, Some(first_track_id));

        Ok(())
    }

    #[test]
    fn delete_local_file_removes_cache_record() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "https://example.test/feed.xml")?;
        let track_id = create_track(
            &conn,
            feed_id,
            "item-guid",
            "https://example.test/audio.mp3",
        )?;
        mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;
        set_track_in_library(&conn, track_id, false)?;

        delete_local_file(&conn, "/tmp/track.mp3")?;

        assert!(cached_tracks(&conn)?.is_empty());

        Ok(())
    }
}
