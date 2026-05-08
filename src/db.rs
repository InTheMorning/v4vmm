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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalIdentityOwner {
    Feed(i64),
    Track(i64),
    FeedContributor {
        feed_id: i64,
        contributor_position: i64,
    },
    TrackContributor {
        track_id: i64,
        contributor_position: i64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalEntityOwner {
    Feed(i64),
    Track(i64),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalIdentityLinkInput {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub link_type: Option<String>,
    pub url: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
    pub raw_json: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalIdentityLinkRow {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub link_type: Option<String>,
    pub url: Option<String>,
    pub source: String,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
    pub raw_json: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalIdentityIdInput {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub scheme: Option<String>,
    pub value: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
    pub raw_json: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalIdentityIdRow {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub scheme: Option<String>,
    pub value: Option<String>,
    pub source: String,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
    pub raw_json: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalContributorInput {
    pub position: i64,
    pub name: Option<String>,
    pub role: Option<String>,
    pub group_name: Option<String>,
    pub href: Option<String>,
    pub image_url: Option<String>,
    pub nostr_npub: Option<String>,
    pub raw_json: Option<String>,
    pub observed_at: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalContributorRow {
    pub position: i64,
    pub name: Option<String>,
    pub role: Option<String>,
    pub group_name: Option<String>,
    pub href: Option<String>,
    pub image_url: Option<String>,
    pub nostr_npub: Option<String>,
    pub source: String,
    pub raw_json: Option<String>,
    pub observed_at: Option<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArtistSourceFactInput {
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub image_url: Option<String>,
    pub website_url: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub area: Option<String>,
    pub begin_year: Option<i64>,
    pub end_year: Option<i64>,
    pub observed_at: Option<i64>,
    pub raw_json: Option<String>,
    pub source_links: Vec<LocalIdentityLinkInput>,
    pub source_ids: Vec<LocalIdentityIdInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtistSourceFactRow {
    pub source: String,
    pub source_artist_id: String,
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub image_url: Option<String>,
    pub website_url: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub area: Option<String>,
    pub begin_year: Option<i64>,
    pub end_year: Option<i64>,
    pub observed_at: Option<i64>,
    pub raw_json: Option<String>,
    pub source_links: Vec<LocalIdentityLinkRow>,
    pub source_ids: Vec<LocalIdentityIdRow>,
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

pub fn feed_id_by_url(conn: &Connection, feed_url: &str) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM feeds WHERE feed_url = ?1 LIMIT 1",
        [feed_url],
        |row| row.get(0),
    )
    .optional()
    .context("query feed_id_by_url")
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

pub fn track_row_by_id(conn: &Connection, track_id: i64) -> Result<Option<TrackRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.enclosure_type, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             LEFT JOIN local_files lf ON lf.track_id = t.id
             WHERE t.id = ?1",
        )
        .context("prepare track_row_by_id")?;

    let mut rows = stmt
        .query_map([track_id], track_row_from_sql)
        .context("query track_row_by_id")?;
    match rows.next() {
        Some(row) => Ok(Some(row.context("read track_row_by_id")?)),
        None => Ok(None),
    }
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

pub fn playlist_reference_count_for_track(conn: &Connection, track_id: i64) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM playlist_tracks WHERE track_id = ?1",
        [track_id],
        |row| row.get(0),
    )
    .context("playlist_reference_count_for_track")
}

pub fn playlist_referenced_library_track_count_for_feed(
    conn: &Connection,
    feed_id: i64,
) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(DISTINCT t.id)
         FROM tracks t
         JOIN playlist_tracks pt ON pt.track_id = t.id
         WHERE t.feed_id = ?1 AND t.is_in_library = 1",
        [feed_id],
        |row| row.get(0),
    )
    .context("playlist_referenced_library_track_count_for_feed")
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

/// Listing scope for paged track queries (ADR 0041).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackListing {
    /// Tracks marked `is_in_library = 1` (library view).
    Library,
    /// Tracks marked `is_in_library = 0` (cached / discovery view).
    Cached,
    /// Tracks attached to one playlist, ordered by `playlist_tracks.position`.
    Playlist {
        /// `playlists.id` of the playlist whose tracks should be listed.
        playlist_id: i64,
    },
    /// All tracks belonging to one feed (subscribed or cached),
    /// ordered by disc/track number then title.
    Feed {
        /// `feeds.id` whose tracks should be listed.
        feed_id: i64,
    },
}

impl TrackListing {
    fn where_clause(self) -> &'static str {
        match self {
            Self::Library => "t.is_in_library = 1",
            Self::Cached => "t.is_in_library = 0",
            // Playlist and Feed listings have dedicated query helpers;
            // these branches are unused but kept for exhaustiveness.
            Self::Playlist { .. } | Self::Feed { .. } => "1 = 1",
        }
    }
}

/// ADR 0041 step 1: cheap identity index for a paged track listing.
///
/// Returns `(track_id, sort_key)` in display order. The sort key is a
/// stable string suitable for jump-to-key UI; v1 uses
/// `feed_title|disc|track_no|title` for library/cached listings and a
/// zero-padded `position` for playlist listings. The key is opaque to
/// callers.
pub fn track_ids_ordered_by(
    conn: &Connection,
    listing: TrackListing,
) -> Result<Vec<(i64, String)>> {
    if let TrackListing::Playlist { playlist_id } = listing {
        return playlist_track_ids_ordered(conn, playlist_id);
    }
    if let TrackListing::Feed { feed_id } = listing {
        return feed_track_ids_ordered(conn, feed_id);
    }

    let sql = format!(
        "SELECT t.id,
                COALESCE(f.title, '')                  AS feed_title,
                COALESCE(t.disc_number, 0)             AS disc_no,
                COALESCE(t.track_number, 0)            AS track_no,
                COALESCE(t.track_title, '')            AS title
         FROM tracks t
         JOIN feeds f ON f.id = t.feed_id
         WHERE {}
         ORDER BY f.title COLLATE NOCASE, t.disc_number, t.track_number, t.track_title COLLATE NOCASE",
        listing.where_clause()
    );

    let mut stmt = conn.prepare(&sql).context("prepare track_ids_ordered_by")?;
    let rows = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let feed_title: String = row.get(1)?;
            let disc_no: i64 = row.get(2)?;
            let track_no: i64 = row.get(3)?;
            let title: String = row.get(4)?;
            let key = format!("{feed_title}|{disc_no:04}|{track_no:04}|{title}");
            Ok((id, key))
        })
        .context("query track_ids_ordered_by")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect track_ids_ordered_by")?;
    Ok(rows)
}

fn playlist_track_ids_ordered(conn: &Connection, playlist_id: i64) -> Result<Vec<(i64, String)>> {
    let mut stmt = conn
        .prepare(
            "SELECT pt.track_id, pt.position
             FROM playlist_tracks pt
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position",
        )
        .context("prepare playlist_track_ids_ordered")?;
    let rows = stmt
        .query_map([playlist_id], |row| {
            let id: i64 = row.get(0)?;
            let position: i64 = row.get(1)?;
            // Zero-pad so jump-to-key sorts numerically; eight digits
            // covers any plausible playlist length.
            Ok((id, format!("{position:08}")))
        })
        .context("query playlist_track_ids_ordered")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect playlist_track_ids_ordered")?;
    Ok(rows)
}

fn feed_track_ids_ordered(conn: &Connection, feed_id: i64) -> Result<Vec<(i64, String)>> {
    // Feed listings sort by disc/track number then title — same intra-feed
    // ordering as the library/cached listings, just scoped to one feed.
    let mut stmt = conn
        .prepare(
            "SELECT t.id,
                    COALESCE(t.disc_number, 0)             AS disc_no,
                    COALESCE(t.track_number, 0)            AS track_no,
                    COALESCE(t.track_title, '')            AS title
             FROM tracks t
             WHERE t.feed_id = ?1
             ORDER BY t.disc_number, t.track_number, t.track_title COLLATE NOCASE",
        )
        .context("prepare feed_track_ids_ordered")?;
    let rows = stmt
        .query_map([feed_id], |row| {
            let id: i64 = row.get(0)?;
            let disc_no: i64 = row.get(1)?;
            let track_no: i64 = row.get(2)?;
            let title: String = row.get(3)?;
            let key = format!("{disc_no:04}|{track_no:04}|{title}");
            Ok((id, key))
        })
        .context("query feed_track_ids_ordered")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect feed_track_ids_ordered")?;
    Ok(rows)
}

/// ADR 0041 step 2: hydrate a slice of track ids into full rows.
///
/// Chunks at 500 ids to respect SQLite's compile-time variable limit.
/// Returned rows are in input order; missing ids are silently skipped.
pub fn tracks_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<TrackRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut by_id: std::collections::HashMap<i64, TrackRow> =
        std::collections::HashMap::with_capacity(ids.len());

    for chunk in ids.chunks(500) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.enclosure_type, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.extra_json
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             LEFT JOIN local_files lf ON lf.track_id = t.id
             WHERE t.id IN ({placeholders})"
        );
        let mut stmt = conn.prepare(&sql).context("prepare tracks_by_ids chunk")?;
        let params = rusqlite::params_from_iter(chunk.iter().copied());
        let rows = stmt
            .query_map(params, track_row_from_sql)
            .context("query tracks_by_ids chunk")?;
        for row in rows {
            let row = row.context("collect tracks_by_ids chunk")?;
            by_id.insert(row.id, row);
        }
    }

    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
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

impl LocalIdentityOwner {
    fn sql_parts(self) -> (&'static str, Option<i64>, Option<i64>, Option<i64>) {
        match self {
            Self::Feed(feed_id) => ("feed", Some(feed_id), None, None),
            Self::Track(track_id) => ("track", None, Some(track_id), None),
            Self::FeedContributor {
                feed_id,
                contributor_position,
            } => (
                "feed_contributor",
                Some(feed_id),
                None,
                Some(contributor_position),
            ),
            Self::TrackContributor {
                track_id,
                contributor_position,
            } => (
                "track_contributor",
                None,
                Some(track_id),
                Some(contributor_position),
            ),
        }
    }
}

impl LocalEntityOwner {
    fn sql_parts(self) -> (&'static str, Option<i64>, Option<i64>) {
        match self {
            Self::Feed(feed_id) => ("feed", Some(feed_id), None),
            Self::Track(track_id) => ("track", None, Some(track_id)),
        }
    }
}

fn explicit_source_token(source: &str) -> Result<&str> {
    let source = source.trim();
    anyhow::ensure!(!source.is_empty(), "source token cannot be empty");
    Ok(source)
}

fn explicit_source_artist_id(source_artist_id: &str) -> Result<&str> {
    let source_artist_id = source_artist_id.trim();
    anyhow::ensure!(
        !source_artist_id.is_empty(),
        "source artist id cannot be empty"
    );
    Ok(source_artist_id)
}

pub fn replace_local_identity_links(
    conn: &mut Connection,
    owner: LocalIdentityOwner,
    source: &str,
    links: &[LocalIdentityLinkInput],
) -> Result<()> {
    let source = explicit_source_token(source)?;
    let tx = conn.transaction().context("start transaction")?;
    let (owner_kind, feed_id, track_id, contributor_position) = owner.sql_parts();

    tx.execute(
        "DELETE FROM entity_identity_links
         WHERE owner_kind = ?1
           AND feed_id IS ?2
           AND track_id IS ?3
           AND contributor_position IS ?4
           AND source = ?5",
        rusqlite::params![owner_kind, feed_id, track_id, contributor_position, source],
    )
    .context("delete local identity links for source")?;

    for link in links {
        tx.execute(
            "INSERT INTO entity_identity_links (
                 owner_kind, feed_id, track_id, contributor_position,
                 entity_type, entity_id, position, link_type, url, source,
                 extraction_path, observed_at, raw_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                owner_kind,
                feed_id,
                track_id,
                contributor_position,
                link.entity_type.as_deref(),
                link.entity_id.as_deref(),
                link.position,
                link.link_type.as_deref(),
                link.url.as_deref(),
                source,
                link.extraction_path.as_deref(),
                link.observed_at,
                link.raw_json.as_deref(),
            ],
        )
        .context("insert local identity link")?;
    }

    tx.commit().context("commit transaction")?;
    Ok(())
}

pub fn replace_local_identity_ids(
    conn: &mut Connection,
    owner: LocalIdentityOwner,
    source: &str,
    ids: &[LocalIdentityIdInput],
) -> Result<()> {
    let source = explicit_source_token(source)?;
    let tx = conn.transaction().context("start transaction")?;
    let (owner_kind, feed_id, track_id, contributor_position) = owner.sql_parts();

    tx.execute(
        "DELETE FROM entity_identity_ids
         WHERE owner_kind = ?1
           AND feed_id IS ?2
           AND track_id IS ?3
           AND contributor_position IS ?4
           AND source = ?5",
        rusqlite::params![owner_kind, feed_id, track_id, contributor_position, source],
    )
    .context("delete local identity ids for source")?;

    for id in ids {
        tx.execute(
            "INSERT INTO entity_identity_ids (
                 owner_kind, feed_id, track_id, contributor_position,
                 entity_type, entity_id, position, scheme, value, source,
                 extraction_path, observed_at, raw_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                owner_kind,
                feed_id,
                track_id,
                contributor_position,
                id.entity_type.as_deref(),
                id.entity_id.as_deref(),
                id.position,
                id.scheme.as_deref(),
                id.value.as_deref(),
                source,
                id.extraction_path.as_deref(),
                id.observed_at,
                id.raw_json.as_deref(),
            ],
        )
        .context("insert local identity id")?;
    }

    tx.commit().context("commit transaction")?;
    Ok(())
}

pub fn replace_local_contributors(
    conn: &mut Connection,
    owner: LocalEntityOwner,
    source: &str,
    contributors: &[LocalContributorInput],
) -> Result<()> {
    let source = explicit_source_token(source)?;
    let tx = conn.transaction().context("start transaction")?;
    let (owner_kind, feed_id, track_id) = owner.sql_parts();

    tx.execute(
        "DELETE FROM entity_contributors
         WHERE owner_kind = ?1
           AND feed_id IS ?2
           AND track_id IS ?3
           AND source = ?4",
        rusqlite::params![owner_kind, feed_id, track_id, source],
    )
    .context("delete local contributors for source")?;

    for contributor in contributors {
        tx.execute(
            "INSERT INTO entity_contributors (
                 owner_kind, feed_id, track_id, position, name, role,
                 group_name, href, image_url, nostr_npub, source, raw_json,
                 observed_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                owner_kind,
                feed_id,
                track_id,
                contributor.position,
                contributor.name.as_deref(),
                contributor.role.as_deref(),
                contributor.group_name.as_deref(),
                contributor.href.as_deref(),
                contributor.image_url.as_deref(),
                contributor.nostr_npub.as_deref(),
                source,
                contributor.raw_json.as_deref(),
                contributor.observed_at,
            ],
        )
        .context("insert local contributor")?;
    }

    tx.commit().context("commit transaction")?;
    Ok(())
}

pub fn local_identity_links(
    conn: &Connection,
    owner: LocalIdentityOwner,
) -> Result<Vec<LocalIdentityLinkRow>> {
    let (owner_kind, feed_id, track_id, contributor_position) = owner.sql_parts();
    let mut stmt = conn
        .prepare(
            "SELECT entity_type, entity_id, position, link_type, url, source,
                    extraction_path, observed_at, raw_json
             FROM entity_identity_links
             WHERE owner_kind = ?1
               AND feed_id IS ?2
               AND track_id IS ?3
               AND contributor_position IS ?4
             ORDER BY source COLLATE NOCASE, position, id",
        )
        .context("prepare local_identity_links")?;

    let rows = stmt
        .query_map(
            rusqlite::params![owner_kind, feed_id, track_id, contributor_position],
            local_identity_link_row_from_sql,
        )
        .context("query local_identity_links")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect local_identity_links")?;

    Ok(rows)
}

pub fn local_identity_ids(
    conn: &Connection,
    owner: LocalIdentityOwner,
) -> Result<Vec<LocalIdentityIdRow>> {
    let (owner_kind, feed_id, track_id, contributor_position) = owner.sql_parts();
    let mut stmt = conn
        .prepare(
            "SELECT entity_type, entity_id, position, scheme, value, source,
                    extraction_path, observed_at, raw_json
             FROM entity_identity_ids
             WHERE owner_kind = ?1
               AND feed_id IS ?2
               AND track_id IS ?3
               AND contributor_position IS ?4
             ORDER BY source COLLATE NOCASE, position, id",
        )
        .context("prepare local_identity_ids")?;

    let rows = stmt
        .query_map(
            rusqlite::params![owner_kind, feed_id, track_id, contributor_position],
            local_identity_id_row_from_sql,
        )
        .context("query local_identity_ids")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect local_identity_ids")?;

    Ok(rows)
}

pub fn local_contributors(
    conn: &Connection,
    owner: LocalEntityOwner,
) -> Result<Vec<LocalContributorRow>> {
    let (owner_kind, feed_id, track_id) = owner.sql_parts();
    let mut stmt = conn
        .prepare(
            "SELECT position, name, role, group_name, href, image_url, nostr_npub,
                    source, raw_json, observed_at
             FROM entity_contributors
             WHERE owner_kind = ?1
               AND feed_id IS ?2
               AND track_id IS ?3
             ORDER BY source COLLATE NOCASE, position, id",
        )
        .context("prepare local_contributors")?;

    let rows = stmt
        .query_map(
            rusqlite::params![owner_kind, feed_id, track_id],
            local_contributor_row_from_sql,
        )
        .context("query local_contributors")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect local_contributors")?;

    Ok(rows)
}

pub fn replace_artist_source_fact(
    conn: &mut Connection,
    source: &str,
    source_artist_id: &str,
    fact: &ArtistSourceFactInput,
) -> Result<()> {
    let source = explicit_source_token(source)?;
    let source_artist_id = explicit_source_artist_id(source_artist_id)?;
    let aliases_json = serde_json::to_string(&fact.aliases).context("serialize artist aliases")?;
    let tags_json = serde_json::to_string(&fact.tags).context("serialize artist tags")?;
    let tx = conn.transaction().context("start transaction")?;

    tx.execute(
        "INSERT INTO artist_source_facts (
             source, source_artist_id, name, sort_name, image_url, website_url,
             aliases_json, tags_json, area, begin_year, end_year, observed_at, raw_json
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(source, source_artist_id) DO UPDATE SET
             name = excluded.name,
             sort_name = excluded.sort_name,
             image_url = excluded.image_url,
             website_url = excluded.website_url,
             aliases_json = excluded.aliases_json,
             tags_json = excluded.tags_json,
             area = excluded.area,
             begin_year = excluded.begin_year,
             end_year = excluded.end_year,
             observed_at = excluded.observed_at,
             raw_json = excluded.raw_json,
             updated_at = datetime('now')",
        rusqlite::params![
            source,
            source_artist_id,
            fact.name.as_deref(),
            fact.sort_name.as_deref(),
            fact.image_url.as_deref(),
            fact.website_url.as_deref(),
            aliases_json,
            tags_json,
            fact.area.as_deref(),
            fact.begin_year,
            fact.end_year,
            fact.observed_at,
            fact.raw_json.as_deref(),
        ],
    )
    .context("upsert artist source fact")?;

    let artist_source_fact_id: i64 = tx
        .query_row(
            "SELECT id FROM artist_source_facts
             WHERE source = ?1 AND source_artist_id = ?2",
            rusqlite::params![source, source_artist_id],
            |row| row.get(0),
        )
        .context("query artist source fact id")?;

    tx.execute(
        "DELETE FROM artist_source_links WHERE artist_source_fact_id = ?1",
        [artist_source_fact_id],
    )
    .context("delete artist source links")?;
    tx.execute(
        "DELETE FROM artist_source_ids WHERE artist_source_fact_id = ?1",
        [artist_source_fact_id],
    )
    .context("delete artist source ids")?;

    for link in &fact.source_links {
        tx.execute(
            "INSERT INTO artist_source_links (
                 artist_source_fact_id, entity_type, entity_id, position,
                 link_type, url, extraction_path, observed_at, raw_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                artist_source_fact_id,
                link.entity_type.as_deref(),
                link.entity_id.as_deref(),
                link.position,
                link.link_type.as_deref(),
                link.url.as_deref(),
                link.extraction_path.as_deref(),
                link.observed_at,
                link.raw_json.as_deref(),
            ],
        )
        .context("insert artist source link")?;
    }

    for id in &fact.source_ids {
        tx.execute(
            "INSERT INTO artist_source_ids (
                 artist_source_fact_id, entity_type, entity_id, position,
                 scheme, value, extraction_path, observed_at, raw_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                artist_source_fact_id,
                id.entity_type.as_deref(),
                id.entity_id.as_deref(),
                id.position,
                id.scheme.as_deref(),
                id.value.as_deref(),
                id.extraction_path.as_deref(),
                id.observed_at,
                id.raw_json.as_deref(),
            ],
        )
        .context("insert artist source id")?;
    }

    tx.commit().context("commit transaction")?;
    Ok(())
}

pub fn artist_source_fact(
    conn: &Connection,
    source: &str,
    source_artist_id: &str,
) -> Result<Option<ArtistSourceFactRow>> {
    let source = explicit_source_token(source)?;
    let source_artist_id = explicit_source_artist_id(source_artist_id)?;
    let Some(sql_row) = conn
        .query_row(
            "SELECT id, source, source_artist_id, name, sort_name, image_url,
                    website_url, aliases_json, tags_json, area, begin_year,
                    end_year, observed_at, raw_json
             FROM artist_source_facts
             WHERE source = ?1 AND source_artist_id = ?2",
            rusqlite::params![source, source_artist_id],
            artist_source_fact_sql_row_from_sql,
        )
        .optional()
        .context("query artist_source_fact")?
    else {
        return Ok(None);
    };

    let source_links = artist_source_links(conn, sql_row.id, &sql_row.source)?;
    let source_ids = artist_source_ids(conn, sql_row.id, &sql_row.source)?;
    let aliases = parse_string_array(&sql_row.aliases_json, "artist aliases")?;
    let tags = parse_string_array(&sql_row.tags_json, "artist tags")?;

    Ok(Some(ArtistSourceFactRow {
        source: sql_row.source,
        source_artist_id: sql_row.source_artist_id,
        name: sql_row.name,
        sort_name: sql_row.sort_name,
        image_url: sql_row.image_url,
        website_url: sql_row.website_url,
        aliases,
        tags,
        area: sql_row.area,
        begin_year: sql_row.begin_year,
        end_year: sql_row.end_year,
        observed_at: sql_row.observed_at,
        raw_json: sql_row.raw_json,
        source_links,
        source_ids,
    }))
}

fn local_identity_link_row_from_sql(row: &rusqlite::Row) -> rusqlite::Result<LocalIdentityLinkRow> {
    Ok(LocalIdentityLinkRow {
        entity_type: row.get(0)?,
        entity_id: row.get(1)?,
        position: row.get(2)?,
        link_type: row.get(3)?,
        url: row.get(4)?,
        source: row.get(5)?,
        extraction_path: row.get(6)?,
        observed_at: row.get(7)?,
        raw_json: row.get(8)?,
    })
}

fn local_identity_id_row_from_sql(row: &rusqlite::Row) -> rusqlite::Result<LocalIdentityIdRow> {
    Ok(LocalIdentityIdRow {
        entity_type: row.get(0)?,
        entity_id: row.get(1)?,
        position: row.get(2)?,
        scheme: row.get(3)?,
        value: row.get(4)?,
        source: row.get(5)?,
        extraction_path: row.get(6)?,
        observed_at: row.get(7)?,
        raw_json: row.get(8)?,
    })
}

fn local_contributor_row_from_sql(row: &rusqlite::Row) -> rusqlite::Result<LocalContributorRow> {
    Ok(LocalContributorRow {
        position: row.get(0)?,
        name: row.get(1)?,
        role: row.get(2)?,
        group_name: row.get(3)?,
        href: row.get(4)?,
        image_url: row.get(5)?,
        nostr_npub: row.get(6)?,
        source: row.get(7)?,
        raw_json: row.get(8)?,
        observed_at: row.get(9)?,
    })
}

#[derive(Debug)]
struct ArtistSourceFactSqlRow {
    id: i64,
    source: String,
    source_artist_id: String,
    name: Option<String>,
    sort_name: Option<String>,
    image_url: Option<String>,
    website_url: Option<String>,
    aliases_json: String,
    tags_json: String,
    area: Option<String>,
    begin_year: Option<i64>,
    end_year: Option<i64>,
    observed_at: Option<i64>,
    raw_json: Option<String>,
}

fn artist_source_fact_sql_row_from_sql(
    row: &rusqlite::Row,
) -> rusqlite::Result<ArtistSourceFactSqlRow> {
    Ok(ArtistSourceFactSqlRow {
        id: row.get(0)?,
        source: row.get(1)?,
        source_artist_id: row.get(2)?,
        name: row.get(3)?,
        sort_name: row.get(4)?,
        image_url: row.get(5)?,
        website_url: row.get(6)?,
        aliases_json: row.get(7)?,
        tags_json: row.get(8)?,
        area: row.get(9)?,
        begin_year: row.get(10)?,
        end_year: row.get(11)?,
        observed_at: row.get(12)?,
        raw_json: row.get(13)?,
    })
}

fn artist_source_links(
    conn: &Connection,
    artist_source_fact_id: i64,
    source: &str,
) -> Result<Vec<LocalIdentityLinkRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT entity_type, entity_id, position, link_type, url,
                    extraction_path, observed_at, raw_json
             FROM artist_source_links
             WHERE artist_source_fact_id = ?1
             ORDER BY position, id",
        )
        .context("prepare artist_source_links")?;
    let rows = stmt
        .query_map([artist_source_fact_id], |row| {
            Ok(LocalIdentityLinkRow {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                position: row.get(2)?,
                link_type: row.get(3)?,
                url: row.get(4)?,
                source: source.to_owned(),
                extraction_path: row.get(5)?,
                observed_at: row.get(6)?,
                raw_json: row.get(7)?,
            })
        })
        .context("query artist_source_links")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect artist_source_links")?;
    Ok(rows)
}

fn artist_source_ids(
    conn: &Connection,
    artist_source_fact_id: i64,
    source: &str,
) -> Result<Vec<LocalIdentityIdRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT entity_type, entity_id, position, scheme, value,
                    extraction_path, observed_at, raw_json
             FROM artist_source_ids
             WHERE artist_source_fact_id = ?1
             ORDER BY position, id",
        )
        .context("prepare artist_source_ids")?;
    let rows = stmt
        .query_map([artist_source_fact_id], |row| {
            Ok(LocalIdentityIdRow {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                position: row.get(2)?,
                scheme: row.get(3)?,
                value: row.get(4)?,
                source: source.to_owned(),
                extraction_path: row.get(5)?,
                observed_at: row.get(6)?,
                raw_json: row.get(7)?,
            })
        })
        .context("query artist_source_ids")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect artist_source_ids")?;
    Ok(rows)
}

fn parse_string_array(raw_json: &str, label: &str) -> Result<Vec<String>> {
    serde_json::from_str(raw_json).with_context(|| format!("parse {label}"))
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

pub fn update_playback_session_paused(
    conn: &Connection,
    session_id: &str,
    paused: bool,
) -> Result<PlaybackSessionRow> {
    let session_id = session_id.trim();
    anyhow::ensure!(!session_id.is_empty(), "session id cannot be empty");
    let state = if paused { "paused" } else { "playing" };
    let changed = conn
        .execute(
            "UPDATE playback_sessions
             SET sequence = sequence + 1,
                 state = ?1,
                 updated_at = datetime('now')
             WHERE session_id = ?2
               AND state != 'stopped'",
            rusqlite::params![state, session_id],
        )
        .context("update playback session pause state")?;
    anyhow::ensure!(changed > 0, "no active playback session {session_id:?}");
    playback_session(conn, session_id)?.context("playback session missing after pause update")
}

pub fn reconcile_playback_session_driver_status(
    conn: &Connection,
    session_id: &str,
    position_ms: u64,
    paused: bool,
) -> Result<Option<PlaybackSessionRow>> {
    let session_id = session_id.trim();
    anyhow::ensure!(!session_id.is_empty(), "session id cannot be empty");
    let position_ms = i64::try_from(position_ms).context("position_ms is too large")?;
    let state = if paused { "paused" } else { "playing" };
    conn.execute(
        "UPDATE playback_sessions
         SET sequence = sequence + 1,
             position_ms = ?1,
             state = ?2,
             updated_at = datetime('now')
         WHERE session_id = ?3
           AND state != 'stopped'
           AND (position_ms != ?1 OR state != ?2)",
        rusqlite::params![position_ms, state, session_id],
    )
    .context("reconcile playback session driver status")?;
    playback_session(conn, session_id)
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
    Migration {
        version: 3,
        name: "identity_source_facts",
        apply: migration_identity_source_facts,
    },
    Migration {
        version: 4,
        name: "artist_source_facts",
        apply: migration_artist_source_facts,
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

fn migration_identity_source_facts(conn: &Connection) -> Result<()> {
    create_identity_source_fact_tables(conn)
}

fn migration_artist_source_facts(conn: &Connection) -> Result<()> {
    create_artist_source_fact_tables(conn)
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

    create_identity_source_fact_tables(conn)?;
    create_artist_source_fact_tables(conn)?;

    Ok(())
}

fn create_identity_source_fact_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS entity_identity_links (
            id INTEGER PRIMARY KEY,
            owner_kind TEXT NOT NULL,
            feed_id INTEGER NULL REFERENCES feeds(id) ON DELETE CASCADE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
            contributor_position INTEGER NULL,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            link_type TEXT NULL,
            url TEXT NULL,
            source TEXT NOT NULL CHECK (source != ''),
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (
                (owner_kind = 'feed'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'track'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'feed_contributor'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NOT NULL)
                OR (owner_kind = 'track_contributor'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NOT NULL)
            )
        );

        CREATE TABLE IF NOT EXISTS entity_identity_ids (
            id INTEGER PRIMARY KEY,
            owner_kind TEXT NOT NULL,
            feed_id INTEGER NULL REFERENCES feeds(id) ON DELETE CASCADE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
            contributor_position INTEGER NULL,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            scheme TEXT NULL,
            value TEXT NULL,
            source TEXT NOT NULL CHECK (source != ''),
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (
                (owner_kind = 'feed'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'track'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'feed_contributor'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NOT NULL)
                OR (owner_kind = 'track_contributor'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NOT NULL)
            )
        );

        CREATE TABLE IF NOT EXISTS entity_contributors (
            id INTEGER PRIMARY KEY,
            owner_kind TEXT NOT NULL,
            feed_id INTEGER NULL REFERENCES feeds(id) ON DELETE CASCADE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
            position INTEGER NOT NULL,
            name TEXT NULL,
            role TEXT NULL,
            group_name TEXT NULL,
            href TEXT NULL,
            image_url TEXT NULL,
            nostr_npub TEXT NULL,
            source TEXT NOT NULL CHECK (source != ''),
            raw_json TEXT NULL,
            observed_at INTEGER NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (
                (owner_kind = 'feed'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL)
                OR (owner_kind = 'track'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL)
            )
        );

        CREATE INDEX IF NOT EXISTS idx_entity_identity_links_owner
            ON entity_identity_links(owner_kind, feed_id, track_id, contributor_position);
        CREATE INDEX IF NOT EXISTS idx_entity_identity_links_owner_source
            ON entity_identity_links(owner_kind, feed_id, track_id, contributor_position, source);
        CREATE INDEX IF NOT EXISTS idx_entity_identity_links_feed_id
            ON entity_identity_links(feed_id);
        CREATE INDEX IF NOT EXISTS idx_entity_identity_links_track_id
            ON entity_identity_links(track_id);

        CREATE INDEX IF NOT EXISTS idx_entity_identity_ids_owner
            ON entity_identity_ids(owner_kind, feed_id, track_id, contributor_position);
        CREATE INDEX IF NOT EXISTS idx_entity_identity_ids_owner_source
            ON entity_identity_ids(owner_kind, feed_id, track_id, contributor_position, source);
        CREATE INDEX IF NOT EXISTS idx_entity_identity_ids_feed_id
            ON entity_identity_ids(feed_id);
        CREATE INDEX IF NOT EXISTS idx_entity_identity_ids_track_id
            ON entity_identity_ids(track_id);

        CREATE INDEX IF NOT EXISTS idx_entity_contributors_owner
            ON entity_contributors(owner_kind, feed_id, track_id);
        CREATE INDEX IF NOT EXISTS idx_entity_contributors_owner_source
            ON entity_contributors(owner_kind, feed_id, track_id, source);
        CREATE INDEX IF NOT EXISTS idx_entity_contributors_feed_id
            ON entity_contributors(feed_id);
        CREATE INDEX IF NOT EXISTS idx_entity_contributors_track_id
            ON entity_contributors(track_id);
        "#,
    )
    .context("create identity source fact tables")?;
    Ok(())
}

fn create_artist_source_fact_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS artist_source_facts (
            id INTEGER PRIMARY KEY,
            source TEXT NOT NULL CHECK (source != ''),
            source_artist_id TEXT NOT NULL CHECK (source_artist_id != ''),
            name TEXT NULL,
            sort_name TEXT NULL,
            image_url TEXT NULL,
            website_url TEXT NULL,
            aliases_json TEXT NOT NULL DEFAULT '[]',
            tags_json TEXT NOT NULL DEFAULT '[]',
            area TEXT NULL,
            begin_year INTEGER NULL,
            end_year INTEGER NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(source, source_artist_id)
        );

        CREATE TABLE IF NOT EXISTS artist_source_links (
            id INTEGER PRIMARY KEY,
            artist_source_fact_id INTEGER NOT NULL
                REFERENCES artist_source_facts(id) ON DELETE CASCADE,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            link_type TEXT NULL,
            url TEXT NULL,
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS artist_source_ids (
            id INTEGER PRIMARY KEY,
            artist_source_fact_id INTEGER NOT NULL
                REFERENCES artist_source_facts(id) ON DELETE CASCADE,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            scheme TEXT NULL,
            value TEXT NULL,
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_artist_source_facts_source_artist
            ON artist_source_facts(source, source_artist_id);
        CREATE INDEX IF NOT EXISTS idx_artist_source_links_fact
            ON artist_source_links(artist_source_fact_id);
        CREATE INDEX IF NOT EXISTS idx_artist_source_ids_fact
            ON artist_source_ids(artist_source_fact_id);
        "#,
    )
    .context("create artist source fact tables")?;
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

    fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |_| Ok(()),
        )
        .optional()
        .with_context(|| format!("query table_exists for {table}"))
        .map(|value| value.is_some())
    }

    fn table_row_count(conn: &Connection, table: &str) -> Result<i64> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .with_context(|| format!("count rows in {table}"))
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

    fn insert_track_full(
        conn: &Connection,
        feed_id: i64,
        title: &str,
        track_no: Option<i64>,
        in_library: bool,
    ) -> Result<i64> {
        static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(10_000);
        let guid = format!(
            "guid-full-{}",
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, track_number, is_in_library)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![feed_id, guid, title, track_no, i64::from(in_library)],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn track_ids_ordered_by_returns_only_listing_scope_in_sorted_order() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let lib_b = insert_track_full(&conn, feed_id, "B", Some(2), true)?;
        let lib_a = insert_track_full(&conn, feed_id, "A", Some(1), true)?;
        let _cached = insert_track_full(&conn, feed_id, "C", Some(3), false)?;

        let library = track_ids_ordered_by(&conn, TrackListing::Library)?;
        assert_eq!(
            library.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![lib_a, lib_b]
        );
        assert!(library[0].1 < library[1].1, "sort keys must be monotonic");

        let cached = track_ids_ordered_by(&conn, TrackListing::Cached)?;
        assert_eq!(cached.len(), 1);

        Ok(())
    }

    #[test]
    fn tracks_by_ids_preserves_input_order_and_skips_unknown() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let a = insert_track_full(&conn, feed_id, "A", Some(1), true)?;
        let b = insert_track_full(&conn, feed_id, "B", Some(2), true)?;
        let c = insert_track_full(&conn, feed_id, "C", Some(3), true)?;

        let rows = tracks_by_ids(&conn, &[c, a, b, 9_999])?;
        assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![c, a, b]);

        Ok(())
    }

    #[test]
    fn tracks_by_ids_handles_empty_input() -> Result<()> {
        let conn = setup_test_db()?;
        assert!(tracks_by_ids(&conn, &[])?.is_empty());
        Ok(())
    }

    #[test]
    fn track_ids_ordered_by_playlist_follows_position_order() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let a = insert_track_full(&conn, feed_id, "A", Some(1), true)?;
        let b = insert_track_full(&conn, feed_id, "B", Some(2), true)?;
        let c = insert_track_full(&conn, feed_id, "C", Some(3), true)?;
        let playlist_id = playlist_create(&conn, "P")?;
        // Append in non-alphabetical order; result must follow append
        // order, not the library's title-sorted order.
        playlist_append(&conn, playlist_id, c)?;
        playlist_append(&conn, playlist_id, a)?;
        playlist_append(&conn, playlist_id, b)?;

        let rows = track_ids_ordered_by(&conn, TrackListing::Playlist { playlist_id })?;
        assert_eq!(
            rows.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![c, a, b]
        );
        // Sort keys must be monotonic so jump-to-key UIs stay stable.
        assert!(rows[0].1 < rows[1].1);
        assert!(rows[1].1 < rows[2].1);
        Ok(())
    }

    #[test]
    fn track_ids_ordered_by_playlist_isolated_from_other_playlists() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let a = insert_track_full(&conn, feed_id, "A", Some(1), true)?;
        let b = insert_track_full(&conn, feed_id, "B", Some(2), true)?;
        let p1 = playlist_create(&conn, "P1")?;
        let p2 = playlist_create(&conn, "P2")?;
        playlist_append(&conn, p1, a)?;
        playlist_append(&conn, p2, b)?;

        let p1_rows = track_ids_ordered_by(&conn, TrackListing::Playlist { playlist_id: p1 })?;
        assert_eq!(
            p1_rows.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![a]
        );

        let empty = track_ids_ordered_by(&conn, TrackListing::Playlist { playlist_id: 9_999 })?;
        assert!(empty.is_empty());
        Ok(())
    }

    fn create_named_feed(conn: &Connection, url: &str, title: &str) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, title) VALUES (?1, ?2)",
            rusqlite::params![url, title],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn track_ids_ordered_by_feed_orders_by_disc_track_title() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let b = insert_track_full(&conn, feed_id, "B", Some(2), true)?;
        let a = insert_track_full(&conn, feed_id, "A", Some(1), true)?;
        let c = insert_track_full(&conn, feed_id, "C", Some(3), false)?;

        let rows = track_ids_ordered_by(&conn, TrackListing::Feed { feed_id })?;
        // Feed listing includes both library + cached tracks and sorts
        // by track number regardless of `is_in_library`.
        assert_eq!(
            rows.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![a, b, c]
        );
        assert!(rows[0].1 < rows[1].1);
        assert!(rows[1].1 < rows[2].1);
        Ok(())
    }

    #[test]
    fn track_ids_ordered_by_feed_isolates_to_one_feed() -> Result<()> {
        let conn = setup_test_db()?;
        let f1 = create_named_feed(&conn, "http://a.feed", "Feed A")?;
        let f2 = create_named_feed(&conn, "http://b.feed", "Feed B")?;
        let a1 = insert_track_full(&conn, f1, "A", Some(1), true)?;
        let _b1 = insert_track_full(&conn, f2, "B", Some(1), true)?;

        let rows = track_ids_ordered_by(&conn, TrackListing::Feed { feed_id: f1 })?;
        assert_eq!(rows.iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![a1]);
        Ok(())
    }

    #[test]
    fn track_ids_ordered_by_feed_unknown_id_is_empty() -> Result<()> {
        let conn = setup_test_db()?;
        let rows = track_ids_ordered_by(&conn, TrackListing::Feed { feed_id: 9_999 })?;
        assert!(rows.is_empty());
        Ok(())
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
            vec![1, 2, 3, 4],
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
            vec![1, 2, 3, 4],
            "migration registry should be idempotent"
        );

        Ok(())
    }

    #[test]
    fn test_identity_source_fact_schema_creates_tables() -> Result<()> {
        let conn = setup_test_db()?;

        assert!(
            table_exists(&conn, "entity_identity_links")?,
            "schema should include entity_identity_links"
        );
        assert!(
            table_exists(&conn, "entity_identity_ids")?,
            "schema should include entity_identity_ids"
        );
        assert!(
            table_exists(&conn, "entity_contributors")?,
            "schema should include entity_contributors"
        );

        Ok(())
    }

    #[test]
    fn test_artist_source_fact_schema_creates_tables() -> Result<()> {
        let conn = setup_test_db()?;

        assert!(
            table_exists(&conn, "artist_source_facts")?,
            "schema should include artist_source_facts"
        );
        assert!(
            table_exists(&conn, "artist_source_links")?,
            "schema should include artist_source_links"
        );
        assert!(
            table_exists(&conn, "artist_source_ids")?,
            "schema should include artist_source_ids"
        );

        Ok(())
    }

    #[test]
    fn test_artist_source_facts_round_trip() -> Result<()> {
        let mut conn = setup_test_db()?;

        replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &ArtistSourceFactInput {
                name: Some("Alice".to_owned()),
                sort_name: Some("Alice, The".to_owned()),
                image_url: Some("https://example.test/artist.jpg".to_owned()),
                website_url: Some("https://example.test/artist".to_owned()),
                aliases: vec!["A. Example".to_owned()],
                tags: vec!["rock".to_owned()],
                area: Some("Montreal".to_owned()),
                begin_year: Some(2020),
                end_year: Some(2025),
                observed_at: Some(1_714_000_000),
                raw_json: Some(r#"{"artist_id":"artist-123"}"#.to_owned()),
                source_links: vec![LocalIdentityLinkInput {
                    entity_type: Some("artist".to_owned()),
                    entity_id: Some("artist-123".to_owned()),
                    position: Some(0),
                    link_type: Some("website".to_owned()),
                    url: Some("https://example.test/artist".to_owned()),
                    extraction_path: Some("$.url".to_owned()),
                    observed_at: Some(1_714_000_001),
                    raw_json: Some(r#"{"url":"https://example.test/artist"}"#.to_owned()),
                }],
                source_ids: vec![LocalIdentityIdInput {
                    entity_type: Some("artist".to_owned()),
                    entity_id: Some("artist-123".to_owned()),
                    position: Some(0),
                    scheme: Some("musicindex_artist_id".to_owned()),
                    value: Some("artist-123".to_owned()),
                    extraction_path: Some("$.artist_id".to_owned()),
                    observed_at: Some(1_714_000_002),
                    raw_json: Some(r#"{"artist_id":"artist-123"}"#.to_owned()),
                }],
            },
        )?;

        let row = artist_source_fact(&conn, "musicindex", "artist-123")?
            .context("artist source fact should exist")?;

        assert_eq!(row.source, "musicindex");
        assert_eq!(row.source_artist_id, "artist-123");
        assert_eq!(row.name.as_deref(), Some("Alice"));
        assert_eq!(row.sort_name.as_deref(), Some("Alice, The"));
        assert_eq!(
            row.image_url.as_deref(),
            Some("https://example.test/artist.jpg")
        );
        assert_eq!(
            row.website_url.as_deref(),
            Some("https://example.test/artist")
        );
        assert_eq!(row.aliases, vec!["A. Example"]);
        assert_eq!(row.tags, vec!["rock"]);
        assert_eq!(row.area.as_deref(), Some("Montreal"));
        assert_eq!(row.begin_year, Some(2020));
        assert_eq!(row.end_year, Some(2025));
        assert_eq!(row.observed_at, Some(1_714_000_000));
        assert_eq!(
            row.raw_json.as_deref(),
            Some(r#"{"artist_id":"artist-123"}"#)
        );
        assert_eq!(row.source_links.len(), 1);
        assert_eq!(row.source_links[0].source, "musicindex");
        assert_eq!(
            row.source_links[0].url.as_deref(),
            Some("https://example.test/artist")
        );
        assert_eq!(row.source_ids.len(), 1);
        assert_eq!(row.source_ids[0].source, "musicindex");
        assert_eq!(
            row.source_ids[0].scheme.as_deref(),
            Some("musicindex_artist_id")
        );

        Ok(())
    }

    #[test]
    fn test_artist_source_facts_replace_source_scoped_rows() -> Result<()> {
        let mut conn = setup_test_db()?;

        replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &ArtistSourceFactInput {
                name: Some("Old".to_owned()),
                source_links: vec![LocalIdentityLinkInput {
                    url: Some("https://old.example".to_owned()),
                    ..LocalIdentityLinkInput::default()
                }],
                ..ArtistSourceFactInput::default()
            },
        )?;
        replace_artist_source_fact(
            &mut conn,
            "rss",
            "artist-123",
            &ArtistSourceFactInput {
                name: Some("RSS".to_owned()),
                ..ArtistSourceFactInput::default()
            },
        )?;
        replace_artist_source_fact(
            &mut conn,
            "musicindex",
            "artist-123",
            &ArtistSourceFactInput {
                name: Some("New".to_owned()),
                source_links: vec![LocalIdentityLinkInput {
                    url: Some("https://new.example".to_owned()),
                    ..LocalIdentityLinkInput::default()
                }],
                ..ArtistSourceFactInput::default()
            },
        )?;

        let musicindex_row = artist_source_fact(&conn, "musicindex", "artist-123")?
            .context("musicindex artist source fact should exist")?;
        let rss_row = artist_source_fact(&conn, "rss", "artist-123")?
            .context("rss artist source fact should exist")?;

        assert_eq!(musicindex_row.name.as_deref(), Some("New"));
        assert_eq!(musicindex_row.source_links.len(), 1);
        assert_eq!(
            musicindex_row.source_links[0].url.as_deref(),
            Some("https://new.example")
        );
        assert_eq!(rss_row.name.as_deref(), Some("RSS"));
        assert_eq!(table_row_count(&conn, "artist_source_facts")?, 2);
        assert_eq!(table_row_count(&conn, "artist_source_links")?, 1);

        Ok(())
    }

    #[test]
    fn test_artist_source_fact_requires_explicit_keys() -> Result<()> {
        let mut conn = setup_test_db()?;
        let fact = ArtistSourceFactInput::default();

        assert!(
            replace_artist_source_fact(&mut conn, "", "artist-123", &fact).is_err(),
            "artist source facts require a non-empty source"
        );
        assert!(
            replace_artist_source_fact(&mut conn, "musicindex", "", &fact).is_err(),
            "artist source facts require a non-empty source artist id"
        );

        let invalid = conn.execute(
            "INSERT INTO artist_source_facts (source, source_artist_id)
             VALUES ('musicindex', '')",
            [],
        );
        assert!(
            invalid.is_err(),
            "schema should reject empty source artist ids"
        );

        Ok(())
    }

    #[test]
    fn test_identity_source_facts_round_trip() -> Result<()> {
        let mut conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let track_id = create_test_track(&conn, feed_id)?;

        replace_local_identity_links(
            &mut conn,
            LocalIdentityOwner::Feed(feed_id),
            "musicindex",
            &[LocalIdentityLinkInput {
                entity_type: Some("feed".to_owned()),
                entity_id: Some("feed-123".to_owned()),
                position: Some(0),
                link_type: Some("website".to_owned()),
                url: Some("https://example.test".to_owned()),
                extraction_path: Some("$.source_links[0]".to_owned()),
                observed_at: Some(1_714_000_000),
                raw_json: Some(r#"{"url":"https://example.test"}"#.to_owned()),
            }],
        )?;
        replace_local_identity_ids(
            &mut conn,
            LocalIdentityOwner::Track(track_id),
            "musicindex",
            &[LocalIdentityIdInput {
                entity_type: Some("track".to_owned()),
                entity_id: Some("track-123".to_owned()),
                position: Some(0),
                scheme: Some("isrc".to_owned()),
                value: Some("US-AAA-24-00001".to_owned()),
                extraction_path: Some("$.source_ids[0]".to_owned()),
                observed_at: Some(1_714_000_001),
                raw_json: Some(r#"{"scheme":"isrc"}"#.to_owned()),
            }],
        )?;
        replace_local_contributors(
            &mut conn,
            LocalEntityOwner::Feed(feed_id),
            "musicindex",
            &[LocalContributorInput {
                position: 0,
                name: Some("Alice".to_owned()),
                role: Some("host".to_owned()),
                group_name: Some("hosts".to_owned()),
                href: Some("https://example.test/alice".to_owned()),
                image_url: Some("https://example.test/alice.jpg".to_owned()),
                nostr_npub: Some("npub1alice".to_owned()),
                raw_json: Some(r#"{"name":"Alice"}"#.to_owned()),
                observed_at: Some(1_714_000_002),
            }],
        )?;

        let links = local_identity_links(&conn, LocalIdentityOwner::Feed(feed_id))?;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type.as_deref(), Some("website"));
        assert_eq!(links[0].url.as_deref(), Some("https://example.test"));
        assert_eq!(links[0].source, "musicindex");
        assert_eq!(
            links[0].raw_json.as_deref(),
            Some(r#"{"url":"https://example.test"}"#)
        );

        let ids = local_identity_ids(&conn, LocalIdentityOwner::Track(track_id))?;
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].scheme.as_deref(), Some("isrc"));
        assert_eq!(ids[0].value.as_deref(), Some("US-AAA-24-00001"));
        assert_eq!(ids[0].source, "musicindex");

        let contributors = local_contributors(&conn, LocalEntityOwner::Feed(feed_id))?;
        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].name.as_deref(), Some("Alice"));
        assert_eq!(
            contributors[0].image_url.as_deref(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(contributors[0].nostr_npub.as_deref(), Some("npub1alice"));

        Ok(())
    }

    #[test]
    fn test_identity_source_replacement_is_source_scoped() -> Result<()> {
        let mut conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let owner = LocalIdentityOwner::Feed(feed_id);

        replace_local_identity_links(
            &mut conn,
            owner,
            "musicindex",
            &[LocalIdentityLinkInput {
                url: Some("https://musicindex.example/old".to_owned()),
                ..LocalIdentityLinkInput::default()
            }],
        )?;
        replace_local_identity_links(
            &mut conn,
            owner,
            "rss",
            &[LocalIdentityLinkInput {
                url: Some("https://rss.example/source".to_owned()),
                ..LocalIdentityLinkInput::default()
            }],
        )?;
        replace_local_identity_links(
            &mut conn,
            owner,
            "musicindex",
            &[LocalIdentityLinkInput {
                url: Some("https://musicindex.example/new".to_owned()),
                ..LocalIdentityLinkInput::default()
            }],
        )?;

        let links = local_identity_links(&conn, owner)?;
        assert_eq!(links.len(), 2);
        assert!(
            links.iter().any(|link| {
                link.source == "musicindex"
                    && link.url.as_deref() == Some("https://musicindex.example/new")
            }),
            "musicindex source should be replaced"
        );
        assert!(
            links.iter().any(|link| {
                link.source == "rss" && link.url.as_deref() == Some("https://rss.example/source")
            }),
            "rss source should remain intact"
        );

        Ok(())
    }

    #[test]
    fn test_identity_source_fact_discriminator_rejects_invalid_owner_shape() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let track_id = create_test_track(&conn, feed_id)?;

        let invalid_link = conn.execute(
            "INSERT INTO entity_identity_links (
                 owner_kind, feed_id, track_id, source, url
             )
             VALUES ('feed', ?1, ?2, 'musicindex', 'https://invalid.example')",
            rusqlite::params![feed_id, track_id],
        );
        assert!(
            invalid_link.is_err(),
            "feed identity link cannot also set track_id"
        );

        let invalid_contributor = conn.execute(
            "INSERT INTO entity_contributors (
                 owner_kind, feed_id, track_id, position, source, name
             )
             VALUES ('feed_contributor', ?1, NULL, 0, 'musicindex', 'Alice')",
            rusqlite::params![feed_id],
        );
        assert!(
            invalid_contributor.is_err(),
            "contributors table only accepts feed or track owners"
        );

        Ok(())
    }

    #[test]
    fn test_identity_source_facts_cascade_when_feed_is_deleted() -> Result<()> {
        let mut conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let track_id = create_test_track(&conn, feed_id)?;

        replace_local_identity_links(
            &mut conn,
            LocalIdentityOwner::Feed(feed_id),
            "musicindex",
            &[LocalIdentityLinkInput {
                url: Some("https://feed.example".to_owned()),
                ..LocalIdentityLinkInput::default()
            }],
        )?;
        replace_local_identity_ids(
            &mut conn,
            LocalIdentityOwner::Track(track_id),
            "musicindex",
            &[LocalIdentityIdInput {
                scheme: Some("isrc".to_owned()),
                value: Some("US-AAA-24-00001".to_owned()),
                ..LocalIdentityIdInput::default()
            }],
        )?;
        replace_local_identity_links(
            &mut conn,
            LocalIdentityOwner::FeedContributor {
                feed_id,
                contributor_position: 0,
            },
            "musicindex",
            &[LocalIdentityLinkInput {
                url: Some("https://contributor.example".to_owned()),
                ..LocalIdentityLinkInput::default()
            }],
        )?;
        replace_local_identity_ids(
            &mut conn,
            LocalIdentityOwner::TrackContributor {
                track_id,
                contributor_position: 0,
            },
            "musicindex",
            &[LocalIdentityIdInput {
                scheme: Some("npub".to_owned()),
                value: Some("npub1trackcontributor".to_owned()),
                ..LocalIdentityIdInput::default()
            }],
        )?;
        replace_local_contributors(
            &mut conn,
            LocalEntityOwner::Track(track_id),
            "musicindex",
            &[LocalContributorInput {
                position: 0,
                name: Some("Alice".to_owned()),
                ..LocalContributorInput::default()
            }],
        )?;

        conn.execute("DELETE FROM feeds WHERE id = ?1", [feed_id])
            .context("delete test feed")?;

        assert_eq!(table_row_count(&conn, "entity_identity_links")?, 0);
        assert_eq!(table_row_count(&conn, "entity_identity_ids")?, 0);
        assert_eq!(table_row_count(&conn, "entity_contributors")?, 0);

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
    fn playlist_reference_counts_detect_tracks_that_will_be_stranded() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_test_feed(&conn)?;
        let in_library = insert_track_full(&conn, feed_id, "In", Some(1), true)?;
        let cached = insert_track_full(&conn, feed_id, "Cached", Some(2), false)?;
        let playlist_id = playlist_create(&conn, "Refs")?;
        playlist_append(&conn, playlist_id, in_library)?;
        playlist_append(&conn, playlist_id, cached)?;

        assert_eq!(playlist_reference_count_for_track(&conn, in_library)?, 1);
        assert_eq!(
            playlist_referenced_library_track_count_for_feed(&conn, feed_id)?,
            1
        );

        set_track_in_library(&conn, in_library, false)?;
        assert_eq!(
            playlist_referenced_library_track_count_for_feed(&conn, feed_id)?,
            0
        );

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
