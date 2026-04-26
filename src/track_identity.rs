//! Canonical local track identity read model.

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

#[derive(Clone, Debug)]
pub struct TrackIdentity {
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
    pub item_value_block: serde_json::Value,
    pub feed_value_block: serde_json::Value,
    pub raw_extra_json: serde_json::Value,
}

impl TrackIdentity {
    pub fn value_block(&self) -> serde_json::Value {
        if self.item_value_block.is_null() {
            self.feed_value_block.clone()
        } else {
            self.item_value_block.clone()
        }
    }
}

pub fn local_track_identity(conn: &Connection, track_id: i64) -> Result<TrackIdentity> {
    let row = conn
        .query_row(
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name,
                    t.album_artist_name, t.album_title, t.track_image_href,
                    f.album_image_href, t.duration_seconds, lf.path, t.item_value_json,
                    f.podcast_value_json, t.extra_json
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             JOIN local_files lf ON lf.track_id = t.id
             WHERE t.id = ?1
             ORDER BY lf.id
             LIMIT 1",
            rusqlite::params![track_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, String>(14)?,
                ))
            },
        )
        .optional()
        .context("query local track identity")?;

    let Some((
        local_track_id,
        feed_id,
        feed_guid,
        item_guid,
        title,
        artist,
        album_artist,
        album,
        track_image,
        album_image,
        duration_seconds,
        local_path,
        item_value_json,
        feed_value_json,
        raw_extra_json,
    )) = row
    else {
        anyhow::bail!("track {track_id} is not bound to a local file");
    };

    let feed_guid = feed_guid
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .with_context(|| format!("track {track_id} has no feed GUID"))?;
    let local_path = local_path.trim().to_string();
    anyhow::ensure!(
        !local_path.is_empty(),
        "track {track_id} has an empty local file path"
    );

    let duration_ms = duration_seconds
        .filter(|value| *value >= 0)
        .map(|value| value as u64 * 1000);
    let item_value_block = parse_json_value(
        item_value_json.as_deref(),
        serde_json::Value::Null,
        "item value block",
    )?;
    let feed_value_block = parse_json_value(
        feed_value_json.as_deref(),
        serde_json::Value::Null,
        "feed value block",
    )?;
    let raw_extra_json = parse_json_value(
        Some(raw_extra_json.as_str()),
        serde_json::json!({}),
        "track extra_json",
    )?;

    Ok(TrackIdentity {
        local_track_id,
        feed_id,
        feed_guid,
        item_guid,
        title: title.unwrap_or_default(),
        artist: artist.or(album_artist).unwrap_or_default(),
        album,
        image: track_image.or(album_image),
        duration_ms,
        local_path,
        item_value_block,
        feed_value_block,
        raw_extra_json,
    })
}

fn parse_json_value(
    raw: Option<&str>,
    default: serde_json::Value,
    label: &str,
) -> Result<serde_json::Value> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(default);
    };
    serde_json::from_str(raw).with_context(|| format!("parse {label}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        Ok(conn)
    }

    fn create_feed(
        conn: &Connection,
        feed_guid: Option<&str>,
        feed_value_json: Option<&str>,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (
                 feed_url, feed_guid, title, album_image_href, podcast_value_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                format!(
                    "https://example.test/{}.xml",
                    feed_guid.unwrap_or("missing")
                ),
                feed_guid,
                "Feed Title",
                "https://example.test/feed.png",
                feed_value_json
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(
        conn: &Connection,
        feed_id: i64,
        item_value_json: Option<&str>,
        extra_json: &str,
    ) -> Result<i64> {
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
                item_value_json,
                extra_json
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn local_track_identity_reads_source_facts() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"), Some(r#"{"feed":"value"}"#))?;
        let track_id = create_track(
            &conn,
            feed_id,
            Some(r#"{"item":"value"}"#),
            r#"{"source":"rss"}"#,
        )?;
        db::mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;

        let identity = local_track_identity(&conn, track_id)?;

        assert_eq!(identity.local_track_id, track_id);
        assert_eq!(identity.feed_id, feed_id);
        assert_eq!(identity.feed_guid, "feed-guid");
        assert_eq!(identity.item_guid, "item-guid");
        assert_eq!(identity.local_path, "/tmp/track.mp3");
        assert_eq!(identity.duration_ms, Some(123_000));
        assert_eq!(identity.value_block()["item"], "value");
        assert_eq!(identity.raw_extra_json["source"], "rss");

        Ok(())
    }

    #[test]
    fn missing_local_file_is_rejected() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"), None)?;
        let track_id = create_track(&conn, feed_id, None, "{}")?;

        let result = local_track_identity(&conn, track_id);

        assert!(result.is_err(), "missing local file should be rejected");

        Ok(())
    }

    #[test]
    fn missing_feed_guid_is_rejected() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, None, None)?;
        let track_id = create_track(&conn, feed_id, None, "{}")?;
        db::mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;

        let result = local_track_identity(&conn, track_id);

        assert!(result.is_err(), "missing feed GUID should be rejected");

        Ok(())
    }

    #[test]
    fn malformed_json_is_rejected() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"), None)?;
        let track_id = create_track(&conn, feed_id, Some("{not-json"), "{}")?;
        db::mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;

        let result = local_track_identity(&conn, track_id);

        assert!(result.is_err(), "malformed source JSON should be rejected");

        Ok(())
    }

    #[test]
    fn feed_value_block_is_used_when_item_value_is_missing() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, Some("feed-guid"), Some(r#"{"feed":"value"}"#))?;
        let track_id = create_track(&conn, feed_id, None, "{}")?;
        db::mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;

        let identity = local_track_identity(&conn, track_id)?;

        assert_eq!(identity.value_block()["feed"], "value");

        Ok(())
    }
}
