// src/db.rs
use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::config::Config;

#[derive(Clone, Debug)]
pub struct FeedRow {
    pub id: i64,
    pub feed_url: String,
    pub feed_guid: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub album_image_href: Option<String>,
    pub is_subscribed: bool,
}

#[derive(Clone, Debug)]
pub struct TrackRow {
    pub id: i64,
    pub feed_id: i64,
    pub item_guid: String,
    pub track_title: Option<String>,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub track_number: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub enclosure_url: Option<String>,
    pub track_image_href: Option<String>,
    pub is_in_library: bool,
    pub feed_title: Option<String>,
    pub local_path: Option<String>,
}

pub fn subscribed_feeds(conn: &Connection) -> Result<Vec<FeedRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, feed_url, feed_guid, title, description, album_image_href, is_subscribed
             FROM feeds WHERE is_subscribed = 1 ORDER BY title COLLATE NOCASE",
        )
        .context("prepare subscribed_feeds")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(FeedRow {
                id: row.get(0)?,
                feed_url: row.get(1)?,
                feed_guid: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                album_image_href: row.get(5)?,
                is_subscribed: row.get::<_, i64>(6)? != 0,
            })
        })
        .context("query subscribed_feeds")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect subscribed_feeds")?;

    Ok(rows)
}

pub fn feed_tracks(conn: &Connection, feed_id: i64) -> Result<Vec<TrackRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.feed_id, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.track_number, t.duration_seconds, t.enclosure_url, t.track_image_href,
                    t.is_in_library, f.title, lf.path
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             LEFT JOIN local_files lf ON lf.track_id = t.id
             WHERE t.feed_id = ?1
             ORDER BY t.track_number, t.track_title COLLATE NOCASE",
        )
        .context("prepare feed_tracks")?;

    let rows = stmt
        .query_map([feed_id], track_row_from_sql)
        .context("query feed_tracks")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect feed_tracks")?;

    Ok(rows)
}

pub fn library_tracks(conn: &Connection) -> Result<Vec<TrackRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.feed_id, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.track_number, t.duration_seconds, t.enclosure_url, t.track_image_href,
                    t.is_in_library, f.title, lf.path
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             LEFT JOIN local_files lf ON lf.track_id = t.id
             WHERE t.is_in_library = 1
             ORDER BY f.title COLLATE NOCASE, t.track_number, t.track_title COLLATE NOCASE",
        )
        .context("prepare library_tracks")?;

    let rows = stmt
        .query_map([], track_row_from_sql)
        .context("query library_tracks")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect library_tracks")?;

    Ok(rows)
}

pub fn set_feed_subscribed(conn: &Connection, feed_id: i64, subscribed: bool) -> Result<()> {
    conn.execute(
        "UPDATE feeds SET is_subscribed = ?1 WHERE id = ?2",
        rusqlite::params![subscribed as i64, feed_id],
    )
    .context("set_feed_subscribed")?;
    Ok(())
}

pub fn set_track_in_library(conn: &Connection, track_id: i64, in_library: bool) -> Result<()> {
    conn.execute(
        "UPDATE tracks SET is_in_library = ?1 WHERE id = ?2",
        rusqlite::params![in_library as i64, track_id],
    )
    .context("set_track_in_library")?;
    Ok(())
}

fn track_row_from_sql(row: &rusqlite::Row) -> rusqlite::Result<TrackRow> {
    Ok(TrackRow {
        id: row.get(0)?,
        feed_id: row.get(1)?,
        item_guid: row.get(2)?,
        track_title: row.get(3)?,
        artist_name: row.get(4)?,
        album_title: row.get(5)?,
        track_number: row.get(6)?,
        duration_seconds: row.get(7)?,
        enclosure_url: row.get(8)?,
        track_image_href: row.get(9)?,
        is_in_library: row.get::<_, i64>(10)? != 0,
        feed_title: row.get(11)?,
        local_path: row.get(12)?,
    })
}

pub fn open_db(cfg: &Config) -> Result<Connection> {
    let db_path = &cfg.db_path;

    let conn = Connection::open(db_path)
        .with_context(|| format!("open/create db {}", db_path.display()))?;

    // Basic sanity / good defaults
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("enable foreign_keys pragma")?;

    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS feeds (
            id INTEGER PRIMARY KEY,
            feed_url TEXT NOT NULL UNIQUE,
            feed_guid TEXT NULL,              -- podcast:guid if present
            title TEXT NULL,
            link TEXT NULL,
            language TEXT NULL,
            description TEXT NULL,
            podcast_medium TEXT NULL,
            album_image_href TEXT NULL,
            album_image_mime TEXT NULL,
            people_json TEXT NULL,
            podcast_value_json TEXT NULL,
            is_subscribed INTEGER NOT NULL DEFAULT 0,
            last_fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
            extra_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_feeds_guid ON feeds(feed_guid);
        CREATE INDEX IF NOT EXISTS idx_feeds_is_subscribed ON feeds(is_subscribed);

        CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
            item_guid TEXT NOT NULL,
            enclosure_url TEXT NULL,
            link TEXT NULL,
            pub_date TEXT NULL,
            track_title TEXT NULL,
            artist_name TEXT NULL,
            album_title TEXT NULL,
            album_artist_name TEXT NULL,
            disc_number INTEGER NULL,
            track_number INTEGER NULL,
            duration_seconds INTEGER NULL,
            itunes_duration_raw TEXT NULL,
            itunes_explicit TEXT NULL,
            track_image_href TEXT NULL,
            track_image_mime TEXT NULL,
            people_json TEXT NULL,
            item_value_json TEXT NULL,
            is_in_library INTEGER NOT NULL DEFAULT 0,
            extra_json TEXT NOT NULL DEFAULT '{}',
            UNIQUE(feed_id, item_guid)
        );

        CREATE TABLE IF NOT EXISTS local_files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE SET NULL,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            file_size_bytes INTEGER NULL,
            audio_duration_sec INTEGER NULL,
            checksum TEXT NULL,
            extra_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_tracks_feed_id       ON tracks(feed_id);
        CREATE INDEX IF NOT EXISTS idx_tracks_track_number  ON tracks(feed_id, track_number);
        CREATE INDEX IF NOT EXISTS idx_tracks_is_in_library ON tracks(is_in_library);
        CREATE INDEX IF NOT EXISTS idx_local_files_track_id ON local_files(track_id);
        "#,
    )
    .context("create tables")?;

    Ok(())
}
