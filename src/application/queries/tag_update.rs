//! Tag update scan for ADR 0076 Decision 8 (packet 004).
//!
//! The scan compares the tags of each Library file with the stored
//! metadata. The expected frames come from the stored value projection of
//! ADR 0075 packet 020. The expected payment route comes from the stored
//! route of ADR 0076 packet 003.
//!
//! The scan reads the database and the files. It writes no file and no
//! database row. The confirm command of
//! `application::commands::tag_update` writes the files. A guard in
//! `tests/architecture_tests.rs` keeps each write call out of this module.
//!
//! The scan has two steps. `plan_tag_update_scan` reads the database.
//! `compare_planned_files` reads the files without a database lock.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::api::PaymentRoute;
use crate::audio_format::AudioFormat;
use crate::audio_tags::{read_audio_tags, sanitize_title_text, AudioTags, Id3v24Edit};
use crate::db;
use crate::metadata::{
    audio_tags_value_routes, frame_destination_for_format, id3_frame_base, parse_value_routes,
    payment_routes_equal, pending_id3_target_key, summarize_value_routes,
    MUSICINDEX_VALUE_ROUTES_FRAME,
};

/// The result of one scan: each Library file whose tags differ from the
/// stored metadata, and each file that the scan could not read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TagUpdateScan {
    pub(crate) files: Vec<TagUpdateFile>,
}

impl TagUpdateScan {
    /// The number of listed files. The "Update n file(s)" button shows it.
    #[must_use]
    pub(crate) fn count(&self) -> usize {
        self.files.len()
    }

    /// The number of listed files that the show plays or holds.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn in_use_count(&self) -> usize {
        self.files.iter().filter(|file| file.in_use).count()
    }
}

/// One listed file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagUpdateFile {
    pub(crate) track_id: i64,
    /// The stored track title.
    pub(crate) title: String,
    /// The stored album title.
    pub(crate) album: Option<String>,
    pub(crate) path: PathBuf,
    /// `true` when the show plays or holds the track (ADR 0076 Decision 8).
    pub(crate) in_use: bool,
    pub(crate) content: TagUpdateFileContent,
}

impl TagUpdateFile {
    /// The edits that a confirm writes. An unreadable file has none.
    #[must_use]
    pub(crate) fn edits(&self) -> Vec<Id3v24Edit> {
        match &self.content {
            TagUpdateFileContent::Differs { frames } => frames
                .iter()
                .map(|frame| Id3v24Edit {
                    frame_label: frame.frame_label.clone(),
                    value: frame.expected_value.clone(),
                })
                .collect(),
            TagUpdateFileContent::Unreadable { .. } => Vec::new(),
        }
    }

    /// `true` when a confirm can write the file.
    #[must_use]
    pub(crate) fn has_write_action(&self) -> bool {
        matches!(self.content, TagUpdateFileContent::Differs { .. })
    }
}

/// What the scan found in one listed file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TagUpdateFileContent {
    /// Each frame whose file value differs from the stored value.
    Differs { frames: Vec<TagFrameChange> },
    /// The scan could not read the tags. The file has no write action.
    Unreadable { error: String },
}

/// One frame to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagFrameChange {
    pub(crate) frame_label: String,
    /// The value in the file, or `None` when the file has no such frame.
    pub(crate) file_value: Option<String>,
    /// The stored value that a confirm writes.
    pub(crate) expected_value: String,
}

/// The database part of the scan for one file.
#[derive(Clone, Debug)]
pub(crate) struct PlannedFile {
    track_id: i64,
    title: String,
    album: Option<String>,
    path: PathBuf,
    in_use: bool,
    /// The expected frames without the payment route frame.
    expected: Vec<Id3v24Edit>,
    /// The stored route, when it has one or more recipients.
    route: Option<Vec<PaymentRoute>>,
}

/// Read the stored values of each Library track with a file.
///
/// A track without a recorded file, or with a recorded file that does not
/// exist, is not planned. The readiness report lists those tracks.
///
/// # Errors
///
/// Returns an error when a database row cannot be read.
pub(crate) fn plan_tag_update_scan(
    conn: &Connection,
    music_dir: &Path,
) -> Result<Vec<PlannedFile>> {
    let in_use = db::tracks_in_use_by_show(conn)?;
    let tracks = db::library_tracks(conn).context("load local library tracks")?;
    let mut planned = Vec::new();
    for track in &tracks {
        let Some(path) = track
            .local_path
            .as_ref()
            .map(|path| path.resolve(music_dir))
        else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        planned.push(plan_file(conn, track, path, &in_use)?);
    }
    Ok(planned)
}

fn plan_file(
    conn: &Connection,
    track: &db::TrackRow,
    path: PathBuf,
    in_use: &BTreeSet<i64>,
) -> Result<PlannedFile> {
    let context = crate::feed_service::track_row_to_track_context_with_local_identity(conn, track)
        .with_context(|| format!("read the stored values of track {}", track.id))?;
    let mut expected = crate::metadata_service::id3_edits_for_track_context(&context);
    // ADR 0076 Decision 9: the route frame comes from the stored route only.
    expected.retain(|edit| edit.frame_label != MUSICINDEX_VALUE_ROUTES_FRAME);
    let route = db::payment_routes::stored_route(conn, track.id)?.filter(|route| !route.is_empty());
    let title = context
        .track
        .title
        .clone()
        .or_else(|| track.track_title.clone())
        .or_else(|| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let album = context
        .feed
        .as_ref()
        .and_then(|feed| feed.title.clone())
        .or_else(|| track.album_title.clone());
    Ok(PlannedFile {
        track_id: track.id,
        title,
        album,
        path,
        in_use: in_use.contains(&track.id),
        expected,
        route,
    })
}

/// Read the tags of each planned file and keep each file that differs or
/// that the scan could not read.
#[must_use]
pub(crate) fn compare_planned_files(planned: Vec<PlannedFile>) -> TagUpdateScan {
    let files = planned
        .into_iter()
        .filter_map(|file| {
            let content = match read_audio_tags(&file.path) {
                Ok(tags) => {
                    let format = AudioFormat::detect_from_file(&file.path).ok();
                    let frames = changed_frames(&file, &tags, format);
                    if frames.is_empty() {
                        return None;
                    }
                    TagUpdateFileContent::Differs { frames }
                }
                Err(error) => TagUpdateFileContent::Unreadable {
                    error: format!("{error:#}"),
                },
            };
            Some(TagUpdateFile {
                track_id: file.track_id,
                title: file.title,
                album: file.album,
                path: file.path,
                in_use: file.in_use,
                content,
            })
        })
        .collect();
    TagUpdateScan { files }
}

/// Scan each Library file. The runtime actor runs the two steps apart, so
/// that the file reads hold no database lock.
///
/// # Errors
///
/// Returns an error when a database row cannot be read. A file that cannot
/// be read is listed with its error.
#[cfg(test)]
pub(crate) fn scan_tag_updates(conn: &Connection, music_dir: &Path) -> Result<TagUpdateScan> {
    Ok(compare_planned_files(plan_tag_update_scan(
        conn, music_dir,
    )?))
}

fn changed_frames(
    file: &PlannedFile,
    tags: &AudioTags,
    format: Option<AudioFormat>,
) -> Vec<TagFrameChange> {
    let mut frames = Vec::new();
    for edit in &file.expected {
        if !frame_is_compared(&edit.frame_label, format) {
            continue;
        }
        let target = pending_id3_target_key(&edit.frame_label);
        let file_values = tags
            .fields
            .iter()
            .filter(|field| pending_id3_target_key(&field.frame_id) == target)
            .map(|field| field.value.as_str())
            .collect::<Vec<_>>();
        if file_values
            .iter()
            .any(|value| values_match(&edit.frame_label, value, &edit.value))
        {
            continue;
        }
        frames.push(TagFrameChange {
            frame_label: edit.frame_label.clone(),
            file_value: file_values.first().map(|value| (*value).to_owned()),
            expected_value: edit.value.clone(),
        });
    }
    if let Some(route) = file.route.as_deref() {
        let file_text = audio_tags_value_routes(tags);
        let equal = file_text
            .and_then(parse_value_routes)
            .is_some_and(|file_route| payment_routes_equal(&file_route, route));
        if !equal {
            if let Some(expected_value) = summarize_value_routes(route) {
                frames.push(TagFrameChange {
                    frame_label: MUSICINDEX_VALUE_ROUTES_FRAME.to_owned(),
                    file_value: file_text.map(str::to_owned),
                    expected_value,
                });
            }
        }
    }
    frames
}

/// The scan does not compare a frame whose stored value is a reference to
/// other data (artwork, lyrics and transcripts). A text comparison of a
/// reference with the embedded data always differs. The scan also skips a
/// frame that the file format cannot store.
fn frame_is_compared(frame_label: &str, format: Option<AudioFormat>) -> bool {
    if matches!(id3_frame_base(frame_label), "APIC" | "USLT" | "SYLT") {
        return false;
    }
    match format {
        Some(format) if format != AudioFormat::Mp3 => {
            frame_destination_for_format(frame_label, format).is_some()
        }
        _ => true,
    }
}

/// Compare one file value with one stored value as the tag writer stores it.
fn values_match(frame_label: &str, file_value: &str, expected: &str) -> bool {
    let file_value = file_value.trim();
    let expected = expected.trim();
    match id3_frame_base(frame_label) {
        "TIT2" => file_value == sanitize_title_text(expected).trim(),
        // The track and disc numbers compare their own number. A total that
        // only one side has is not a difference.
        "TRCK" | "TPOS" => leading_number(file_value) == leading_number(expected),
        _ => file_value == expected,
    }
}

fn leading_number(value: &str) -> &str {
    value.split('/').next().unwrap_or_default().trim()
}

#[cfg(test)]
mod tests {
    use super::test_support::{library, settle, start_session, STORED_ROUTE};
    use super::*;

    use std::sync::Mutex;

    use crate::audio_tags::write_id3v24_edits;

    fn frame<'a>(file: &'a TagUpdateFile, label: &str) -> Option<&'a TagFrameChange> {
        match &file.content {
            TagUpdateFileContent::Differs { frames } => {
                frames.iter().find(|frame| frame.frame_label == label)
            }
            TagUpdateFileContent::Unreadable { .. } => None,
        }
    }

    /// R4-01: a file whose title frame differs from the projection is
    /// listed with that frame. A file that equals the projection is not.
    #[test]
    fn adr_0076_tag_update_title_difference_is_listed_and_equal_file_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1, 2]);
        settle(&conn, dir.path());
        write_id3v24_edits(
            &dir.path().join("song-1.mp3"),
            &[Id3v24Edit {
                frame_label: "TIT2".into(),
                value: "Old title".into(),
            }],
        )
        .unwrap();

        let scan = scan_tag_updates(&conn, dir.path()).unwrap();

        assert_eq!(scan.count(), 1);
        let file = &scan.files[0];
        assert_eq!(file.track_id, 1);
        assert_eq!(file.title, "Song 1");
        assert_eq!(file.album.as_deref(), Some("Album One"));
        let title = frame(file, "TIT2").expect("the title frame is listed");
        assert_eq!(title.file_value.as_deref(), Some("Old title"));
        assert_eq!(title.expected_value, "Song 1");
        assert_eq!(file.edits().len(), 1, "only the changed frame is written");
    }

    /// R4-02: a file whose route frame differs from the stored route is
    /// listed with the route frame.
    #[test]
    fn adr_0076_tag_update_route_difference_is_listed() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1]);
        conn.execute(
            "UPDATE tracks SET payment_routes_json = ?1 WHERE id = 1",
            [STORED_ROUTE],
        )
        .unwrap();
        settle(&conn, dir.path());
        write_id3v24_edits(
            &dir.path().join("song-1.mp3"),
            &[Id3v24Edit {
                frame_label: MUSICINDEX_VALUE_ROUTES_FRAME.into(),
                value: r#"[{"recipient_name":"Old","route_type":"node","split":100.0,"fee":false,"address":"03abcdef"}]"#.into(),
            }],
        )
        .unwrap();

        let scan = scan_tag_updates(&conn, dir.path()).unwrap();

        assert_eq!(scan.count(), 1);
        let route = frame(&scan.files[0], MUSICINDEX_VALUE_ROUTES_FRAME)
            .expect("the route frame is listed");
        let expected = parse_value_routes(&route.expected_value).unwrap();
        assert_eq!(expected[0].recipient_name.as_deref(), Some("Stored"));
        assert!(route.file_value.as_deref().unwrap().contains("Old"));
    }

    /// R4-03: a file that cannot be read is listed with its error and has
    /// no write action.
    #[test]
    fn adr_0076_tag_update_unreadable_file_is_listed_without_write_action() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1]);
        let path = dir.path().join("song-1.mp3");
        // A FLAC marker with no valid metadata block after it.
        std::fs::write(&path, b"fLaC\xff\xff\xff\xff not a stream").unwrap();

        let scan = scan_tag_updates(&conn, dir.path()).unwrap();

        assert_eq!(scan.count(), 1);
        let file = &scan.files[0];
        assert!(matches!(
            &file.content,
            TagUpdateFileContent::Unreadable { error } if !error.is_empty()
        ));
        assert!(!file.has_write_action());
        assert!(file.edits().is_empty());
    }

    /// R4-04: a playing session on the track marks the file in use. A
    /// session on its playlist does the same. A stopped session does not.
    #[test]
    fn adr_0076_tag_update_in_use_follows_the_playback_session() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1, 2, 3]);
        conn.execute_batch(
            "INSERT INTO playlists(id,name,created_at,updated_at) VALUES(1,'Show',0,0); \
             INSERT INTO playlist_tracks(playlist_id,track_id,position) VALUES(1,2,0),(1,3,1);",
        )
        .unwrap();
        let in_use = |conn: &Connection| {
            scan_tag_updates(conn, dir.path())
                .unwrap()
                .files
                .into_iter()
                .filter(|file| file.in_use)
                .map(|file| file.track_id)
                .collect::<Vec<_>>()
        };

        start_session(&conn, "track", 1, None);
        assert_eq!(in_use(&conn), vec![1]);

        db::stop_playback_session(&conn, "track").unwrap();
        assert!(in_use(&conn).is_empty(), "a stopped session holds no file");

        start_session(&conn, "playlist", 2, Some(1));
        assert_eq!(in_use(&conn), vec![2, 3], "the playlist holds each track");
        db::stop_playback_session(&conn, "playlist").unwrap();
        assert!(in_use(&conn).is_empty());
    }

    /// The scan reads the database only in its plan step. The compare step
    /// needs no connection.
    #[test]
    fn adr_0076_tag_update_compare_step_needs_no_connection() {
        let dir = tempfile::tempdir().unwrap();
        let conn = Mutex::new(library(dir.path(), &[1]));
        let planned = plan_tag_update_scan(&conn.lock().unwrap(), dir.path()).unwrap();
        let _held = conn.lock().unwrap();

        let scan = compare_planned_files(planned);

        assert_eq!(scan.count(), 1);
    }

    /// R80-10 (ADR 0080): the scan reports a file that still holds the
    /// earlier labeled channel value in `WOAR`. It does not report a file
    /// whose `WOAR` holds the current channel value alongside an unrelated
    /// (foreign) value from another tool.
    #[test]
    fn adr_0080_scan_reports_earlier_labeled_woar_and_ignores_a_foreign_one() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = library(dir.path(), &[1, 2]);
        db::replace_local_identity_links(
            &mut conn,
            db::LocalIdentityOwner::Feed(1),
            "rss",
            &[db::LocalIdentityLinkInput {
                entity_type: None,
                entity_id: None,
                position: None,
                link_type: Some("website".into()),
                url: Some("https://example.test/feed".into()),
                extraction_path: None,
                observed_at: None,
                raw_json: None,
            }],
        )
        .unwrap();
        settle(&conn, dir.path());

        // Track 1 still holds the earlier labeled channel value.
        write_id3v24_edits(
            &dir.path().join("song-1.mp3"),
            &[Id3v24Edit {
                frame_label: "WOAR".into(),
                value: "download for free (url, forward): https://example.test/feed".into(),
            }],
        )
        .unwrap();
        // Track 2 holds the current channel value, plus one from another
        // tool.
        write_id3v24_edits(
            &dir.path().join("song-2.mp3"),
            &[
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: "https://example.test/feed".into(),
                },
                Id3v24Edit {
                    frame_label: "WOAR".into(),
                    value: "https://foreign.example/from-another-tool".into(),
                },
            ],
        )
        .unwrap();

        let scan = scan_tag_updates(&conn, dir.path()).unwrap();

        let track_1 = scan
            .files
            .iter()
            .find(|file| file.track_id == 1)
            .expect("track 1 is listed for its earlier labeled value");
        assert!(
            frame(track_1, "WOAR").is_some(),
            "the earlier labeled value must be reported"
        );
        assert!(
            scan.files.iter().all(|file| file.track_id != 2),
            "a foreign value alongside the current one must not be reported"
        );
    }

    /// Orchestrator review, defect 1 (ADR 0080 Decision 3): the scan reports
    /// no difference for a FLAC track without its own description, after a
    /// write of the channel's description to
    /// `COMM:MusicIndex Album Description`.
    ///
    /// Before the fix, that value read back only under the shared Comment
    /// key, as `COMM:MusicIndex Description`, so the scan reported a
    /// difference on every read of a file this app had written.
    #[test]
    fn adr_0080_flac_album_description_round_trips_with_no_difference() {
        let flac_bytes = include_bytes!("../../../docs/runbooks/fixtures/conversion.flac");
        let temp = tempfile::Builder::new().suffix(".flac").tempfile().unwrap();
        std::fs::write(temp.path(), flac_bytes).unwrap();
        let expected = vec![Id3v24Edit {
            frame_label: "COMM:MusicIndex Album Description".into(),
            value: "Feed description".into(),
        }];
        write_id3v24_edits(temp.path(), &expected).unwrap();
        let tags = crate::audio_tags::read_audio_tags(temp.path()).unwrap();

        let planned = PlannedFile {
            track_id: 1,
            title: "Song".into(),
            album: Some("Feed".into()),
            path: temp.path().to_path_buf(),
            in_use: false,
            expected,
            route: None,
        };

        let frames = changed_frames(&planned, &tags, Some(AudioFormat::Flac));

        assert!(
            frames.is_empty(),
            "a FLAC file the app wrote must show no difference: {frames:?}"
        );
    }
}

/// Fixtures that the tag update tests of other modules share.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    use crate::audio_tags::write_id3v24_edits;

    pub(crate) const STORED_ROUTE: &str = r#"[{"recipient_name":"Stored","route_type":"node","split":100.0,"fee":false,"address":"03abcdef"}]"#;

    /// A fixture database with one feed and the given Library tracks. Each
    /// track `id` has the title `Song {id}` and the file `song-{id}.mp3`.
    pub(crate) fn library(dir: &Path, track_ids: &[i64]) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::upgrades::create_fixture(&conn, db::CURRENT_VERSION).unwrap();
        conn.execute_batch(
            "INSERT INTO feeds(id,feed_url,feed_guid,title,is_subscribed) \
             VALUES(1,'http://fixture.invalid/feed.xml','f1','Album One',1);",
        )
        .unwrap();
        for id in track_ids {
            conn.execute(
                "INSERT INTO tracks(id,feed_id,item_guid,track_title,is_in_library) \
                 VALUES(?1,1,?2,?3,1)",
                rusqlite::params![id, format!("t{id}"), format!("Song {id}")],
            )
            .unwrap();
            let name = format!("song-{id}.mp3");
            std::fs::write(dir.join(&name), b"not really an mp3").unwrap();
            conn.execute(
                "INSERT INTO local_files(path,track_id) VALUES(?1,?2)",
                rusqlite::params![name, id],
            )
            .unwrap();
        }
        conn
    }

    /// Write the expected frames of each listed file, so that the file
    /// equals the stored metadata.
    pub(crate) fn settle(conn: &Connection, dir: &Path) {
        for file in scan_tag_updates(conn, dir).unwrap().files {
            write_id3v24_edits(&file.path, &file.edits()).unwrap();
        }
        assert_eq!(scan_tag_updates(conn, dir).unwrap().count(), 0);
    }

    pub(crate) fn start_session(conn: &Connection, id: &str, track: i64, playlist: Option<i64>) {
        db::set_playback_session_track(conn, id, track, playlist, playlist.map(|_| 0), "now", 0)
            .unwrap();
    }
}
