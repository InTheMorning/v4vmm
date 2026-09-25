//! Tag update confirm for ADR 0076 Decision 8 (packet 004).
//!
//! The operator confirms the "Update n file(s)" popup. This command writes
//! the listed frames of each listed file through the tag boundary of ADRs
//! 0004 and 0008. It does not write a file that the show plays or holds.
//! It reads the playback sessions again before the first write, because a
//! show can start or stop after the scan. That read decides, and the mark
//! of the scan does not.
//!
//! The payment route frame goes through
//! `metadata_service::with_stored_route_frame` before the write
//! (ADR 0076 Decision 9).

#![warn(clippy::pedantic)]

use std::sync::Mutex;

use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::application::queries::tag_update::TagUpdateFile;
use crate::audio_tags::write_id3v24_edits;
use crate::db;
use crate::metadata_service::{with_stored_route_frame, RouteFrameWrite};

/// The result of one listed file after a confirm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TagUpdateWriteStatus {
    /// The app wrote the frames to the file.
    Written { frames: usize },
    /// The show plays or holds the file. The app did not write it.
    InUse,
    /// The scan could not read the file. The app did not write it.
    Unreadable,
    /// The write failed. The file can have its earlier tags.
    Failed { error: String },
}

/// One file result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagUpdateWriteResult {
    pub(crate) track_id: i64,
    pub(crate) title: String,
    pub(crate) status: TagUpdateWriteStatus,
}

/// Each file result of one confirm, in list order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TagUpdateWriteReport {
    pub(crate) results: Vec<TagUpdateWriteResult>,
    /// The time at which the confirm completed, in microseconds since the
    /// Unix epoch.
    pub(crate) finished_at_us: i64,
}

impl TagUpdateWriteReport {
    /// The number of written files.
    #[must_use]
    pub(crate) fn written_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| matches!(result.status, TagUpdateWriteStatus::Written { .. }))
            .count()
    }
}

/// Write each listed file that the show does not play or hold.
///
/// `on_written` receives the track id after each successful write. The
/// runtime sends `VmEvent::TrackChanged` from it.
///
/// A write failure is recorded for its file and does not stop the other
/// files.
///
/// # Errors
///
/// Returns an error, and writes no file, when the database lock is not
/// available or the playback sessions cannot be read. The app then cannot
/// know which files are in use.
pub(crate) fn write_tag_updates(
    conn: &Mutex<Connection>,
    files: &[TagUpdateFile],
    mut on_written: impl FnMut(i64),
) -> Result<TagUpdateWriteReport> {
    let in_use = {
        let conn = conn
            .lock()
            .map_err(|_| anyhow!("The database lock is not available."))?;
        db::tracks_in_use_by_show(&conn)?
    };
    let mut report = TagUpdateWriteReport::default();
    for file in files {
        let status = if !file.has_write_action() {
            TagUpdateWriteStatus::Unreadable
        } else if in_use.contains(&file.track_id) {
            TagUpdateWriteStatus::InUse
        } else {
            // ADR 0076 Decision 9: a listed route frame gets the stored
            // route. The function adds no route frame that the list does
            // not have.
            let edits = conn
                .lock()
                .map_err(|_| anyhow!("The database lock is not available."))
                .and_then(|conn| {
                    with_stored_route_frame(
                        &conn,
                        file.track_id,
                        None,
                        file.edits(),
                        RouteFrameWrite::WhenSelected,
                    )
                });
            match edits.and_then(|edits| write_id3v24_edits(&file.path, &edits)) {
                Ok(frames) => {
                    on_written(file.track_id);
                    TagUpdateWriteStatus::Written { frames }
                }
                Err(error) => TagUpdateWriteStatus::Failed {
                    error: format!("{error:#}"),
                },
            }
        };
        report.results.push(TagUpdateWriteResult {
            track_id: file.track_id,
            title: file.title.clone(),
            status,
        });
    }
    report.finished_at_us = chrono::Utc::now().timestamp_micros();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::application::queries::tag_update::scan_tag_updates;
    use crate::application::queries::tag_update::test_support::{library, settle, start_session};
    use crate::audio_tags::{read_audio_tags, Id3v24Edit};

    fn retitle(path: &std::path::Path, title: &str) {
        write_id3v24_edits(
            path,
            &[Id3v24Edit {
                frame_label: "TIT2".into(),
                value: title.into(),
            }],
        )
        .unwrap();
    }

    fn file_title(path: &std::path::Path) -> Option<String> {
        read_audio_tags(path).unwrap().title
    }

    /// R4-06 and R4-07: confirm writes each file that is not in use, and
    /// writes no file that is in use. After confirm, the count equals the
    /// number of files that were in use.
    #[test]
    fn adr_0076_tag_update_confirm_skips_files_in_use_and_keeps_their_count() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1, 2, 3]);
        settle(&conn, dir.path());
        for id in [1, 2, 3] {
            retitle(&dir.path().join(format!("song-{id}.mp3")), "Old title");
        }
        let scan = scan_tag_updates(&conn, dir.path()).unwrap();
        assert_eq!(scan.count(), 3);
        start_session(&conn, "show", 2, None);
        let conn = Mutex::new(conn);
        let mut changed = Vec::new();

        let report = write_tag_updates(&conn, &scan.files, |id| changed.push(id)).unwrap();

        let statuses = report
            .results
            .iter()
            .map(|result| (result.track_id, result.status.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            statuses,
            vec![
                (1, TagUpdateWriteStatus::Written { frames: 1 }),
                (2, TagUpdateWriteStatus::InUse),
                (3, TagUpdateWriteStatus::Written { frames: 1 }),
            ]
        );
        assert_eq!(changed, vec![1, 3], "each written track is reported");
        assert_eq!(
            file_title(&dir.path().join("song-1.mp3")).as_deref(),
            Some("Song 1")
        );
        assert_eq!(
            file_title(&dir.path().join("song-2.mp3")).as_deref(),
            Some("Old title"),
            "the file in use keeps its tags"
        );
        let after = scan_tag_updates(&conn.lock().unwrap(), dir.path()).unwrap();
        assert_eq!(after.count(), 1);
        assert_eq!(after.count(), after.in_use_count());
    }

    /// R4-06: a show that starts after the scan keeps its file. The confirm
    /// reads the sessions again.
    #[test]
    fn adr_0076_tag_update_confirm_reads_sessions_again_before_the_write() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1]);
        let scan = scan_tag_updates(&conn, dir.path()).unwrap();
        assert!(!scan.files[0].in_use);
        start_session(&conn, "late", 1, None);
        let conn = Mutex::new(conn);

        let report = write_tag_updates(&conn, &scan.files, |_| {}).unwrap();

        assert_eq!(report.results[0].status, TagUpdateWriteStatus::InUse);
        assert_eq!(file_title(&dir.path().join("song-1.mp3")), None);
    }

    /// A show that stopped after the scan releases its file. A later
    /// confirm writes it (ADR 0076 Decision 8).
    #[test]
    fn adr_0076_tag_update_confirm_writes_a_file_that_the_show_released() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1]);
        start_session(&conn, "show", 1, None);
        let scan = scan_tag_updates(&conn, dir.path()).unwrap();
        assert!(scan.files[0].in_use);
        db::stop_playback_session(&conn, "show").unwrap();
        let conn = Mutex::new(conn);

        let report = write_tag_updates(&conn, &scan.files, |_| {}).unwrap();

        assert!(matches!(
            report.results[0].status,
            TagUpdateWriteStatus::Written { .. }
        ));
    }

    /// R4-08: a write failure on one file is reported, and the other files
    /// are written.
    #[test]
    fn adr_0076_tag_update_write_failure_is_reported_and_others_are_written() {
        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1, 2]);
        let scan = scan_tag_updates(&conn, dir.path()).unwrap();
        assert_eq!(scan.count(), 2);
        // The first file becomes a directory after the scan.
        let first = dir.path().join("song-1.mp3");
        std::fs::remove_file(&first).unwrap();
        std::fs::create_dir(&first).unwrap();
        let conn = Mutex::new(conn);
        let mut changed = Vec::new();

        let report = write_tag_updates(&conn, &scan.files, |id| changed.push(id)).unwrap();

        assert!(matches!(
            &report.results[0].status,
            TagUpdateWriteStatus::Failed { error } if !error.is_empty()
        ));
        assert!(matches!(
            report.results[1].status,
            TagUpdateWriteStatus::Written { .. }
        ));
        assert_eq!(report.written_count(), 1);
        assert_eq!(changed, vec![2]);
        assert_eq!(
            file_title(&dir.path().join("song-2.mp3")).as_deref(),
            Some("Song 2")
        );
    }

    /// The route frame of a confirm is the stored route (ADR 0076 Decision
    /// 9), also when the stored route changed after the scan.
    #[test]
    fn adr_0076_tag_update_confirm_writes_the_current_stored_route() {
        use crate::application::queries::tag_update::test_support::STORED_ROUTE;
        use crate::metadata::{audio_tags_value_routes, parse_value_routes};

        let dir = tempfile::tempdir().unwrap();
        let conn = library(dir.path(), &[1]);
        conn.execute(
            "UPDATE tracks SET payment_routes_json = ?1 WHERE id = 1",
            [STORED_ROUTE],
        )
        .unwrap();
        let scan = scan_tag_updates(&conn, dir.path()).unwrap();
        conn.execute(
            "UPDATE tracks SET payment_routes_json = ?1 WHERE id = 1",
            [STORED_ROUTE.replace("Stored", "Newer")],
        )
        .unwrap();
        let conn = Mutex::new(conn);

        write_tag_updates(&conn, &scan.files, |_| {}).unwrap();

        let tags = read_audio_tags(&dir.path().join("song-1.mp3")).unwrap();
        let routes = parse_value_routes(audio_tags_value_routes(&tags).unwrap()).unwrap();
        assert_eq!(routes[0].recipient_name.as_deref(), Some("Newer"));
    }
}
