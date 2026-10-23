//! The query builder of the Library track search (ADR 0087 Decision 2,
//! packet 002).
//!
//! Each term of the typed search query gives at most one clause. Each
//! clause adds one condition to one SQL statement, so `LIMIT` gives the
//! first rows that pass each term. A clause with `-` adds `NOT (condition)`.
//! Each condition gives `TRUE` or `FALSE`, never `NULL`, so a row with an
//! empty column passes an exclusion.
//!
//! ADR 0087 invariant: each value goes into SQL only as a bound parameter.
//! Each text value goes through `like_contains_pattern`, which escapes the
//! `LIKE` characters. The SQL text holds placeholders only.
//!
//! This module reads no view model type. The application layer converts the
//! typed query into a [`LibraryTrackSearch`].

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::types::Value;
use rusqlite::Connection;

use super::{like_contains_pattern, track_row_from_sql, TrackRow};

/// The Library track search of one query: each clause applies with `AND`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibraryTrackSearch {
    /// The clauses, in the text order of their terms.
    pub clauses: Vec<LibraryTrackClause>,
}

/// One clause of the Library track search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryTrackClause {
    /// `true` when the term has a `-`. The clause then adds
    /// `NOT (condition)`.
    pub excluded: bool,
    /// The condition of the clause.
    pub kind: LibraryClauseKind,
}

/// The condition of one clause (ADR 0087 Decision 2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LibraryClauseKind {
    /// A word or a phrase: one pattern against the track title, the
    /// artist, the album title, the album artist and the feed title.
    AnyText(String),
    /// `artist:`: the artist or the album artist.
    Artist(String),
    /// `title:`: the track title, the album title or the feed title.
    Title(String),
    /// `release:`: the album title or the feed title.
    Release(String),
    /// `duration:`: a track with a duration in the inclusive range.
    Duration {
        min_secs: Option<u32>,
        max_secs: Option<u32>,
    },
    /// A set of track ids from a fact that SQL does not hold.
    TrackIds(Vec<i64>),
    /// `playlist:`: a track of a playlist whose name holds the text.
    Playlist(String),
    /// `is:downloaded`: no counted track of the same feed is missing.
    Downloaded,
    /// `is:partial`: the feed has a missing counted track and a downloaded
    /// track.
    Partial,
    /// `is:removed`: the track has the "removed from feed" mark.
    Removed,
    /// `has:split`: the stored route has one or more recipients.
    HasSplit,
    /// `has:image`: the track has its own image.
    HasImage,
}

/// The columns of a word or a phrase: the five columns of the search before
/// ADR 0087.
const ANY_TEXT_COLUMNS: [&str; 5] = [
    "t.track_title",
    "t.artist_name",
    "t.album_title",
    "t.album_artist_name",
    "f.title",
];

/// The columns of `artist:` (ADR 0087 Decision 2).
const ARTIST_COLUMNS: [&str; 2] = ["t.artist_name", "t.album_artist_name"];

/// The columns of `title:` (operator answer 2 of ADR 0087).
const TITLE_COLUMNS: [&str; 3] = ["t.track_title", "t.album_title", "f.title"];

/// The columns of `release:` (operator answer 2 of ADR 0087).
const RELEASE_COLUMNS: [&str; 2] = ["t.album_title", "f.title"];

/// A downloaded track `o`: it is in the Library and a local file names it
/// (packet 002 decision D8).
const DOWNLOADED_TRACK: &str = "(o.is_in_library = 1
             AND EXISTS (SELECT 1 FROM local_files l WHERE l.track_id = o.id))";

/// The SELECT part of the search, with the Library rows only.
const SELECT_LIBRARY_TRACKS: &str =
    "SELECT t.id, t.feed_id, f.feed_guid, t.item_guid, t.track_title, t.artist_name, t.album_title,
                    t.album_artist_name, t.track_number, t.disc_number, t.duration_seconds,
                    t.enclosure_url, t.enclosure_type, t.track_image_href,
                    t.is_in_library, f.title, f.album_image_href, lf.path, t.pub_date,
                    t.itunes_explicit, t.extra_json
             FROM tracks t
             JOIN feeds f ON f.id = t.feed_id
             LEFT JOIN local_files lf ON lf.track_id = t.id
             WHERE t.is_in_library = 1";

/// The order of the rows. It is the order of the search before ADR 0087.
const ORDER_LIBRARY_TRACKS: &str =
    "ORDER BY f.title COLLATE NOCASE, t.track_number, t.track_title COLLATE NOCASE";

/// The parameters of one statement. Each placeholder binds one value, so
/// the count of `?` in the SQL text equals the count of values.
#[derive(Default)]
struct Parameters {
    values: Vec<Value>,
}

impl Parameters {
    /// Adds `value` and gives its numbered placeholder, for example `?3`.
    fn bind(&mut self, value: Value) -> String {
        self.values.push(value);
        format!("?{}", self.values.len())
    }

    /// Adds the escaped `LIKE` pattern of `text` and gives its placeholder.
    fn bind_pattern(&mut self, text: &str) -> String {
        self.bind(Value::Text(like_contains_pattern(text)))
    }
}

/// Builds the SQL text and the parameter list of `search`. `LIMIT` binds
/// as the last parameter.
#[must_use]
pub fn library_track_search_sql(search: &LibraryTrackSearch, limit: usize) -> (String, Vec<Value>) {
    let mut parameters = Parameters::default();
    let mut sql = String::from(SELECT_LIBRARY_TRACKS);
    for clause in &search.clauses {
        let condition = clause_condition(&clause.kind, &mut parameters);
        sql.push_str(if clause.excluded {
            "\n               AND NOT ("
        } else {
            "\n               AND ("
        });
        sql.push_str(&condition);
        sql.push(')');
    }
    sql.push_str("\n             ");
    sql.push_str(ORDER_LIBRARY_TRACKS);
    sql.push_str("\n             LIMIT ");
    let limit = i64::try_from(limit).unwrap_or(i64::MAX);
    sql.push_str(&parameters.bind(Value::Integer(limit)));
    (sql, parameters.values)
}

/// Searches the Library tracks with each clause of `search`, at most
/// `limit` rows. A search with no clause gives no row.
///
/// # Errors
///
/// Returns an error when the statement cannot be prepared or the rows
/// cannot be read.
pub fn search_library_tracks(
    conn: &Connection,
    search: &LibraryTrackSearch,
    limit: usize,
) -> Result<Vec<TrackRow>> {
    if search.clauses.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let (sql, values) = library_track_search_sql(search, limit);
    let mut stmt = conn
        .prepare(&sql)
        .context("prepare search_library_tracks")?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(values), track_row_from_sql)
        .context("query search_library_tracks")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("collect search_library_tracks")?;
    Ok(rows)
}

/// The condition of one clause kind.
fn clause_condition(kind: &LibraryClauseKind, parameters: &mut Parameters) -> String {
    match kind {
        LibraryClauseKind::AnyText(text) => text_condition(&ANY_TEXT_COLUMNS, text, parameters),
        LibraryClauseKind::Artist(text) => text_condition(&ARTIST_COLUMNS, text, parameters),
        LibraryClauseKind::Title(text) => text_condition(&TITLE_COLUMNS, text, parameters),
        LibraryClauseKind::Release(text) => text_condition(&RELEASE_COLUMNS, text, parameters),
        LibraryClauseKind::Duration { min_secs, max_secs } => {
            duration_condition(*min_secs, *max_secs, parameters)
        }
        LibraryClauseKind::TrackIds(ids) => track_ids_condition(ids, parameters),
        LibraryClauseKind::Playlist(text) => playlist_condition(text, parameters),
        LibraryClauseKind::Downloaded => downloaded_condition(),
        LibraryClauseKind::Partial => partial_condition(),
        LibraryClauseKind::Removed => removed_condition(),
        LibraryClauseKind::HasSplit => has_split_condition(),
        LibraryClauseKind::HasImage => has_image_condition(),
    }
}

/// One escaped pattern against each column of `columns`, joined with `OR`.
/// `COALESCE` makes an empty column an empty text, so the condition is
/// never `NULL`.
fn text_condition(columns: &[&str], text: &str, parameters: &mut Parameters) -> String {
    columns
        .iter()
        .map(|column| {
            format!(
                "COALESCE({column}, '') LIKE {} ESCAPE '\\'",
                parameters.bind_pattern(text)
            )
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// A track with a duration. Each limit with a value adds a bound minimum
/// or a bound maximum (packet 002 decision D7).
fn duration_condition(
    min_secs: Option<u32>,
    max_secs: Option<u32>,
    parameters: &mut Parameters,
) -> String {
    let mut condition = String::from("t.duration_seconds IS NOT NULL");
    if let Some(min) = min_secs {
        condition.push_str(" AND t.duration_seconds >= ");
        condition.push_str(&parameters.bind(Value::Integer(i64::from(min))));
    }
    if let Some(max) = max_secs {
        condition.push_str(" AND t.duration_seconds <= ");
        condition.push_str(&parameters.bind(Value::Integer(i64::from(max))));
    }
    condition
}

/// A track whose id is in the bound JSON array of `ids`.
fn track_ids_condition(ids: &[i64], parameters: &mut Parameters) -> String {
    let json = serde_json::Value::from(ids.to_vec()).to_string();
    format!(
        "t.id IN (SELECT value FROM json_each({}))",
        parameters.bind(Value::Text(json))
    )
}

/// A track of a playlist whose name holds `text`, without ASCII case
/// (packet 002 decision D11).
fn playlist_condition(text: &str, parameters: &mut Parameters) -> String {
    format!(
        "EXISTS (SELECT 1 FROM playlist_tracks pt
                  JOIN playlists p ON p.id = pt.playlist_id
                  WHERE pt.track_id = t.id AND p.name LIKE {} ESCAPE '\\')",
        parameters.bind_pattern(text)
    )
}

/// A missing track `o` of the feed of `t`: it is not removed from the feed,
/// and it is not downloaded (packet 002 decision D8).
fn missing_feed_track() -> String {
    format!(
        "EXISTS (SELECT 1 FROM tracks o
                  WHERE o.feed_id = t.feed_id
                    AND o.removed_from_feed_at IS NULL
                    AND NOT {DOWNLOADED_TRACK})"
    )
}

/// No counted track of the feed is missing (packet 002 decision D8).
fn downloaded_condition() -> String {
    format!("NOT {}", missing_feed_track())
}

/// One or more counted tracks of the feed are missing, and one or more are
/// downloaded (packet 002 decision D8).
fn partial_condition() -> String {
    format!(
        "{} AND EXISTS (SELECT 1 FROM tracks o
                  WHERE o.feed_id = t.feed_id AND {DOWNLOADED_TRACK})",
        missing_feed_track()
    )
}

/// The "removed from feed" mark of ADR 0076. A confirmed mark stays a mark
/// (packet 002 decision D9).
fn removed_condition() -> String {
    String::from("t.removed_from_feed_at IS NOT NULL")
}

/// The stored route of the track, else of its feed, is a JSON array with one
/// or more entries. Bad JSON and `NULL` give `FALSE` (packet 002 decision
/// D12).
fn has_split_condition() -> String {
    let route = "COALESCE(t.payment_routes_json, f.payment_routes_json)";
    format!("COALESCE(CASE WHEN json_valid({route}) THEN json_array_length({route}) > 0 END, 0)")
}

/// The track image is not empty and is not the release image (packet 002
/// decision D12).
fn has_image_condition() -> String {
    String::from(
        "TRIM(COALESCE(t.track_image_href, '')) <> ''
                 AND t.track_image_href IS NOT f.album_image_href",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        crate::db::init_schema(&conn)?;
        crate::db::migrate_schema(&conn)?;
        Ok(conn)
    }

    /// The recorded columns of one test track.
    #[derive(Clone, Copy, Default)]
    struct Track<'a> {
        title: Option<&'a str>,
        artist: Option<&'a str>,
        album: Option<&'a str>,
        album_artist: Option<&'a str>,
        not_in_library: bool,
        duration: Option<i64>,
        removed_at: Option<i64>,
        confirmed_at: Option<i64>,
        route_json: Option<&'a str>,
        image: Option<&'a str>,
        file: bool,
    }

    fn feed(conn: &Connection, title: &str) -> Result<i64> {
        feed_with(conn, title, None, None)
    }

    fn feed_with(
        conn: &Connection,
        title: &str,
        route_json: Option<&str>,
        image: Option<&str>,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, title, payment_routes_json, album_image_href)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                format!("https://example.test/{title}.xml"),
                title,
                route_json,
                image
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn track(conn: &Connection, feed_id: i64, track: Track<'_>) -> Result<i64> {
        static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        conn.execute(
            "INSERT INTO tracks (
                feed_id, item_guid, track_title, artist_name, album_title, album_artist_name,
                is_in_library, duration_seconds, removed_from_feed_at,
                removed_from_feed_confirmed_at, payment_routes_json, track_image_href
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                feed_id,
                format!("guid-library-search-{n}"),
                track.title,
                track.artist,
                track.album,
                track.album_artist,
                i64::from(!track.not_in_library),
                track.duration,
                track.removed_at,
                track.confirmed_at,
                track.route_json,
                track.image,
            ],
        )?;
        let id = conn.last_insert_rowid();
        if track.file {
            conn.execute(
                "INSERT INTO local_files (path, track_id) VALUES (?1, ?2)",
                rusqlite::params![format!("music/library-search-{n}.mp3"), id],
            )?;
        }
        Ok(id)
    }

    fn titled(title: &str) -> Track<'_> {
        Track {
            title: Some(title),
            ..Track::default()
        }
    }

    fn clause(kind: LibraryClauseKind) -> LibraryTrackClause {
        LibraryTrackClause {
            excluded: false,
            kind,
        }
    }

    fn excluded(kind: LibraryClauseKind) -> LibraryTrackClause {
        LibraryTrackClause {
            excluded: true,
            kind,
        }
    }

    fn ids(conn: &Connection, clauses: Vec<LibraryTrackClause>) -> Result<Vec<i64>> {
        let mut ids: Vec<i64> = search_library_tracks(conn, &LibraryTrackSearch { clauses }, 50)?
            .into_iter()
            .map(|row| row.id)
            .collect();
        ids.sort_unstable();
        Ok(ids)
    }

    fn text(value: &str) -> String {
        value.to_owned()
    }

    /// R87-2-01: a word finds a track by each of the five columns.
    #[test]
    fn adr_0087_library_fields_r87_2_01_any_text_matches_the_five_columns() -> Result<()> {
        let conn = setup_test_db()?;
        let plain = feed(&conn, "Plain Feed")?;
        let by_title = track(&conn, plain, titled("Needle Title"))?;
        let by_artist = track(
            &conn,
            plain,
            Track {
                artist: Some("Needle Artist"),
                ..titled("A")
            },
        )?;
        let by_album = track(
            &conn,
            plain,
            Track {
                album: Some("Needle Album"),
                ..titled("B")
            },
        )?;
        let by_album_artist = track(
            &conn,
            plain,
            Track {
                album_artist: Some("Needle Album Artist"),
                ..titled("C")
            },
        )?;
        let needle_feed = feed(&conn, "Needle Feed")?;
        let by_feed = track(&conn, needle_feed, titled("D"))?;
        let other = track(&conn, plain, titled("Other"))?;

        let found = ids(
            &conn,
            vec![clause(LibraryClauseKind::AnyText(text("needle")))],
        )?;

        for id in [by_title, by_artist, by_album, by_album_artist, by_feed] {
            assert!(found.contains(&id), "R87-2-01: a column did not match");
        }
        assert!(!found.contains(&other));
        Ok(())
    }

    /// R87-2-02: two words must both match. A phrase matches one string.
    #[test]
    fn adr_0087_library_fields_r87_2_02_each_word_must_match() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        let phrase = track(&conn, feed_id, titled("Night Drive"))?;
        let split = track(
            &conn,
            feed_id,
            Track {
                album: Some("Drive"),
                ..titled("Night")
            },
        )?;

        let words = ids(
            &conn,
            vec![
                clause(LibraryClauseKind::AnyText(text("night"))),
                clause(LibraryClauseKind::AnyText(text("drive"))),
            ],
        )?;
        assert_eq!(words, vec![phrase, split]);

        let one_phrase = ids(
            &conn,
            vec![clause(LibraryClauseKind::AnyText(text("night drive")))],
        )?;
        assert_eq!(one_phrase, vec![phrase]);
        Ok(())
    }

    /// R87-2-03: `artist:` reads the artist and the album artist only.
    #[test]
    fn adr_0087_library_fields_r87_2_03_artist_reads_two_columns() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        let artist = track(
            &conn,
            feed_id,
            Track {
                artist: Some("jsmovie"),
                ..titled("A")
            },
        )?;
        let album_artist = track(
            &conn,
            feed_id,
            Track {
                album_artist: Some("JSMovie"),
                ..titled("B")
            },
        )?;
        track(&conn, feed_id, titled("jsmovie song"))?;

        let found = ids(
            &conn,
            vec![clause(LibraryClauseKind::Artist(text("jsmovie")))],
        )?;
        assert_eq!(found, vec![artist, album_artist]);
        Ok(())
    }

    /// R87-2-04: `title:` reads the track title, the album title and the
    /// feed title. `release:` reads the album title and the feed title.
    #[test]
    fn adr_0087_library_fields_r87_2_04_title_and_release_columns() -> Result<()> {
        let conn = setup_test_db()?;
        let plain = feed(&conn, "Plain")?;
        let remix_feed = feed(&conn, "Remix Feed")?;
        let track_title = track(&conn, plain, titled("Remix One"))?;
        let album_title = track(
            &conn,
            plain,
            Track {
                album: Some("Remix Album"),
                ..titled("A")
            },
        )?;
        let feed_title = track(&conn, remix_feed, titled("B"))?;

        let title = ids(&conn, vec![clause(LibraryClauseKind::Title(text("remix")))])?;
        assert_eq!(title, vec![track_title, album_title, feed_title]);

        let aged_feed = feed(&conn, "Aged Feed")?;
        let aged_track = track(&conn, plain, titled("Aged Track"))?;
        let aged_album = track(
            &conn,
            plain,
            Track {
                album: Some("Aged Album"),
                ..titled("C")
            },
        )?;
        let aged_by_feed = track(&conn, aged_feed, titled("D"))?;
        let release = ids(
            &conn,
            vec![clause(LibraryClauseKind::Release(text("aged")))],
        )?;
        assert_eq!(release, vec![aged_album, aged_by_feed]);
        assert!(!release.contains(&aged_track));
        Ok(())
    }

    /// R87-2-09: `duration:` reads `t.duration_seconds`. A track with no
    /// duration does not match, and it passes the exclusion.
    #[test]
    fn adr_0087_library_fields_r87_2_09_duration_bounds_and_null() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        let long = track(
            &conn,
            feed_id,
            Track {
                duration: Some(302),
                ..titled("Long")
            },
        )?;
        let short = track(
            &conn,
            feed_id,
            Track {
                duration: Some(300),
                ..titled("Short")
            },
        )?;
        let unknown = track(&conn, feed_id, titled("Unknown"))?;
        let range = LibraryClauseKind::Duration {
            min_secs: Some(301),
            max_secs: None,
        };

        assert_eq!(ids(&conn, vec![clause(range.clone())])?, vec![long]);
        assert_eq!(ids(&conn, vec![excluded(range)])?, vec![short, unknown]);
        Ok(())
    }

    /// R87-2-10: `is:downloaded` and `is:partial` read the counted tracks
    /// of the feed (decision D8).
    #[test]
    fn adr_0087_library_fields_r87_2_10_downloaded_and_partial() -> Result<()> {
        let conn = setup_test_db()?;
        let with_file = |title| Track {
            file: true,
            ..titled(title)
        };
        let feed_a = feed(&conn, "A")?;
        let a1 = track(&conn, feed_a, with_file("A1"))?;
        let a2 = track(&conn, feed_a, with_file("A2"))?;
        let feed_b = feed(&conn, "B")?;
        let b1 = track(&conn, feed_b, with_file("B1"))?;
        track(
            &conn,
            feed_b,
            Track {
                not_in_library: true,
                ..titled("B2")
            },
        )?;
        let feed_c = feed(&conn, "C")?;
        let c1 = track(&conn, feed_c, with_file("C1"))?;
        track(
            &conn,
            feed_c,
            Track {
                not_in_library: true,
                removed_at: Some(1_700_000_000),
                ..titled("C2")
            },
        )?;

        assert_eq!(
            ids(&conn, vec![clause(LibraryClauseKind::Downloaded)])?,
            vec![a1, a2, c1]
        );
        assert_eq!(
            ids(&conn, vec![clause(LibraryClauseKind::Partial)])?,
            vec![b1]
        );
        Ok(())
    }

    /// R87-2-11: `is:removed` reads the mark, also a confirmed mark.
    #[test]
    fn adr_0087_library_fields_r87_2_11_removed_mark() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        let removed = track(
            &conn,
            feed_id,
            Track {
                removed_at: Some(1_700_000_000),
                ..titled("Removed")
            },
        )?;
        let confirmed = track(
            &conn,
            feed_id,
            Track {
                removed_at: Some(1_700_000_000),
                confirmed_at: Some(1_700_000_100),
                ..titled("Confirmed")
            },
        )?;
        let kept = track(&conn, feed_id, titled("Kept"))?;

        assert_eq!(
            ids(&conn, vec![clause(LibraryClauseKind::Removed)])?,
            vec![removed, confirmed]
        );
        assert_eq!(
            ids(&conn, vec![excluded(LibraryClauseKind::Removed)])?,
            vec![kept]
        );
        Ok(())
    }

    /// R87-2-13: `playlist:` matches a part of the playlist name, without
    /// ASCII case.
    #[test]
    fn adr_0087_library_fields_r87_2_13_playlist_name() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        let friday = track(&conn, feed_id, titled("Friday"))?;
        let monday = track(&conn, feed_id, titled("Monday"))?;
        let none = track(&conn, feed_id, titled("None"))?;
        for (name, track_id) in [("Friday show", friday), ("Monday show", monday)] {
            conn.execute("INSERT INTO playlists (name) VALUES (?1)", [name])?;
            conn.execute(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, 0)",
                rusqlite::params![conn.last_insert_rowid(), track_id],
            )?;
        }
        let playlist = LibraryClauseKind::Playlist(text("friday"));

        assert_eq!(ids(&conn, vec![clause(playlist.clone())])?, vec![friday]);
        assert_eq!(ids(&conn, vec![excluded(playlist)])?, vec![monday, none]);
        Ok(())
    }

    /// R87-2-14: `has:split` and `has:image` are safe for `NULL` and for bad
    /// JSON (decision D12).
    #[test]
    fn adr_0087_library_fields_r87_2_14_split_and_image() -> Result<()> {
        let conn = setup_test_db()?;
        let route = r#"[{"type":"node","address":"a","split":100}]"#;
        let plain = feed_with(&conn, "Plain", None, Some("https://example.test/cover.jpg"))?;
        let with_route = feed_with(&conn, "Routed", Some(route), None)?;
        let own_route = track(
            &conn,
            plain,
            Track {
                route_json: Some(route),
                ..titled("Own")
            },
        )?;
        let feed_route = track(&conn, with_route, titled("Feed"))?;
        let empty = track(
            &conn,
            plain,
            Track {
                route_json: Some("[]"),
                ..titled("Empty")
            },
        )?;
        let null = track(&conn, plain, titled("Null"))?;
        let bad = track(
            &conn,
            plain,
            Track {
                route_json: Some("{x"),
                ..titled("Bad")
            },
        )?;

        assert_eq!(
            ids(&conn, vec![clause(LibraryClauseKind::HasSplit)])?,
            vec![own_route, feed_route]
        );
        assert_eq!(
            ids(&conn, vec![excluded(LibraryClauseKind::HasSplit)])?,
            vec![empty, null, bad]
        );

        let own_image = track(
            &conn,
            plain,
            Track {
                image: Some("https://example.test/track.jpg"),
                ..titled("Own image")
            },
        )?;
        track(
            &conn,
            plain,
            Track {
                image: Some("https://example.test/cover.jpg"),
                ..titled("Release image")
            },
        )?;
        track(
            &conn,
            plain,
            Track {
                image: Some(" "),
                ..titled("Blank image")
            },
        )?;
        assert_eq!(
            ids(&conn, vec![clause(LibraryClauseKind::HasImage)])?,
            vec![own_image]
        );
        Ok(())
    }

    /// R87-2-15: `-artist:x` keeps a track whose artist columns are `NULL`
    /// (decision D13).
    #[test]
    fn adr_0087_library_fields_r87_2_15_exclusion_keeps_null_columns() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        let no_artist = track(&conn, feed_id, titled("No artist"))?;
        track(
            &conn,
            feed_id,
            Track {
                artist: Some("x"),
                ..titled("Artist x")
            },
        )?;

        assert_eq!(
            ids(&conn, vec![excluded(LibraryClauseKind::Artist(text("x")))])?,
            vec![no_artist]
        );
        Ok(())
    }

    /// R87-2-16: the `LIKE` characters of a value match only themselves.
    #[test]
    fn adr_0087_library_fields_r87_2_16_like_characters_are_literal() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        let artist = |name| Track {
            artist: Some(name),
            ..titled("Song")
        };
        let percent = track(&conn, feed_id, artist("100%"))?;
        let underscore = track(&conn, feed_id, artist("a_b"))?;
        let backslash = track(&conn, feed_id, artist("a\\b"))?;
        track(&conn, feed_id, artist("1000"))?;
        track(&conn, feed_id, artist("aXb"))?;
        track(&conn, feed_id, artist("ab"))?;

        for (value, expected) in [("100%", percent), ("a_b", underscore), ("a\\b", backslash)] {
            assert_eq!(
                ids(&conn, vec![clause(LibraryClauseKind::Artist(text(value)))])?,
                vec![expected],
                "R87-2-16: {value} must match only the literal text"
            );
        }
        Ok(())
    }

    /// R87-2-17: a value never reaches the SQL text. It is a bound
    /// parameter, and each `?` binds one value.
    #[test]
    fn adr_0087_library_fields_r87_2_17_values_are_bound_parameters() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = feed(&conn, "Feed")?;
        track(&conn, feed_id, titled("Song"))?;
        let value = "'; DROP TABLE tracks; --";
        let search = LibraryTrackSearch {
            clauses: vec![
                clause(LibraryClauseKind::AnyText(text(value))),
                excluded(LibraryClauseKind::Artist(text(value))),
                clause(LibraryClauseKind::Playlist(text(value))),
                clause(LibraryClauseKind::TrackIds(vec![1, 2])),
                clause(LibraryClauseKind::Duration {
                    min_secs: Some(1),
                    max_secs: Some(2),
                }),
            ],
        };

        let (sql, values) = library_track_search_sql(&search, 50);

        assert!(!sql.contains(value));
        assert!(!sql.contains("DROP"));
        assert!(values
            .iter()
            .any(|bound| *bound == Value::Text(like_contains_pattern(value))));
        assert_eq!(sql.matches('?').count(), values.len());
        assert_eq!(values.last(), Some(&Value::Integer(50)));

        search_library_tracks(&conn, &search, 50)?;
        let tracks: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'tracks'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(tracks, 1);
        Ok(())
    }
}
