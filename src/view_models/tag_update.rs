//! View model of the "Update n file(s)" button, its popup and its report
//! (ADR 0076 Decision 8, packet 004).
//!
//! The Music section renders these displays. The view model decides the
//! count, the labels, the in-use mark and each report sentence. No renderer
//! decides which file is in use or if the button is available.

#![warn(clippy::pedantic)]

use std::fmt::Write as _;

use crate::application::commands::tag_update::{TagUpdateWriteReport, TagUpdateWriteStatus};
use crate::application::queries::tag_update::{
    TagFrameChange, TagUpdateFile, TagUpdateFileContent, TagUpdateScan,
};
use crate::metadata::MUSICINDEX_VALUE_ROUTES_FRAME;
use crate::runtime::TagUpdateSnapshot;

/// The longest file or stored value that a popup line shows, in
/// characters. A longer value ends with an ellipsis.
const VALUE_PREVIEW_CHARS: usize = 80;

/// The mark of a file that the show plays or holds.
pub(crate) const IN_USE_MARK: &str = "In use by the show";
/// The mark of a file that the scan could not read.
pub(crate) const UNREADABLE_MARK: &str = "Cannot read";

/// Availability of the "Update n file(s)" button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TagUpdateAvailability {
    /// The button opens the popup.
    Available,
    /// A confirm writes files. The button waits for its result.
    Writing,
}

/// Display contract of the "Update n file(s)" button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagUpdateButtonDisplay {
    pub(crate) button_id: &'static str,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
    pub(crate) availability: TagUpdateAvailability,
    pub(crate) enabled: bool,
}

/// The button, or `None` when the scan lists no file.
#[must_use]
pub(crate) fn button(snapshot: Option<&TagUpdateSnapshot>) -> Option<TagUpdateButtonDisplay> {
    let snapshot = snapshot?;
    let count = snapshot.scan.as_ref().map_or(0, TagUpdateScan::count);
    if count == 0 {
        return None;
    }
    let availability = if snapshot.writing {
        TagUpdateAvailability::Writing
    } else {
        TagUpdateAvailability::Available
    };
    let files = files_text(count);
    let (label, a11y_label) = match availability {
        TagUpdateAvailability::Available => (
            format!("Update {files}"),
            format!("Show the {files} whose tags differ from the stored metadata"),
        ),
        TagUpdateAvailability::Writing => (
            format!("Updating {files}"),
            "The app writes the tags. Wait for the result".to_owned(),
        ),
    };
    Some(TagUpdateButtonDisplay {
        button_id: "library-tag-update",
        label,
        a11y_label,
        availability,
        enabled: availability == TagUpdateAvailability::Available,
    })
}

/// Role of the mark of one popup file. The mark text states the reason, so
/// the role is never the only signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TagUpdateMarkRole {
    InUse,
    Unreadable,
}

/// One file of the popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagUpdateFileDisplay {
    pub(crate) title: String,
    pub(crate) album: Option<String>,
    /// One line for each frame to write, or the read error.
    pub(crate) lines: Vec<String>,
    pub(crate) mark: Option<(&'static str, TagUpdateMarkRole)>,
}

/// Display contract of the popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagUpdatePopupDisplay {
    pub(crate) title: String,
    pub(crate) message: String,
    pub(crate) cancel_button_id: &'static str,
    pub(crate) cancel_label: &'static str,
    pub(crate) cancel_a11y_label: &'static str,
    pub(crate) confirm_button_id: &'static str,
    pub(crate) confirm_label: &'static str,
    pub(crate) confirm_a11y_label: String,
    pub(crate) files: Vec<TagUpdateFileDisplay>,
}

/// The popup of the latest scan, or `None` when the scan lists no file.
#[must_use]
pub(crate) fn popup(snapshot: Option<&TagUpdateSnapshot>) -> Option<TagUpdatePopupDisplay> {
    let scan = snapshot?.scan.as_ref()?;
    if scan.count() == 0 {
        return None;
    }
    let writable = scan
        .files
        .iter()
        .filter(|file| file.has_write_action() && !file.in_use)
        .count();
    let held = scan.files.len() - writable;
    let mut message = format!(
        "The app writes the stored metadata to the tags of {}. Each file below shows the frames to write.",
        files_text(writable)
    );
    if held > 0 {
        let _ = write!(
            message,
            " The app does not write {}: each one is in use by the show or cannot be read.",
            files_text(held)
        );
    }
    Some(TagUpdatePopupDisplay {
        title: format!("Update {}?", files_text(scan.count())),
        message,
        cancel_button_id: "library-tag-update-cancel",
        cancel_label: "Cancel",
        cancel_a11y_label: "Close the list and write no file",
        confirm_button_id: "library-tag-update-confirm",
        confirm_label: "Write Tags",
        confirm_a11y_label: format!(
            "Write the stored metadata to the tags of {}",
            files_text(writable)
        ),
        files: scan.files.iter().map(file_display).collect(),
    })
}

fn file_display(file: &TagUpdateFile) -> TagUpdateFileDisplay {
    let (lines, unreadable) = match &file.content {
        TagUpdateFileContent::Differs { frames, .. } => {
            (frames.iter().map(frame_line).collect(), false)
        }
        TagUpdateFileContent::Unreadable { error } => (
            vec![format!(
                "The app cannot read the tags of this file: {error}"
            )],
            true,
        ),
    };
    let mark = if file.in_use {
        Some((IN_USE_MARK, TagUpdateMarkRole::InUse))
    } else if unreadable {
        Some((UNREADABLE_MARK, TagUpdateMarkRole::Unreadable))
    } else {
        None
    };
    TagUpdateFileDisplay {
        title: file.title.clone(),
        album: file.album.clone(),
        lines,
        mark,
    }
}

fn frame_line(frame: &TagFrameChange) -> String {
    let name = if frame.frame_label == MUSICINDEX_VALUE_ROUTES_FRAME {
        format!("Payment route ({})", frame.frame_label)
    } else {
        frame.frame_label.clone()
    };
    let file_value = frame
        .file_value
        .as_deref()
        .map_or_else(|| "no value".to_owned(), preview);
    format!(
        "{name}: {} (file: {file_value})",
        preview(&frame.expected_value)
    )
}

fn preview(value: &str) -> String {
    let value = value.trim();
    if value.chars().count() <= VALUE_PREVIEW_CHARS {
        return value.to_owned();
    }
    let mut short = value.chars().take(VALUE_PREVIEW_CHARS).collect::<String>();
    short.push('\u{2026}');
    short
}

/// Role of one report line. The line text states the result, so the role is
/// never the only signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TagUpdateReportRole {
    Written,
    NotWritten,
    Failed,
}

/// One line of the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagUpdateReportLine {
    pub(crate) text: String,
    pub(crate) role: TagUpdateReportRole,
}

/// The report of the latest confirm and the latest scan error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TagUpdateReportDisplay {
    pub(crate) summary: String,
    pub(crate) lines: Vec<TagUpdateReportLine>,
}

/// The report, or `None` when no confirm ran and the scan had no error.
/// `request_failed` is `true` when the runtime did not accept the latest
/// confirm.
#[must_use]
pub(crate) fn report(
    snapshot: Option<&TagUpdateSnapshot>,
    request_failed: bool,
) -> Option<TagUpdateReportDisplay> {
    if request_failed {
        return Some(TagUpdateReportDisplay {
            summary:
                "The tag update wrote no file. The background runtime did not accept the request."
                    .to_owned(),
            lines: Vec::new(),
        });
    }
    let snapshot = snapshot?;
    if let Some(error) = &snapshot.write_error {
        return Some(TagUpdateReportDisplay {
            summary: format!("The tag update wrote no file. {error}"),
            lines: Vec::new(),
        });
    }
    if let Some(write) = &snapshot.last_write {
        return Some(write_report(write));
    }
    snapshot
        .scan_error
        .as_ref()
        .map(|error| TagUpdateReportDisplay {
            summary: format!(
                "The app could not compare the file tags with the stored metadata. {error}"
            ),
            lines: Vec::new(),
        })
}

fn write_report(write: &TagUpdateWriteReport) -> TagUpdateReportDisplay {
    let at = chrono::DateTime::from_timestamp_micros(write.finished_at_us).map_or_else(
        || "an unknown time".to_owned(),
        |at| at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    );
    let written = write.written_count();
    let not_written = write.results.len() - written;
    let mut summary = format!("Tag update at {at}: the app wrote {}.", files_text(written));
    if not_written > 0 {
        let _ = write!(
            summary,
            " It did not write {}. The button keeps their count.",
            files_text(not_written)
        );
    }
    let lines = write
        .results
        .iter()
        .map(|result| {
            let title = &result.title;
            match &result.status {
                TagUpdateWriteStatus::Written { frames } => TagUpdateReportLine {
                    text: format!("Wrote {} to {title}.", frames_text(*frames)),
                    role: TagUpdateReportRole::Written,
                },
                TagUpdateWriteStatus::InUse => TagUpdateReportLine {
                    text: format!("Did not write {title}. The file is in use by the show."),
                    role: TagUpdateReportRole::NotWritten,
                },
                TagUpdateWriteStatus::Unreadable => TagUpdateReportLine {
                    text: format!("Did not write {title}. The app cannot read its tags."),
                    role: TagUpdateReportRole::NotWritten,
                },
                TagUpdateWriteStatus::Failed { error } => TagUpdateReportLine {
                    text: format!("Could not write {title}. {error}"),
                    role: TagUpdateReportRole::Failed,
                },
            }
        })
        .collect();
    TagUpdateReportDisplay { summary, lines }
}

fn files_text(count: usize) -> String {
    if count == 1 {
        "1 file".to_owned()
    } else {
        format!("{count} files")
    }
}

fn frames_text(count: usize) -> String {
    if count == 1 {
        "1 frame".to_owned()
    } else {
        format!("{count} frames")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::application::commands::tag_update::TagUpdateWriteResult;

    fn differs(track_id: i64, in_use: bool) -> TagUpdateFile {
        TagUpdateFile {
            track_id,
            title: format!("Song {track_id}"),
            album: Some("Album One".into()),
            path: PathBuf::from(format!("/music/song-{track_id}.mp3")),
            in_use,
            content: TagUpdateFileContent::Differs {
                frames: vec![TagFrameChange {
                    frame_label: "TIT2".into(),
                    file_value: Some("Old title".into()),
                    expected_value: format!("Song {track_id}"),
                }],
                shared_key_edits: Vec::new(),
            },
        }
    }

    fn snapshot(files: Vec<TagUpdateFile>) -> TagUpdateSnapshot {
        TagUpdateSnapshot {
            scan: Some(TagUpdateScan { files }),
            ..TagUpdateSnapshot::default()
        }
    }

    /// R4-05: the button exists only when the count is above 0. Its label
    /// carries the count, and it has an accessibility label.
    #[test]
    fn adr_0076_tag_update_button_shows_only_above_zero_with_count() {
        assert_eq!(button(None), None);
        assert_eq!(button(Some(&TagUpdateSnapshot::default())), None);
        assert_eq!(button(Some(&snapshot(Vec::new()))), None);

        let one = button(Some(&snapshot(vec![differs(1, false)]))).unwrap();
        assert_eq!(one.label, "Update 1 file");
        assert!(one.enabled);

        let three = button(Some(&snapshot(vec![
            differs(1, false),
            differs(2, true),
            differs(3, false),
        ])))
        .unwrap();
        assert_eq!(three.label, "Update 3 files");
        assert_eq!(
            three.a11y_label,
            "Show the 3 files whose tags differ from the stored metadata"
        );
        assert_eq!(three.availability, TagUpdateAvailability::Available);

        let mut writing = snapshot(vec![differs(1, false)]);
        writing.writing = true;
        let busy = button(Some(&writing)).unwrap();
        assert_eq!(busy.availability, TagUpdateAvailability::Writing);
        assert!(!busy.enabled);
    }

    /// The popup lists each file with its title, album, frames and in-use
    /// mark. It has one confirm and one cancel button.
    #[test]
    fn adr_0076_tag_update_popup_lists_files_with_frames_and_marks() {
        let mut unreadable = differs(3, false);
        unreadable.content = TagUpdateFileContent::Unreadable {
            error: "probe failed".into(),
        };
        let popup = popup(Some(&snapshot(vec![
            differs(1, false),
            differs(2, true),
            unreadable,
        ])))
        .unwrap();

        assert_eq!(popup.title, "Update 3 files?");
        assert_eq!(popup.confirm_label, "Write Tags");
        assert_eq!(popup.cancel_label, "Cancel");
        assert_eq!(
            popup.confirm_a11y_label,
            "Write the stored metadata to the tags of 1 file"
        );
        assert_eq!(popup.files.len(), 3);
        assert_eq!(popup.files[0].title, "Song 1");
        assert_eq!(popup.files[0].album.as_deref(), Some("Album One"));
        assert_eq!(
            popup.files[0].lines,
            vec!["TIT2: Song 1 (file: Old title)".to_owned()]
        );
        assert_eq!(popup.files[0].mark, None);
        assert_eq!(
            popup.files[1].mark,
            Some((IN_USE_MARK, TagUpdateMarkRole::InUse))
        );
        assert_eq!(
            popup.files[2].mark,
            Some((UNREADABLE_MARK, TagUpdateMarkRole::Unreadable))
        );
        assert!(popup.files[2].lines[0].contains("probe failed"));
    }

    /// Each result has its own report line, and the summary gives the
    /// recorded time.
    #[test]
    fn adr_0076_tag_update_report_names_each_result_and_its_time() {
        let state = TagUpdateSnapshot {
            last_write: Some(TagUpdateWriteReport {
                results: vec![
                    TagUpdateWriteResult {
                        track_id: 1,
                        title: "Song 1".into(),
                        status: TagUpdateWriteStatus::Written { frames: 2 },
                    },
                    TagUpdateWriteResult {
                        track_id: 2,
                        title: "Song 2".into(),
                        status: TagUpdateWriteStatus::InUse,
                    },
                    TagUpdateWriteResult {
                        track_id: 3,
                        title: "Song 3".into(),
                        status: TagUpdateWriteStatus::Failed {
                            error: "disk full".into(),
                        },
                    },
                ],
                finished_at_us: 1_000_000_000,
            }),
            ..TagUpdateSnapshot::default()
        };

        let report = report(Some(&state), false).unwrap();

        assert_eq!(
            report.summary,
            "Tag update at 1970-01-01 00:16:40 UTC: the app wrote 1 file. It did not write 2 files. The button keeps their count."
        );
        let lines = report
            .lines
            .iter()
            .map(|line| (line.text.as_str(), line.role))
            .collect::<Vec<_>>();
        assert_eq!(
            lines,
            vec![
                ("Wrote 2 frames to Song 1.", TagUpdateReportRole::Written),
                (
                    "Did not write Song 2. The file is in use by the show.",
                    TagUpdateReportRole::NotWritten
                ),
                (
                    "Could not write Song 3. disk full",
                    TagUpdateReportRole::Failed
                ),
            ]
        );
    }
}
