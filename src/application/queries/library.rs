//! Library local query family.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::library_removal::{self, LibraryRemovalIntent, LibraryRemovalPlan};
use crate::db::TrackRow;
use crate::feed_service::{self, track_row_to_track_context};
use crate::metadata::{source_text_missing, TagCompareResult, TrackContext};
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

/// Fetches remote track context with local hydrated metadata fallback.
#[derive(Clone, Debug)]
pub(crate) struct FetchLibraryTrackContext {
    conn: SharedConnection,
    track: TrackRow,
    musicindex_endpoint: String,
}

impl FetchLibraryTrackContext {
    /// Creates a library track context query command.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        track: TrackRow,
        musicindex_endpoint: impl Into<String>,
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
    musicindex_endpoint: String,
    feed_id: i64,
    feed_guid: String,
}

impl HydrateAlbumIdentity {
    /// Creates an album identity hydration query command.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        musicindex_endpoint: impl Into<String>,
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
        .map_err(|error| query_error(&error))
        .map(CommandOutcome::without_events)
    }
}

/// Compares one downloaded library track against its source metadata.
#[derive(Clone, Debug)]
pub(crate) struct CompareLibraryTrack {
    track: TrackRow,
    musicindex_endpoint: String,
}

impl CompareLibraryTrack {
    /// Creates a library track comparison query command.
    #[must_use]
    pub(crate) fn new(track: TrackRow, musicindex_endpoint: impl Into<String>) -> Self {
        Self {
            track,
            musicindex_endpoint: musicindex_endpoint.into(),
        }
    }
}

impl ApplicationCommand for CompareLibraryTrack {
    type Output = LibraryTrackCompare;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        compare_library_track(&self.track, &self.musicindex_endpoint)
            .map_err(|error| query_error(&error))
            .map(CommandOutcome::without_events)
    }
}

impl ApplicationQueryService {
    /// Lists cached local tracks that are not currently in the library.
    ///
    /// # Errors
    ///
    /// Returns an error when local cached-track state cannot be read.
    pub fn cached_tracks(&self, conn: &Connection) -> Result<Vec<db::TrackRow>, CommandError> {
        library_service::cached_tracks(conn).map_err(|error| query_error(&error))
    }

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
    musicindex_endpoint: &str,
) -> anyhow::Result<TrackContext> {
    let local_context = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))
        .and_then(|db| feed_service::track_row_to_track_context_with_local_identity(&db, track));
    match feed_service::fetch_library_track_context(track, musicindex_endpoint) {
        Ok(mut remote_context) => {
            if let Ok(local_context) = local_context {
                apply_local_track_metadata_defaults(&mut remote_context, &local_context);
            }
            Ok(remote_context)
        }
        Err(_) => local_context,
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
    musicindex_endpoint: &str,
    feed_id: i64,
    feed_guid: &str,
) -> anyhow::Result<AlbumIdentityHydration> {
    let client = crate::api::Client::new_with_base_url(musicindex_endpoint.to_string());
    let feed = client.fetch_feed(
        feed_guid,
        Some("source_links,source_ids,source_release_claims,source_contributors"),
    )?;
    let description = FeedView::from_api(feed.clone()).description;
    let mut db = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
    if description.is_some() {
        db::set_feed_description(&db, feed_id, description.as_deref())?;
    }
    crate::identity_ingest::persist_musicindex_feed(&mut db, feed_id, &feed)?;
    let identity_facts = crate::local_identity::feed_facts(&db, feed_id)?;
    let metadata_facts = crate::local_metadata::feed_facts(&db, feed_id)?;
    Ok(AlbumIdentityHydration {
        identity_facts,
        metadata_facts,
        description,
    })
}

fn compare_library_track(
    track: &TrackRow,
    musicindex_endpoint: &str,
) -> anyhow::Result<LibraryTrackCompare> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("library track has no local file"))?;
    let context = feed_service::fetch_library_track_context(track, musicindex_endpoint)
        .unwrap_or_else(|_| track_row_to_track_context(track));
    let tag_compare = subscribe_service::compare_downloaded_track_path(Path::new(path), &context)?;
    Ok(LibraryTrackCompare {
        tag_compare,
        track_context: context,
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
    let context = feed_service::track_row_to_track_context_with_local_identity(&db, &track)?;
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
    CommandError::Query(format!("{error:#}"))
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
    fn library_queries_return_cached_tracks() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn)?;
        let track_id = create_track(&conn, feed_id)?;
        library_service::mark_track_downloaded(
            &conn,
            track_id,
            std::path::Path::new("/tmp/track.mp3"),
            None,
        )?;
        library_service::set_track_in_library(&conn, track_id, false)?;

        let rows = ApplicationQueryService::new().cached_tracks(&conn)?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, track_id);

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
