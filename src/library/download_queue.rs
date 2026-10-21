//! The one download queue of the Library and the Index (ADR 0084
//! Decision 10, packet 022).
//!
//! This is a pure scheduler. It holds no GPUI type and does no I/O.
//! `LibraryApp` owns one [`DownloadQueue`], starts each job that
//! [`DownloadQueue::start_next`] gives, and reports each completion
//! through [`DownloadQueue::finish`]. The rules:
//!
//! - At most [`MAX_IN_FLIGHT`] jobs run at the same time.
//! - Two jobs of one [`FeedKey`] never run at the same time.
//! - The jobs start in their order. A job of a busy release waits, and the
//!   jobs after it can start.
//! - A failure records the item name and its error text. The queue
//!   continues.

#![warn(clippy::pedantic)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Local};

use crate::api;
use crate::metadata::TrackContext;
use crate::view_models::download_progress::{
    DownloadActivity, DownloadBatch, DownloadFailure, FeedKey,
};

/// The maximum number of downloads that run at the same time.
pub(crate) const MAX_IN_FLIGHT: usize = 3;

/// One download job.
#[derive(Clone, Debug)]
pub(crate) enum DownloadJob {
    /// A Library release, by its Library feed id.
    LibraryFeed { feed_id: i64 },
    /// A Library track, by its Library track id.
    LibraryTrack { track_id: i64 },
    /// An Index track. `context` holds the track and its feed, built from
    /// the Index views when the job was queued.
    IndexTrack {
        feed_guid: Option<String>,
        track_guid: String,
        context: Box<TrackContext>,
    },
    /// An Index release. With `feed: None`, the job reads the feed from
    /// `MusicIndex` in its own slot first.
    IndexRelease {
        feed_guid: Option<String>,
        feed_url: Option<String>,
        feed: Option<Box<api::Feed>>,
    },
}

impl DownloadJob {
    const fn is_release(&self) -> bool {
        matches!(self, Self::LibraryFeed { .. } | Self::IndexRelease { .. })
    }

    const fn library_track_id(&self) -> Option<i64> {
        match self {
            Self::LibraryTrack { track_id } => Some(*track_id),
            Self::LibraryFeed { .. } | Self::IndexTrack { .. } | Self::IndexRelease { .. } => None,
        }
    }
}

/// The identity of a job. The queue drops a job whose identity is
/// already queued or in flight.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum JobIdentity {
    /// A release job. A Library job and an Index job of one release have
    /// one identity.
    Release(FeedKey),
    /// A Library track job.
    LibraryTrack(i64),
    /// An Index track job.
    IndexTrack {
        feed_key: FeedKey,
        track: IndexTrackIdentity,
    },
}

/// The identity of an Index track in its release (orchestrator correction
/// of 2026-10-20). Two tracks of one release with no track GUID are two
/// jobs.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum IndexTrackIdentity {
    /// The track GUID.
    Guid(String),
    /// The enclosure URL, when the track has no track GUID.
    EnclosureUrl(String),
    /// A unique number, when the track has no track GUID and no enclosure
    /// URL. The queue drops no such job as a duplicate.
    Unique(u64),
}

impl IndexTrackIdentity {
    /// The identity of a track: the track GUID first, then the enclosure
    /// URL, then a new unique number.
    fn new(track_guid: &str, enclosure_url: Option<&str>) -> Self {
        static NEXT_UNIQUE: AtomicU64 = AtomicU64::new(0);
        let present = |value: &str| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        };
        present(track_guid)
            .map(Self::Guid)
            .or_else(|| enclosure_url.and_then(present).map(Self::EnclosureUrl))
            .unwrap_or_else(|| Self::Unique(NEXT_UNIQUE.fetch_add(1, Ordering::Relaxed)))
    }
}

/// A job with its identity, its release key and its item name.
#[derive(Clone, Debug)]
pub(crate) struct QueuedJob {
    pub(crate) identity: JobIdentity,
    pub(crate) feed_key: FeedKey,
    pub(crate) label: String,
    pub(crate) job: DownloadJob,
}

/// A job that the queue refused, with the report for the operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DownloadRefusal {
    pub(crate) label: String,
    pub(crate) reason: String,
}

impl QueuedJob {
    /// A Library release job.
    pub(crate) fn library_feed(feed_id: i64, feed_key: FeedKey, label: String) -> Self {
        Self {
            identity: JobIdentity::Release(feed_key.clone()),
            feed_key,
            label,
            job: DownloadJob::LibraryFeed { feed_id },
        }
    }

    /// A Library track job.
    pub(crate) fn library_track(track_id: i64, feed_key: FeedKey, label: String) -> Self {
        Self {
            identity: JobIdentity::LibraryTrack(track_id),
            feed_key,
            label,
            job: DownloadJob::LibraryTrack { track_id },
        }
    }

    /// An Index track job.
    ///
    /// # Errors
    ///
    /// Refuses a track whose feed has no GUID and no URL, because such a
    /// track has no queue key.
    pub(crate) fn index_track(
        feed_guid: Option<String>,
        feed_url: Option<&str>,
        track_guid: String,
        context: TrackContext,
        label: String,
    ) -> Result<Self, DownloadRefusal> {
        let Some(feed_key) = FeedKey::new(feed_guid.as_deref(), feed_url, None) else {
            return Err(DownloadRefusal {
                reason: format!(
                    "The app did not download {label}: its feed has no feed GUID and no feed URL."
                ),
                label,
            });
        };
        Ok(Self {
            identity: JobIdentity::IndexTrack {
                feed_key: feed_key.clone(),
                track: IndexTrackIdentity::new(&track_guid, context.track.enclosure_url.as_deref()),
            },
            feed_key,
            label,
            job: DownloadJob::IndexTrack {
                feed_guid,
                track_guid,
                context: Box::new(context),
            },
        })
    }

    /// An Index release job. With `feed: None`, the job reads the feed
    /// from `MusicIndex` when it starts.
    ///
    /// # Errors
    ///
    /// Refuses a release with no feed GUID and no feed URL, because such
    /// a release has no queue key.
    pub(crate) fn index_release(
        feed_guid: Option<String>,
        feed_url: Option<String>,
        feed: Option<api::Feed>,
        label: String,
    ) -> Result<Self, DownloadRefusal> {
        let Some(feed_key) = FeedKey::new(feed_guid.as_deref(), feed_url.as_deref(), None) else {
            return Err(DownloadRefusal {
                reason: format!(
                    "The app did not download {label}: the release has no feed GUID and no feed URL."
                ),
                label,
            });
        };
        if feed.is_none() && feed_guid.is_none() {
            return Err(DownloadRefusal {
                reason: format!(
                    "The app did not download {label}: the app needs a feed GUID to read the release from MusicIndex."
                ),
                label,
            });
        }
        Ok(Self {
            identity: JobIdentity::Release(feed_key.clone()),
            feed_key,
            label,
            job: DownloadJob::IndexRelease {
                feed_guid,
                feed_url,
                feed: feed.map(Box::new),
            },
        })
    }
}

/// The result of one job, for [`DownloadQueue::finish`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DownloadOutcome {
    Succeeded,
    Failed { error: String },
}

/// What [`DownloadQueue::enqueue`] did with a job.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EnqueueResult {
    Queued,
    /// The same job, or a release job of its key, is queued or in flight.
    Dropped,
}

/// A job that runs. The runner holds the job data.
#[derive(Clone, Debug)]
struct InFlight {
    identity: JobIdentity,
    feed_key: FeedKey,
    label: String,
    release: bool,
    library_track_id: Option<i64>,
}

/// The counts of the open batch.
#[derive(Clone, Debug, Default)]
struct BatchState {
    total: usize,
    done: usize,
    failures: Vec<DownloadFailure>,
    finished_at: Option<DateTime<Local>>,
}

impl BatchState {
    const fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }
}

/// The download queue (ADR 0084 Decision 10).
#[derive(Debug, Default)]
pub(crate) struct DownloadQueue {
    queued: VecDeque<QueuedJob>,
    /// The jobs that run, oldest first.
    in_flight: Vec<InFlight>,
    batch: Option<BatchState>,
}

impl DownloadQueue {
    /// Adds `job` at the end of the queue. The first job with no running
    /// batch opens a batch. A job queued during a running batch joins it.
    pub(crate) fn enqueue(&mut self, job: QueuedJob) -> EnqueueResult {
        if self.holds_identity(&job.identity)
            || (!job.job.is_release() && self.release_holds_key(&job.feed_key))
        {
            return EnqueueResult::Dropped;
        }
        self.open_batch_slot().total += 1;
        self.queued.push_back(job);
        EnqueueResult::Queued
    }

    /// Records a refused job as a failure of the batch, so the progress
    /// item names it.
    pub(crate) fn refuse(&mut self, refusal: DownloadRefusal, now: DateTime<Local>) {
        let batch = self.open_batch_slot();
        batch.total += 1;
        batch.failures.push(DownloadFailure {
            label: refusal.label,
            error: refusal.reason,
        });
        self.close_batch_when_idle(now);
    }

    /// The first queued job whose key is not in flight. `None` when
    /// [`MAX_IN_FLIGHT`] jobs run. [`Self::start_next`] starts this job.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn next_startable(&self) -> Option<&QueuedJob> {
        self.next_startable_index().map(|index| &self.queued[index])
    }

    /// Moves the next startable job in flight and gives it to the runner.
    pub(crate) fn start_next(&mut self) -> Option<QueuedJob> {
        let index = self.next_startable_index()?;
        let job = self.queued.remove(index)?;
        self.in_flight.push(InFlight {
            identity: job.identity.clone(),
            feed_key: job.feed_key.clone(),
            label: job.label.clone(),
            release: job.job.is_release(),
            library_track_id: job.job.library_track_id(),
        });
        Some(job)
    }

    /// Records the result of the job `identity`. When no job is queued or
    /// in flight after it, the batch is finished at `now`.
    pub(crate) fn finish(
        &mut self,
        identity: &JobIdentity,
        outcome: DownloadOutcome,
        now: DateTime<Local>,
    ) {
        let Some(position) = self
            .in_flight
            .iter()
            .position(|job| &job.identity == identity)
        else {
            return;
        };
        let job = self.in_flight.remove(position);
        if let Some(batch) = self.batch.as_mut() {
            match outcome {
                DownloadOutcome::Succeeded => batch.done += 1,
                DownloadOutcome::Failed { error } => batch.failures.push(DownloadFailure {
                    label: job.label,
                    error,
                }),
            }
        }
        self.close_batch_when_idle(now);
    }

    /// Changes the item name of the job `identity`, for example after an
    /// Index release job reads its title from `MusicIndex`.
    pub(crate) fn rename(&mut self, identity: &JobIdentity, label: &str) {
        if let Some(job) = self
            .in_flight
            .iter_mut()
            .find(|job| &job.identity == identity)
        {
            label.clone_into(&mut job.label);
        }
    }

    /// `true` while a job of `key` is queued or in flight.
    #[must_use]
    pub(crate) fn is_active(&self, key: &FeedKey) -> bool {
        self.in_flight.iter().any(|job| &job.feed_key == key)
            || self.queued.iter().any(|job| &job.feed_key == key)
    }

    /// The report that refuses a removal of the release `release_name`
    /// while a job of its `key` is queued or in flight.
    #[must_use]
    pub(crate) fn removal_refusal(&self, key: &FeedKey, release_name: &str) -> Option<String> {
        self.is_active(key).then(|| {
            format!(
                "The app did not remove {release_name}: a download of this release is queued or running. Remove it after the download ends."
            )
        })
    }

    /// The busy state of each page: the tracks and the keys of each queued
    /// or running job.
    #[must_use]
    pub(crate) fn activity(&self) -> DownloadActivity {
        let mut activity = DownloadActivity::default();
        let queued = self.queued.iter().map(|job| {
            (
                &job.identity,
                &job.feed_key,
                job.job.is_release(),
                job.job.library_track_id(),
                false,
            )
        });
        let in_flight = self.in_flight.iter().map(|job| {
            (
                &job.identity,
                &job.feed_key,
                job.release,
                job.library_track_id,
                true,
            )
        });
        for (identity, key, release, track_id, running) in queued.chain(in_flight) {
            activity.keys.insert(key.clone());
            if running {
                activity.in_flight_keys.insert(key.clone());
            }
            if release {
                activity.release_keys.insert(key.clone());
                if running {
                    activity.in_flight_release_keys.insert(key.clone());
                }
            }
            if let Some(track_id) = track_id {
                activity.tracks.insert(track_id);
                if running {
                    activity.in_flight_tracks.insert(track_id);
                }
            }
            // ADR 0083 task 006, decision D8: an Index track row reads the
            // phase of its own job by the track GUID.
            if let JobIdentity::IndexTrack {
                feed_key,
                track: IndexTrackIdentity::Guid(guid),
            } = identity
            {
                let entry = (feed_key.clone(), guid.clone());
                activity.index_tracks.insert(entry.clone());
                if running {
                    activity.in_flight_index_tracks.insert(entry);
                }
            }
        }
        activity
    }

    /// A snapshot of the batch for the progress item. `None` when no batch
    /// is open.
    #[must_use]
    pub(crate) fn batch(&self) -> Option<DownloadBatch> {
        let batch = self.batch.as_ref()?;
        Some(DownloadBatch {
            total: batch.total,
            done: batch.done,
            failures: batch.failures.clone(),
            current_label: self.in_flight.first().map(|job| job.label.clone()),
            finished_at: batch.finished_at,
        })
    }

    /// Closes a finished batch. A running batch stays. Returns `true` when
    /// the batch closed.
    pub(crate) fn dismiss(&mut self) -> bool {
        if self.batch.as_ref().is_some_and(BatchState::is_finished) {
            self.batch = None;
            return true;
        }
        false
    }

    fn next_startable_index(&self) -> Option<usize> {
        if self.in_flight.len() >= MAX_IN_FLIGHT {
            return None;
        }
        self.queued.iter().position(|job| {
            !self
                .in_flight
                .iter()
                .any(|running| running.feed_key == job.feed_key)
        })
    }

    fn holds_identity(&self, identity: &JobIdentity) -> bool {
        self.in_flight.iter().any(|job| &job.identity == identity)
            || self.queued.iter().any(|job| &job.identity == identity)
    }

    fn release_holds_key(&self, key: &FeedKey) -> bool {
        self.in_flight
            .iter()
            .any(|job| job.release && &job.feed_key == key)
            || self
                .queued
                .iter()
                .any(|job| job.job.is_release() && &job.feed_key == key)
    }

    /// The running batch, or a new batch when none runs.
    fn open_batch_slot(&mut self) -> &mut BatchState {
        if self.batch.as_ref().is_none_or(BatchState::is_finished) {
            self.batch = Some(BatchState::default());
        }
        self.batch.get_or_insert_with(BatchState::default)
    }

    fn close_batch_when_idle(&mut self, now: DateTime<Local>) {
        if !self.queued.is_empty() || !self.in_flight.is_empty() {
            return;
        }
        if let Some(batch) = self.batch.as_mut() {
            batch.finished_at.get_or_insert(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::view_models::download_progress::DownloadPhase;

    fn recorded_time() -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 10, 20, 14, 2, 0)
            .single()
            .expect("the recorded time is not ambiguous")
    }

    fn guid_key(guid: &str) -> FeedKey {
        FeedKey::Guid(guid.to_owned())
    }

    fn feed_job(feed_id: i64, guid: &str) -> QueuedJob {
        QueuedJob::library_feed(feed_id, guid_key(guid), format!("Release {feed_id}"))
    }

    fn track_job(track_id: i64, guid: &str) -> QueuedJob {
        QueuedJob::library_track(track_id, guid_key(guid), format!("Track {track_id}"))
    }

    fn start_all(queue: &mut DownloadQueue) -> Vec<JobIdentity> {
        std::iter::from_fn(|| queue.start_next())
            .map(|job| job.identity)
            .collect()
    }

    /// R22-01: no job starts when three are in flight.
    #[test]
    fn adr_0084_download_queue_r22_01_no_job_starts_with_three_in_flight() {
        let mut queue = DownloadQueue::default();
        for feed_id in 1..=5 {
            assert_eq!(
                queue.enqueue(feed_job(feed_id, &format!("g{feed_id}"))),
                EnqueueResult::Queued
            );
        }
        let started = start_all(&mut queue);
        assert_eq!(started.len(), MAX_IN_FLIGHT);
        assert!(queue.next_startable().is_none());
        assert!(queue.start_next().is_none());
    }

    /// R22-02: two jobs of one key are never in flight together. A Library
    /// feed id and its Index GUID give the same key.
    #[test]
    fn adr_0084_download_queue_r22_02_one_key_never_runs_twice() {
        let mut queue = DownloadQueue::default();
        let library_key =
            FeedKey::new(Some("g1"), Some("https://feed.test/a"), Some(7)).expect("a key");
        queue.enqueue(QueuedJob::library_track(
            70,
            library_key,
            "Track 70".to_owned(),
        ));
        let index = QueuedJob::index_release(
            Some("g1".to_owned()),
            None,
            Some(api::Feed::default()),
            "Release g1".to_owned(),
        )
        .expect("a key");
        assert_eq!(index.feed_key, guid_key("g1"));
        queue.enqueue(index);
        let started = start_all(&mut queue);
        assert_eq!(started, vec![JobIdentity::LibraryTrack(70)]);
        assert!(queue.next_startable().is_none());

        queue.finish(
            &JobIdentity::LibraryTrack(70),
            DownloadOutcome::Succeeded,
            recorded_time(),
        );
        assert_eq!(
            start_all(&mut queue),
            vec![JobIdentity::Release(guid_key("g1"))]
        );
    }

    /// R22-03: the jobs start in their order. A job of a busy key does not
    /// stop a later job of another key.
    #[test]
    fn adr_0084_download_queue_r22_03_busy_key_does_not_stop_later_jobs() {
        let mut queue = DownloadQueue::default();
        queue.enqueue(track_job(1, "a"));
        queue.enqueue(track_job(2, "a"));
        queue.enqueue(track_job(3, "b"));
        queue.enqueue(track_job(4, "c"));
        assert_eq!(
            start_all(&mut queue),
            vec![
                JobIdentity::LibraryTrack(1),
                JobIdentity::LibraryTrack(3),
                JobIdentity::LibraryTrack(4),
            ]
        );
        queue.finish(
            &JobIdentity::LibraryTrack(1),
            DownloadOutcome::Succeeded,
            recorded_time(),
        );
        assert_eq!(start_all(&mut queue), vec![JobIdentity::LibraryTrack(2)]);
    }

    /// R22-04: a job whose identity is queued or in flight is dropped. A
    /// track job is dropped when a release job of its key is queued.
    #[test]
    fn adr_0084_download_queue_r22_04_duplicate_jobs_are_dropped() {
        let mut queue = DownloadQueue::default();
        assert_eq!(queue.enqueue(feed_job(1, "a")), EnqueueResult::Queued);
        assert_eq!(queue.enqueue(feed_job(1, "a")), EnqueueResult::Dropped);
        assert_eq!(queue.enqueue(track_job(10, "a")), EnqueueResult::Dropped);
        assert_eq!(queue.enqueue(track_job(20, "b")), EnqueueResult::Queued);
        start_all(&mut queue);
        assert_eq!(queue.enqueue(track_job(20, "b")), EnqueueResult::Dropped);
        assert_eq!(queue.enqueue(feed_job(1, "a")), EnqueueResult::Dropped);
        assert_eq!(queue.batch().expect("a batch").total, 2);
    }

    /// R22-05: a failure adds to the failed count, and the queue starts
    /// the next job.
    #[test]
    fn adr_0084_download_queue_r22_05_failure_counts_and_queue_continues() {
        let mut queue = DownloadQueue::default();
        for feed_id in 1..=4 {
            queue.enqueue(feed_job(feed_id, &format!("g{feed_id}")));
        }
        start_all(&mut queue);
        queue.finish(
            &JobIdentity::Release(guid_key("g1")),
            DownloadOutcome::Failed {
                error: "connection refused".to_owned(),
            },
            recorded_time(),
        );
        let batch = queue.batch().expect("a batch");
        assert_eq!(
            batch.failures,
            vec![DownloadFailure {
                label: "Release 1".to_owned(),
                error: "connection refused".to_owned(),
            }]
        );
        assert_eq!(batch.finished_at, None);
        assert_eq!(
            start_all(&mut queue),
            vec![JobIdentity::Release(guid_key("g4"))]
        );
    }

    /// R22-06: a job queued during a running batch adds to the total. The
    /// first job after a finished batch opens a new batch.
    #[test]
    fn adr_0084_download_queue_r22_06_batch_grows_then_reopens() {
        let mut queue = DownloadQueue::default();
        queue.enqueue(feed_job(1, "a"));
        start_all(&mut queue);
        queue.enqueue(feed_job(2, "b"));
        assert_eq!(queue.batch().expect("a batch").total, 2);
        start_all(&mut queue);
        for key in ["a", "b"] {
            queue.finish(
                &JobIdentity::Release(guid_key(key)),
                DownloadOutcome::Succeeded,
                recorded_time(),
            );
        }
        let finished = queue.batch().expect("a batch");
        assert_eq!((finished.total, finished.done), (2, 2));
        assert_eq!(finished.finished_at, Some(recorded_time()));

        queue.enqueue(feed_job(3, "c"));
        let reopened = queue.batch().expect("a batch");
        assert_eq!((reopened.total, reopened.done), (1, 0));
        assert_eq!(reopened.finished_at, None);
    }

    /// R22-08: a finished batch stays until "Dismiss". A running batch
    /// does not close.
    #[test]
    fn adr_0084_download_queue_r22_08_finished_batch_stays_until_dismiss() {
        let mut queue = DownloadQueue::default();
        queue.enqueue(feed_job(1, "a"));
        start_all(&mut queue);
        assert!(!queue.dismiss(), "a running batch does not close");
        assert!(queue.batch().is_some());
        queue.finish(
            &JobIdentity::Release(guid_key("a")),
            DownloadOutcome::Succeeded,
            recorded_time(),
        );
        assert!(queue.batch().is_some(), "the finished batch stays");
        assert!(queue.dismiss());
        assert!(queue.batch().is_none());
    }

    /// R22-09: the activity holds each key and each Library track that is
    /// queued or in flight. A job in flight gives `InFlight`, and a job
    /// that only waits gives `Queued` (orchestrator correction of
    /// 2026-10-20).
    #[test]
    fn adr_0084_download_queue_r22_09_activity_holds_queued_and_running_jobs() {
        let mut queue = DownloadQueue::default();
        for feed_id in 1..=3 {
            queue.enqueue(feed_job(feed_id, &format!("g{feed_id}")));
        }
        queue.enqueue(track_job(40, "g4"));
        start_all(&mut queue);
        let activity = queue.activity();
        assert!(activity.is_active(&guid_key("g1")), "a running release");
        assert!(activity.is_active(&guid_key("g4")), "a queued track");
        assert_eq!(
            activity.release_phase(&guid_key("g2")),
            Some(DownloadPhase::InFlight)
        );
        assert_eq!(activity.release_phase(&guid_key("g4")), None);
        assert_eq!(activity.track_phase(40), Some(DownloadPhase::Queued));
        assert_eq!(
            activity.key_phase(&guid_key("g4")),
            Some(DownloadPhase::Queued)
        );
        queue.finish(
            &JobIdentity::Release(guid_key("g1")),
            DownloadOutcome::Succeeded,
            recorded_time(),
        );
        let activity = queue.activity();
        assert!(!activity.is_active(&guid_key("g1")));
        assert_eq!(activity.key_phase(&guid_key("g1")), None);
        start_all(&mut queue);
        let activity = queue.activity();
        assert_eq!(activity.track_phase(40), Some(DownloadPhase::InFlight));
        assert_eq!(
            activity.key_phase(&guid_key("g4")),
            Some(DownloadPhase::InFlight)
        );
    }

    /// R22-10: a removal is refused while its key is queued or in flight.
    #[test]
    fn adr_0084_download_queue_r22_10_removal_refused_while_active() {
        let mut queue = DownloadQueue::default();
        for feed_id in 1..=4 {
            queue.enqueue(feed_job(feed_id, &format!("g{feed_id}")));
        }
        start_all(&mut queue);
        let running = queue
            .removal_refusal(&guid_key("g1"), "Face Pale")
            .expect("a running release is refused");
        assert!(running.contains("Face Pale"));
        assert!(running.contains("queued or running"));
        assert!(queue.removal_refusal(&guid_key("g4"), "Queued").is_some());
        assert_eq!(queue.removal_refusal(&guid_key("g9"), "Other"), None);
    }

    /// An Index track with no feed GUID and no feed URL has no key. The
    /// refusal names the track, and the batch counts it as a failure.
    #[test]
    fn adr_0084_download_queue_index_track_without_key_is_refused() {
        let context = TrackContext::new(api::Track::default(), None);
        let refusal =
            QueuedJob::index_track(None, None, "t1".to_owned(), context, "Face Pale".to_owned())
                .expect_err("no key");
        assert!(refusal.reason.contains("Face Pale"));
        let mut queue = DownloadQueue::default();
        queue.refuse(refusal, recorded_time());
        let batch = queue.batch().expect("a batch");
        assert_eq!((batch.total, batch.failures.len()), (1, 1));
        assert_eq!(batch.finished_at, Some(recorded_time()));
    }

    fn guidless_index_track(enclosure_url: Option<&str>, label: &str) -> QueuedJob {
        let track = api::Track {
            enclosure_url: enclosure_url.map(str::to_owned),
            ..api::Track::default()
        };
        QueuedJob::index_track(
            Some("g1".to_owned()),
            None,
            String::new(),
            TrackContext::new(track, None),
            label.to_owned(),
        )
        .expect("a key")
    }

    /// Orchestrator correction of 2026-10-20: two tracks of one release
    /// with no track GUID are two jobs. The enclosure URL is the identity.
    /// With no enclosure URL, each job has a unique identity.
    #[test]
    fn adr_0084_download_queue_guidless_tracks_of_one_release_are_two_jobs() {
        let mut queue = DownloadQueue::default();
        let first = guidless_index_track(Some("https://example.test/a.mp3"), "A");
        let second = guidless_index_track(Some("https://example.test/b.mp3"), "B");
        assert_ne!(first.identity, second.identity);
        assert_eq!(queue.enqueue(first), EnqueueResult::Queued);
        assert_eq!(queue.enqueue(second), EnqueueResult::Queued);
        let same = guidless_index_track(Some("https://example.test/a.mp3"), "A");
        assert_eq!(queue.enqueue(same), EnqueueResult::Dropped);

        let third = guidless_index_track(None, "C");
        let fourth = guidless_index_track(None, "D");
        assert_ne!(third.identity, fourth.identity);
        assert_eq!(queue.enqueue(third), EnqueueResult::Queued);
        assert_eq!(queue.enqueue(fourth), EnqueueResult::Queued);
        assert_eq!(queue.batch().expect("a batch").total, 4);
    }

    fn guid_index_track(track_guid: &str) -> QueuedJob {
        QueuedJob::index_track(
            Some("g1".to_owned()),
            None,
            track_guid.to_owned(),
            TrackContext::new(api::Track::default(), None),
            format!("Track {track_guid}"),
        )
        .expect("a key")
    }

    /// R83-6-06 (ADR 0083 Decision 6, task 006): the activity gives the
    /// phase of each Index track job by its release key and its track
    /// GUID. A track with no job has no phase.
    #[test]
    fn adr_0083_row_actions_index_track_phase_follows_the_queue() {
        let mut queue = DownloadQueue::default();
        queue.enqueue(guid_index_track("t1"));
        queue.enqueue(guid_index_track("t2"));
        let activity = queue.activity();
        assert_eq!(
            activity.index_track_phase(&guid_key("g1"), "t1"),
            Some(DownloadPhase::Queued)
        );
        assert_eq!(activity.index_track_phase(&guid_key("g1"), "t3"), None);

        // One job of the key runs at a time, so only "t1" starts.
        start_all(&mut queue);
        let activity = queue.activity();
        assert_eq!(
            activity.index_track_phase(&guid_key("g1"), "t1"),
            Some(DownloadPhase::InFlight)
        );
        assert_eq!(
            activity.index_track_phase(&guid_key("g1"), "t2"),
            Some(DownloadPhase::Queued)
        );
        assert_eq!(activity.release_phase(&guid_key("g1")), None);
    }
}
