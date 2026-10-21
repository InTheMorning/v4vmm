//! The download progress item and the queue key of a release (ADR 0084
//! Decision 10, packet 022).
//!
//! The download queue in `src/library/download_queue.rs` gives a
//! [`DownloadBatch`] snapshot. [`DownloadProgressDisplay::project`] makes
//! each text, count, time and action of the progress item from it. The
//! projection is pure: it reads no clock, and the renderer makes no text.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use chrono::{DateTime, Local};

/// The queue key of a release (ADR 0084 Decision 10). A Library release
/// and the Index copy of that release have the same key.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum FeedKey {
    /// The feed GUID. The queue uses it first.
    Guid(String),
    /// The feed URL, when the release has no feed GUID.
    Url(String),
    /// The Library feed id, when the release has no GUID and no URL.
    LibraryFeed(i64),
}

impl FeedKey {
    /// The key of a release: the feed GUID first, then the feed URL, then
    /// the Library feed id. `None` when the release has none of them.
    #[must_use]
    pub(crate) fn new(
        feed_guid: Option<&str>,
        feed_url: Option<&str>,
        library_feed_id: Option<i64>,
    ) -> Option<Self> {
        fn present(value: Option<&str>) -> Option<&str> {
            value.map(str::trim).filter(|value| !value.is_empty())
        }
        present(feed_guid)
            .map(|guid| Self::Guid(guid.to_owned()))
            .or_else(|| present(feed_url).map(|url| Self::Url(url.to_owned())))
            .or_else(|| library_feed_id.map(Self::LibraryFeed))
    }
}

/// The phase of a download job (orchestrator correction of 2026-10-20).
/// A queued job and a job in flight show different state words.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DownloadPhase {
    /// The job waits in the queue.
    Queued,
    /// The job runs.
    InFlight,
}

impl DownloadPhase {
    const fn of(active: bool, in_flight: bool) -> Option<Self> {
        if in_flight {
            Some(Self::InFlight)
        } else if active {
            Some(Self::Queued)
        } else {
            None
        }
    }
}

/// The downloads that are queued or in flight, for the busy state of each
/// page (ADR 0084 Decision 10). The `in_flight_*` sets hold only the jobs
/// that run. Each one is a part of the set of the same name without the
/// prefix.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DownloadActivity {
    /// The Library track ids of each queued or running track job.
    pub(crate) tracks: BTreeSet<i64>,
    /// The key of each queued or running job.
    pub(crate) keys: BTreeSet<FeedKey>,
    /// The key of each queued or running release job.
    pub(crate) release_keys: BTreeSet<FeedKey>,
    /// The Library track ids of each running track job.
    pub(crate) in_flight_tracks: BTreeSet<i64>,
    /// The key of each running job.
    pub(crate) in_flight_keys: BTreeSet<FeedKey>,
    /// The key of each running release job.
    pub(crate) in_flight_release_keys: BTreeSet<FeedKey>,
    /// The release key and the track GUID of each queued or running Index
    /// track job (ADR 0083 task 006, decision D8). A job of a track with
    /// no track GUID is not in this set.
    pub(crate) index_tracks: BTreeSet<(FeedKey, String)>,
    /// The release key and the track GUID of each running Index track job.
    pub(crate) in_flight_index_tracks: BTreeSet<(FeedKey, String)>,
}

impl DownloadActivity {
    /// `true` while a job of `key` is queued or in flight.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_active(&self, key: &FeedKey) -> bool {
        self.keys.contains(key)
    }

    /// The phase of the jobs of `key`. `InFlight` when a job of `key`
    /// runs, `Queued` when a job of `key` only waits.
    #[must_use]
    pub(crate) fn key_phase(&self, key: &FeedKey) -> Option<DownloadPhase> {
        DownloadPhase::of(self.keys.contains(key), self.in_flight_keys.contains(key))
    }

    /// The phase of the release jobs of `key`.
    #[must_use]
    pub(crate) fn release_phase(&self, key: &FeedKey) -> Option<DownloadPhase> {
        DownloadPhase::of(
            self.release_keys.contains(key),
            self.in_flight_release_keys.contains(key),
        )
    }

    /// The phase of the Index track job of the track GUID `guid` in the
    /// release `key` (ADR 0083 task 006, decision D8).
    #[must_use]
    pub(crate) fn index_track_phase(&self, key: &FeedKey, guid: &str) -> Option<DownloadPhase> {
        let entry = (key.clone(), guid.to_owned());
        DownloadPhase::of(
            self.index_tracks.contains(&entry),
            self.in_flight_index_tracks.contains(&entry),
        )
    }

    /// The phase of the job of the Library track `track_id`.
    #[must_use]
    pub(crate) fn track_phase(&self, track_id: i64) -> Option<DownloadPhase> {
        DownloadPhase::of(
            self.tracks.contains(&track_id),
            self.in_flight_tracks.contains(&track_id),
        )
    }
}

/// One failed download of a batch: the item name and its error text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DownloadFailure {
    pub(crate) label: String,
    pub(crate) error: String,
}

/// A snapshot of the download batch. The queue makes it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DownloadBatch {
    /// Each job that joined the batch.
    pub(crate) total: usize,
    /// Each job that completed with no error.
    pub(crate) done: usize,
    /// Each job that failed, in completion order.
    pub(crate) failures: Vec<DownloadFailure>,
    /// Each job that the operator stopped while it waited (ADR 0084
    /// Decision 10, packet 027: "Stop queued").
    pub(crate) stopped: usize,
    /// Each job that still waits. The "Stop queued" action shows this
    /// count, and shows only while it is more than zero.
    pub(crate) queued: usize,
    /// The name of the oldest job in flight.
    pub(crate) current_label: Option<String>,
    /// The time that the last completion handler recorded. `None` while
    /// the batch runs.
    pub(crate) finished_at: Option<DateTime<Local>>,
}

/// The state of the progress item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DownloadProgressState {
    /// One or more jobs are queued or in flight.
    Running,
    /// Each job completed. The item stays until the operator dismisses it.
    Finished,
}

/// The icon role of the progress item. The composite maps it to a named
/// icon and a named color.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DownloadProgressIconRole {
    /// The batch runs.
    InProgress,
    /// The batch finished with no failure.
    Success,
    /// One or more jobs failed.
    Warning,
}

/// The typed "Dismiss" action of a finished batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DownloadProgressDismissAction {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
}

const DISMISS_ACTION: DownloadProgressDismissAction = DownloadProgressDismissAction {
    id: "download-progress-dismiss",
    label: "Dismiss",
    a11y_label: "Dismiss the download report",
};

/// The typed "Stop queued" action (ADR 0084 Decision 10, packet 027: the
/// operator decision of 2026-10-20). It removes each job that waits. A
/// job in flight keeps writing its file, because a stop during a write
/// leaves a partial file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DownloadProgressStopQueuedAction {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    /// Names the count of the jobs that wait, for example "Stop 6 queued
    /// downloads".
    pub(crate) a11y_label: String,
}

const STOP_QUEUED_ACTION_ID: &str = "download-progress-stop-queued";
const STOP_QUEUED_ACTION_LABEL: &str = "Stop queued";

/// The display facts of the progress item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DownloadProgressDisplay {
    pub(crate) id: &'static str,
    pub(crate) state: DownloadProgressState,
    pub(crate) icon_role: DownloadProgressIconRole,
    /// "Downloading 3 of 9", or "Downloaded 8 of 9, 1 failed. Finished
    /// 14:02".
    pub(crate) title: String,
    /// The name of the oldest job in flight, while the batch runs.
    pub(crate) item_name: Option<String>,
    /// "1 failed", while the batch runs and a job failed.
    pub(crate) failure_text: Option<String>,
    /// One report line for each failed job, with its name and its error.
    pub(crate) failure_lines: Vec<String>,
    pub(crate) a11y_label: String,
    /// Present only while the batch runs and one or more jobs wait.
    pub(crate) stop_queued: Option<DownloadProgressStopQueuedAction>,
    /// Present only when the batch is finished.
    pub(crate) dismiss: Option<DownloadProgressDismissAction>,
}

impl DownloadProgressDisplay {
    /// Projects the progress item of `batch`. `None` when the batch holds
    /// no job.
    #[must_use]
    pub(crate) fn project(batch: &DownloadBatch) -> Option<Self> {
        if batch.total == 0 {
            return None;
        }
        let failed = batch.failures.len();
        let failure_lines = batch
            .failures
            .iter()
            .map(|failure| {
                format!(
                    "The app could not download {}: {}",
                    failure.label, failure.error
                )
            })
            .collect();
        let icon_role = |finished: bool| {
            if failed > 0 {
                DownloadProgressIconRole::Warning
            } else if finished {
                DownloadProgressIconRole::Success
            } else {
                DownloadProgressIconRole::InProgress
            }
        };
        let stopped = batch.stopped;
        let display = match batch.finished_at {
            None => {
                let position = (batch.done + failed + 1).min(batch.total);
                let mut a11y_label = format!("Downloading item {position} of {}", batch.total);
                if let Some(label) = &batch.current_label {
                    a11y_label.push_str(": ");
                    a11y_label.push_str(label);
                }
                a11y_label.push('.');
                if failed > 0 {
                    write!(a11y_label, " {}.", failed_downloads(failed))
                        .expect("writing to a String cannot fail");
                }
                let stop_queued = (batch.queued > 0).then(|| DownloadProgressStopQueuedAction {
                    id: STOP_QUEUED_ACTION_ID,
                    label: STOP_QUEUED_ACTION_LABEL,
                    a11y_label: stop_queued_a11y_label(batch.queued),
                });
                Self {
                    id: "download-progress-item",
                    state: DownloadProgressState::Running,
                    icon_role: icon_role(false),
                    title: format!("Downloading {position} of {}", batch.total),
                    item_name: batch.current_label.clone(),
                    failure_text: (failed > 0).then(|| format!("{failed} failed")),
                    failure_lines,
                    a11y_label,
                    stop_queued,
                    dismiss: None,
                }
            }
            Some(finished_at) => {
                let time = finished_at.format("%H:%M");
                let mut title = format!("Downloaded {} of {}", batch.done, batch.total);
                if failed > 0 {
                    write!(title, ", {failed} failed").expect("writing to a String cannot fail");
                }
                if stopped > 0 {
                    write!(title, ", {stopped} stopped").expect("writing to a String cannot fail");
                }
                write!(title, ". Finished {time}").expect("writing to a String cannot fail");
                let mut a11y_label = format!(
                    "Downloaded {} of {} items. Finished at {time}.",
                    batch.done, batch.total
                );
                if failed > 0 {
                    write!(a11y_label, " {}.", failed_downloads(failed))
                        .expect("writing to a String cannot fail");
                }
                if stopped > 0 {
                    write!(a11y_label, " {}.", stopped_downloads(stopped))
                        .expect("writing to a String cannot fail");
                }
                Self {
                    id: "download-progress-item",
                    state: DownloadProgressState::Finished,
                    icon_role: icon_role(true),
                    title,
                    item_name: None,
                    failure_text: None,
                    failure_lines,
                    a11y_label,
                    stop_queued: None,
                    dismiss: Some(DISMISS_ACTION),
                }
            }
        };
        Some(display)
    }
}

/// "1 download failed" or "2 downloads failed".
fn failed_downloads(failed: usize) -> String {
    if failed == 1 {
        "1 download failed".to_owned()
    } else {
        format!("{failed} downloads failed")
    }
}

/// "1 download stopped" or "2 downloads stopped".
fn stopped_downloads(stopped: usize) -> String {
    if stopped == 1 {
        "1 download stopped".to_owned()
    } else {
        format!("{stopped} downloads stopped")
    }
}

/// "Stop 1 queued download" or "Stop 2 queued downloads".
fn stop_queued_a11y_label(queued: usize) -> String {
    if queued == 1 {
        "Stop 1 queued download".to_owned()
    } else {
        format!("Stop {queued} queued downloads")
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn recorded_time() -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 10, 20, 14, 2, 0)
            .single()
            .expect("the recorded time is not ambiguous")
    }

    fn failure(label: &str) -> DownloadFailure {
        DownloadFailure {
            label: label.to_owned(),
            error: "connection refused".to_owned(),
        }
    }

    /// R22-02 (key part): a Library feed id and its Index GUID give the
    /// same key. The GUID comes first, then the URL, then the feed id.
    #[test]
    fn adr_0084_download_queue_feed_key_prefers_guid_then_url_then_feed_id() {
        let library = FeedKey::new(Some("guid-1"), Some("https://feed.test/a"), Some(7));
        let index = FeedKey::new(Some("guid-1"), None, None);
        assert_eq!(library, index);
        assert_eq!(
            FeedKey::new(None, Some("https://feed.test/a"), Some(7)),
            Some(FeedKey::Url("https://feed.test/a".to_owned()))
        );
        assert_eq!(
            FeedKey::new(Some(" "), None, Some(7)),
            Some(FeedKey::LibraryFeed(7))
        );
        assert_eq!(FeedKey::new(None, None, None), None);
    }

    /// R22-07: a running batch gives its text, its failure text and its
    /// accessibility label. The position is done plus failed plus one.
    #[test]
    fn adr_0084_download_queue_progress_display_running_batch() {
        let batch = DownloadBatch {
            total: 9,
            done: 1,
            failures: vec![failure("Old Song")],
            stopped: 0,
            queued: 0,
            current_label: Some("Face Pale".to_owned()),
            finished_at: None,
        };
        let display = DownloadProgressDisplay::project(&batch).expect("a display");
        assert_eq!(display.state, DownloadProgressState::Running);
        assert_eq!(display.title, "Downloading 3 of 9");
        assert_eq!(display.item_name.as_deref(), Some("Face Pale"));
        assert_eq!(display.failure_text.as_deref(), Some("1 failed"));
        assert_eq!(
            display.failure_lines,
            vec!["The app could not download Old Song: connection refused".to_owned()]
        );
        assert_eq!(
            display.a11y_label,
            "Downloading item 3 of 9: Face Pale. 1 download failed."
        );
        assert_eq!(display.icon_role, DownloadProgressIconRole::Warning);
    }

    /// R22-07: the position is never more than the total.
    #[test]
    fn adr_0084_download_queue_progress_display_position_stays_in_total() {
        let batch = DownloadBatch {
            total: 2,
            done: 2,
            failures: Vec::new(),
            stopped: 0,
            queued: 0,
            current_label: None,
            finished_at: None,
        };
        let display = DownloadProgressDisplay::project(&batch).expect("a display");
        assert_eq!(display.title, "Downloading 2 of 2");
        assert_eq!(display.failure_text, None);
        assert_eq!(display.a11y_label, "Downloading item 2 of 2.");
        assert_eq!(display.icon_role, DownloadProgressIconRole::InProgress);
    }

    /// R22-07: a finished batch gives its recorded finish time. The time
    /// comes only from `finished_at`.
    #[test]
    fn adr_0084_download_queue_progress_display_finished_batch() {
        let batch = DownloadBatch {
            total: 9,
            done: 8,
            failures: vec![failure("Face Pale")],
            stopped: 0,
            queued: 0,
            current_label: None,
            finished_at: Some(recorded_time()),
        };
        let display = DownloadProgressDisplay::project(&batch).expect("a display");
        assert_eq!(display.state, DownloadProgressState::Finished);
        assert_eq!(display.title, "Downloaded 8 of 9, 1 failed. Finished 14:02");
        assert_eq!(display.item_name, None);
        assert_eq!(display.failure_text, None);
        assert_eq!(
            display.a11y_label,
            "Downloaded 8 of 9 items. Finished at 14:02. 1 download failed."
        );

        let clean = DownloadBatch {
            failures: Vec::new(),
            done: 9,
            ..batch
        };
        let display = DownloadProgressDisplay::project(&clean).expect("a display");
        assert_eq!(display.title, "Downloaded 9 of 9. Finished 14:02");
        assert_eq!(display.icon_role, DownloadProgressIconRole::Success);
    }

    /// R22-08: "Dismiss" is absent while the batch runs, and present when
    /// it is finished.
    #[test]
    fn adr_0084_download_queue_progress_display_dismiss_only_when_finished() {
        let running = DownloadBatch {
            total: 1,
            done: 0,
            failures: Vec::new(),
            stopped: 0,
            queued: 0,
            current_label: Some("Face Pale".to_owned()),
            finished_at: None,
        };
        assert_eq!(
            DownloadProgressDisplay::project(&running)
                .expect("a display")
                .dismiss,
            None
        );
        let finished = DownloadBatch {
            done: 1,
            current_label: None,
            finished_at: Some(recorded_time()),
            ..running
        };
        let dismiss = DownloadProgressDisplay::project(&finished)
            .expect("a display")
            .dismiss
            .expect("a finished batch has Dismiss");
        assert_eq!(dismiss.label, "Dismiss");
        assert_eq!(dismiss.a11y_label, "Dismiss the download report");
    }

    #[test]
    fn adr_0084_download_queue_progress_display_empty_batch_gives_none() {
        let batch = DownloadBatch {
            total: 0,
            done: 0,
            failures: Vec::new(),
            stopped: 0,
            queued: 0,
            current_label: None,
            finished_at: None,
        };
        assert_eq!(DownloadProgressDisplay::project(&batch), None);
    }

    /// R27-03: "Stop queued" shows only while the batch runs with one or
    /// more queued jobs. Its accessibility label names the count.
    #[test]
    fn adr_0084_stop_queued_progress_display_shows_the_action_only_while_jobs_wait() {
        let running_with_queue = DownloadBatch {
            total: 9,
            done: 3,
            failures: Vec::new(),
            stopped: 0,
            queued: 6,
            current_label: Some("Face Pale".to_owned()),
            finished_at: None,
        };
        let stop_queued = DownloadProgressDisplay::project(&running_with_queue)
            .expect("a display")
            .stop_queued
            .expect("one or more jobs wait");
        assert_eq!(stop_queued.label, "Stop queued");
        assert_eq!(stop_queued.a11y_label, "Stop 6 queued downloads");

        let running_without_queue = DownloadBatch {
            queued: 0,
            ..running_with_queue.clone()
        };
        assert_eq!(
            DownloadProgressDisplay::project(&running_without_queue)
                .expect("a display")
                .stop_queued,
            None,
            "no job waits"
        );

        let finished = DownloadBatch {
            finished_at: Some(recorded_time()),
            ..running_with_queue
        };
        assert_eq!(
            DownloadProgressDisplay::project(&finished)
                .expect("a display")
                .stop_queued,
            None,
            "the batch is finished"
        );
    }

    /// R27-03: the accessibility label uses the singular word for one
    /// queued job.
    #[test]
    fn adr_0084_stop_queued_progress_display_a11y_label_is_singular_for_one() {
        let batch = DownloadBatch {
            total: 2,
            done: 1,
            failures: Vec::new(),
            stopped: 0,
            queued: 1,
            current_label: Some("Face Pale".to_owned()),
            finished_at: None,
        };
        let stop_queued = DownloadProgressDisplay::project(&batch)
            .expect("a display")
            .stop_queued
            .expect("one job waits");
        assert_eq!(stop_queued.a11y_label, "Stop 1 queued download");
    }

    /// R27-04: the finished text names the stopped count, with and
    /// without failures.
    #[test]
    fn adr_0084_stop_queued_progress_display_finished_title_names_the_stopped_count() {
        let batch = DownloadBatch {
            total: 9,
            done: 3,
            failures: Vec::new(),
            stopped: 6,
            queued: 0,
            current_label: None,
            finished_at: Some(recorded_time()),
        };
        let display = DownloadProgressDisplay::project(&batch).expect("a display");
        assert_eq!(
            display.title,
            "Downloaded 3 of 9, 6 stopped. Finished 14:02"
        );
        assert!(display.a11y_label.contains("6 downloads stopped"));

        let with_failure = DownloadBatch {
            done: 2,
            failures: vec![failure("Face Pale")],
            ..batch
        };
        let display = DownloadProgressDisplay::project(&with_failure).expect("a display");
        assert_eq!(
            display.title,
            "Downloaded 2 of 9, 1 failed, 6 stopped. Finished 14:02"
        );
    }
}
