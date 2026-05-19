//! `MusicBrainz` feed lookup saga actor (ADR 0040).
//!
//! Feed-level `MusicBrainz` lookup is a multi-step workflow: album release
//! search, per-track candidate matching, per-track staging, and fallback
//! recording lookup. The runtime owns that sequence and publishes plain
//! snapshots for the Library screen to reduce into its existing view model.

#![warn(clippy::pedantic)]

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, watch};

use crate::application::command_bus::CommandBus;
use crate::application::commands::metadata::{
    LookupMusicBrainzAlbumReleases, StageMusicBrainzCandidate, StageMusicBrainzTrack,
};
use crate::application::CommandContext;
use crate::db::TrackRow;
use crate::feed_service::StagedMusicBrainzLookup;
use crate::metadata::MusicBrainzLookupResult;
use crate::musicbrainz::{LookupMetadata, MusicBrainzCandidate};

const SAGA_INBOX_CAPACITY: usize = 4;
const FALLBACK_TRACK_PAUSE: Duration = Duration::from_millis(1100);

/// Start message for a feed-level `MusicBrainz` lookup saga.
#[derive(Clone, Debug)]
pub struct StartFeedLookup {
    feed_id: i64,
    feed_title: Option<String>,
    downloadable_tracks: Vec<TrackRow>,
}

impl StartFeedLookup {
    /// Creates a feed-level lookup request.
    #[must_use]
    pub fn new(
        feed_id: i64,
        feed_title: Option<String>,
        downloadable_tracks: Vec<TrackRow>,
    ) -> Self {
        Self {
            feed_id,
            feed_title,
            downloadable_tracks,
        }
    }
}

/// Latest progress snapshot published by the `MusicBrainz` feed saga.
#[derive(Clone, Debug)]
pub enum MusicBrainzFeedSagaState {
    /// No feed lookup has started yet.
    Idle,
    /// Album-level release search is in flight.
    AlbumSearchInFlight { feed_id: i64 },
    /// Album-level release search failed; per-track fallback follows.
    AlbumSearchFailed { feed_id: i64, error: String },
    /// Album-level release search returned no candidates; per-track fallback follows.
    AlbumSearchEmpty { feed_id: i64 },
    /// A track is being staged.
    PerTrackInFlight {
        feed_id: i64,
        progress: usize,
        total: usize,
        track_id: i64,
    },
    /// A track staged successfully.
    TrackDone {
        feed_id: i64,
        progress: usize,
        total: usize,
        track_id: i64,
        edit_count: usize,
        lookup: MusicBrainzLookupResult,
    },
    /// A track failed and was skipped.
    TrackSkipped {
        feed_id: i64,
        progress: usize,
        total: usize,
        track_id: i64,
        reason: String,
    },
    /// The feed lookup saga completed.
    Completed {
        feed_id: i64,
        total_edits: usize,
        processed: usize,
    },
}

/// Caller-side handle to the running feed saga actor.
#[derive(Clone, Debug)]
pub struct MusicBrainzFeedSagaHandle {
    inbox: mpsc::Sender<StartFeedLookup>,
    snapshot: watch::Receiver<MusicBrainzFeedSagaState>,
}

impl MusicBrainzFeedSagaHandle {
    /// Try to enqueue a feed lookup request.
    #[must_use]
    pub fn try_start(&self, request: StartFeedLookup) -> bool {
        self.inbox.try_send(request).is_ok()
    }

    /// Subscribe to progress snapshots.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<MusicBrainzFeedSagaState> {
        self.snapshot.clone()
    }
}

trait MusicBrainzFeedSagaExecutor: Send + Sync + 'static {
    fn lookup_album_releases(
        &self,
        metadata: LookupMetadata,
    ) -> Result<Vec<MusicBrainzCandidate>, String>;

    fn stage_track(&self, track: TrackRow) -> Result<StagedMusicBrainzLookup, String>;

    fn stage_candidate(
        &self,
        track: TrackRow,
        candidate: MusicBrainzCandidate,
    ) -> Result<StagedMusicBrainzLookup, String>;
}

type SharedExecutor = Arc<dyn MusicBrainzFeedSagaExecutor>;

#[derive(Debug)]
struct CommandBusMusicBrainzExecutor {
    command_bus: Arc<CommandBus>,
}

impl CommandBusMusicBrainzExecutor {
    const fn new(command_bus: Arc<CommandBus>) -> Self {
        Self { command_bus }
    }
}

impl MusicBrainzFeedSagaExecutor for CommandBusMusicBrainzExecutor {
    fn lookup_album_releases(
        &self,
        metadata: LookupMetadata,
    ) -> Result<Vec<MusicBrainzCandidate>, String> {
        let outcome = self
            .command_bus
            .execute(
                LookupMusicBrainzAlbumReleases::new(metadata, 3),
                &CommandContext::next(),
            )
            .map_err(|error| format!("{error:#}"))?;
        Ok(outcome.into_parts().0)
    }

    fn stage_track(&self, track: TrackRow) -> Result<StagedMusicBrainzLookup, String> {
        let outcome = self
            .command_bus
            .execute(StageMusicBrainzTrack::new(track), &CommandContext::next())
            .map_err(|error| format!("{error:#}"))?;
        Ok(outcome.into_parts().0)
    }

    fn stage_candidate(
        &self,
        track: TrackRow,
        candidate: MusicBrainzCandidate,
    ) -> Result<StagedMusicBrainzLookup, String> {
        let outcome = self
            .command_bus
            .execute(
                StageMusicBrainzCandidate::new(track, candidate),
                &CommandContext::next(),
            )
            .map_err(|error| format!("{error:#}"))?;
        Ok(outcome.into_parts().0)
    }
}

/// Spawns the `MusicBrainz` feed saga actor on the current tokio runtime.
#[must_use]
pub fn spawn(command_bus: Arc<CommandBus>) -> MusicBrainzFeedSagaHandle {
    spawn_with_executor(Arc::new(CommandBusMusicBrainzExecutor::new(command_bus)))
}

fn spawn_with_executor(executor: SharedExecutor) -> MusicBrainzFeedSagaHandle {
    let (snapshot_tx, snapshot_rx) = watch::channel(MusicBrainzFeedSagaState::Idle);
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<StartFeedLookup>(SAGA_INBOX_CAPACITY);

    tokio::spawn(async move {
        while let Some(request) = inbox_rx.recv().await {
            run_feed_lookup(request, Arc::clone(&executor), |state| {
                let _ = snapshot_tx.send(state);
            })
            .await;
        }
    });

    MusicBrainzFeedSagaHandle {
        inbox: inbox_tx,
        snapshot: snapshot_rx,
    }
}

async fn run_feed_lookup<P>(request: StartFeedLookup, executor: SharedExecutor, mut publish: P)
where
    P: FnMut(MusicBrainzFeedSagaState) + Send,
{
    let total = request.downloadable_tracks.len();
    publish(MusicBrainzFeedSagaState::AlbumSearchInFlight {
        feed_id: request.feed_id,
    });

    let album_metadata = album_lookup_metadata(&request);
    let release_candidates = lookup_album_releases(Arc::clone(&executor), album_metadata).await;
    let candidates = match release_candidates {
        Ok(candidates) => candidates,
        Err(error) => {
            publish(MusicBrainzFeedSagaState::AlbumSearchFailed {
                feed_id: request.feed_id,
                error,
            });
            run_track_staging(
                request.feed_id,
                request.downloadable_tracks,
                total,
                Vec::new(),
                Arc::clone(&executor),
                true,
                &mut publish,
            )
            .await;
            return;
        }
    };

    if candidates.is_empty() {
        publish(MusicBrainzFeedSagaState::AlbumSearchEmpty {
            feed_id: request.feed_id,
        });
        run_track_staging(
            request.feed_id,
            request.downloadable_tracks,
            total,
            candidates,
            Arc::clone(&executor),
            true,
            &mut publish,
        )
        .await;
        return;
    }

    run_track_staging(
        request.feed_id,
        request.downloadable_tracks,
        total,
        candidates,
        executor,
        false,
        &mut publish,
    )
    .await;
}

fn album_lookup_metadata(request: &StartFeedLookup) -> LookupMetadata {
    let first_artist = request
        .downloadable_tracks
        .iter()
        .find_map(|track| track.artist_name.clone());
    LookupMetadata {
        title: None,
        artist: first_artist,
        album: request.feed_title.clone(),
        track_number: None,
        total_tracks: Some(request.downloadable_tracks.len().to_string()),
        duration_secs: None,
        isrc: None,
    }
}

async fn run_track_staging<P>(
    feed_id: i64,
    tracks: Vec<TrackRow>,
    total: usize,
    candidates: Vec<MusicBrainzCandidate>,
    executor: SharedExecutor,
    pause_between_tracks: bool,
    publish: &mut P,
) where
    P: FnMut(MusicBrainzFeedSagaState) + Send,
{
    let mut total_edits = 0usize;
    let mut processed = 0usize;

    for track in tracks {
        let track_id = track.id;
        let progress = processed + 1;
        publish(MusicBrainzFeedSagaState::PerTrackInFlight {
            feed_id,
            progress,
            total,
            track_id,
        });

        let matched = match_candidate_to_track(&candidates, &track).cloned();
        let result = if let Some(candidate) = matched {
            stage_candidate(Arc::clone(&executor), track, candidate).await
        } else {
            stage_track(Arc::clone(&executor), track).await
        };

        match result {
            Ok(staged) => {
                total_edits += staged.edit_count;
                publish(MusicBrainzFeedSagaState::TrackDone {
                    feed_id,
                    progress,
                    total,
                    track_id,
                    edit_count: staged.edit_count,
                    lookup: staged.lookup,
                });
            }
            Err(reason) => publish(MusicBrainzFeedSagaState::TrackSkipped {
                feed_id,
                progress,
                total,
                track_id,
                reason,
            }),
        }
        processed += 1;

        if pause_between_tracks && processed < total {
            tokio::time::sleep(FALLBACK_TRACK_PAUSE).await;
        }
    }

    publish(MusicBrainzFeedSagaState::Completed {
        feed_id,
        total_edits,
        processed,
    });
}

async fn lookup_album_releases(
    executor: SharedExecutor,
    metadata: LookupMetadata,
) -> Result<Vec<MusicBrainzCandidate>, String> {
    run_blocking(move || executor.lookup_album_releases(metadata)).await
}

async fn stage_track(
    executor: SharedExecutor,
    track: TrackRow,
) -> Result<StagedMusicBrainzLookup, String> {
    run_blocking(move || executor.stage_track(track)).await
}

async fn stage_candidate(
    executor: SharedExecutor,
    track: TrackRow,
    candidate: MusicBrainzCandidate,
) -> Result<StagedMusicBrainzLookup, String> {
    run_blocking(move || executor.stage_candidate(track, candidate)).await
}

async fn run_blocking<T>(
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .unwrap_or_else(|error| Err(format!("{error:#}")))
}

fn match_candidate_to_track<'a>(
    candidates: &'a [MusicBrainzCandidate],
    track: &TrackRow,
) -> Option<&'a MusicBrainzCandidate> {
    if let Some(track_num) = track
        .track_number
        .and_then(|track_num| i32::try_from(track_num).ok())
    {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.track_position == Some(track_num))
        {
            return Some(candidate);
        }
    }

    let track_title = track.track_title.as_deref()?;
    let normalized_title = track_title.to_lowercase();
    candidates.iter().max_by_key(|candidate| {
        let candidate_title = candidate
            .track_title
            .as_deref()
            .or(Some(&candidate.title))
            .unwrap_or("")
            .to_lowercase();
        if candidate_title == normalized_title {
            return 1000;
        }
        let title_words: Vec<&str> = normalized_title.split_whitespace().collect();
        let candidate_words: Vec<&str> = candidate_title.split_whitespace().collect();
        title_words
            .iter()
            .filter(|word| candidate_words.contains(word))
            .count()
            * 100
            / title_words.len().max(1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use crate::metadata::MusicBrainzLookupResult;
    use crate::musicbrainz::MusicBrainzLookup;

    #[derive(Default)]
    struct FakeExecutor {
        album_results: Mutex<VecDeque<Result<Vec<MusicBrainzCandidate>, String>>>,
        stage_track_results: Mutex<VecDeque<Result<StagedMusicBrainzLookup, String>>>,
        stage_candidate_results: Mutex<VecDeque<Result<StagedMusicBrainzLookup, String>>>,
    }

    impl FakeExecutor {
        fn with_album_results(
            results: impl IntoIterator<Item = Result<Vec<MusicBrainzCandidate>, String>>,
        ) -> Self {
            Self {
                album_results: Mutex::new(results.into_iter().collect()),
                stage_track_results: Mutex::new(VecDeque::new()),
                stage_candidate_results: Mutex::new(VecDeque::new()),
            }
        }

        fn push_stage_track(&self, result: Result<StagedMusicBrainzLookup, String>) {
            self.stage_track_results
                .lock()
                .expect("lock stage track")
                .push_back(result);
        }

        fn push_stage_candidate(&self, result: Result<StagedMusicBrainzLookup, String>) {
            self.stage_candidate_results
                .lock()
                .expect("lock stage candidate")
                .push_back(result);
        }
    }

    impl MusicBrainzFeedSagaExecutor for FakeExecutor {
        fn lookup_album_releases(
            &self,
            _metadata: LookupMetadata,
        ) -> Result<Vec<MusicBrainzCandidate>, String> {
            self.album_results
                .lock()
                .expect("lock album results")
                .pop_front()
                .expect("album result queued")
        }

        fn stage_track(&self, _track: TrackRow) -> Result<StagedMusicBrainzLookup, String> {
            self.stage_track_results
                .lock()
                .expect("lock stage track results")
                .pop_front()
                .expect("stage track result queued")
        }

        fn stage_candidate(
            &self,
            _track: TrackRow,
            _candidate: MusicBrainzCandidate,
        ) -> Result<StagedMusicBrainzLookup, String> {
            self.stage_candidate_results
                .lock()
                .expect("lock stage candidate results")
                .pop_front()
                .expect("stage candidate result queued")
        }
    }

    #[tokio::test]
    async fn failed_album_search_falls_back_to_per_track_lookup() {
        let executor = Arc::new(FakeExecutor::with_album_results([Err("offline".into())]));
        executor.push_stage_track(Ok(staged(2)));
        let states = run_test_saga(executor, vec![track(10, "First", 1)]).await;

        assert!(matches!(
            states[0],
            MusicBrainzFeedSagaState::AlbumSearchInFlight { feed_id: 7 }
        ));
        assert!(matches!(
            &states[1],
            MusicBrainzFeedSagaState::AlbumSearchFailed { error, .. } if error == "offline"
        ));
        assert!(matches!(
            states[2],
            MusicBrainzFeedSagaState::PerTrackInFlight {
                track_id: 10,
                progress: 1,
                total: 1,
                ..
            }
        ));
        assert!(matches!(
            states[3],
            MusicBrainzFeedSagaState::TrackDone {
                track_id: 10,
                edit_count: 2,
                ..
            }
        ));
        assert!(matches!(
            states[4],
            MusicBrainzFeedSagaState::Completed {
                total_edits: 2,
                processed: 1,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn empty_album_search_falls_back_to_per_track_lookup() {
        let executor = Arc::new(FakeExecutor::with_album_results([Ok(Vec::new())]));
        executor.push_stage_track(Ok(staged(1)));
        let states = run_test_saga(executor, vec![track(11, "Only", 1)]).await;

        assert!(matches!(
            states[1],
            MusicBrainzFeedSagaState::AlbumSearchEmpty { feed_id: 7 }
        ));
        assert!(matches!(
            states[3],
            MusicBrainzFeedSagaState::TrackDone {
                track_id: 11,
                edit_count: 1,
                ..
            }
        ));
        assert!(matches!(
            states[4],
            MusicBrainzFeedSagaState::Completed {
                total_edits: 1,
                processed: 1,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn album_candidate_path_stages_matched_tracks() {
        let candidate = candidate("Episode One", 1);
        let executor = Arc::new(FakeExecutor::with_album_results([Ok(vec![candidate])]));
        executor.push_stage_candidate(Ok(staged(3)));
        let states = run_test_saga(executor, vec![track(12, "Episode One", 1)]).await;

        assert!(matches!(
            states[1],
            MusicBrainzFeedSagaState::PerTrackInFlight {
                track_id: 12,
                progress: 1,
                total: 1,
                ..
            }
        ));
        assert!(matches!(
            states[2],
            MusicBrainzFeedSagaState::TrackDone {
                track_id: 12,
                edit_count: 3,
                ..
            }
        ));
        assert!(matches!(
            states[3],
            MusicBrainzFeedSagaState::Completed {
                total_edits: 3,
                processed: 1,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn per_track_failure_publishes_skipped_status_and_continues() {
        let executor = Arc::new(FakeExecutor::with_album_results([Ok(Vec::new())]));
        executor.push_stage_track(Err("no MusicBrainz results".into()));
        executor.push_stage_track(Ok(staged(1)));
        let states = run_test_saga(executor, vec![track(13, "Miss", 1), track(14, "Hit", 2)]).await;

        assert!(matches!(
            &states[3],
            MusicBrainzFeedSagaState::TrackSkipped {
                track_id: 13,
                reason,
                ..
            } if reason == "no MusicBrainz results"
        ));
        assert!(matches!(
            states[5],
            MusicBrainzFeedSagaState::TrackDone {
                track_id: 14,
                edit_count: 1,
                ..
            }
        ));
        assert!(matches!(
            states[6],
            MusicBrainzFeedSagaState::Completed {
                total_edits: 1,
                processed: 2,
                ..
            }
        ));
    }

    async fn run_test_saga(
        executor: Arc<FakeExecutor>,
        tracks: Vec<TrackRow>,
    ) -> Vec<MusicBrainzFeedSagaState> {
        let mut states = Vec::new();
        run_feed_lookup(
            StartFeedLookup::new(7, Some("Feed".into()), tracks),
            executor,
            |state| states.push(state),
        )
        .await;
        states
    }

    fn track(id: i64, title: &str, track_number: i64) -> TrackRow {
        TrackRow {
            id,
            track_title: Some(title.into()),
            track_number: Some(track_number),
            artist_name: Some("Artist".into()),
            local_path: Some(format!("/tmp/{id}.mp3")),
            ..TrackRow::default()
        }
    }

    fn candidate(title: &str, track_position: i32) -> MusicBrainzCandidate {
        MusicBrainzCandidate {
            recording_id: format!("recording-{track_position}"),
            title: title.into(),
            track_title: Some(title.into()),
            track_position: Some(track_position),
            ..MusicBrainzCandidate::default()
        }
    }

    fn staged(edit_count: usize) -> StagedMusicBrainzLookup {
        StagedMusicBrainzLookup {
            edit_count,
            lookup: MusicBrainzLookupResult {
                lookup: MusicBrainzLookup {
                    query: "test".into(),
                    candidates: Vec::new(),
                },
                image: None,
            },
        }
    }
}
