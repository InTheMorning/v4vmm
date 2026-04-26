// src/db.rs
use anyhow::{Context, Result};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::config::Config;

#[derive(Clone, Debug, Default)]
pub struct FeedRow {
    pub id: i64,
    pub feed_url: String,
    pub feed_guid: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub album_image_href: Option<String>,
    pub is_subscribed: bool,
}

#[derive(Clone, Debug, Default)]
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
    pub enclosure_type: Option<String>,
    pub track_image_href: Option<String>,
    pub is_in_library: bool,
    pub feed_title: Option<String>,
    pub album_image_href: Option<String>,
    pub local_path: Option<String>,
    pub transcript_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub track_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug)]
pub struct PlaybackSessionRow {
    pub session_id: String,
    pub sequence: u64,
    pub local_track_id: i64,
    pub playlist_id: Option<i64>,
    pub playlist_position: Option<i64>,
    pub started_at: String,
    pub position_ms: u64,
    pub state: String,
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

#[derive(Clone, Debug)]
pub struct FeedStaleCheckRow {
    pub id: i64,
    pub feed_guid: String,
    pub title: Option<String>,
    pub musicindex_updated_at: Option<i64>,
}

pub fn subscribed_feeds_for_stale_check(conn: &Connection) -> Result<Vec<FeedStaleCheckRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, feed_guid, title, musicindex_updated_at
             FROM feeds
             WHERE is_subscribed = 1 AND feed_guid IS NOT NULL AND feed_guid != ''
             ORDER BY title COLLATE NOCASE",
        )
        .context("prepare subscribed_feeds_for_stale_check")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(FeedStaleCheckRow {
                id: row.get(0)?,
                feed_guid: row.get(1)?,
                title: row.get(2)?,
                musicindex_updated_at: row.get(3)?,
            })
        })
        .context("query subscribed_feeds_for_stale_check")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect subscribed_feeds_for_stale_check")?;

    Ok(rows)
}

pub fn feed_url_by_id(conn: &Connection, feed_id: i64) -> Result<Option<String>> {
    conn.query_row(
        "SELECT feed_url FROM feeds WHERE id = ?1",
        [feed_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .context("query feed_url_by_id")
}

pub fn feed_stale_check_row(conn: &Connection, feed_id: i64) -> Result<Option<FeedStaleCheckRow>> {
    conn.query_row(
        "SELECT id, feed_guid, title, musicindex_updated_at
         FROM feeds
         WHERE id = ?1 AND feed_guid IS NOT NULL AND feed_guid != ''",
        [feed_id],
        |row| {
            Ok(FeedStaleCheckRow {
                id: row.get(0)?,
                feed_guid: row.get(1)?,
                title: row.get(2)?,
                musicindex_updated_at: row.get(3)?,
            })
        },
    )
    .optional()
    .context("feed_stale_check_row")
}

pub fn set_feed_musicindex_updated_at(
    conn: &Connection,
    feed_id: i64,
    updated_at: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE feeds SET musicindex_updated_at = ?1 WHERE id = ?2",
        rusqlite::params![updated_at, feed_id],
    )
    .context("set_feed_musicindex_updated_at")?;
    Ok(())
}

pub fn library_tracks_for_feed(conn: &Connection, feed_id: i64) -> Result<Vec<TrackRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name,
                    t.album_title, t.album_artist_name, t.track_number, t.disc_number,
                    t.duration_seconds, t.enclosure_url, t.enclosure_type, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             JOIN local_files lf ON lf.track_id = t.id
             WHERE t.feed_id = ?1 AND t.is_in_library = 1",
        )
        .context("prepare library_tracks_for_feed")?;

    let rows = stmt
        .query_map([feed_id], track_row_from_sql)
        .context("query library_tracks_for_feed")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect library_tracks_for_feed")?;

    Ok(rows)
}

pub fn feed_tracks(conn: &Connection, feed_id: i64) -> Result<Vec<TrackRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.enclosure_type, t.track_image_href,
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
                    t.enclosure_url, t.enclosure_type, t.track_image_href,
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

pub fn find_feed_id_by_guid(conn: &Connection, feed_guid: &str) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM feeds WHERE feed_guid = ?1 LIMIT 1",
        rusqlite::params![feed_guid],
        |row| row.get(0),
    )
    .optional()
    .context("find feed by guid")
}

pub fn find_track_id(
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
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.enclosure_type, t.track_image_href,
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

/// Recompute and persist `feeds.is_subscribed` for the given feed URL based on
/// per-track library state: subscribed iff every track of the feed is in the
/// library (and the feed has at least one track).
pub fn reconcile_feed_subscription_by_url(conn: &Connection, feed_url: &str) -> Result<bool> {
    let counts: Option<(i64, i64)> = conn
        .query_row(
            "SELECT
                 COUNT(*) AS total,
                 COALESCE(SUM(CASE WHEN t.is_in_library = 1 THEN 1 ELSE 0 END), 0) AS in_library
             FROM feeds f
             LEFT JOIN tracks t ON t.feed_id = f.id
             WHERE f.feed_url = ?1",
            [feed_url],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .context("reconcile_feed_subscription_by_url counts")?;
    let subscribed =
        matches!(counts, Some((total, in_library)) if total > 0 && in_library == total);
    set_feed_subscribed_by_url(conn, feed_url, subscribed)?;
    Ok(subscribed)
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
        enclosure_type: row.get(12)?,
        track_image_href: row.get(13)?,
        is_in_library: row.get::<_, i64>(14)? != 0,
        feed_title: row.get(15)?,
        album_image_href: row.get(16)?,
        local_path: row.get(17)?,
        transcript_url: transcript_url_from_extra_json(
            row.get::<_, Option<String>>(18)?.as_deref(),
        ),
    })
}

pub fn transcript_url_from_extra_json(extra_json: Option<&str>) -> Option<String> {
    let extra_json = extra_json?;
    let value = serde_json::from_str::<serde_json::Value>(extra_json).ok()?;
    value
        .get("transcript_url")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn playback_session_from_sql(row: &rusqlite::Row) -> rusqlite::Result<PlaybackSessionRow> {
    let sequence = row.get::<_, i64>(1)?;
    let position_ms = row.get::<_, i64>(6)?;
    Ok(PlaybackSessionRow {
        session_id: row.get(0)?,
        sequence: u64::try_from(sequence).unwrap_or_default(),
        local_track_id: row.get(2)?,
        playlist_id: row.get(3)?,
        playlist_position: row.get(4)?,
        started_at: row.get(5)?,
        position_ms: u64::try_from(position_ms).unwrap_or_default(),
        state: row.get(7)?,
    })
}

pub fn playlists_list(conn: &Connection) -> Result<Vec<Playlist>> {
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.name, p.description, COALESCE(COUNT(pt.position), 0), p.created_at, p.updated_at
             FROM playlists p
             LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
             GROUP BY p.id
             ORDER BY p.name COLLATE NOCASE",
        )
        .context("prepare playlists_list")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                track_count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .context("query playlists_list")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect playlists_list")?;

    Ok(rows)
}

pub fn playlist_create(conn: &Connection, name: &str) -> Result<i64> {
    let trimmed = name.trim();
    anyhow::ensure!(!trimmed.is_empty(), "Playlist name cannot be empty");

    conn.execute(
        "INSERT INTO playlists (name) VALUES (?1)",
        rusqlite::params![trimmed],
    )
    .context("insert playlist")?;

    Ok(conn.last_insert_rowid())
}

pub fn playlist_rename(conn: &Connection, playlist_id: i64, new_name: &str) -> Result<()> {
    let trimmed = new_name.trim();
    anyhow::ensure!(!trimmed.is_empty(), "Playlist name cannot be empty");

    conn.execute(
        "UPDATE playlists SET name = ?1, updated_at = strftime('%s','now') WHERE id = ?2",
        rusqlite::params![trimmed, playlist_id],
    )
    .context("rename playlist")?;

    Ok(())
}

pub fn playlist_set_description(
    conn: &Connection,
    playlist_id: i64,
    desc: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE playlists SET description = ?1, updated_at = strftime('%s','now') WHERE id = ?2",
        rusqlite::params![desc, playlist_id],
    )
    .context("set playlist description")?;

    Ok(())
}

pub fn playlist_delete(conn: &Connection, playlist_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM playlists WHERE id = ?1",
        rusqlite::params![playlist_id],
    )
    .context("delete playlist")?;

    Ok(())
}

pub fn playlist_tracks(conn: &Connection, playlist_id: i64) -> Result<Vec<TrackRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name,
                    t.album_title, t.album_artist_name, t.track_number, t.disc_number,
                    t.duration_seconds, t.enclosure_url, t.enclosure_type, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             LEFT JOIN local_files lf ON lf.track_id = t.id
             JOIN playlist_tracks pt ON pt.track_id = t.id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position",
        )
        .context("prepare playlist_tracks")?;

    let rows = stmt
        .query_map([playlist_id], track_row_from_sql)
        .context("query playlist_tracks")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect playlist_tracks")?;

    Ok(rows)
}

pub fn playlist_append(conn: &Connection, playlist_id: i64, track_id: i64) -> Result<()> {
    let position: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
            |row| row.get(0),
        )
        .context("query max position")?;

    conn.execute(
        "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
        rusqlite::params![playlist_id, track_id, position],
    )
    .context("append track to playlist")?;

    conn.execute(
        "UPDATE playlists SET updated_at = strftime('%s','now') WHERE id = ?1",
        rusqlite::params![playlist_id],
    )
    .context("update playlist timestamp")?;

    Ok(())
}

pub fn playlist_remove_at(conn: &mut Connection, playlist_id: i64, position: i64) -> Result<()> {
    let tx = conn.transaction().context("start transaction")?;

    tx.execute(
        "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND position = ?2",
        rusqlite::params![playlist_id, position],
    )
    .context("delete track at position")?;

    tx.execute(
        "UPDATE playlist_tracks SET position = position - 1 WHERE playlist_id = ?1 AND position > ?2",
        rusqlite::params![playlist_id, position],
    )
    .context("shift positions")?;

    tx.execute(
        "UPDATE playlists SET updated_at = strftime('%s','now') WHERE id = ?1",
        rusqlite::params![playlist_id],
    )
    .context("update playlist timestamp")?;

    tx.commit().context("commit transaction")?;

    Ok(())
}

pub fn playlist_reorder(conn: &mut Connection, playlist_id: i64, from: i64, to: i64) -> Result<()> {
    let tx = conn.transaction().context("start transaction")?;

    let mut stmt = tx
        .prepare("SELECT track_id, position FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position")
        .context("prepare select tracks")?;

    let mut tracks: Vec<(i64, i64)> = stmt
        .query_map([playlist_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .context("query tracks")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect tracks")?;
    drop(stmt);

    if from < 0 || from >= tracks.len() as i64 || to < 0 || to >= tracks.len() as i64 {
        anyhow::bail!("Invalid from/to position");
    }

    let (track_id, _) = tracks.remove(from as usize);
    tracks.insert(to as usize, (track_id, to));

    tx.execute(
        "DELETE FROM playlist_tracks WHERE playlist_id = ?1",
        rusqlite::params![playlist_id],
    )
    .context("delete all playlist tracks")?;

    for (idx, (tid, _)) in tracks.iter().enumerate() {
        tx.execute(
            "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            rusqlite::params![playlist_id, tid, idx as i64],
        )
        .context("reinsert track")?;
    }

    tx.execute(
        "UPDATE playlists SET updated_at = strftime('%s','now') WHERE id = ?1",
        rusqlite::params![playlist_id],
    )
    .context("update playlist timestamp")?;

    tx.commit().context("commit transaction")?;

    Ok(())
}

pub fn playlist_first_track(conn: &Connection, playlist_id: i64) -> Result<Option<(i64, i64)>> {
    playlist_track_at(conn, playlist_id, 0)
}

pub fn playlist_track_at(
    conn: &Connection,
    playlist_id: i64,
    position: i64,
) -> Result<Option<(i64, i64)>> {
    conn.query_row(
        "SELECT track_id, position
         FROM playlist_tracks
         WHERE playlist_id = ?1 AND position = ?2
         ORDER BY position
         LIMIT 1",
        rusqlite::params![playlist_id, position],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .context("query playlist track at position")
}

pub fn set_playback_session_track(
    conn: &Connection,
    session_id: &str,
    local_track_id: i64,
    playlist_id: Option<i64>,
    playlist_position: Option<i64>,
    started_at: &str,
    position_ms: u64,
) -> Result<PlaybackSessionRow> {
    let session_id = session_id.trim();
    anyhow::ensure!(!session_id.is_empty(), "session id cannot be empty");
    let sequence = conn
        .query_row(
            "SELECT sequence FROM playback_sessions WHERE session_id = ?1",
            rusqlite::params![session_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .context("query playback session sequence")?
        .unwrap_or_default()
        + 1;
    let position_ms = i64::try_from(position_ms).context("position_ms is too large")?;

    conn.execute(
        "INSERT INTO playback_sessions (
             session_id, sequence, local_track_id, playlist_id, playlist_position,
             started_at, position_ms, state, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'playing', datetime('now'))
         ON CONFLICT(session_id) DO UPDATE SET
             sequence = excluded.sequence,
             local_track_id = excluded.local_track_id,
             playlist_id = excluded.playlist_id,
             playlist_position = excluded.playlist_position,
             started_at = excluded.started_at,
             position_ms = excluded.position_ms,
             state = excluded.state,
             updated_at = datetime('now')",
        rusqlite::params![
            session_id,
            sequence,
            local_track_id,
            playlist_id,
            playlist_position,
            started_at,
            position_ms
        ],
    )
    .context("upsert playback session")?;

    playback_session(conn, session_id)?.context("playback session missing after upsert")
}

pub fn playback_session(conn: &Connection, session_id: &str) -> Result<Option<PlaybackSessionRow>> {
    conn.query_row(
        "SELECT session_id, sequence, local_track_id, playlist_id, playlist_position,
                started_at, position_ms, state
         FROM playback_sessions
         WHERE session_id = ?1",
        rusqlite::params![session_id],
        playback_session_from_sql,
    )
    .optional()
    .context("query playback session")
}

pub fn update_playback_session_position(
    conn: &Connection,
    session_id: &str,
    position_ms: u64,
) -> Result<PlaybackSessionRow> {
    let session_id = session_id.trim();
    anyhow::ensure!(!session_id.is_empty(), "session id cannot be empty");
    let position_ms = i64::try_from(position_ms).context("position_ms is too large")?;
    let changed = conn
        .execute(
            "UPDATE playback_sessions
             SET sequence = sequence + 1,
                 position_ms = ?1,
                 state = 'playing',
                 updated_at = datetime('now')
             WHERE session_id = ?2",
            rusqlite::params![position_ms, session_id],
        )
        .context("update playback session position")?;
    anyhow::ensure!(changed > 0, "no playback session {session_id:?}");
    playback_session(conn, session_id)?.context("playback session missing after position update")
}

pub fn stop_playback_session(conn: &Connection, session_id: &str) -> Result<PlaybackSessionRow> {
    let session_id = session_id.trim();
    anyhow::ensure!(!session_id.is_empty(), "session id cannot be empty");
    let changed = conn
        .execute(
            "UPDATE playback_sessions
             SET sequence = sequence + 1,
                 position_ms = 0,
                 state = 'stopped',
                 updated_at = datetime('now')
             WHERE session_id = ?1",
            rusqlite::params![session_id],
        )
        .context("stop playback session")?;
    anyhow::ensure!(changed > 0, "no playback session {session_id:?}");
    playback_session(conn, session_id)?.context("playback session missing after stop")
}

pub fn open_db(cfg: &Config) -> Result<Connection> {
    let db_path = &cfg.db_path;

    let conn = Connection::open(db_path)
        .with_context(|| format!("open/create db {}", db_path.display()))?;

    // Basic sanity / good defaults
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("enable foreign_keys pragma")?;

    init_schema(&conn)?;
    migrate_schema(&conn)?;
    Ok(conn)
}

struct Migration {
    version: i64,
    name: &'static str,
    apply: fn(&Connection) -> Result<()>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "feeds_musicindex_updated_at",
        apply: migration_feeds_musicindex_updated_at,
    },
    Migration {
        version: 2,
        name: "tracks_enclosure_type",
        apply: migration_tracks_enclosure_type,
    },
];

pub(crate) fn migrate_schema(conn: &Connection) -> Result<()> {
    ensure_schema_migrations_table(conn)?;
    for migration in MIGRATIONS {
        if migration_applied(conn, migration.version)? {
            continue;
        }
        (migration.apply)(conn)
            .with_context(|| format!("apply migration {} {}", migration.version, migration.name))?;
        record_migration(conn, migration.version, migration.name)?;
    }
    Ok(())
}

fn ensure_schema_migrations_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )
    .context("create schema_migrations table")?;
    Ok(())
}

fn migration_applied(conn: &Connection, version: i64) -> Result<bool> {
    conn.query_row(
        "SELECT 1 FROM schema_migrations WHERE version = ?1",
        rusqlite::params![version],
        |_| Ok(()),
    )
    .optional()
    .with_context(|| format!("query schema migration {version}"))
    .map(|value| value.is_some())
}

fn record_migration(conn: &Connection, version: i64, name: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        rusqlite::params![version, name],
    )
    .with_context(|| format!("record schema migration {version} {name}"))?;
    Ok(())
}

fn migration_feeds_musicindex_updated_at(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "feeds", "musicindex_updated_at", "INTEGER")
}

fn migration_tracks_enclosure_type(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "tracks", "enclosure_type", "TEXT")
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    column_type: &str,
) -> Result<()> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("prepare table_info for {table}"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .with_context(|| format!("query table_info for {table}"))?
        .filter_map(Result::ok)
        .any(|name| name.eq_ignore_ascii_case(column));
    drop(stmt);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}"),
            [],
        )
        .with_context(|| format!("add column {column} to {table}"))?;
    }
    Ok(())
}

pub(crate) fn init_schema(conn: &Connection) -> Result<()> {
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
            enclosure_type TEXT NULL,
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

        CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );

        CREATE TABLE IF NOT EXISTS playlist_tracks (
            playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
            track_id    INTEGER NOT NULL REFERENCES tracks(id)    ON DELETE CASCADE,
            position    INTEGER NOT NULL,
            PRIMARY KEY (playlist_id, position)
        );

        CREATE INDEX IF NOT EXISTS idx_playlist_tracks_track_id ON playlist_tracks(track_id);

        CREATE TABLE IF NOT EXISTS playback_sessions (
            session_id TEXT PRIMARY KEY,
            sequence INTEGER NOT NULL DEFAULT 0,
            local_track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
            playlist_id INTEGER NULL REFERENCES playlists(id) ON DELETE SET NULL,
            playlist_position INTEGER NULL,
            started_at TEXT NOT NULL,
            position_ms INTEGER NOT NULL DEFAULT 0,
            state TEXT NOT NULL DEFAULT 'stopped',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_playback_sessions_track_id
            ON playback_sessions(local_track_id);
        "#,
    )
    .context("create tables")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        init_schema(&conn)?;
        migrate_schema(&conn)?;
        Ok(conn)
    }

    fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .with_context(|| format!("prepare table_info for {table}"))?;
        let has_column = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .with_context(|| format!("query table_info for {table}"))?
            .filter_map(Result::ok)
            .any(|name| name.eq_ignore_ascii_case(column));
        Ok(has_column)
    }

    fn applied_migration_versions(conn: &Connection) -> Result<Vec<i64>> {
        let mut stmt = conn
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .context("prepare applied_migration_versions")?;
        let versions = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .context("query applied_migration_versions")?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("collect applied_migration_versions")?;
        Ok(versions)
    }

    fn create_test_feed(conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, title) VALUES (?1, ?2)",
            rusqlite::params!["http://test.feed", "Test Feed"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_test_track(conn: &Connection, feed_id: i64) -> Result<i64> {
        static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
        let guid = format!(
            "guid-{}",
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title) VALUES (?1, ?2, ?3)",
            rusqlite::params![feed_id, guid, "Test Track"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn test_migrations_record_versions_on_fresh_schema() -> Result<()> {
        let conn = setup_test_db()?;

        assert!(
            table_has_column(&conn, "feeds", "musicindex_updated_at")?,
            "fresh schema should include feeds.musicindex_updated_at"
        );
        assert!(
            table_has_column(&conn, "tracks", "enclosure_type")?,
            "fresh schema should include tracks.enclosure_type"
        );
        assert_eq!(
            applied_migration_versions(&conn)?,
            vec![1, 2],
            "fresh schema should record all registry migrations"
        );

        Ok(())
    }

    #[test]
    fn test_migrations_update_legacy_schema() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            r#"
            CREATE TABLE feeds (
                id INTEGER PRIMARY KEY,
                feed_url TEXT NOT NULL UNIQUE,
                feed_guid TEXT NULL,
                title TEXT NULL
            );

            CREATE TABLE tracks (
                id INTEGER PRIMARY KEY,
                feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
                item_guid TEXT NOT NULL,
                enclosure_url TEXT NULL,
                track_title TEXT NULL,
                UNIQUE(feed_id, item_guid)
            );
            "#,
        )
        .context("create legacy schema")?;

        migrate_schema(&conn)?;
        migrate_schema(&conn)?;

        assert!(
            table_has_column(&conn, "feeds", "musicindex_updated_at")?,
            "migration should add feeds.musicindex_updated_at"
        );
        assert!(
            table_has_column(&conn, "tracks", "enclosure_type")?,
            "migration should add tracks.enclosure_type"
        );
        assert_eq!(
            applied_migration_versions(&conn)?,
            vec![1, 2],
            "migration registry should be idempotent"
        );

        Ok(())
    }

    #[test]
    fn test_create_playlist_and_list() -> Result<()> {
        let conn = setup_test_db()?;

        let id = playlist_create(&conn, "My Playlist")?;
        assert!(id > 0);

        let playlists = playlists_list(&conn)?;
        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].name, "My Playlist");
        assert_eq!(playlists[0].track_count, 0);
        assert_eq!(playlists[0].id, id);

        Ok(())
    }

    #[test]
    fn test_append_tracks_to_playlist() -> Result<()> {
        let conn = setup_test_db()?;

        let feed_id = create_test_feed(&conn)?;
        let track_id1 = create_test_track(&conn, feed_id)?;
        let track_id2 = create_test_track(&conn, feed_id)?;

        let playlist_id = playlist_create(&conn, "My Playlist")?;

        playlist_append(&conn, playlist_id, track_id1)?;
        playlist_append(&conn, playlist_id, track_id2)?;

        let tracks = playlist_tracks(&conn, playlist_id)?;
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].id, track_id1);
        assert_eq!(tracks[1].id, track_id2);

        let playlists = playlists_list(&conn)?;
        assert_eq!(playlists[0].track_count, 2);

        Ok(())
    }

    #[test]
    fn test_remove_track_from_middle() -> Result<()> {
        let mut conn = setup_test_db()?;

        let feed_id = create_test_feed(&conn)?;
        let track_id1 = create_test_track(&conn, feed_id)?;
        let track_id2 = create_test_track(&conn, feed_id)?;
        let track_id3 = create_test_track(&conn, feed_id)?;

        let playlist_id = playlist_create(&conn, "My Playlist")?;
        playlist_append(&conn, playlist_id, track_id1)?;
        playlist_append(&conn, playlist_id, track_id2)?;
        playlist_append(&conn, playlist_id, track_id3)?;

        playlist_remove_at(&mut conn, playlist_id, 1)?;

        let tracks = playlist_tracks(&conn, playlist_id)?;
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].id, track_id1);
        assert_eq!(tracks[1].id, track_id3);

        Ok(())
    }

    #[test]
    fn test_reorder_tracks() -> Result<()> {
        let mut conn = setup_test_db()?;

        let feed_id = create_test_feed(&conn)?;
        let track_id1 = create_test_track(&conn, feed_id)?;
        let track_id2 = create_test_track(&conn, feed_id)?;
        let track_id3 = create_test_track(&conn, feed_id)?;

        let playlist_id = playlist_create(&conn, "My Playlist")?;
        playlist_append(&conn, playlist_id, track_id1)?;
        playlist_append(&conn, playlist_id, track_id2)?;
        playlist_append(&conn, playlist_id, track_id3)?;

        playlist_reorder(&mut conn, playlist_id, 0, 2)?;

        let tracks = playlist_tracks(&conn, playlist_id)?;
        assert_eq!(tracks[0].id, track_id2);
        assert_eq!(tracks[1].id, track_id3);
        assert_eq!(tracks[2].id, track_id1);

        Ok(())
    }

    #[test]
    fn test_delete_playlist_cascades() -> Result<()> {
        let conn = setup_test_db()?;

        let feed_id = create_test_feed(&conn)?;
        let track_id = create_test_track(&conn, feed_id)?;

        let playlist_id = playlist_create(&conn, "My Playlist")?;
        playlist_append(&conn, playlist_id, track_id)?;

        playlist_delete(&conn, playlist_id)?;

        let playlists = playlists_list(&conn)?;
        assert_eq!(playlists.len(), 0);

        Ok(())
    }

    #[test]
    fn test_duplicate_playlist_name() -> Result<()> {
        let conn = setup_test_db()?;

        playlist_create(&conn, "My Playlist")?;
        let result = playlist_create(&conn, "My Playlist");

        assert!(result.is_err());

        Ok(())
    }
}
