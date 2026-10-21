//! The download jobs of `LibraryApp` (ADR 0084 Decision 10, packets 022
//! and 027).
//!
//! Each download entry point queues a job here: the action bars, the
//! release page, the track page, the playlist page and the singles group
//! page. [`LibraryApp::pump_downloads`] starts jobs while fewer than
//! [`MAX_IN_FLIGHT`](super::download_queue::MAX_IN_FLIGHT) run. Each job
//! runs through `present_command` on the ADR 0040 command runner. An Index
//! completion sends [`LibraryAppEvent::IndexDownloadFinished`], and
//! `TopApp` updates its open pages in place.
//! [`LibraryApp::stop_queued_downloads`] removes each waiting job. A
//! running job keeps writing its file.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{AnyElement, Context, IntoElement};

use super::download_queue::{
    DownloadJob, DownloadOutcome, DownloadRefusal, JobIdentity, QueuedJob,
};
use super::{LibraryApp, LibraryAppEvent};
use crate::application::commands::download::SubscribeTrack;
use crate::application::commands::feed::SubscribeFeed;
use crate::application::queries::search::FetchIndexFeedDetail;
use crate::application::CommandContext;
use crate::db;
use crate::metadata::TrackContext;
use crate::presentation::present_command;
use crate::subscribe_service::{SubscribeFeedRequest, SubscribeTrackRequest};
use crate::ui::composites::download_status_item;
use crate::view_models::detail_selection::index_track_guid;
use crate::view_models::download_progress::{DownloadProgressDisplay, FeedKey};
use crate::view_models::library::LibraryTrackRowVm;
use crate::view_models::release_page::index_feed_guid;
use crate::view_models::releases_everything::ReleasesDownloadPlan;
use crate::view_models::workspace::{ReleaseKey, TrackKey};
use crate::views::{FeedView, TrackView};

const UNTITLED_RELEASE: &str = "Untitled release";
const UNTITLED_TRACK: &str = "Untitled track";

impl LibraryApp {
    /// Queues a download of each Library release of `feed_ids` (ADR 0084
    /// Decision 10).
    pub(crate) fn download_library_feeds(&mut self, feed_ids: &[i64], cx: &mut Context<Self>) {
        let jobs: Vec<_> = feed_ids
            .iter()
            .map(|&feed_id| Ok(self.library_feed_job(feed_id)))
            .collect();
        self.queue_downloads(jobs, cx);
    }

    /// Queues the jobs of `plan`: the Library jobs first, then one Index
    /// release job with no feed for each `MusicIndex` release (packet 016
    /// Required Change 15). Each job reads its feed in its own queue slot.
    /// The Releases action bar and the singles group page use it (packet
    /// 026 Part B).
    pub(crate) fn download_release_plan(
        &mut self,
        plan: ReleasesDownloadPlan,
        cx: &mut Context<Self>,
    ) {
        self.download_library_feeds(&plan.library_feed_ids, cx);
        let index_jobs = plan
            .index_releases
            .into_iter()
            .map(|(feed_guid, title)| index_release_job_by_guid(feed_guid, title))
            .collect();
        self.queue_downloads(index_jobs, cx);
    }

    /// Queues a download of each Library track of `track_ids` (ADR 0084
    /// Decision 10).
    pub(crate) fn download_library_tracks(&mut self, track_ids: &[i64], cx: &mut Context<Self>) {
        let jobs: Vec<_> = track_ids
            .iter()
            .map(|&track_id| self.library_track_job(track_id))
            .collect();
        self.queue_downloads(jobs, cx);
    }

    /// Queues each job of `jobs`, records each refusal in the batch, and
    /// starts the jobs that can start.
    pub(crate) fn queue_downloads(
        &mut self,
        jobs: Vec<Result<QueuedJob, DownloadRefusal>>,
        cx: &mut Context<Self>,
    ) {
        for job in jobs {
            match job {
                // A dropped job is the same job, or a release job of its
                // key, that is queued or in flight already.
                Ok(job) => {
                    self.download_queue.enqueue(job);
                }
                Err(refusal) => self.download_queue.refuse(refusal, chrono::Local::now()),
            }
        }
        self.pump_downloads(cx);
    }

    /// `true` while a job of the Library track `track_id` is queued or in
    /// flight.
    pub(crate) fn library_track_download_active(&self, track_id: i64) -> bool {
        self.download_queue
            .activity()
            .track_phase(track_id)
            .is_some()
    }

    /// Starts jobs while fewer than three run, then updates the busy state
    /// of each page.
    pub(crate) fn pump_downloads(&mut self, cx: &mut Context<Self>) {
        while let Some(job) = self.download_queue.start_next() {
            self.run_download_job(job, cx);
        }
        self.vm
            .set_download_activity(self.download_queue.activity());
        cx.notify();
    }

    /// Records the result of the job `identity` with the time of this
    /// completion, then starts the next jobs.
    pub(super) fn finish_download(
        &mut self,
        identity: &JobIdentity,
        outcome: DownloadOutcome,
        cx: &mut Context<Self>,
    ) {
        self.download_queue
            .finish(identity, outcome, chrono::Local::now());
        self.pump_downloads(cx);
    }

    /// The report of a removal of the Library release `feed_id` while a
    /// download of that release is queued or in flight (ADR 0084
    /// Decision 10). `None` when the removal can run.
    pub(super) fn release_removal_refusal(&self, feed_id: i64) -> Option<String> {
        let album = self.album_for_detail_by_feed_id(feed_id);
        let feed_guid = album.as_ref().and_then(|album| album.feed_guid.as_deref());
        let key = self.library_feed_key(feed_id, feed_guid);
        let name = album
            .as_ref()
            .map_or(UNTITLED_RELEASE, |album| album.name.as_str());
        self.download_queue.removal_refusal(&key, name)
    }

    /// The progress item of the download batch, when a batch is open.
    pub(crate) fn download_progress(&self) -> Option<DownloadProgressDisplay> {
        DownloadProgressDisplay::project(&self.download_queue.batch()?)
    }

    /// "Dismiss" of a finished batch. A running batch stays.
    pub(crate) fn dismiss_download_report(&mut self, cx: &mut Context<Self>) {
        if self.download_queue.dismiss() {
            cx.notify();
        }
    }

    /// "Stop queued" removes each job that waits. A running job keeps
    /// writing its file, because a stop during a write leaves a partial
    /// file (ADR 0084 Decision 10, packet 027: the operator decision of
    /// 2026-10-20).
    pub(crate) fn stop_queued_downloads(&mut self, cx: &mut Context<Self>) {
        let stopped = self.download_queue.stop_queued(chrono::Local::now());
        if stopped > 0 {
            self.vm
                .set_download_activity(self.download_queue.activity());
            cx.notify();
        }
    }

    /// Renders the progress item, or an empty element when no batch is
    /// open. `TopApp::render` mounts it on each section.
    pub(crate) fn render_download_status(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(display) = self.download_progress() else {
            return gpui::Empty.into_any_element();
        };
        let stop_queued_entity = cx.entity();
        let dismiss_entity = cx.entity();
        download_status_item(
            display,
            move |_, _, cx| {
                stop_queued_entity.update(cx, LibraryApp::stop_queued_downloads);
            },
            move |_, _, cx| {
                dismiss_entity.update(cx, LibraryApp::dismiss_download_report);
            },
        )
        .into_any_element()
    }

    /// The queue key of a Library release: its feed GUID, then its feed
    /// URL, then its Library feed id.
    fn library_feed_key(&self, feed_id: i64, feed_guid: Option<&str>) -> FeedKey {
        let feed_url = self
            .find_album_by_feed_id(feed_id)
            .and_then(|album| album.feed_url.as_deref());
        FeedKey::new(feed_guid, feed_url, Some(feed_id)).unwrap_or(FeedKey::LibraryFeed(feed_id))
    }

    fn library_feed_job(&self, feed_id: i64) -> QueuedJob {
        let album = self.album_for_detail_by_feed_id(feed_id);
        let feed_guid = album.as_ref().and_then(|album| album.feed_guid.clone());
        let label = album.map_or_else(|| UNTITLED_RELEASE.to_owned(), |album| album.name);
        let key = self.library_feed_key(feed_id, feed_guid.as_deref());
        QueuedJob::library_feed(feed_id, key, label)
    }

    fn library_track_job(&self, track_id: i64) -> Result<QueuedJob, DownloadRefusal> {
        let track = match self.conn.lock() {
            Ok(conn) => db::track_row_by_id(&conn, track_id),
            Err(_) => Err(anyhow::anyhow!("The database connection is not available.")),
        };
        let track = match track {
            Ok(Some(track)) => track,
            Ok(None) => {
                return Err(DownloadRefusal {
                    label: format!("track {track_id}"),
                    reason: "The track is no longer stored, so the app cannot download it."
                        .to_owned(),
                })
            }
            Err(error) => {
                return Err(DownloadRefusal {
                    label: format!("track {track_id}"),
                    reason: format!(
                        "The app could not read the track from the database. {error:#}"
                    ),
                })
            }
        };
        let key = self.library_feed_key(track.feed_id, track.feed_guid.as_deref());
        let label = LibraryTrackRowVm::new(&track, None).display_title();
        Ok(QueuedJob::library_track(track_id, key, label))
    }

    fn run_download_job(&mut self, job: QueuedJob, cx: &mut Context<Self>) {
        let QueuedJob { identity, job, .. } = job;
        match job {
            DownloadJob::LibraryFeed { feed_id } => {
                self.run_library_feed_job(identity, feed_id, cx);
            }
            DownloadJob::LibraryTrack { track_id } => {
                self.run_library_track_job(identity, track_id, cx);
            }
            DownloadJob::IndexTrack {
                feed_guid,
                track_guid,
                context: Some(context),
            } => self.run_index_track_job(identity, feed_guid, track_guid, context, cx),
            DownloadJob::IndexTrack {
                feed_guid: Some(feed_guid),
                track_guid,
                context: None,
            } => self.read_index_track_job(identity, feed_guid, track_guid, cx),
            DownloadJob::IndexTrack {
                feed_guid: None,
                context: None,
                ..
            } => self.finish_download(
                &identity,
                DownloadOutcome::Failed {
                    error: "the app has no feed GUID to read the track from MusicIndex.".to_owned(),
                },
                cx,
            ),
            DownloadJob::IndexRelease {
                feed_guid,
                feed_url,
                feed: Some(feed),
            } => self.run_index_release_job(identity, feed_guid, feed_url, *feed, cx),
            DownloadJob::IndexRelease {
                feed_guid: Some(feed_guid),
                feed: None,
                ..
            } => self.read_index_release_job(identity, feed_guid, cx),
            DownloadJob::IndexRelease {
                feed_guid: None,
                feed: None,
                ..
            } => self.finish_download(
                &identity,
                DownloadOutcome::Failed {
                    error: "the app has no feed GUID to read the release from MusicIndex."
                        .to_owned(),
                },
                cx,
            ),
        }
    }

    fn run_index_track_job(
        &mut self,
        identity: JobIdentity,
        feed_guid: Option<String>,
        track_guid: String,
        context: Box<TrackContext>,
        cx: &mut Context<Self>,
    ) {
        let finished_job = DownloadJob::IndexTrack {
            feed_guid,
            track_guid,
            context: Some(context.clone()),
        };
        let failed_job = finished_job.clone();
        let failed_identity = identity.clone();
        let command = SubscribeTrack::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeTrackRequest::SearchTrack {
                track_context: context,
                musicindex_endpoint: self.musicindex_endpoint.clone(),
                mark_feed_subscribed: false,
                return_tag_compare: true,
            },
            "Downloaded track".to_string(),
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, _result, cx| {
                this.finish_index_download(&identity, finished_job, DownloadOutcome::Succeeded, cx);
            },
            move |this, error, cx| {
                let outcome = DownloadOutcome::Failed {
                    error: error.to_string(),
                };
                this.finish_index_download(&failed_identity, failed_job, outcome, cx);
            },
        );
    }

    fn run_index_release_job(
        &mut self,
        identity: JobIdentity,
        feed_guid: Option<String>,
        feed_url: Option<String>,
        feed: crate::api::Feed,
        cx: &mut Context<Self>,
    ) {
        let feed_url = feed_url.or_else(|| feed.feed_url.clone());
        let finished_job = DownloadJob::IndexRelease {
            feed_guid,
            feed_url,
            feed: None,
        };
        let failed_job = finished_job.clone();
        let failed_identity = identity.clone();
        let command = SubscribeFeed::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeFeedRequest {
                feed,
                musicindex_endpoint: self.musicindex_endpoint.clone(),
            },
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, _result, cx| {
                this.finish_index_download(&identity, finished_job, DownloadOutcome::Succeeded, cx);
            },
            move |this, error, cx| {
                let outcome = DownloadOutcome::Failed {
                    error: error.to_string(),
                };
                this.finish_index_download(&failed_identity, failed_job, outcome, cx);
            },
        );
    }

    /// An Index release job with no feed: it reads the feed from
    /// `MusicIndex` in its own slot, then downloads it (responsiveness
    /// item 6: the progress item now shows this read).
    fn read_index_release_job(
        &mut self,
        identity: JobIdentity,
        feed_guid: String,
        cx: &mut Context<Self>,
    ) {
        let failed_identity = identity.clone();
        let failed_job = DownloadJob::IndexRelease {
            feed_guid: Some(feed_guid.clone()),
            feed_url: None,
            feed: None,
        };
        let command =
            FetchIndexFeedDetail::new(self.musicindex_endpoint.clone(), feed_guid.clone());
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, feed, cx| {
                if let Some(title) = feed.title.as_deref() {
                    this.download_queue.rename(&identity, title);
                }
                let feed_url = feed.feed_url.clone();
                let api_feed = api_feed_from_view(&feed);
                this.run_index_release_job(identity, Some(feed_guid), feed_url, api_feed, cx);
            },
            move |this, error, cx| {
                let outcome = DownloadOutcome::Failed {
                    error: format!("the app could not read the release from MusicIndex. {error}"),
                };
                this.finish_index_download(&failed_identity, failed_job, outcome, cx);
            },
        );
    }

    /// An Index track job with no context (ADR 0084 packet 010): it reads
    /// the feed of the track from `MusicIndex` in its own slot, finds the
    /// track by its GUID, then downloads it. The progress item shows the
    /// read, the same as for a release job with no feed.
    fn read_index_track_job(
        &mut self,
        identity: JobIdentity,
        feed_guid: String,
        track_guid: String,
        cx: &mut Context<Self>,
    ) {
        let failed_identity = identity.clone();
        let failed_job = DownloadJob::IndexTrack {
            feed_guid: Some(feed_guid.clone()),
            track_guid: track_guid.clone(),
            context: None,
        };
        let missing_job = failed_job.clone();
        let command =
            FetchIndexFeedDetail::new(self.musicindex_endpoint.clone(), feed_guid.clone());
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, feed, cx| {
                let Some(track) = feed
                    .tracks
                    .iter()
                    .find(|track| index_track_guid(track).as_deref() == Some(track_guid.as_str()))
                else {
                    let outcome = DownloadOutcome::Failed {
                        error: format!(
                            "MusicIndex gave the release {feed_guid} with no track {track_guid}."
                        ),
                    };
                    this.finish_index_download(&identity, missing_job, outcome, cx);
                    return;
                };
                if let Some(title) = track.title.as_deref() {
                    this.download_queue.rename(&identity, title);
                }
                let context = TrackContext::new(
                    api_track_from_view(&feed, track),
                    Some(api_feed_from_view(&feed)),
                );
                this.run_index_track_job(
                    identity,
                    Some(feed_guid),
                    track_guid,
                    Box::new(context),
                    cx,
                );
            },
            move |this, error, cx| {
                let outcome = DownloadOutcome::Failed {
                    error: format!("the app could not read the track from MusicIndex. {error}"),
                };
                this.finish_index_download(&failed_identity, failed_job, outcome, cx);
            },
        );
    }

    /// Records an Index completion, then sends it to `TopApp`, which
    /// updates its open pages in place.
    fn finish_index_download(
        &mut self,
        identity: &JobIdentity,
        job: DownloadJob,
        outcome: DownloadOutcome,
        cx: &mut Context<Self>,
    ) {
        self.finish_download(identity, outcome.clone(), cx);
        cx.emit(LibraryAppEvent::IndexDownloadFinished { job, outcome });
    }
}

/// The download job of an Index release, from its `FeedView` (ADR 0084
/// Decision 10).
///
/// # Errors
///
/// Refuses a release with no feed GUID and no feed URL.
pub(crate) fn index_release_job(feed: &FeedView) -> Result<QueuedJob, DownloadRefusal> {
    QueuedJob::index_release(
        index_feed_guid(feed),
        feed.feed_url.clone(),
        Some(api_feed_from_view(feed)),
        feed.title
            .clone()
            .unwrap_or_else(|| UNTITLED_RELEASE.to_owned()),
    )
}

/// The download job of an Index release that the app reads from
/// `MusicIndex` when the job starts. `label` names the release until the
/// read gives its title.
///
/// # Errors
///
/// Refuses an empty feed GUID.
pub(crate) fn index_release_job_by_guid(
    feed_guid: String,
    label: Option<String>,
) -> Result<QueuedJob, DownloadRefusal> {
    let label = label.unwrap_or_else(|| UNTITLED_RELEASE.to_owned());
    QueuedJob::index_release(Some(feed_guid), None, None, label)
}

/// The download job of an Index track that the search found with no feed
/// (ADR 0084 packet 010 decision D10). The job reads the feed in its own
/// slot. `label` names the track until the read gives its title.
///
/// # Errors
///
/// Refuses a key with no feed GUID or no track GUID.
pub(crate) fn index_track_job_by_key(
    key: &TrackKey,
    label: String,
) -> Result<QueuedJob, DownloadRefusal> {
    let feed_guid = match &key.release {
        Some(ReleaseKey::FeedGuid(feed_guid)) => feed_guid.as_str(),
        Some(ReleaseKey::LibraryFeed(_)) | None => "",
    };
    QueuedJob::index_track_by_guid(feed_guid, &key.item_guid, label)
}

/// The download job of an Index track, from its `FeedView` and its
/// `TrackView` (ADR 0084 Decision 10).
///
/// # Errors
///
/// Refuses a track whose feed has no feed GUID and no feed URL.
pub(crate) fn index_track_job(
    feed: &FeedView,
    track: &TrackView,
) -> Result<QueuedJob, DownloadRefusal> {
    let context = TrackContext::new(
        api_track_from_view(feed, track),
        Some(api_feed_from_view(feed)),
    );
    QueuedJob::index_track(
        index_feed_guid(feed),
        feed.feed_url.as_deref(),
        index_track_guid(track).unwrap_or_default(),
        context,
        track
            .title
            .clone()
            .unwrap_or_else(|| UNTITLED_TRACK.to_owned()),
    )
}

/// The `MusicIndex` feed of an Index `FeedView`, for a download.
pub(crate) fn api_feed_from_view(feed: &FeedView) -> crate::api::Feed {
    crate::api::Feed {
        feed_guid: index_feed_guid(feed),
        title: feed.title.clone(),
        feed_url: feed.feed_url.clone(),
        release_artist: feed.artist.clone(),
        release_kind: feed.release_kind.clone(),
        release_date: feed.release_date,
        publisher_text: feed.publisher_text.clone(),
        language: feed.language.clone(),
        explicit: feed.explicit,
        episode_count: feed.episode_count,
        description: feed.description.clone(),
        image_url: feed.image_url.clone(),
        tracks: Some(
            feed.tracks
                .iter()
                .map(|track| api_track_from_view(feed, track))
                .collect(),
        ),
        payment_routes: Some(feed.payment_routes.clone()),
        ..crate::api::Feed::default()
    }
}

/// The `MusicIndex` track of an Index `TrackView` of `feed`, for a
/// download.
pub(crate) fn api_track_from_view(feed: &FeedView, track: &TrackView) -> crate::api::Track {
    crate::api::Track {
        track_guid: index_track_guid(track),
        feed_guid: track.feed_guid.clone().or_else(|| index_feed_guid(feed)),
        feed_title: track.feed_title.clone().or_else(|| feed.title.clone()),
        title: track.title.clone(),
        duration_secs: track.duration_secs,
        pub_date: track.pub_date,
        track_number: track.track_number,
        explicit: track.explicit,
        description: track.description.clone(),
        enclosure_url: track.audio_url.clone(),
        enclosure_type: track.mime.clone(),
        enclosure_bytes: track.bytes,
        image_url: track.image_url.clone().or_else(|| feed.image_url.clone()),
        // ADR 0075 packet 048: carry the track's own claimed image
        // forward. Never set it from the feed's image (Decision C).
        track_image_url: track.track_image_url.clone(),
        feed_image_url: track.feed_image_url.clone(),
        track_artist: track.artist.clone(),
        release_artist: feed.artist.clone(),
        publisher_text: track.publisher_text.clone(),
        payment_routes: Some(track.payment_routes.clone()),
        ..crate::api::Track::default()
    }
}
