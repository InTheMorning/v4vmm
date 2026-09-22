//! Library local query family.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::{CommandError, ObservedQueryFailure};
use crate::application::library_removal::{self, LibraryRemovalIntent, LibraryRemovalPlan};
use crate::db::TrackRow;
use crate::feed_service::{self, track_row_to_track_context};
use crate::metadata::{source_text_missing, TagCompareResult, TrackContext};
use crate::provider_observation::{
    ObservationCommandFailure, ObservationReceipt, ObservationStorageError,
    ObservationWriteFailure, ProviderObservationRecorder, ProviderReadError,
};
use crate::subscribe_service;
use crate::view_models::library::{
    AlbumNode, ArtistNode, LibraryTrackRowVm, LibraryTree, LibraryViewModel,
};
use crate::views::{FeedMetadataFacts, FeedView, LocalIdentityFacts};
use crate::{db, library_service};

type SharedConnection = Arc<Mutex<Connection>>;

/// Loaded library tree and source row count.
#[derive(Clone, Debug)]
pub(crate) struct LibraryTracksTree {
    pub(crate) count: usize,
    pub(crate) tree: LibraryTree,
}

/// Hydrated album identity and metadata facts.
#[derive(Clone, Debug)]
pub(crate) struct AlbumIdentityHydration {
    pub(crate) identity_facts: LocalIdentityFacts,
    pub(crate) metadata_facts: FeedMetadataFacts,
    pub(crate) description: Option<String>,
    pub(crate) observation_receipts: Vec<ObservationReceipt>,
}

/// Library track tag comparison with its resolved source context.
#[derive(Clone, Debug)]
pub(crate) struct LibraryTrackCompare {
    pub(crate) tag_compare: TagCompareResult,
    pub(crate) track_context: TrackContext,
}

/// Local track inspector payload plus an optional artwork URL.
#[derive(Clone, Debug)]
pub(crate) struct LocalTrackContextResult {
    pub(crate) context: TrackContext,
    pub(crate) image_url: Option<String>,
}

/// Loads local library tracks into the sidebar tree.
#[derive(Clone, Debug)]
pub(crate) struct LoadLibraryTracksTree {
    conn: SharedConnection,
}

impl LoadLibraryTracksTree {
    /// Creates a library tree load query command.
    #[must_use]
    pub(crate) const fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl ApplicationCommand for LoadLibraryTracksTree {
    type Output = LibraryTracksTree;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        let rows = library_service::library_tracks(&conn).map_err(|error| query_error(&error))?;
        let count = rows.len();
        let tree = build_tree(&rows, &conn);
        Ok(CommandOutcome::without_events(LibraryTracksTree {
            count,
            tree,
        }))
    }
}

/// ADR 0040: reads the Settings cache list away from the render thread.
#[derive(Clone, Debug)]
pub(crate) struct LoadCachedTracksTree {
    conn: SharedConnection,
}

impl LoadCachedTracksTree {
    pub(crate) const fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl ApplicationCommand for LoadCachedTracksTree {
    type Output = LibraryTracksTree;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        let rows = library_service::cached_tracks(&conn).map_err(|error| query_error(&error))?;
        Ok(CommandOutcome::without_events(LibraryTracksTree {
            count: rows.len(),
            tree: build_tree(&rows, &conn),
        }))
    }
}

/// Fetches remote track context with local hydrated metadata fallback.
#[derive(Clone, Debug)]
pub(crate) struct FetchLibraryTrackContext {
    conn: SharedConnection,
    track: TrackRow,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
}

impl FetchLibraryTrackContext {
    /// Creates a library track context query command.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        track: TrackRow,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
    ) -> Self {
        Self {
            conn,
            track,
            musicindex_endpoint: musicindex_endpoint.into(),
        }
    }
}

impl ApplicationCommand for FetchLibraryTrackContext {
    type Output = TrackContext;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        fetch_library_track_context_with_local_fallback(
            &self.conn,
            &self.track,
            &self.musicindex_endpoint,
        )
        .map_err(|error| query_error(&error))
        .map(CommandOutcome::without_events)
    }
}

/// Fetches local track context for a parked Discover inspector.
#[derive(Clone, Debug)]
pub(crate) struct FetchLocalTrackContext {
    conn: SharedConnection,
    track_id: i64,
}

impl FetchLocalTrackContext {
    /// Creates a local track inspector query command.
    #[must_use]
    pub(crate) const fn new(conn: SharedConnection, track_id: i64) -> Self {
        Self { conn, track_id }
    }
}

impl ApplicationCommand for FetchLocalTrackContext {
    type Output = LocalTrackContextResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        fetch_local_track_context(&self.conn, self.track_id)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

/// Hydrates album identity facts from MusicIndex.
#[derive(Clone, Debug)]
pub(crate) struct HydrateAlbumIdentity {
    conn: SharedConnection,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    feed_id: i64,
    feed_guid: String,
}

impl HydrateAlbumIdentity {
    /// Creates an album identity hydration query command.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        feed_id: i64,
        feed_guid: impl Into<String>,
    ) -> Self {
        Self {
            conn,
            musicindex_endpoint: musicindex_endpoint.into(),
            feed_id,
            feed_guid: feed_guid.into(),
        }
    }
}

impl ApplicationCommand for HydrateAlbumIdentity {
    type Output = AlbumIdentityHydration;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        hydrate_album_identity_facts(
            self.conn,
            &self.musicindex_endpoint,
            self.feed_id,
            &self.feed_guid,
        )
        .map(CommandOutcome::without_events)
    }
}

/// Compares one downloaded library track against its source metadata.
#[derive(Clone, Debug)]
pub(crate) struct CompareLibraryTrack {
    conn: SharedConnection,
    track: TrackRow,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    music_dir: PathBuf,
}

impl CompareLibraryTrack {
    /// Creates a library track comparison query command.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        track: TrackRow,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        music_dir: PathBuf,
    ) -> Self {
        Self {
            conn,
            track,
            musicindex_endpoint: musicindex_endpoint.into(),
            music_dir,
        }
    }
}

impl ApplicationCommand for CompareLibraryTrack {
    type Output = LibraryTrackCompare;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        compare_library_track(
            &self.conn,
            &self.track,
            &self.musicindex_endpoint,
            &self.music_dir,
        )
        .map(CommandOutcome::without_events)
    }
}

impl ApplicationQueryService {
    /// Counts playlists that currently reference a local track.
    ///
    /// # Errors
    ///
    /// Returns an error when playlist membership state cannot be read.
    pub fn playlist_reference_count_for_track(
        &self,
        conn: &Connection,
        track_id: i64,
    ) -> Result<i64, CommandError> {
        library_service::playlist_reference_count_for_track(conn, track_id)
            .map_err(|error| query_error(&error))
    }

    /// Counts in-library feed tracks that are present in one or more playlists.
    ///
    /// # Errors
    ///
    /// Returns an error when playlist membership state cannot be read.
    pub fn playlist_referenced_library_track_count_for_feed(
        &self,
        conn: &Connection,
        feed_id: i64,
    ) -> Result<i64, CommandError> {
        library_service::playlist_referenced_library_track_count_for_feed(conn, feed_id)
            .map_err(|error| query_error(&error))
    }

    /// Resolves a library-removal intent to its canonical local target.
    ///
    /// # Errors
    ///
    /// Returns an error when the target cannot be resolved or playlist impact
    /// cannot be queried.
    pub fn library_removal_plan(
        &self,
        conn: &Connection,
        intent: &LibraryRemovalIntent,
    ) -> Result<LibraryRemovalPlan, CommandError> {
        library_removal::plan_library_removal(conn, intent).map_err(|error| query_error(&error))
    }
}

pub(crate) fn build_tree(tracks: &[TrackRow], conn: &Connection) -> LibraryTree {
    let mut artist_map: BTreeMap<String, BTreeMap<String, Vec<TrackRow>>> = BTreeMap::new();
    for track in tracks {
        let row_vm = LibraryTrackRowVm::new(track, None);
        let artist = row_vm.display_artist();
        let album = row_vm.display_album();
        artist_map
            .entry(artist)
            .or_default()
            .entry(album)
            .or_default()
            .push(track.clone());
    }

    let subscribed_feeds: BTreeMap<i64, db::FeedRow> = db::subscribed_feeds(conn)
        .unwrap_or_default()
        .into_iter()
        .map(|feed| (feed.id, feed))
        .collect();
    let mut feed_url_cache: BTreeMap<i64, Option<String>> = BTreeMap::new();
    let mut feed_language_cache: BTreeMap<i64, Option<String>> = BTreeMap::new();
    let artists = artist_map
        .into_iter()
        .map(|(artist_name, album_map)| {
            let albums = album_map
                .into_iter()
                .map(|(album_name, mut tracks)| {
                    tracks.sort_by_key(|track| track.track_number);
                    let feed_id = tracks.first().map(|t| t.feed_id);
                    let feed_guid = tracks.first().and_then(|t| t.feed_guid.clone());
                    let feed_url = feed_id.and_then(|fid| {
                        subscribed_feeds.get(&fid).map_or_else(
                            || {
                                feed_url_cache
                                    .entry(fid)
                                    .or_insert_with(|| db::feed_url_by_id(conn, fid).ok().flatten())
                                    .clone()
                            },
                            |feed| Some(feed.feed_url.clone()),
                        )
                    });
                    let description = feed_id.and_then(|fid| {
                        subscribed_feeds.get(&fid).and_then(|feed| {
                            LibraryViewModel::display_description_text(feed.description.as_deref())
                                .map(str::to_owned)
                        })
                    });
                    let language = feed_id.and_then(|fid| {
                        subscribed_feeds.get(&fid).map_or_else(
                            || {
                                feed_language_cache
                                    .entry(fid)
                                    .or_insert_with(|| {
                                        db::feed_language_by_id(conn, fid).ok().flatten()
                                    })
                                    .clone()
                            },
                            |feed| feed.language.clone(),
                        )
                    });
                    let image_href = tracks
                        .iter()
                        .find_map(|t| t.album_image_href.clone())
                        .or_else(|| tracks.iter().find_map(|t| t.track_image_href.clone()));
                    AlbumNode {
                        name: album_name,
                        feed_id,
                        feed_guid,
                        feed_url,
                        language,
                        description,
                        image_href,
                        identity_facts: feed_id
                            .and_then(|fid| crate::local_identity::feed_facts(conn, fid).ok())
                            .unwrap_or_default(),
                        metadata_facts: Box::new(
                            feed_id
                                .and_then(|fid| crate::local_metadata::feed_facts(conn, fid).ok())
                                .unwrap_or_default(),
                        ),
                        tracks,
                    }
                })
                .collect();
            ArtistNode {
                name: artist_name,
                albums,
            }
        })
        .collect();

    LibraryTree { artists }
}

pub(crate) fn fetch_library_track_context_with_local_fallback(
    conn: &SharedConnection,
    track: &TrackRow,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
) -> anyhow::Result<TrackContext> {
    let local = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))
        .and_then(|db| {
            let context = feed_service::track_row_to_track_context_with_local_identity(&db, track)?;
            let request = feed_service::local_provider_request(&db, track, &context)?;
            Ok((context, request))
        });
    let (local_context, local_request) = match local {
        Ok((context, request)) => (Ok(context), request),
        Err(error) => (Err(error), None),
    };
    let recorder =
        Arc::new(crate::provider_observation::ProviderObservationRecorder::new(Arc::clone(conn)));
    let mut detail_receipts = Vec::new();
    let result = match feed_service::fetch_library_track_context_with_recorder(
        track,
        musicindex_endpoint,
        Some(Arc::clone(&recorder)),
        crate::application::request_reuse::RefreshIntent::Normal,
        &mut detail_receipts,
    ) {
        Ok(mut remote_context) => {
            if let Ok(local_context) = local_context {
                apply_local_track_metadata_defaults(&mut remote_context, &local_context);
            }
            Ok(remote_context)
        }
        Err(error)
            if error.is::<crate::provider_observation::ObservationWriteFailure>()
                || error.is::<crate::provider_observation::ObservationStorageError>() =>
        {
            Err(error)
        }
        Err(_) => local_context,
    };
    // `detail_receipts` carries the track and feed receipts (packet 018
    // R18B-07, R18B-11); `recorder` still holds only the RSS receipt, since
    // `fetch_library_track_context_with_recorder` already drained the rest
    // for `detail_receipts` (see its own documentation).
    let mut receipts = detail_receipts;
    receipts.extend(recorder.take_receipts());
    assemble_provider_context(conn, result, local_request.as_ref(), receipts)
}

fn assemble_provider_context(
    conn: &SharedConnection,
    result: anyhow::Result<TrackContext>,
    local_request: Option<&crate::provider_observation::ProviderRequestSpec>,
    receipts: Vec<crate::provider_observation::ObservationReceipt>,
) -> anyhow::Result<TrackContext> {
    use crate::provider_observation::{
        ObservationCommandFailure, ObservationWriteFailure, ProviderReadError,
    };
    let request = result
        .as_ref()
        .ok()
        .and_then(subscribe_service::rss_request_spec);
    let capsule = result
        .as_ref()
        .err()
        .and_then(|error| error.downcast_ref::<ObservationWriteFailure>())
        .map(|failure| Arc::new(failure.clone()));
    let failed_request = capsule
        .as_ref()
        .filter(|failure| {
            failure.token.spec.provider == crate::provider_observation::ProviderKind::Rss
        })
        .map(|failure| failure.token.spec.as_ref());
    let provider_state = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))
        .and_then(|db| {
            db::provider_observations::read_track_provider_state(
                &db,
                request.as_ref().or(failed_request).or(local_request),
            )
        });
    let read_error = provider_state
        .as_ref()
        .err()
        .map(|_| ProviderReadError::Storage);
    match (result, provider_state) {
        (Ok(mut context), Ok(state)) => {
            context.provider_state = state;
            context.observation_receipts.extend(receipts);
            Ok(context)
        }
        (result, _) => Err(ObservationCommandFailure {
            write_failure: capsule,
            storage_error: result
                .as_ref()
                .err()
                .and_then(|error| {
                    error.downcast_ref::<crate::provider_observation::ObservationStorageError>()
                })
                .copied(),
            read_error,
            receipts: receipts.into(),
        }
        .into()),
    }
}

pub(crate) fn apply_local_track_metadata_defaults(remote: &mut TrackContext, local: &TrackContext) {
    if source_text_missing(remote.track.publisher_text.as_deref()) {
        remote
            .track
            .publisher_text
            .clone_from(&local.track.publisher_text);
    }
    if source_text_missing(remote.track.description.as_deref()) {
        remote
            .track
            .description
            .clone_from(&local.track.description);
    }
    if remote.track.pub_date.is_none() {
        remote.track.pub_date = local.track.pub_date;
    }
    if remote.track.explicit.is_none() {
        remote.track.explicit = local.track.explicit;
    }
}

fn hydrate_album_identity_facts(
    conn: SharedConnection,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    feed_id: i64,
    feed_guid: &str,
) -> Result<AlbumIdentityHydration, CommandError> {
    use crate::application::request_reuse::{self, RefreshIntent, RequestKey, SharedFetchError};
    let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&conn)));
    let owner = request_reuse::shared();
    let provider_identity = musicindex_endpoint
        .require()
        .map(str::to_owned)
        .unwrap_or_default();
    let key = RequestKey::feed(
        provider_identity,
        feed_guid,
        crate::application::request_profiles::LIBRARY_ALBUM_HYDRATION_FEED.include(),
    );
    // Holds the winning request's receipts as soon as the fetch succeeds, so
    // a later, ordinary failure in this function (for example local
    // persistence) does not drop them. `assemble_observed_query` folds them
    // in on either outcome.
    let owner_receipts = std::cell::RefCell::new(Vec::new());
    let result = (|| {
        let client = crate::api::Client::new_with_base_url(musicindex_endpoint.clone())
            .with_observation_recorder(Some(Arc::clone(&recorder)));
        // ADR 0075 section 6, packet 018 Part A: the owner shares this feed
        // request with a concurrent duplicate. The winning caller drains its
        // own recorder right after the fetch, and every caller — the winner
        // and a caller that joins it — receives that one request's exact
        // receipts (R18A-05). No caller records a second observation.
        let (fetch_result, generation) =
            owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, {
                let recorder = Arc::clone(&recorder);
                move || {
                    let feed = client.fetch_feed_with_profile(
                        feed_guid,
                        &crate::application::request_profiles::LIBRARY_ALBUM_HYDRATION_FEED,
                    )?;
                    Ok((feed, recorder.take_receipts()))
                }
            });
        let (feed, receipts) = fetch_result.map_err(SharedFetchError::into_anyhow)?;
        *owner_receipts.borrow_mut() = receipts;
        let description = FeedView::from_api(feed.clone()).description;
        let mut db = conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        // Packet 018 R18B-01/R18B-07: a retained response returns without a
        // request and names the observation that already produced it. This
        // local write cascade only belongs to a genuinely new response;
        // repeating it for a reused one would write the same facts again
        // for no new evidence (the packet's measurement target: zero row
        // changes on a repeated, reused hydration).
        if generation != crate::application::request_reuse::REUSED_GENERATION {
            if description.is_some() {
                db::set_feed_description(&db, feed_id, description.as_deref())?;
            }
            crate::identity_ingest::persist_musicindex_feed(&mut db, feed_id, &feed)?;
        }
        let identity_facts = crate::local_identity::feed_facts(&db, feed_id)?;
        let metadata_facts = crate::local_metadata::feed_facts(&db, feed_id)?;
        Ok(AlbumIdentityHydration {
            identity_facts,
            metadata_facts,
            description,
            observation_receipts: Vec::new(),
        })
    })();
    assemble_observed_query(
        &recorder,
        result,
        owner_receipts.into_inner(),
        |hydration, receipts| {
            hydration.observation_receipts.extend(receipts);
        },
    )
}

fn compare_library_track(
    conn: &SharedConnection,
    track: &TrackRow,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    music_dir: &Path,
) -> Result<LibraryTrackCompare, CommandError> {
    let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(conn)));
    // `fetch_library_track_context_with_recorder` drains its own track and
    // feed receipts into an out-parameter, not into `recorder` (packet 018
    // R18B-07, R18B-11): see its documentation. Holding them here, outside
    // the closure below, keeps them from being dropped if a later step in
    // this same operation fails; `assemble_observed_query` folds them in on
    // either outcome, alongside whatever `recorder` still holds (the RSS
    // receipt).
    let detail_receipts = std::cell::RefCell::new(Vec::new());
    let result = (|| {
        let path = track
            .local_path
            .as_ref()
            .map(|path| path.resolve(music_dir))
            .ok_or_else(|| anyhow::anyhow!("library track has no local file"))?;
        // ADR 0075 section 6, packet 018: a manual comparison wants a fresh
        // value, so it always sends its own request. It never joins an
        // active passive read, and it never reuses a retained response
        // (R18A-09, R18A-10).
        let context = match feed_service::fetch_library_track_context_with_recorder(
            track,
            musicindex_endpoint,
            Some(Arc::clone(&recorder)),
            crate::application::request_reuse::RefreshIntent::Explicit,
            &mut detail_receipts.borrow_mut(),
        ) {
            Ok(context) => Ok(context),
            Err(error) if observation_storage_failure(&error).is_some() => Err(error),
            Err(_) => Ok(track_row_to_track_context(track)),
        };
        let local_request = conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))
            .and_then(|db| {
                feed_service::local_provider_request(&db, track, &track_row_to_track_context(track))
            });
        let context = match local_request {
            Ok(request) => assemble_provider_context(conn, context, request.as_ref(), Vec::new())?,
            Err(_) => {
                let mut failure = context
                    .as_ref()
                    .err()
                    .and_then(observation_storage_failure)
                    .unwrap_or(ObservationCommandFailure {
                        write_failure: None,
                        storage_error: None,
                        read_error: None,
                        receipts: Arc::from([]),
                    });
                failure.read_error = Some(ProviderReadError::Storage);
                return Err(failure.into());
            }
        };
        let tag_compare = subscribe_service::compare_downloaded_track_path(&path, &context)?;
        Ok(LibraryTrackCompare {
            tag_compare,
            track_context: context,
        })
    })();
    assemble_observed_query(
        &recorder,
        result,
        detail_receipts.into_inner(),
        |comparison, receipts| {
            comparison
                .track_context
                .observation_receipts
                .extend(receipts);
        },
    )
}

/// ADR 0075 drains receipts once after every fallible Library reader operation.
///
/// `extra_receipts` carries receipts a packet 018 owner call already drained
/// from `recorder` earlier in the same operation, to share with a joining
/// caller (R18A-05). They are folded in on either outcome, because an
/// ordinary later failure — for example a local persistence error — must
/// not drop the receipt of a request that already succeeded.
fn assemble_observed_query<T>(
    recorder: &ProviderObservationRecorder,
    result: anyhow::Result<T>,
    extra_receipts: Vec<ObservationReceipt>,
    attach: impl FnOnce(&mut T, Vec<ObservationReceipt>),
) -> Result<T, CommandError> {
    let mut receipts = extra_receipts;
    receipts.extend(recorder.take_receipts());
    match result {
        Ok(mut value) => {
            attach(&mut value, receipts);
            Ok(value)
        }
        Err(error) => {
            if let Some(mut failure) = observation_storage_failure(&error) {
                let mut committed = failure.receipts.to_vec();
                committed.extend(receipts);
                failure.receipts = committed.into();
                Err(CommandError::ObservationWriteFailure(Arc::new(failure)))
            } else {
                Err(CommandError::ObservedQueryFailure(Arc::new(
                    ObservedQueryFailure::new(format!("{error:#}"), receipts),
                )))
            }
        }
    }
}

fn observation_storage_failure(error: &anyhow::Error) -> Option<ObservationCommandFailure> {
    if let Some(failure) = error.downcast_ref::<ObservationCommandFailure>() {
        return Some(failure.clone());
    }
    let write_failure = error
        .downcast_ref::<ObservationWriteFailure>()
        .cloned()
        .map(Arc::new);
    let storage_error = error.downcast_ref::<ObservationStorageError>().copied();
    (write_failure.is_some() || storage_error.is_some()).then_some(ObservationCommandFailure {
        write_failure,
        storage_error,
        receipts: Arc::from([]),
        read_error: None,
    })
}

fn fetch_local_track_context(
    conn: &SharedConnection,
    track_id: i64,
) -> anyhow::Result<LocalTrackContextResult> {
    let db = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
    let Some(track) = library_service::track_row_by_id(&db, track_id)? else {
        anyhow::bail!("local track not found: {track_id}");
    };
    let mut context = feed_service::track_row_to_track_context_with_local_identity(&db, &track)?;
    context.provider_state = feed_service::local_provider_request(&db, &track, &context)
        .and_then(|request| {
            db::provider_observations::read_track_provider_state(&db, request.as_ref())
        })
        .map_err(|_| crate::provider_observation::ObservationCommandFailure {
            write_failure: None,
            storage_error: None,
            receipts: Arc::from([]),
            read_error: Some(crate::provider_observation::ProviderReadError::Storage),
        })?;
    let image_url = context
        .track
        .image_url
        .as_deref()
        .and_then(nonempty_url)
        .map(str::to_string);
    Ok(LocalTrackContextResult { context, image_url })
}

fn nonempty_url(url: &str) -> Option<&str> {
    let trimmed = url.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn poisoned_lock() -> CommandError {
    CommandError::Query("database lock poisoned".into())
}

fn query_error(error: &anyhow::Error) -> CommandError {
    if let Some(failure) =
        error.downcast_ref::<crate::provider_observation::ObservationCommandFailure>()
    {
        return CommandError::ObservationWriteFailure(std::sync::Arc::new(failure.clone()));
    }
    CommandError::Query(format!("{error:#}"))
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    struct Fixture {
        endpoint: crate::config::MusicIndexEndpoint,
        address: String,
        conn: SharedConnection,
        tracks: Vec<TrackRow>,
        mode: Arc<AtomicUsize>,
        requests: Arc<Mutex<Vec<String>>>,
        commits: Arc<AtomicUsize>,
        held_response: Arc<Mutex<Option<(TcpStream, String)>>>,
        stop: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl Fixture {
        fn start() -> Self {
            // Packet 018 Part B: the shared owner is one process-wide value
            // (P18-6), so this and every other test share it. A fresh
            // ephemeral port makes each fixture's `RequestKey` distinct
            // (the key carries the endpoint), so tests never collide on a
            // retained entry; nothing here clears the shared owner, which
            // would race a concurrently running test's own state.
            let conn = Connection::open_in_memory().unwrap();
            db::upgrades::create_fixture(&conn, 12).unwrap();
            conn.execute("INSERT INTO feeds(feed_url,feed_guid,title) VALUES('http://fixture.invalid/feed','f1','Local feed')",[]).unwrap();
            for guid in ["t1", "t2"] {
                conn.execute("INSERT INTO tracks(feed_id,item_guid,track_title,is_in_library) VALUES(1,?1,'Local title',1)",[guid]).unwrap();
            }
            let tracks = library_service::library_tracks(&conn).unwrap();
            let commits = Arc::new(AtomicUsize::new(0));
            let counts = Arc::clone(&commits);
            conn.commit_hook(Some(move || {
                counts.fetch_add(1, Ordering::SeqCst);
                false
            }))
            .unwrap();
            let conn = Arc::new(Mutex::new(conn));
            let mode = Arc::new(AtomicUsize::new(0));
            let requests = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let held_response = Arc::new(Mutex::new(None));
            let held = Arc::clone(&held_response);
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap().to_string();
            let base = format!("http://{address}");
            let db = Arc::clone(&conn);
            let selected = Arc::clone(&mode);
            let received = Arc::clone(&requests);
            let stopped = Arc::clone(&stop);
            let url = base.clone();
            let worker = std::thread::spawn(move || {
                while !stopped.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut bytes = Vec::new();
                            let mut buffer = [0; 4096];
                            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                                match stream.read(&mut buffer) {
                                    Ok(0) | Err(_) => break,
                                    Ok(n) => bytes.extend_from_slice(&buffer[..n]),
                                }
                            }
                            let request = String::from_utf8_lossy(&bytes);
                            let Some(path) = request
                                .lines()
                                .next()
                                .and_then(|l| l.split_whitespace().nth(1))
                            else {
                                continue;
                            };
                            received.lock().unwrap().push(path.into());
                            let mode = selected.load(Ordering::SeqCst);
                            if mode != 9 {
                                // Unobserved compatibility requests have no slots.
                                let database = db.lock().unwrap();
                                let pending:i64=database.query_row("SELECT count(*) FROM metadata_request_slots WHERE state='pending' AND (resource_id IN (SELECT id FROM metadata_resources WHERE request_uri=?1))",[format!("{url}{path}")],|r|r.get(0)).unwrap();
                                assert_eq!(pending, 1, "generation must commit before HTTP");
                            }
                            if mode == 8 || (matches!(mode, 10 | 21) && path == "/feed.xml") {
                                db.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_observation AFTER INSERT ON metadata_observations BEGIN SELECT RAISE(ABORT,'fixture response rejection'); END").unwrap();
                            }
                            if mode == 21 && path == "/feed.xml" {
                                db.lock()
                                    .unwrap()
                                    .authorizer(Some(
                                        |context: rusqlite::hooks::AuthContext<'_>| {
                                            if matches!(
                                                context.action,
                                                rusqlite::hooks::AuthAction::Read {
                                                    table_name: "feeds",
                                                    ..
                                                }
                                            ) {
                                                rusqlite::hooks::Authorization::Deny
                                            } else {
                                                rusqlite::hooks::Authorization::Allow
                                            }
                                        },
                                    ))
                                    .unwrap();
                            }
                            if mode == 11 && path.starts_with("/v1/feeds/f1?") {
                                db.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_next_allocation BEFORE UPDATE ON metadata_generation BEGIN SELECT RAISE(ABORT,'fixture later allocation'); END").unwrap();
                            }
                            if mode == 15 && path == "/feed.xml" {
                                let committed = Arc::new(AtomicBool::new(false));
                                let flag = Arc::clone(&committed);
                                let database = db.lock().unwrap();
                                database
                                    .commit_hook(Some(move || {
                                        flag.store(true, Ordering::SeqCst);
                                        false
                                    }))
                                    .unwrap();
                                database
                                    .authorizer(Some(
                                        move |context: rusqlite::hooks::AuthContext<'_>| {
                                            if committed.load(Ordering::SeqCst)
                                                && matches!(
                                                    context.action,
                                                    rusqlite::hooks::AuthAction::Read {
                                                        table_name: "metadata_request_slots",
                                                        ..
                                                    }
                                                )
                                            {
                                                rusqlite::hooks::Authorization::Deny
                                            } else {
                                                rusqlite::hooks::Authorization::Allow
                                            }
                                        },
                                    ))
                                    .unwrap();
                            }
                            if mode == 17 {
                                db.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_legacy BEFORE INSERT ON entity_identity_links BEGIN SELECT RAISE(ABORT,'fixture legacy rejection'); END").unwrap();
                            }
                            if mode == 18 {
                                db.lock()
                                    .unwrap()
                                    .authorizer(Some(
                                        |context: rusqlite::hooks::AuthContext<'_>| {
                                            if matches!(
                                                context.action,
                                                rusqlite::hooks::AuthAction::Read {
                                                    table_name: "entity_identity_ids",
                                                    ..
                                                }
                                            ) {
                                                rusqlite::hooks::Authorization::Deny
                                            } else {
                                                rusqlite::hooks::Authorization::Allow
                                            }
                                        },
                                    ))
                                    .unwrap();
                            }
                            let (status, body) = response(&url, path, mode);
                            if mode == 20 && received.lock().unwrap().len() == 1 {
                                *held.lock().unwrap() = Some((stream, format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())));
                                continue;
                            }
                            write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2))
                        }
                        Err(e) => panic!("fixture listener: {e}"),
                    }
                }
            });
            Self {
                endpoint: base.into(),
                address,
                conn,
                tracks,
                mode,
                requests,
                commits,
                held_response,
                stop,
                worker: Some(worker),
            }
        }
        /// Selects the fixture's next response mode. Packet 018 P18-3
        /// retains a parsed RSS document by feed URL, and every fixture
        /// mode shares the same feed URL, so this also clears that
        /// retained document: each mode's request-count and content
        /// expectations below are about that mode's own response, not
        /// about a still-fresh document an earlier mode's call retained.
        fn set_mode(&self, mode: usize) {
            self.mode.store(mode, Ordering::SeqCst);
            crate::rss::invalidate_feed_document(&format!("http://{}/feed.xml", self.address));
        }
        fn load(&self, index: usize) -> Result<TrackContext, CommandError> {
            FetchLibraryTrackContext::new(
                Arc::clone(&self.conn),
                self.tracks[index].clone(),
                self.endpoint.clone(),
            )
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
        }
        fn row_counts(&self) -> Vec<i64> {
            let conn = self.conn.lock().unwrap();
            [
                "metadata_bodies",
                "metadata_observations",
                "metadata_coverage",
                "metadata_facts",
            ]
            .iter()
            .map(|table| {
                conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                    .unwrap()
            })
            .collect()
        }
        fn compare(&self, root: &Path) -> Result<LibraryTrackCompare, CommandError> {
            let mut track = self.tracks[0].clone();
            track.local_path = Some(crate::library_path::LibraryRelativePath::for_test(
                "track.mp3",
            ));
            CompareLibraryTrack::new(
                Arc::clone(&self.conn),
                track,
                self.endpoint.clone(),
                root.to_owned(),
            )
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0)
        }
        fn hydrate(&self) -> Result<AlbumIdentityHydration, CommandError> {
            HydrateAlbumIdentity::new(Arc::clone(&self.conn), self.endpoint.clone(), 1, "f1")
                .execute(&CommandContext::next())
                .map(|outcome| outcome.into_parts().0)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(&self.address);
            self.worker.take().unwrap().join().unwrap();
        }
    }
    fn response(base: &str, path: &str, mode: usize) -> (&'static str, String) {
        let path = path.split('?').next().unwrap();
        if mode == 1 || (mode == 2 && path.contains("/tracks/") && path.contains("/feeds/")) {
            return (
                "503 Service Unavailable",
                "{\"error\":\"retained failure\"}".into(),
            );
        }
        if path == "/feed.xml" {
            if mode == 14 {
                return ("503 Service Unavailable", "retained RSS failure".into());
            }
            if matches!(mode, 12 | 13 | 15) {
                let txt = if mode == 13 {
                    ""
                } else {
                    "<podcast:txt purpose=\"npub\">npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqujme</podcast:txt>"
                };
                return ("200 OK",format!("<rss xmlns:podcast=\"https://podcastindex.org/namespace/1.0\"><channel><podcast:guid>f1</podcast:guid>{txt}<item><guid>t1</guid>{txt}</item></channel></rss>"));
            }
            return (
                "200 OK",
                if mode == 5 {
                    "<rss><broken>".into()
                } else {
                    format!("<rss xmlns:podcast=\"https://podcastindex.org/namespace/1.0\"><channel><title>RSS feed</title><podcast:guid>f1</podcast:guid><podcast:txt purpose=\"other\">rejected-original</podcast:txt><item><guid>{}</guid><title>RSS title</title><description>RSS description</description><podcast:txt purpose=\"npub\">bad-original</podcast:txt></item><item><guid>t2</guid><title>Second</title></item></channel></rss>",if mode==6 {"other"}else{"t1"})
                },
            );
        }
        if path.contains("/tracks/") {
            if mode == 22 {
                return ("200 OK", json!({"data": {
                    "track_guid":"t1", "feed_guid":"f1", "feed_url":format!("{base}/feed.xml"),
                    "title":"Payment track",
                    "source_contributors":[{"entity_type":"track","entity_id":"t1","name":"Payee","role":"performer","source":"rss"}],
                    "payment_routes":[{"recipient_name":"Payee","split":100.0,"route_type":"lightning"}]
                }}).to_string());
            }
            if mode == 3 {
                return ("200 OK", "{broken JSON".into());
            }
            if mode == 4 {
                return (
                    "200 OK",
                    "{\"data\":{\"title\":\"first\",\"title\":\"second\"}}".into(),
                );
            }
            if mode == 7 {
                return("200 OK",format!("{{\"data\":{{\"track_guid\":\"t1\",\"feed_guid\":\"f1\",\"feed_url\":\"{base}/feed.xml\",\"unknown\":1e999}}}}"));
            }
            return("200 OK",json!({"data":{"track_guid":path.rsplit('/').next(),"feed_guid":"f1","feed_url":format!("{base}/feed.xml"),"title":"...","description":"original Index","source_links":[],"source_ids":null,"source_contributors":[{"entity_type":"track","entity_id":"t1","name":"Artist","role_norm":"performer","source":"rss","future":"original"}]}}).to_string());
        }
        if matches!(mode, 16..=18) {
            return ("200 OK", json!({"unknown_envelope":"retained", "data": {
                "feed_guid":"f1", "feed_url":format!("{base}/feed.xml"), "title":"Hydrated feed",
                "description":"Hydrated description", "language":"en", "release_kind":"album",
                "source_links":[{"entity_type":"feed","entity_id":"f1","link_type":"website","url":"https://example.test/feed","source":"rss","unknown_claim":"retained"}],
                "source_ids":[], "source_contributors":[]
            }}).to_string());
        }
        ("200 OK",json!({"data":{"feed_guid":"f1","feed_url":format!("{base}/feed.xml"),"title":"Index feed","source_links":[],"source_ids":[],"source_contributors":[]}}).to_string())
    }

    fn comparison_audio() -> tempfile::TempDir {
        use id3::TagLike;
        let directory = tempfile::tempdir().unwrap();
        let mut file = std::fs::File::create(directory.path().join("track.mp3")).unwrap();
        let mut tag = id3::Tag::new();
        tag.set_title("Embedded title");
        tag.set_artist("Embedded artist");
        tag.set_track(7);
        tag.write_to(&mut file, id3::Version::Id3v24).unwrap();
        directory
    }

    fn assert_comparison_equal(actual: &TagCompareResult, expected: &TagCompareResult) {
        assert_eq!(format!("{:?}", actual.rows), format!("{:?}", expected.rows));
        assert_eq!(
            serde_json::to_value(&actual.contributors).unwrap(),
            serde_json::to_value(&expected.contributors).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&actual.value_routes).unwrap(),
            serde_json::to_value(&expected.value_routes).unwrap()
        );
        assert_eq!(actual.path, expected.path);
        assert_eq!(
            format!("{:?}", actual.id3_fields),
            format!("{:?}", expected.id3_fields)
        );
    }

    #[test]
    fn adr_0075_library_observation_comparison_requests_fallback_and_tags_stay_equal() {
        let directory = comparison_audio();
        let path = directory.path().join("track.mp3");
        let original = std::fs::read(&path).unwrap();
        let fixture = Fixture::start();
        let command = CompareLibraryTrack::new(
            Arc::clone(&fixture.conn),
            fixture.tracks[0].clone(),
            fixture.endpoint.clone(),
            directory.path().to_owned(),
        );
        let error = command.execute(&CommandContext::next()).unwrap_err();
        assert!(matches!(error, CommandError::ObservedQueryFailure(_)));
        assert_eq!(
            error.to_string(),
            CommandError::Query("library track has no local file".into()).to_string()
        );
        assert!(fixture.requests.lock().unwrap().is_empty());
        assert_eq!(fixture.commits.load(Ordering::SeqCst), 0);
        for (mode, paths) in [
            (
                0,
                vec!["/v1/feeds/f1/tracks/t1", "/v1/feeds/f1", "/feed.xml"],
            ),
            (
                2,
                vec![
                    "/v1/feeds/f1/tracks/t1",
                    "/v1/tracks/t1",
                    "/v1/feeds/f1",
                    "/feed.xml",
                ],
            ),
            (
                1,
                vec!["/v1/feeds/f1/tracks/t1", "/v1/tracks/t1", "/v1/feeds/f1"],
            ),
        ] {
            fixture.set_mode(mode);
            fixture.requests.lock().unwrap().clear();
            let result = fixture.compare(directory.path()).unwrap();
            assert_eq!(result.track_context.observation_receipts.len(), paths.len());
            let expected: Vec<_> = paths.iter().map(|path| if *path == "/feed.xml" { (*path).into() } else {
                format!("{path}?include=source_links%2Csource_ids%2Csource_release_claims%2Csource_contributors%2Cpayment_routes")
            }).collect();
            assert_eq!(*fixture.requests.lock().unwrap(), expected);
            let mut local_track = fixture.tracks[0].clone();
            local_track.local_path = Some(crate::library_path::LibraryRelativePath::for_test(
                "track.mp3",
            ));
            let expected_context = if mode == 1 {
                track_row_to_track_context(&local_track)
            } else {
                fixture.set_mode(9);
                feed_service::fetch_library_track_context(&local_track, &fixture.endpoint).unwrap()
            };
            assert_eq!(
                serde_json::to_value((&result.track_context.track, &result.track_context.feed))
                    .unwrap(),
                serde_json::to_value((&expected_context.track, &expected_context.feed)).unwrap()
            );
            assert_comparison_equal(
                &result.tag_compare,
                &subscribe_service::compare_downloaded_track_path(&path, &expected_context)
                    .unwrap(),
            );
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
        fixture.set_mode(0);
        fixture.requests.lock().unwrap().clear();
        let mut unscoped = fixture.tracks[0].clone();
        unscoped.feed_guid = None;
        unscoped.local_path = Some(crate::library_path::LibraryRelativePath::for_test(
            "track.mp3",
        ));
        let result = CompareLibraryTrack::new(
            Arc::clone(&fixture.conn),
            unscoped,
            fixture.endpoint.clone(),
            directory.path().to_owned(),
        )
        .execute(&CommandContext::next())
        .unwrap()
        .into_parts()
        .0;
        assert_eq!(result.track_context.observation_receipts.len(), 3);
        assert!(fixture.requests.lock().unwrap()[0].starts_with("/v1/tracks/t1?"));
    }

    #[test]
    fn adr_0075_library_observation_hydration_repetition_counts_and_reopened_evidence() {
        let fixture = Fixture::start();
        fixture.set_mode(16);
        let mutations = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let observed = Arc::clone(&mutations);
        fixture
            .conn
            .lock()
            .unwrap()
            .update_hook(Some(
                move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                    *observed.lock().unwrap().entry(table.into()).or_default() += 1;
                },
            ))
            .unwrap();
        for repetition in [false, true] {
            mutations.lock().unwrap().clear();
            fixture.commits.store(0, Ordering::SeqCst);
            let hydration = fixture.hydrate().unwrap();
            assert_eq!(hydration.observation_receipts.len(), 1);
            assert_eq!(
                hydration.description.as_deref(),
                Some("Hydrated description")
            );
            assert_eq!(
                hydration.identity_facts.source_links[0]
                    .entity_type
                    .as_deref(),
                Some("feed")
            );
            assert_eq!(hydration.metadata_facts.language.as_deref(), Some("en"));
            let mutations = mutations.lock().unwrap();
            let legacy: usize = mutations
                .iter()
                .filter(|(table, _)| !table.starts_with("metadata_"))
                .map(|(_, count)| count)
                .sum();
            let observation: usize = mutations
                .iter()
                .filter(|(table, _)| table.starts_with("metadata_"))
                .map(|(_, count)| count)
                .sum();
            // Packet 018 R18B-01/R18B-07: the second, repeated hydration is
            // a retained-response reuse (well inside its 15-minute window).
            // It sends no request, records no new observation, and writes
            // no legacy row again: the packet's measurement target for
            // "Repeated Library album hydration" is 0 Index requests and 0
            // row changes, matched here at 0 legacy rows.
            assert_eq!(legacy, if repetition { 0 } else { 5 });
            assert_eq!(
                fixture
                    .conn
                    .lock()
                    .unwrap()
                    .query_row("SELECT count(*) FROM metadata_snapshots", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            println!("ADR0075_LIBRARY_HYDRATION repetition={repetition} legacy_rows={legacy} observation_rows={observation} snapshot_rows=0 transactions={} tables={mutations:?}", fixture.commits.load(Ordering::SeqCst));
        }
        // Only the first pass reached the network; the second reused its
        // retained response (R18B-01).
        assert_eq!(*fixture.requests.lock().unwrap(), vec!["/v1/feeds/f1?include=source_links%2Csource_ids%2Csource_release_claims%2Csource_contributors"; 1]);
        fixture.set_mode(1);
        // This call means a genuinely new attempt, so it must reach the
        // fixture's now-failing response rather than reuse the retained
        // success from above (P18-7's own mechanism, used here to keep the
        // test's own premise intact under packet 018 Part B's reuse).
        crate::application::request_reuse::shared()
            .invalidate_feed(fixture.endpoint.require().unwrap(), "f1");
        let CommandError::ObservedQueryFailure(failure) = fixture.hydrate().unwrap_err() else {
            panic!("ordinary failure required")
        };
        assert_eq!(failure.receipts().len(), 1);
        assert_eq!(
            failure.receipts()[0].outcome,
            crate::provider_observation::ObservationOutcome::Failed
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("retained.sqlite");
        {
            let mut target = Connection::open(&path).unwrap();
            let source = fixture.conn.lock().unwrap();
            rusqlite::backup::Backup::new(&source, &mut target)
                .unwrap()
                .run_to_completion(16, Duration::from_millis(1), None)
                .unwrap();
        }
        let reopened = Connection::open(path).unwrap();
        let body: String = reopened.query_row("SELECT CAST(bytes AS TEXT) FROM metadata_bodies WHERE CAST(bytes AS TEXT) LIKE '%unknown_envelope%'", [], |row| row.get(0)).unwrap();
        assert!(body.contains("unknown_claim"));
        assert!(body.contains("\"entity_type\":\"feed\""));
        assert_eq!(
            reopened
                .query_row(
                    "SELECT count(*) FROM metadata_observations WHERE outcome='failed'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            reopened
                .query_row(
                    "SELECT occurrence_count FROM metadata_observations WHERE outcome='success'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1,
            "packet 018 Part B: the retained-response reuse on the second pass recorded no second observation"
        );
    }

    #[test]
    fn adr_0075_library_observation_ordinary_failures_keep_receipts_and_exact_causes() {
        let fixture = Fixture::start();
        let directory = tempfile::tempdir().unwrap();
        let cause = subscribe_service::compare_downloaded_track_path(
            &directory.path().join("track.mp3"),
            &track_row_to_track_context(&fixture.tracks[0]),
        )
        .unwrap_err();
        let error = fixture.compare(directory.path()).unwrap_err();
        let CommandError::ObservedQueryFailure(failure) = &error else {
            panic!("ordinary failure required")
        };
        assert_eq!(failure.receipts().len(), 3);
        assert_eq!(failure.message(), format!("{cause:#}"));
        assert_eq!(
            error.to_string(),
            CommandError::Query(format!("{cause:#}")).to_string()
        );
        assert!(!format!("{error:?}").contains(&fixture.address));
        assert!(!format!("{error:?}").contains("track.mp3"));
        assert_eq!(error.clone(), error);
        for mode in [17, 18] {
            let fixture = Fixture::start();
            fixture.set_mode(mode);
            let error = fixture.hydrate().unwrap_err();
            let CommandError::ObservedQueryFailure(failure) = &error else {
                panic!("ordinary legacy failure required")
            };
            assert_eq!(failure.receipts().len(), 1);
            let db = fixture.conn.lock().unwrap();
            assert_eq!(
                db.query_row("SELECT description FROM feeds WHERE id=1", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
                "Hydrated description"
            );
            assert_eq!(
                db.query_row("SELECT count(*) FROM metadata_observations", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            if mode == 17 {
                assert!(failure.message().contains("fixture legacy rejection"));
            } else {
                assert!(failure.message().contains("prohibited"));
                assert_eq!(
                    db.query_row("SELECT count(*) FROM entity_identity_links", [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    1
                );
            }
        }
    }
    #[test]
    fn adr_0075_library_observation_storage_failures_stop_roots_and_keep_retry_input() {
        let directory = comparison_audio();
        for hydration in [false, true] {
            let fixture = Fixture::start();
            fixture.conn.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_allocation BEFORE UPDATE ON metadata_generation BEGIN SELECT RAISE(ABORT,'allocation rejected'); END").unwrap();
            let execute = || {
                if hydration {
                    fixture.hydrate().map(|_| ())
                } else {
                    fixture.compare(directory.path()).map(|_| ())
                }
            };
            let CommandError::ObservationWriteFailure(failure) = execute().unwrap_err() else {
                panic!("storage failure required")
            };
            assert_eq!(
                failure.storage_error,
                Some(ObservationStorageError::RequestAllocation)
            );
            assert!(fixture.requests.lock().unwrap().is_empty());
            fixture
                .conn
                .lock()
                .unwrap()
                .execute_batch("DROP TRIGGER reject_allocation")
                .unwrap();
            fixture.set_mode(8);
            let CommandError::ObservationWriteFailure(failure) = execute().unwrap_err() else {
                panic!("storage failure required")
            };
            assert!(failure.receipts.is_empty());
            assert_eq!(fixture.requests.lock().unwrap().len(), 1);
            assert_eq!(fixture.row_counts(), vec![0, 0, 0, 0]);
            let capsule = failure.write_failure.as_ref().unwrap();
            assert_eq!(
                capsule.retry,
                crate::provider_observation::StorageRetry::VerifiedRollback
            );
            let db = fixture.conn.lock().unwrap();
            assert_eq!(
                db.query_row("SELECT description FROM feeds WHERE id=1", [], |row| row
                    .get::<_, Option<
                    String,
                >>(
                    0
                ))
                .unwrap(),
                None
            );
            db.execute_batch("DROP TRIGGER reject_observation").unwrap();
            let receipt = db::provider_observations::record_provider_observation(
                &db,
                capsule.token.clone(),
                Arc::clone(&capsule.observation),
            )
            .unwrap();
            let changes = db.total_changes();
            let commits = fixture.commits.load(Ordering::SeqCst);
            let replay = db::provider_observations::record_provider_observation(
                &db,
                capsule.token.clone(),
                Arc::clone(&capsule.observation),
            )
            .unwrap();
            assert_eq!(replay, receipt);
            assert_eq!(db.total_changes(), changes);
            assert_eq!(fixture.commits.load(Ordering::SeqCst), commits);
            let mut changed = (*capsule.observation).clone();
            changed.finished_at_us += 1;
            assert!(db::provider_observations::record_provider_observation(
                &db,
                capsule.token.clone(),
                Arc::new(changed)
            )
            .is_err());
            assert_eq!(db.total_changes(), changes);
            assert_eq!(fixture.requests.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn adr_0075_library_observation_comparison_later_failures_preserve_receipts_and_read_errors() {
        let directory = comparison_audio();
        for mode in [10, 11, 15, 21] {
            let fixture = Fixture::start();
            fixture.set_mode(mode);
            let CommandError::ObservationWriteFailure(failure) =
                fixture.compare(directory.path()).unwrap_err()
            else {
                panic!("storage failure required")
            };
            assert_eq!(failure.receipts.len(), if mode == 15 { 3 } else { 2 });
            assert_eq!(failure.write_failure.is_some(), matches!(mode, 10 | 21));
            assert_eq!(failure.storage_error.is_some(), mode == 11);
            assert_eq!(
                failure.read_error,
                matches!(mode, 15 | 21).then_some(ProviderReadError::Storage)
            );
        }
        let fixture = Fixture::start();
        fixture.set_mode(10);
        let CommandError::ObservationWriteFailure(original) =
            fixture.compare(directory.path()).unwrap_err()
        else {
            panic!("storage failure required")
        };
        let capsule = original.write_failure.as_ref().unwrap();
        fixture
            .conn
            .lock()
            .unwrap()
            .authorizer(Some(|context: rusqlite::hooks::AuthContext<'_>| {
                if matches!(
                    context.action,
                    rusqlite::hooks::AuthAction::Read {
                        table_name: "metadata_request_slots",
                        ..
                    }
                ) {
                    rusqlite::hooks::Authorization::Deny
                } else {
                    rusqlite::hooks::Authorization::Allow
                }
            }))
            .unwrap();
        let result = assemble_provider_context(
            &fixture.conn,
            Err((**capsule).clone().into()),
            None,
            original.receipts.to_vec(),
        );
        let recorder = ProviderObservationRecorder::new(Arc::clone(&fixture.conn));
        let error = assemble_observed_query(&recorder, result, Vec::new(), |_, _| unreachable!())
            .unwrap_err();
        let CommandError::ObservationWriteFailure(combined) = error else {
            panic!("combined failure required")
        };
        assert_eq!(combined.write_failure, original.write_failure);
        assert_eq!(combined.receipts, original.receipts);
        assert_eq!(combined.read_error, Some(ProviderReadError::Storage));
    }

    #[test]
    fn adr_0075_library_observation_comparison_returns_current_empty_and_failed_refresh() {
        use crate::provider_observation::{CollectionState, RefreshState};
        let fixture = Fixture::start();
        let directory = comparison_audio();
        fixture
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE feeds SET feed_url=?1 WHERE id=1",
                [format!("http://{}/feed.xml", fixture.address)],
            )
            .unwrap();
        fixture.set_mode(12);
        let populated = fixture.load(0).unwrap();
        assert!(populated
            .provider_state
            .collections
            .iter()
            .all(|collection| matches!(collection.state, CollectionState::CompletePopulated(_))));
        fixture.set_mode(13);
        let empty = fixture.compare(directory.path()).unwrap();
        assert_eq!(empty.track_context.provider_state.collections.len(), 2);
        assert!(empty
            .track_context
            .provider_state
            .collections
            .iter()
            .all(|collection| matches!(collection.state, CollectionState::CompleteEmpty(_))));
        fixture.set_mode(14);
        let failed = fixture.compare(directory.path()).unwrap();
        assert_eq!(
            failed.track_context.provider_state.collections,
            empty.track_context.provider_state.collections
        );
        assert_eq!(
            failed
                .track_context
                .provider_state
                .request_refresh
                .unwrap()
                .state,
            RefreshState::Failed
        );
        fixture.set_mode(1);
        let fallback = fixture.compare(directory.path()).unwrap();
        assert_eq!(
            fallback.track_context.provider_state.collections,
            empty.track_context.provider_state.collections
        );
        assert!(fallback.track_context.track.feed_url.is_none());
        assert!(fallback
            .track_context
            .feed
            .as_ref()
            .unwrap()
            .feed_url
            .is_none());
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("comparison.sqlite");
        {
            let mut target = Connection::open(&path).unwrap();
            let source = fixture.conn.lock().unwrap();
            rusqlite::backup::Backup::new(&source, &mut target)
                .unwrap()
                .run_to_completion(16, Duration::from_millis(1), None)
                .unwrap();
        }
        let reopened = Connection::open(path).unwrap();
        let bodies: Vec<String> = reopened
            .prepare("SELECT CAST(bytes AS TEXT) FROM metadata_bodies")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(bodies
            .iter()
            .any(|body| body.contains("\"future\":\"original\"")));
        assert!(bodies.iter().any(|body| body.contains("<podcast:txt")));
        assert!(bodies.iter().any(|body| body == "retained RSS failure"));
        let request = feed_service::local_provider_request(
            &reopened,
            &fixture.tracks[0],
            &track_row_to_track_context(&fixture.tracks[0]),
        )
        .unwrap();
        let state =
            db::provider_observations::read_track_provider_state(&reopened, request.as_ref())
                .unwrap();
        assert_eq!(
            state.collections,
            empty.track_context.provider_state.collections
        );
    }

    #[test]
    fn adr_0075_library_observation_comparison_repetition_preserves_routes_and_counts_mutations() {
        let fixture = Fixture::start();
        let directory = comparison_audio();
        let original = std::fs::read(directory.path().join("track.mp3")).unwrap();
        fixture.set_mode(22);
        let mutations = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let counts = Arc::clone(&mutations);
        fixture
            .conn
            .lock()
            .unwrap()
            .update_hook(Some(
                move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                    *counts.lock().unwrap().entry(table.into()).or_default() += 1;
                },
            ))
            .unwrap();
        let mut prior_rows = None;
        for repetition in [false, true] {
            mutations.lock().unwrap().clear();
            fixture.commits.store(0, Ordering::SeqCst);
            // Packet 018 P18-3 would otherwise reuse the first repetition's
            // retained RSS document on the second pass. This test measures
            // two independent observations, so each pass clears it first.
            crate::rss::invalidate_feed_document(&format!("http://{}/feed.xml", fixture.address));
            let result = fixture.compare(directory.path()).unwrap();
            assert_eq!(
                result.tag_compare.contributors[0].name.as_deref(),
                Some("Payee")
            );
            assert_eq!(
                result.tag_compare.value_routes[0].recipient_name.as_deref(),
                Some("Payee")
            );
            assert_eq!(result.tag_compare.value_routes[0].split, Some(100.0));
            assert_eq!(result.track_context.observation_receipts.len(), 3);
            assert_eq!(
                std::fs::read(directory.path().join("track.mp3")).unwrap(),
                original
            );
            assert_eq!(fixture.commits.load(Ordering::SeqCst), 6);
            let counts = mutations.lock().unwrap();
            let legacy: usize = counts
                .iter()
                .filter(|(table, _)| !table.starts_with("metadata_"))
                .map(|(_, count)| count)
                .sum();
            let snapshots: usize = counts
                .iter()
                .filter(|(table, _)| table.starts_with("metadata_snapshot"))
                .map(|(_, count)| count)
                .sum();
            let heads = counts
                .get("metadata_collection_heads")
                .copied()
                .unwrap_or(0);
            let evidence: usize = counts.values().sum::<usize>() - legacy - snapshots - heads;
            assert_eq!(legacy, 0);
            let rows = fixture.row_counts();
            if let Some(prior) = &prior_rows {
                assert_eq!(&rows, prior);
            }
            prior_rows = Some(rows);
            println!("ADR0075_LIBRARY_COMPARISON repetition={repetition} legacy_rows={legacy} evidence_rows={evidence} snapshot_rows={snapshots} head_rows={heads} transactions=6");
        }
    }

    #[test]
    fn adr_0075_library_observation_interleaved_detail_comparison_and_hydration_share_exact_slots()
    {
        use crate::provider_observation::ObservationRetention;
        let fixture = Fixture::start();
        let directory = comparison_audio();
        fixture.set_mode(20);
        let command = FetchLibraryTrackContext::new(
            Arc::clone(&fixture.conn),
            fixture.tracks[0].clone(),
            fixture.endpoint.clone(),
        );
        let detail = std::thread::spawn(move || {
            command
                .execute(&CommandContext::next())
                .unwrap()
                .into_parts()
                .0
        });
        let start = std::time::Instant::now();
        while fixture.held_response.lock().unwrap().is_none() {
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "detail request did not reach fixture"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        fixture.set_mode(1);
        let comparison = fixture.compare(directory.path()).unwrap();
        assert_eq!(comparison.track_context.observation_receipts.len(), 3);
        let newer_generation = comparison.track_context.observation_receipts[0].generation;
        fixture.set_mode(16);
        let hydration = fixture.hydrate().unwrap();
        assert_eq!(hydration.observation_receipts.len(), 1);
        {
            let db = fixture.conn.lock().unwrap();
            let states: Vec<String> = db.prepare("SELECT state FROM metadata_request_slots WHERE resource_id IN (SELECT id FROM metadata_resources WHERE request_uri LIKE '%/v1/feeds/f1?%') ORDER BY generation").unwrap().query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap();
            assert_eq!(states, vec!["failed", "success"]);
        }
        fixture.set_mode(0);
        let (mut stream, response) = fixture.held_response.lock().unwrap().take().unwrap();
        stream.write_all(response.as_bytes()).unwrap();
        drop(stream);
        let older = detail.join().unwrap();
        assert!(older.observation_receipts[0].generation < newer_generation);
        assert_eq!(
            older.observation_receipts[0].retention,
            ObservationRetention::SupersededAttempt
        );
        let db = fixture.conn.lock().unwrap();
        let slot: (i64, String) = db.query_row("SELECT generation,state FROM metadata_request_slots WHERE resource_id IN (SELECT id FROM metadata_resources WHERE request_uri LIKE '%/v1/feeds/f1/tracks/t1?%')", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(slot, (newer_generation, "failed".into()));
        assert_eq!(fixture.requests.lock().unwrap().len(), 7);
    }

    #[test]
    fn adr_0075_observation_live_library_request_bounds_receipts_and_repeated_mutations() {
        let fixture = Fixture::start();
        let before = fixture.conn.lock().unwrap().total_changes();
        let context = fixture.load(0).unwrap();
        assert_eq!(context.observation_receipts.len(), 3);
        assert_eq!(fixture.requests.lock().unwrap().len(), 3);
        assert_eq!(fixture.commits.load(Ordering::SeqCst), 6);
        let first_changes = fixture.conn.lock().unwrap().total_changes() - before;
        let rows = fixture.row_counts();
        assert_eq!(context.track.title.as_deref(), Some("Local title"));
        let cloned = context.clone();
        assert_eq!(cloned.observation_receipts, context.observation_receipts);
        let before = fixture.conn.lock().unwrap().total_changes();
        let repeated = fixture.load(0).unwrap();
        let repeated_changes = fixture.conn.lock().unwrap().total_changes() - before;
        // Packet 018 R18B-01: the track and feed responses are both still
        // inside their windows, so this second load reuses them and sends
        // no request. Each reused response still carries its own receipt
        // (R18B-07), so the track and feed contribute one receipt each.
        // The RSS document is reused too (P18-3), and it replays the
        // receipt of the fetch that produced it, so the total stays at 3.
        assert_eq!(repeated.observation_receipts.len(), 3);
        // A reused response writes nothing. Each of the three replays the
        // receipt of the observation that produced it (R18B-07), so no
        // table grows. The operator decided this on 2026-09-22, against a
        // new observation for each reuse.
        assert_eq!(fixture.row_counts(), rows);
        // No request reached the network, and no write followed, so the
        // request log and the commit count both stay where the first load
        // left them.
        assert_eq!(fixture.requests.lock().unwrap().len(), 3);
        assert_eq!(fixture.commits.load(Ordering::SeqCst), 6);
        fixture.requests.lock().unwrap().clear();
        let _one = fixture.load(0).unwrap();
        let _two = fixture.load(1).unwrap();
        // R18B-10: two Library tracks of one feed send one feed request and
        // one RSS request inside the windows. Both are already retained
        // from the loads above, so only the second track's own scoped
        // track request (a distinct identity) reaches the network here.
        assert_eq!(fixture.requests.lock().unwrap().len(), 1);
        println!("ADR0075_OBSERVATION_COUNTS first_rows={first_changes} repeated_rows={repeated_changes} first_transactions=6 repeated_transactions=2 stable_evidence={rows:?}");
        let conn = fixture.conn.lock().unwrap();
        let original:Vec<u8>=conn.query_row("SELECT bytes FROM metadata_bodies WHERE CAST(bytes AS TEXT) LIKE '%original Index%' LIMIT 1",[],|r|r.get(0)).unwrap();
        assert!(String::from_utf8(original)
            .unwrap()
            .contains("\"title\":\"...\""));
        let rss_bodies: i64 = conn
            .query_row(
                "SELECT count(*) FROM metadata_bodies WHERE CAST(bytes AS TEXT) LIKE '<rss%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rss_bodies, 1);
    }

    /// Packet 018 Part B measurement: repeated Library track detail
    /// (R18B-09). Target: 0 Index requests and 0 RSS requests inside the
    /// windows. Isolated in this test, no app launch, packet 016 fixture
    /// style.
    #[test]
    fn adr_0075_request_reuse_measurement_repeated_track_detail() {
        let fixture = Fixture::start();
        let first = fixture.load(0).unwrap();
        assert_eq!(
            fixture.requests.lock().unwrap().len(),
            3,
            "the first load sends 2 Index requests and 1 RSS request"
        );
        assert_eq!(first.observation_receipts.len(), 3);
        fixture.requests.lock().unwrap().clear();
        let repeated = fixture.load(0).unwrap();
        let measured_requests = fixture.requests.lock().unwrap().len();
        println!(
            "ADR0075_MEASUREMENT case=\"Repeated Library track detail\" \
target=\"0 Index, 0 RSS\" measured_requests={measured_requests}"
        );
        assert_eq!(
            measured_requests, 0,
            "R18B-09: the repeated case sends no Index request and no RSS request"
        );
        // The reused track and feed responses still carry their own
        // receipts (R18B-07), and the reused RSS document now replays its
        // own receipt too, from the fetch that first produced it: it
        // records no new observation, but it no longer drops the evidence
        // of the one it already made. The total is 3 again.
        assert_eq!(repeated.observation_receipts.len(), 3);
    }

    /// Packet 018 Part B measurement: two Library tracks sharing one feed
    /// (R18B-10). Target: 3 Index requests and 1 RSS request total. Fresh
    /// fixture, isolated in this test.
    #[test]
    fn adr_0075_request_reuse_measurement_two_tracks_sharing_one_feed() {
        let fixture = Fixture::start();
        let _first = fixture.load(0).unwrap();
        let _second = fixture.load(1).unwrap();
        let requests = fixture.requests.lock().unwrap().clone();
        let rss_requests = requests
            .iter()
            .filter(|request| request.starts_with("/feed.xml"))
            .count();
        let index_requests = requests.len() - rss_requests;
        println!(
            "ADR0075_MEASUREMENT case=\"Two Library tracks sharing one feed\" \
target=\"3 Index, 1 RSS\" measured_index={index_requests} measured_rss={rss_requests} \
requests={requests:?}"
        );
        assert_eq!(
            index_requests, 3,
            "R18B-10: two Library tracks of one feed send one feed request and each \
track's own scoped track request"
        );
        assert_eq!(
            rss_requests, 1,
            "R18B-10: two Library tracks of one feed send one RSS request"
        );
    }

    /// Packet 018 Part B measurement: repeated Library album hydration.
    /// Target: 0 Index requests and 0 row changes. Isolated in this test.
    #[test]
    fn adr_0075_request_reuse_measurement_repeated_album_hydration() {
        let fixture = Fixture::start();
        fixture.set_mode(16);
        let mutations = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let observed = Arc::clone(&mutations);
        fixture
            .conn
            .lock()
            .unwrap()
            .update_hook(Some(
                move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                    *observed.lock().unwrap().entry(table.into()).or_default() += 1;
                },
            ))
            .unwrap();
        let _first = fixture.hydrate().unwrap();
        assert_eq!(fixture.requests.lock().unwrap().len(), 1);
        mutations.lock().unwrap().clear();
        fixture.requests.lock().unwrap().clear();
        let _repeated = fixture.hydrate().unwrap();
        let measured_requests = fixture.requests.lock().unwrap().len();
        let measured_row_changes: usize = mutations.lock().unwrap().values().sum();
        println!(
            "ADR0075_MEASUREMENT case=\"Repeated Library album hydration\" \
target=\"0 Index, 0 row changes\" measured_index={measured_requests} \
measured_row_changes={measured_row_changes}"
        );
        assert_eq!(measured_requests, 0);
        assert_eq!(measured_row_changes, 0);
    }

    /// Packet 018 Part B measurement: two concurrent Library track details
    /// of one track. Target: one request set (Part A measured active
    /// sharing at the owner; this measures it against the packet 016
    /// fixtures). The fixture holds the first request reaching it until
    /// both callers have started, forcing a genuine race instead of a fast
    /// sequential pair.
    ///
    /// Measured result: 3 requests, the target this case names. The two
    /// Index requests (the shared track and the shared feed) each stay at
    /// one: both go through the owner's `single_flight`, so the joining
    /// caller waits for the winner instead of sending its own (R18A-03).
    /// The RSS request now does too: `src/rss/enrich.rs` shares an
    /// in-flight GET across concurrent callers of the same feed URL,
    /// reusing the same generic `single_flight` (Job 2 of this packet's
    /// Part B follow-up), so two callers that both reach RSS enrichment
    /// before either one's fetch completes still send only one GET.
    #[test]
    fn adr_0075_request_reuse_measurement_two_concurrent_track_details_of_one_track() {
        let fixture = Fixture::start();
        fixture.set_mode(20);
        let spawn_detail = |fixture: &Fixture| {
            let conn = Arc::clone(&fixture.conn);
            let track = fixture.tracks[0].clone();
            let endpoint = fixture.endpoint.clone();
            std::thread::spawn(move || {
                FetchLibraryTrackContext::new(conn, track, endpoint)
                    .execute(&CommandContext::next())
                    .unwrap()
                    .into_parts()
                    .0
            })
        };
        let first = spawn_detail(&fixture);
        let start = std::time::Instant::now();
        while fixture.held_response.lock().unwrap().is_none() {
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "the first request did not reach the fixture"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        // Mode 9 skips the fixture's own "one pending slot" compatibility
        // check for the rest of this test. That check assumes one caller
        // reaches one resource at a time, which every other test upholds.
        // Both concurrent callers here still call the recorder's own
        // `begin` for the RSS resource before either reaches the shared
        // `single_flight` GET below (R18B-07's rule keeps each caller's
        // own observation independent, so neither call joins the other's
        // `begin`), so two pending rows can exist for that one resource
        // for a moment, which is the fixture check's premise, not this
        // test's own subject. The GET itself still lands once.
        fixture.mode.store(9, Ordering::SeqCst);
        let second = spawn_detail(&fixture);
        // Give the second caller time to join the first's active request
        // before the held response is released.
        std::thread::sleep(Duration::from_millis(50));
        let (mut stream, response) = fixture.held_response.lock().unwrap().take().unwrap();
        stream.write_all(response.as_bytes()).unwrap();
        drop(stream);
        let first_context = first.join().unwrap();
        let second_context = second.join().unwrap();
        let all_requests = fixture.requests.lock().unwrap().clone();
        let measured_requests = all_requests.len();
        println!(
            "ADR0075_MEASUREMENT case=\"Two concurrent Library track details of one track\" \
target=\"1 request set\" measured_requests={measured_requests} requests={all_requests:?}"
        );
        // Measured, at the 3-request target: both Index requests (track
        // and feed) are shared, and the RSS request now is too (Job 2 of
        // this packet's Part B follow-up, closing the gap this test's own
        // documentation named above).
        assert_eq!(
            measured_requests, 3,
            "two shared Index requests plus one shared RSS request"
        );
        assert_eq!(
            first_context.track.track_guid,
            second_context.track.track_guid
        );
    }

    #[test]
    fn adr_0075_observation_retained_failures_preserve_local_and_scoped_fallback() {
        let fixture = Fixture::start();
        fixture.set_mode(1);
        let local = fixture.load(0).unwrap();
        assert_eq!(local.observation_receipts.len(), 3);
        assert!(local
            .observation_receipts
            .iter()
            .all(|r| r.outcome == crate::provider_observation::ObservationOutcome::Failed));
        assert_eq!(fixture.requests.lock().unwrap().len(), 3);
        assert_eq!(local.track.title.as_deref(), Some("Local title"));
        fixture.requests.lock().unwrap().clear();
        fixture.set_mode(2);
        let context = fixture.load(0).unwrap();
        assert_eq!(context.observation_receipts.len(), 4);
        assert_eq!(fixture.requests.lock().unwrap().len(), 4);
        // Packet 018 Job 3: the RSS receipt now travels with its own
        // enrichment call, so it lands ahead of the track and feed
        // receipts here, instead of after them. The scoped track's own
        // failed request is still one of the four, found by outcome
        // rather than by a fixed position.
        assert!(
            context
                .observation_receipts
                .iter()
                .any(|r| r.outcome == crate::provider_observation::ObservationOutcome::Failed),
            "the scoped track's failed request must still be recorded"
        );
    }
    #[test]
    fn adr_0075_observation_decode_failure_and_auxiliary_failure_keep_original_bytes() {
        for mode in [3, 4, 5, 6, 7] {
            let fixture = Fixture::start();
            fixture.set_mode(mode);
            let context = fixture.load(0).unwrap();
            let conn = fixture.conn.lock().unwrap();
            // Packet 018 Job 3: the RSS receipt now travels with its own
            // enrichment call, so a successful or partial RSS fetch's
            // receipt lands ahead of the track and feed receipts here,
            // instead of after them (mode 5's RSS failure is the one
            // exception: an RSS `Err` keeps its receipt on the
            // pre-existing failure path, so it stays last, as before).
            // Each case below finds its own receipt by outcome or by
            // request URI, rather than by a fixed position.
            match mode {
                3 | 4 => {
                    let failed = context
                        .observation_receipts
                        .iter()
                        .find(|r| {
                            r.outcome == crate::provider_observation::ObservationOutcome::Failed
                        })
                        .expect("the track's JSON decode failure must be recorded");
                    let bytes: Vec<u8> = conn
                        .query_row(
                            "SELECT bytes FROM metadata_bodies WHERE sha256=?1",
                            [failed.body_key.as_ref().unwrap()],
                            |r| r.get(0),
                        )
                        .unwrap();
                    if mode == 4 {
                        assert_eq!(
                            String::from_utf8(bytes)
                                .unwrap()
                                .matches("\"title\"")
                                .count(),
                            2
                        );
                    }
                }
                5 => {
                    assert_eq!(
                        context.observation_receipts.last().unwrap().outcome,
                        crate::provider_observation::ObservationOutcome::Failed
                    );
                    assert!(context.rss_observation.is_none());
                    assert_eq!(context.track.description.as_deref(), Some("original Index"));
                }
                6 => {
                    assert!(context
                        .observation_receipts
                        .iter()
                        .any(|r| r.outcome
                            == crate::provider_observation::ObservationOutcome::Partial));
                    assert!(context.rss_observation.is_some());
                }
                7 => {
                    let track_receipt = context
                        .observation_receipts
                        .iter()
                        .find(|r| r.request_uri.contains("/tracks/"))
                        .expect("the track's own receipt must be recorded");
                    assert_eq!(
                        track_receipt.outcome,
                        crate::provider_observation::ObservationOutcome::Success
                    );
                    assert!(conn.query_row("SELECT basis_json FROM metadata_coverage WHERE observation_id=?1 LIMIT 1",[track_receipt.observation_id],|r|r.get::<_,String>(0)).unwrap().contains("raw_extraction_incomplete"));
                }
                _ => unreachable!(),
            }
        }
    }
    #[test]
    fn adr_0075_observation_storage_failures_cross_command_boundary_and_retry_without_http() {
        let fixture = Fixture::start();
        fixture.conn.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_allocation BEFORE UPDATE ON metadata_generation BEGIN SELECT RAISE(ABORT,'fixture allocation rejection'); END").unwrap();
        let error = fixture.load(0).unwrap_err();
        assert!(matches!(error, CommandError::ObservationWriteFailure(_)));
        assert!(fixture.requests.lock().unwrap().is_empty());
        fixture
            .conn
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER reject_allocation")
            .unwrap();
        fixture.set_mode(8);
        let error = fixture.load(0).unwrap_err();
        let CommandError::ObservationWriteFailure(failure) = error else {
            panic!("typed storage failure required")
        };
        assert_eq!(fixture.requests.lock().unwrap().len(), 1);
        assert_eq!(fixture.row_counts(), vec![0, 0, 0, 0]);
        let conn = fixture.conn.lock().unwrap();
        conn.execute_batch("DROP TRIGGER reject_observation")
            .unwrap();
        let receipt = db::provider_observations::record_provider_observation(
            &conn,
            failure.write_failure.as_ref().unwrap().token.clone(),
            Arc::clone(&failure.write_failure.as_ref().unwrap().observation),
        )
        .unwrap();
        assert_eq!(
            receipt.generation,
            failure.write_failure.as_ref().unwrap().token.generation()
        );
        assert_eq!(fixture.requests.lock().unwrap().len(), 1);
        for rendered in [
            format!("{failure:?}"),
            format!("{failure}"),
            format!(
                "{:?}",
                CommandError::ObservationWriteFailure(Arc::clone(&failure))
            ),
        ] {
            assert!(!rendered.contains("original Index"));
            assert!(!rendered.contains(&fixture.address));
        }
    }
    #[test]
    fn adr_0075_observation_earlier_receipts_survive_later_write_and_allocation_failures() {
        for mode in [10, 11] {
            let fixture = Fixture::start();
            fixture.set_mode(mode);
            let CommandError::ObservationWriteFailure(failure) = fixture.load(0).unwrap_err()
            else {
                panic!("typed command failure required")
            };
            assert_eq!(failure.receipts.len(), 2);
            assert!(failure
                .receipts
                .iter()
                .all(|r| r.outcome == crate::provider_observation::ObservationOutcome::Success));
            assert_eq!(failure.write_failure.is_some(), mode == 10);
            assert_eq!(failure.storage_error.is_some(), mode == 11);
            assert_eq!(
                fixture.requests.lock().unwrap().len(),
                if mode == 10 { 3 } else { 2 }
            );
            let mut vm = LibraryViewModel::new();
            vm.retain_observation_failure(&CommandError::ObservationWriteFailure(failure));
            assert!(vm.status_snapshot().is_error);
        }
        let fixture = Fixture::start();
        fixture.set_mode(1);
        fixture
            .conn
            .lock()
            .unwrap()
            .authorizer(Some(|context: rusqlite::hooks::AuthContext<'_>| {
                if matches!(context.action, rusqlite::hooks::AuthAction::Read { table_name, .. } if table_name.starts_with("entity_")) { rusqlite::hooks::Authorization::Deny } else { rusqlite::hooks::Authorization::Allow }
            }))
            .unwrap();
        let CommandError::ObservationWriteFailure(failure) = fixture.load(0).unwrap_err() else {
            panic!("retained local fallback failure required")
        };
        assert_eq!(failure.receipts.len(), 3);
    }

    #[test]
    fn adr_0075_observation_unobserved_service_keeps_compatibility_requests() {
        let fixture = Fixture::start();
        fixture.set_mode(9);
        let context =
            feed_service::fetch_library_track_context(&fixture.tracks[0], &fixture.endpoint)
                .unwrap();
        assert!(context.observation_receipts.is_empty());
        assert_eq!(fixture.requests.lock().unwrap().len(), 3);
        assert_eq!(fixture.row_counts(), vec![0, 0, 0, 0]);
        assert_eq!(fixture.commits.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn adr_0075_snapshot_live_commands_return_empty_and_retained_failed_refresh() {
        use crate::provider_observation::{CollectionState, RefreshState};
        let fixture = Fixture::start();
        fixture
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE feeds SET feed_url=?1 WHERE id=1",
                [format!("http://{}/feed.xml", fixture.address)],
            )
            .unwrap();
        fixture.commits.store(0, Ordering::SeqCst);
        fixture.set_mode(12);
        let populated = fixture.load(0).unwrap();
        assert_eq!(populated.provider_state.collections.len(), 2);
        assert!(populated
            .provider_state
            .collections
            .iter()
            .all(|c| matches!(c.state, CollectionState::CompletePopulated(_))));
        assert_eq!(fixture.requests.lock().unwrap().len(), 3);
        assert_eq!(fixture.commits.load(Ordering::SeqCst), 6);
        fixture.set_mode(13);
        let empty = fixture.load(0).unwrap();
        assert!(empty
            .provider_state
            .collections
            .iter()
            .all(|c| matches!(c.state, CollectionState::CompleteEmpty(_))));
        let changes = fixture.conn.lock().unwrap().total_changes();
        let requests = fixture.requests.lock().unwrap().len();
        let local = FetchLocalTrackContext::new(Arc::clone(&fixture.conn), fixture.tracks[0].id)
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0
            .context;
        assert_eq!(local.provider_state, empty.provider_state);
        assert_eq!(fixture.conn.lock().unwrap().total_changes(), changes);
        assert_eq!(fixture.requests.lock().unwrap().len(), requests);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("local-context.sqlite");
        {
            let mut target = Connection::open(&path).unwrap();
            let source = fixture.conn.lock().unwrap();
            let backup = rusqlite::backup::Backup::new(&source, &mut target).unwrap();
            backup
                .run_to_completion(16, Duration::from_millis(1), None)
                .unwrap();
        }
        let reopened = Arc::new(Mutex::new(Connection::open(path).unwrap()));
        let reopened_changes = reopened.lock().unwrap().total_changes();
        let reopened_local =
            FetchLocalTrackContext::new(Arc::clone(&reopened), fixture.tracks[0].id)
                .execute(&CommandContext::next())
                .unwrap()
                .into_parts()
                .0
                .context;
        assert_eq!(reopened_local.provider_state, empty.provider_state);
        assert_eq!(reopened.lock().unwrap().total_changes(), reopened_changes);
        assert_eq!(fixture.requests.lock().unwrap().len(), requests);
        fixture.set_mode(14);
        let failed = fixture.load(0).unwrap();
        assert_eq!(
            failed.provider_state.collections,
            empty.provider_state.collections
        );
        assert_eq!(
            failed
                .provider_state
                .request_refresh
                .as_ref()
                .unwrap()
                .state,
            RefreshState::Failed
        );
        assert_eq!(failed.observation_receipts.len(), 3);
    }

    #[test]
    fn adr_0075_snapshot_committed_read_failure_reaches_command_and_library_state() {
        let fixture = Fixture::start();
        fixture.set_mode(15);
        let CommandError::ObservationWriteFailure(failure) = fixture.load(0).unwrap_err() else {
            panic!("typed retained failure required");
        };
        assert!(failure.read_error.is_some());
        assert_eq!(failure.receipts.len(), 3);
        assert!(failure.write_failure.is_none());
        assert_eq!(
            fixture
                .conn
                .lock()
                .unwrap()
                .query_row("SELECT count(*) FROM metadata_snapshots", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        let mut vm = LibraryViewModel::new();
        vm.retain_observation_failure(&CommandError::ObservationWriteFailure(Arc::clone(&failure)));
        assert_eq!(vm.retained_observation_evidence_counts(), (3, 0));
        assert!(vm.status_snapshot().is_error);
        assert!(vm
            .status_snapshot()
            .text
            .contains("could not read provider state"));
    }

    #[test]
    fn adr_0075_snapshot_read_failure_also_preserves_preceding_write_capsule() {
        let fixture = Fixture::start();
        fixture.set_mode(10);
        let CommandError::ObservationWriteFailure(failure) = fixture.load(0).unwrap_err() else {
            panic!("typed retained failure required");
        };
        let capsule = failure.write_failure.as_ref().unwrap();
        fixture
            .conn
            .lock()
            .unwrap()
            .authorizer(Some(|context: rusqlite::hooks::AuthContext<'_>| {
                if matches!(
                    context.action,
                    rusqlite::hooks::AuthAction::Read {
                        table_name: "metadata_request_slots",
                        ..
                    }
                ) {
                    rusqlite::hooks::Authorization::Deny
                } else {
                    rusqlite::hooks::Authorization::Allow
                }
            }))
            .unwrap();
        let error = assemble_provider_context(
            &fixture.conn,
            Err((**capsule).clone().into()),
            None,
            failure.receipts.to_vec(),
        )
        .unwrap_err();
        let error = query_error(&error);
        let CommandError::ObservationWriteFailure(ref combined) = error else {
            panic!("typed combined failure required");
        };
        assert_eq!(combined.write_failure, failure.write_failure);
        assert_eq!(combined.receipts, failure.receipts);
        assert!(combined.read_error.is_some());
        let mut vm = LibraryViewModel::new();
        vm.retain_observation_failure(&error);
        assert_eq!(vm.retained_observation_evidence_counts(), (2, 1));
    }
    fn legacy_rss_observation(
        request: &crate::provider_observation::ProviderRequestSpec,
        body: &str,
        failed: bool,
    ) -> crate::provider_observation::ProviderObservation {
        use crate::provider_observation::{
            CoverageEvidence, FactEvidence, ObservationOutcome, ObservationRetention,
            PropertyPresence, ProviderObservation, SubjectKey,
        };
        let mut result = ProviderObservation {
            body: Some(Arc::from(body.as_bytes())),
            http_status: Some(200),
            response_uri: Some(request.request_uri.clone()),
            interpretation: json!({"version":1,"media_type":"application/json","charset":"UTF-8","retained_body_codings":[],"body_state":"complete","effective_base_uri":null}),
            source_revision: None,
            source_times: json!({}),
            decoder_version: "rss-dom-v1".into(),
            outcome: if failed {
                ObservationOutcome::Failed
            } else {
                ObservationOutcome::Success
            },
            failure: failed.then(|| json!({"reason":"xml_decode"})),
            finished_at_us: 2,
            fetched_at_us: Some(1),
            occurrence: json!({"version":1,"headers":{}}),
            coverage: Vec::new(),
        };
        if failed {
            return result;
        }
        let document = roxmltree::Document::parse(body).unwrap();
        for node in document.descendants().filter(|node| {
            node.is_element() && !matches!(node.tag_name().name(), "rss" | "channel" | "item")
        }) {
            let owner = node
                .ancestors()
                .skip(1)
                .find(|node| matches!(node.tag_name().name(), "item" | "channel"))
                .unwrap();
            let item = (owner.tag_name().name() == "item").then_some("t1");
            let subject = SubjectKey::guid("f1", item);
            let declared_owner =
                json!({"feed_guid":"f1","item_guid":item,"feed_resource":request.request_uri});
            let identity = node.tag_name().name() == "txt";
            let collection = if identity {
                "source_ids".into()
            } else {
                format!(
                    "field:xml:{{{}}}{}",
                    node.tag_name().namespace().unwrap_or_default(),
                    node.tag_name().name()
                )
            };
            let position = document.text_pos_at(node.range().start);
            let fact = FactEvidence {
                subject: Some(subject.clone()),
                declared_owner: declared_owner.clone(),
                owner_basis: json!({"basis":"direct_xml_owner"}),
                kind: collection.clone(),
                assertion_source: Some("rss".into()),
                source_position: None,
                extraction_path: identity.then(|| {
                    if item.is_some() {
                        "rss/channel/item[1]/podcast:txt[1]/text()".into()
                    } else {
                        "rss/channel/podcast:txt[1]/text()".into()
                    }
                }),
                source_observed: None,
                representation: "structured".into(),
                validation: if identity { "valid" } else { "unresolved" }.into(),
                value: crate::provider_observation::contracts::rss_element_value(node),
                raw_member: None,
                body_locator: json!({"decoded_byte_offset":node.range().start,"line":position.row,"column":position.col}),
            };
            result.coverage.push(CoverageEvidence {
                collection,
                target: Some(subject),
                target_owner: declared_owner,
                request_intent: "implicit".into(),
                presence: PropertyPresence::Populated,
                retention: ObservationRetention::Partial,
                basis: json!({"reason":"no_registered_completeness_contract"}),
                facts: vec![fact],
                proof: None,
            });
        }
        result
    }

    fn retained_interpretation_rows(
        conn: &Connection,
        observation: i64,
    ) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
        ["SELECT * FROM metadata_observations WHERE id=?1","SELECT * FROM metadata_coverage WHERE observation_id=?1 ORDER BY scope_ordinal","SELECT * FROM metadata_facts WHERE observation_id=?1 ORDER BY scope_ordinal,transport_ordinal"].iter().map(|query| {
            let mut statement = conn.prepare(query).unwrap();
            let columns = statement.column_count();
            let rows = statement.query_map([observation],|row|(0..columns).map(|column|row.get(column)).collect()).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
            rows
        }).collect()
    }

    #[test]
    fn adr_0075_snapshot_live_v2_capture_preserves_actual_v1_per_node_and_failed_rows() {
        use crate::db::provider_observations::{
            begin_provider_request, record_provider_observation,
        };
        for mode in [12, 5] {
            let fixture = Fixture::start();
            fixture.set_mode(mode);
            let base = format!("http://{}", fixture.address);
            let request = crate::provider_observation::contracts::rss_request(
                &format!("{base}/feed.xml"),
                Some("t1"),
                None,
            );
            let (_, body) = response(&base, "/feed.xml", mode);
            let old = legacy_rss_observation(&request, &body, mode == 5);
            if mode == 12 {
                assert_eq!(old.coverage.len(), 4);
                assert!(old
                    .coverage
                    .iter()
                    .all(|coverage| coverage.facts.len() == 1 && coverage.proof.is_none()));
            }
            let (receipt, before) = {
                let conn = fixture.conn.lock().unwrap();
                let token = begin_provider_request(&conn, request.clone()).unwrap();
                let receipt = record_provider_observation(&conn, token, Arc::new(old)).unwrap();
                let before = retained_interpretation_rows(&conn, receipt.observation_id);
                (receipt, before)
            };
            let context = fixture.load(0).unwrap();
            let current = context.observation_receipts.last().unwrap();
            assert_eq!(current.body_key, receipt.body_key);
            assert_ne!(current.observation_id, receipt.observation_id);
            let conn = fixture.conn.lock().unwrap();
            assert_eq!(
                retained_interpretation_rows(&conn, receipt.observation_id),
                before
            );
            let versions:Vec<String> = conn.prepare("SELECT decoder_version FROM metadata_observations WHERE id IN (?1,?2) ORDER BY id").unwrap().query_map(rusqlite::params![receipt.observation_id,current.observation_id],|row|row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
            assert_eq!(versions, ["rss-dom-v1", "rss-dom-v2"]);
            let equal_inputs:bool = conn.query_row("SELECT a.requested_subject_json=b.requested_subject_json AND a.profile_json=b.profile_json AND a.interpretation_metadata_json=b.interpretation_metadata_json AND a.outcome=b.outcome FROM metadata_observations a, metadata_observations b WHERE a.id=?1 AND b.id=?2",rusqlite::params![receipt.observation_id,current.observation_id],|row|row.get(0)).unwrap();
            assert!(equal_inputs);
        }
    }

    /// R17-07: the Library album hydration request sends L5.
    #[test]
    fn adr_0075_request_profile_library_album_hydration_sends_l5() {
        let fixture = Fixture::start();

        fixture.hydrate().unwrap();

        let requests = fixture.requests.lock().unwrap().clone();
        let include = crate::application::request_profiles::LIBRARY_ALBUM_HYDRATION_FEED
            .include()
            .unwrap();
        assert_eq!(
            requests,
            vec![format!(
                "/v1/feeds/f1?include={}",
                include.replace(',', "%2C")
            )],
            "R17-07: the Library album hydration request must send L5"
        );
    }

    /// R17-10: `ProviderRequestSpec` values and their profile JSON equal
    /// the values recorded before this packet, for the converted Library
    /// album hydration call site.
    #[test]
    fn adr_0075_request_profile_album_hydration_profile_json_is_unchanged() {
        let fixture = Fixture::start();

        fixture.hydrate().unwrap();

        let profile_json: String = fixture
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT profile_json FROM metadata_observations ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let profile: serde_json::Value = serde_json::from_str(&profile_json).unwrap();
        let include = crate::application::request_profiles::LIBRARY_ALBUM_HYDRATION_FEED
            .include()
            .unwrap();
        assert_eq!(
            profile,
            json!({
                "version": 1,
                "path": ["v1", "feeds", "f1"],
                "query": [["include", include]]
            }),
            "R17-10: the retained profile JSON must keep its pre-packet shape"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    #[test]
    fn adr_0040_cached_tree_command_excludes_library_files() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id)?;
        let relative_path = crate::library_path::LibraryRelativePath::for_test("tmp/track.mp3");
        library_service::mark_track_downloaded(&conn, track_id, &relative_path, None)?;
        library_service::set_track_in_library(&conn, track_id, false)?;

        let conn = Arc::new(Mutex::new(conn));
        let (result, _) = LoadCachedTracksTree::new(Arc::clone(&conn))
            .execute(&CommandContext::next())?
            .into_parts();
        assert_eq!(result.count, 1);
        assert_eq!(result.tree.artists[0].albums[0].tracks[0].id, track_id);

        library_service::set_track_in_library(&conn.lock().unwrap(), track_id, true)?;
        let (result, _) = LoadCachedTracksTree::new(conn)
            .execute(&CommandContext::next())?
            .into_parts();
        assert_eq!(result.count, 0);
        assert!(result.tree.artists.is_empty());

        Ok(())
    }

    #[test]
    fn library_queries_count_playlist_references_for_removal_warnings() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id)?;
        library_service::set_track_in_library(&conn, track_id, true)?;
        let playlist_id = db::playlist_create(&conn, "Warnings")?;
        db::playlist_append(&conn, playlist_id, track_id)?;

        let service = ApplicationQueryService::new();

        assert_eq!(
            service.playlist_reference_count_for_track(&conn, track_id)?,
            1
        );
        assert_eq!(
            service.playlist_referenced_library_track_count_for_feed(&conn, feed_id)?,
            1
        );

        Ok(())
    }

    fn create_feed(conn: &Connection) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(conn: &Connection, feed_id: i64) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![feed_id, "item-guid", "Track Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }
}
