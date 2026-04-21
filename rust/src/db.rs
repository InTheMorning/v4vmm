// src/db.rs
use anyhow::{Context, Result};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

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
    pub feed_guid: Option<String>,
    pub item_guid: String,
    pub track_title: Option<String>,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub album_artist_name: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub enclosure_url: Option<String>,
    pub track_image_href: Option<String>,
    pub is_in_library: bool,
    pub feed_title: Option<String>,
    pub album_image_href: Option<String>,
    pub local_path: Option<String>,
    pub transcript_url: Option<String>,
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
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
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
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
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

pub fn set_feed_subscribed_by_url(
    conn: &Connection,
    feed_url: &str,
    subscribed: bool,
) -> Result<()> {
    conn.execute(
        "UPDATE feeds SET is_subscribed = ?1 WHERE feed_url = ?2",
        rusqlite::params![subscribed as i64, feed_url],
    )
    .context("set_feed_subscribed_by_url")?;
    Ok(())
}

pub fn feed_is_subscribed_by_url(conn: &Connection, feed_url: &str) -> Result<bool> {
    conn.query_row(
        "SELECT is_subscribed FROM feeds WHERE feed_url = ?1",
        [feed_url],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .context("feed_is_subscribed_by_url")
    .map(|value| value.unwrap_or_default() != 0)
}

pub fn set_track_in_library(conn: &Connection, track_id: i64, in_library: bool) -> Result<()> {
    conn.execute(
        "UPDATE tracks SET is_in_library = ?1 WHERE id = ?2",
        rusqlite::params![in_library as i64, track_id],
    )
    .context("set_track_in_library")?;
    Ok(())
}

pub fn set_track_in_library_by_match(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
    in_library: bool,
) -> Result<bool> {
    let Some(track_id) = find_track_id(conn, feed_url, item_guid, enclosure_url)? else {
        return Ok(false);
    };
    set_track_in_library(conn, track_id, in_library)?;
    Ok(true)
}

pub fn track_is_in_library_by_match(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<bool> {
    let Some(track_id) = find_track_id(conn, feed_url, item_guid, enclosure_url)? else {
        return Ok(false);
    };
    conn.query_row(
        "SELECT is_in_library FROM tracks WHERE id = ?1",
        [track_id],
        |row| row.get::<_, i64>(0),
    )
    .context("track_is_in_library_by_match")
    .map(|value| value != 0)
}

pub fn mark_track_downloaded(
    conn: &Connection,
    track_id: i64,
    path: &Path,
    file_size_bytes: Option<i64>,
) -> Result<()> {
    conn.execute(
        "UPDATE tracks SET is_in_library = 1 WHERE id = ?1",
        [track_id],
    )
    .context("mark downloaded track in library")?;
    upsert_local_file(conn, track_id, path, file_size_bytes)
}

pub fn mark_track_downloaded_by_match(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
    path: &Path,
    file_size_bytes: Option<i64>,
) -> Result<bool> {
    let Some(track_id) = find_track_id(conn, feed_url, item_guid, enclosure_url)? else {
        return Ok(false);
    };
    mark_track_downloaded(conn, track_id, path, file_size_bytes)?;
    Ok(true)
}

fn upsert_local_file(
    conn: &Connection,
    track_id: i64,
    path: &Path,
    file_size_bytes: Option<i64>,
) -> Result<()> {
    let path = path.display().to_string();
    conn.execute(
        r#"
        INSERT INTO local_files (path, track_id, file_size_bytes)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(path) DO UPDATE SET
            track_id = excluded.track_id,
            file_size_bytes = excluded.file_size_bytes
        "#,
        rusqlite::params![path, track_id, file_size_bytes],
    )
    .context("upsert local file")?;
    Ok(())
}

fn find_track_id(
    conn: &Connection,
    feed_url: Option<&str>,
    item_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<Option<i64>> {
    if let (Some(feed_url), Some(item_guid)) = (feed_url, item_guid) {
        if let Some(track_id) = conn
            .query_row(
                "SELECT t.id
                 FROM tracks t
                 JOIN feeds f ON f.id = t.feed_id
                 WHERE f.feed_url = ?1 AND t.item_guid = ?2
                 LIMIT 1",
                rusqlite::params![feed_url, item_guid],
                |row| row.get(0),
            )
            .optional()
            .context("find track by feed URL and item GUID")?
        {
            return Ok(Some(track_id));
        }
    }

    if let (Some(feed_url), Some(enclosure_url)) = (feed_url, enclosure_url) {
        if let Some(track_id) = conn
            .query_row(
                "SELECT t.id
                 FROM tracks t
                 JOIN feeds f ON f.id = t.feed_id
                 WHERE f.feed_url = ?1 AND t.enclosure_url = ?2
                 LIMIT 1",
                rusqlite::params![feed_url, enclosure_url],
                |row| row.get(0),
            )
            .optional()
            .context("find track by feed URL and enclosure URL")?
        {
            return Ok(Some(track_id));
        }
    }

    if let Some(enclosure_url) = enclosure_url {
        return conn
            .query_row(
                "SELECT id FROM tracks WHERE enclosure_url = ?1 LIMIT 1",
                [enclosure_url],
                |row| row.get(0),
            )
            .optional()
            .context("find track by enclosure URL");
    }

    Ok(None)
}

pub fn cached_tracks(conn: &Connection) -> Result<Vec<TrackRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.feed_id, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             JOIN local_files lf ON lf.track_id = t.id
             WHERE t.is_in_library = 0
             ORDER BY f.title COLLATE NOCASE, t.track_number, t.track_title COLLATE NOCASE",
        )
        .context("prepare cached_tracks")?;

    let rows = stmt
        .query_map([], track_row_from_sql)
        .context("query cached_tracks")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect cached_tracks")?;

    Ok(rows)
}

pub fn unsubscribe_feed_tracks(conn: &Connection, feed_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE tracks SET is_in_library = 0 WHERE feed_id = ?1",
        [feed_id],
    )
    .context("unsubscribe_feed_tracks")?;
    Ok(())
}

pub fn delete_local_file(conn: &Connection, local_file_path: &str) -> Result<()> {
    conn.execute("DELETE FROM local_files WHERE path = ?1", [local_file_path])
        .context("delete_local_file")?;
    Ok(())
}

fn track_row_from_sql(row: &rusqlite::Row) -> rusqlite::Result<TrackRow> {
    Ok(TrackRow {
        id: row.get(0)?,
        feed_id: row.get(1)?,
        feed_guid: row.get(2)?,
        item_guid: row.get(3)?,
        track_title: row.get(4)?,
        artist_name: row.get(5)?,
        album_title: row.get(6)?,
        album_artist_name: row.get(7)?,
        track_number: row.get(8)?,
        disc_number: row.get(9)?,
        duration_seconds: row.get(10)?,
        enclosure_url: row.get(11)?,
        track_image_href: row.get(12)?,
        is_in_library: row.get::<_, i64>(13)? != 0,
        feed_title: row.get(14)?,
        album_image_href: row.get(15)?,
        local_path: row.get(16)?,
        transcript_url: transcript_url_from_extra_json(
            row.get::<_, Option<String>>(17)?.as_deref(),
        ),
    })
}

fn transcript_url_from_extra_json(extra_json: Option<&str>) -> Option<String> {
    let extra_json = extra_json?;
    let value = serde_json::from_str::<serde_json::Value>(extra_json).ok()?;
    value
        .get("transcript_url")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
