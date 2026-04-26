//! JSON-ready read contracts for local debugging and future UI checks.

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::{db, track_identity};

#[derive(Clone, Debug, Serialize)]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub track_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TrackSummary {
    pub id: i64,
    pub feed_id: i64,
    pub feed_guid: Option<String>,
    pub item_guid: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub enclosure_url: Option<String>,
    pub enclosure_type: Option<String>,
    pub image: Option<String>,
    pub is_in_library: bool,
    pub feed_title: Option<String>,
    pub album_image: Option<String>,
    pub local_path: Option<String>,
    pub transcript_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TrackIdentityDebug {
    pub local_track_id: i64,
    pub feed_id: i64,
    pub feed_guid: String,
    pub item_guid: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub image: Option<String>,
    pub duration_ms: Option<u64>,
    pub local_path: String,
    pub value_block: serde_json::Value,
    pub item_value_block: serde_json::Value,
    pub feed_value_block: serde_json::Value,
    pub raw_extra_json: serde_json::Value,
}

pub fn playlists(conn: &Connection) -> Result<Vec<PlaylistSummary>> {
    db::playlists_list(conn).map(|rows| rows.into_iter().map(Into::into).collect())
}

pub fn playlist_tracks(conn: &Connection, playlist_id: i64) -> Result<Vec<TrackSummary>> {
    db::playlist_tracks(conn, playlist_id).map(track_summaries)
}

pub fn library_tracks(conn: &Connection) -> Result<Vec<TrackSummary>> {
    db::library_tracks(conn).map(track_summaries)
}

pub fn track_inspect(conn: &Connection, track_id: i64) -> Result<TrackIdentityDebug> {
    track_identity::local_track_identity(conn, track_id).map(Into::into)
}

fn track_summaries(rows: Vec<db::TrackRow>) -> Vec<TrackSummary> {
    rows.into_iter().map(Into::into).collect()
}

impl From<db::Playlist> for PlaylistSummary {
    fn from(value: db::Playlist) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            track_count: value.track_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<db::TrackRow> for TrackSummary {
    fn from(value: db::TrackRow) -> Self {
        Self {
            id: value.id,
            feed_id: value.feed_id,
            feed_guid: value.feed_guid,
            item_guid: value.item_guid,
            title: value.track_title,
            artist: value.artist_name,
            album: value.album_title,
            album_artist: value.album_artist_name,
            track_number: value.track_number,
            disc_number: value.disc_number,
            duration_seconds: value.duration_seconds,
            enclosure_url: value.enclosure_url,
            enclosure_type: value.enclosure_type,
            image: value.track_image_href,
            is_in_library: value.is_in_library,
            feed_title: value.feed_title,
            album_image: value.album_image_href,
            local_path: value.local_path,
            transcript_url: value.transcript_url,
        }
    }
}

impl From<track_identity::TrackIdentity> for TrackIdentityDebug {
    fn from(value: track_identity::TrackIdentity) -> Self {
        let value_block = value.value_block();
        Self {
            local_track_id: value.local_track_id,
            feed_id: value.feed_id,
            feed_guid: value.feed_guid,
            item_guid: value.item_guid,
            title: value.title,
            artist: value.artist,
            album: value.album,
            image: value.image,
            duration_ms: value.duration_ms,
            local_path: value.local_path,
            value_block,
            item_value_block: value.item_value_block,
            feed_value_block: value.feed_value_block,
            raw_extra_json: value.raw_extra_json,
        }
    }
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

    fn create_track(conn: &Connection, feed_id: i64) -> Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, album_title,
                 duration_seconds, item_value_json, extra_json, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
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
    fn track_inspect_returns_json_ready_identity() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id)?;

        let value = track_inspect(&conn, track_id)?;
        let json = serde_json::to_value(&value)?;

        assert_eq!(json["local_track_id"], track_id);
        assert_eq!(json["feed_guid"], "feed-guid");
        assert_eq!(json["value_block"]["item"], "value");
        assert_eq!(json["raw_extra_json"]["source"], "rss");

        Ok(())
    }

    #[test]
    fn playlists_and_tracks_return_json_ready_rows() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id)?;
        let playlist_id = db::playlist_create(&conn, "Debug")?;
        db::playlist_append(&conn, playlist_id, track_id)?;

        let playlist_rows = playlists(&conn)?;
        let playlist_track_rows = playlist_tracks(&conn, playlist_id)?;
        let library_track_rows = library_tracks(&conn)?;

        assert_eq!(playlist_rows.len(), 1);
        assert_eq!(playlist_rows[0].track_count, 1);
        assert_eq!(playlist_track_rows[0].id, track_id);
        assert_eq!(library_track_rows[0].id, track_id);
        assert_eq!(
            library_track_rows[0].local_path.as_deref(),
            Some("/tmp/track.mp3")
        );

        Ok(())
    }
}
