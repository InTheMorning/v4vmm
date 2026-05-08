//! Download command family.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::download::DownloadEvent;
use crate::application::events::feed::FeedEvent;
use crate::application::events::library::LibraryEvent;
use crate::application::events::playlist::PlaylistEvent;
use crate::application::events::ApplicationEvent;
use crate::application::ports::download_manager::{DownloadError, DownloadManager};
use crate::library_service;
use crate::library_service::AppendToPlaylistOutcome;
use crate::metadata::TagCompareResult;
use crate::subscribe_service::{SubscribeTrackOutcome, SubscribeTrackRequest};

type SharedConnection = Arc<Mutex<Connection>>;

/// Command result for subscribing/downloading a track.
#[derive(Clone, Debug)]
pub struct SubscribeTrackResult {
    path: String,
    format_warning: Option<String>,
    applied_edits: usize,
    marked_downloaded: bool,
    compare: Option<TagCompareResult>,
    message: String,
}

impl SubscribeTrackResult {
    /// Creates a track subscription result from the service outcome.
    #[must_use]
    pub fn from_outcome(outcome: SubscribeTrackOutcome, message: String) -> Self {
        Self {
            path: outcome.path.display().to_string(),
            format_warning: outcome.format_warning,
            applied_edits: outcome.applied_edits,
            marked_downloaded: outcome.marked_downloaded,
            compare: outcome.compare,
            message,
        }
    }

    /// Returns the downloaded path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the download format warning, if any.
    #[must_use]
    pub fn format_warning(&self) -> Option<&str> {
        self.format_warning.as_deref()
    }

    /// Returns how many ID3 edits were applied.
    #[must_use]
    pub const fn applied_edits(&self) -> usize {
        self.applied_edits
    }

    /// Returns whether local state was marked downloaded.
    #[must_use]
    pub const fn marked_downloaded(&self) -> bool {
        self.marked_downloaded
    }

    /// Returns the optional tag comparison result.
    #[must_use]
    pub const fn compare(&self) -> Option<&TagCompareResult> {
        self.compare.as_ref()
    }

    /// Consumes the result and returns the optional tag comparison result.
    #[must_use]
    pub fn into_compare(self) -> Option<TagCompareResult> {
        self.compare
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Subscribes/downloads one track.
pub struct SubscribeTrack {
    conn: SharedConnection,
    download_manager: Arc<dyn DownloadManager>,
    request: SubscribeTrackRequest,
    success_message: String,
}

impl SubscribeTrack {
    /// Creates a track subscription command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        download_manager: Arc<dyn DownloadManager>,
        request: SubscribeTrackRequest,
        success_message: impl Into<String>,
    ) -> Self {
        Self {
            conn,
            download_manager,
            request,
            success_message: success_message.into(),
        }
    }
}

impl ApplicationCommand for SubscribeTrack {
    type Output = SubscribeTrackResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let outcome = self
            .download_manager
            .subscribe_track(self.conn, self.request, context)
            .map_err(download_error)?;
        Ok(CommandOutcome::new(
            SubscribeTrackResult::from_outcome(outcome, self.success_message),
            download_changed_events(),
        ))
    }
}

/// Command result for subscribing/downloading tracks and appending to a playlist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscribeThenAppendToPlaylistResult {
    appended: usize,
    downloaded: usize,
    already_in_library: usize,
    failed: Vec<String>,
}

impl SubscribeThenAppendToPlaylistResult {
    /// Creates a playlist append result from the service outcome.
    #[must_use]
    pub fn from_outcome(outcome: AppendToPlaylistOutcome) -> Self {
        Self {
            appended: outcome.appended,
            downloaded: outcome.downloaded,
            already_in_library: outcome.already_in_library,
            failed: outcome.failed,
        }
    }

    /// Returns how many tracks were appended.
    #[must_use]
    pub const fn appended(&self) -> usize {
        self.appended
    }

    /// Returns how many tracks were downloaded.
    #[must_use]
    pub const fn downloaded(&self) -> usize {
        self.downloaded
    }

    /// Returns how many tracks were already in the library.
    #[must_use]
    pub const fn already_in_library(&self) -> usize {
        self.already_in_library
    }

    /// Returns failed item messages.
    #[must_use]
    pub fn failed(&self) -> &[String] {
        &self.failed
    }
}

/// Subscribes/downloads missing tracks and appends them to a playlist.
#[derive(Clone)]
pub struct SubscribeThenAppendToPlaylist {
    conn: SharedConnection,
    download_manager: Arc<dyn DownloadManager>,
    playlist_id: i64,
    track_ids: Vec<i64>,
}

impl SubscribeThenAppendToPlaylist {
    /// Creates a subscribe-then-append command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        download_manager: Arc<dyn DownloadManager>,
        playlist_id: i64,
        track_ids: Vec<i64>,
    ) -> Self {
        Self {
            conn,
            download_manager,
            playlist_id,
            track_ids,
        }
    }
}

impl ApplicationCommand for SubscribeThenAppendToPlaylist {
    type Output = SubscribeThenAppendToPlaylistResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let outcome = self
            .download_manager
            .subscribe_then_append_to_playlist(self.conn, self.playlist_id, self.track_ids, context)
            .map_err(download_error)?;
        Ok(CommandOutcome::new(
            SubscribeThenAppendToPlaylistResult::from_outcome(outcome),
            playlist_download_changed_events(self.playlist_id),
        ))
    }
}

/// Command result for removing a track from the local library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveTrackFromLibraryResult {
    message: String,
}

impl RemoveTrackFromLibraryResult {
    /// Creates a track removal result.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Command result for removing cached local files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveCachedFilesResult {
    removed_count: usize,
    message: String,
}

impl RemoveCachedFilesResult {
    /// Creates a cached-file removal result.
    #[must_use]
    pub fn new(removed_count: usize) -> Self {
        let file_label = if removed_count == 1 { "file" } else { "files" };
        Self {
            removed_count,
            message: format!("Deleted {removed_count} cached {file_label}"),
        }
    }

    /// Returns how many cached files were removed.
    #[must_use]
    pub const fn removed_count(&self) -> usize {
        self.removed_count
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Command result for setting local track library membership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetTrackLibraryMembershipResult {
    in_library: bool,
    message: String,
}

impl SetTrackLibraryMembershipResult {
    /// Creates a track membership result.
    #[must_use]
    pub fn new(in_library: bool, message: impl Into<String>) -> Self {
        Self {
            in_library,
            message: message.into(),
        }
    }

    /// Returns whether the track is now in the library.
    #[must_use]
    pub const fn in_library(&self) -> bool {
        self.in_library
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Sets local track library membership by local track id.
#[derive(Clone, Debug)]
pub struct SetTrackLibraryMembership {
    conn: SharedConnection,
    track_id: i64,
    in_library: bool,
}

impl SetTrackLibraryMembership {
    /// Creates a track membership command.
    #[must_use]
    pub const fn new(conn: SharedConnection, track_id: i64, in_library: bool) -> Self {
        Self {
            conn,
            track_id,
            in_library,
        }
    }
}

impl ApplicationCommand for SetTrackLibraryMembership {
    type Output = SetTrackLibraryMembershipResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| download_lock_error())?;
        library_service::set_track_in_library(&conn, self.track_id, self.in_library)
            .map_err(|error| download_command_error(&error))?;
        let message = if self.in_library {
            "Subscribed track"
        } else {
            "Unsubscribed track"
        };
        Ok(CommandOutcome::new(
            SetTrackLibraryMembershipResult::new(self.in_library, message),
            vec![ApplicationEvent::Library(LibraryEvent::Changed)],
        ))
    }
}

/// Removes a local track from the library by local track id.
#[derive(Clone, Debug)]
pub struct RemoveTrackFromLibrary {
    conn: SharedConnection,
    track_id: i64,
}

impl RemoveTrackFromLibrary {
    /// Creates a track removal command.
    #[must_use]
    pub const fn new(conn: SharedConnection, track_id: i64) -> Self {
        Self { conn, track_id }
    }
}

impl ApplicationCommand for RemoveTrackFromLibrary {
    type Output = RemoveTrackFromLibraryResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| download_lock_error())?;
        let feed_url = match library_service::track_row_by_id(&conn, self.track_id)
            .map_err(|error| download_command_error(&error))?
        {
            Some(track) => crate::db::feed_url_by_id(&conn, track.feed_id)
                .map_err(|error| download_command_error(&error))?,
            None => None,
        };
        library_service::set_track_in_library(&conn, self.track_id, false)
            .map_err(|error| download_command_error(&error))?;
        let feed_changed = if let Some(feed_url) = feed_url.as_deref() {
            crate::db::reconcile_feed_subscription_by_url(&conn, feed_url)
                .map_err(|error| download_command_error(&error))?;
            true
        } else {
            false
        };
        Ok(CommandOutcome::new(
            RemoveTrackFromLibraryResult::new("Removed track"),
            track_removed_events(feed_changed),
        ))
    }
}

/// Removes a local track from the library by feed/item/enclosure identity.
#[derive(Clone, Debug)]
pub struct RemoveTrackFromLibraryByMatch {
    conn: SharedConnection,
    feed_url: Option<String>,
    item_guid: Option<String>,
    enclosure_url: Option<String>,
}

impl RemoveTrackFromLibraryByMatch {
    /// Creates a matched track removal command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        feed_url: Option<String>,
        item_guid: Option<String>,
        enclosure_url: Option<String>,
    ) -> Self {
        Self {
            conn,
            feed_url,
            item_guid,
            enclosure_url,
        }
    }
}

impl ApplicationCommand for RemoveTrackFromLibraryByMatch {
    type Output = RemoveTrackFromLibraryResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| download_lock_error())?;
        library_service::set_track_in_library_by_match(
            &conn,
            self.feed_url.as_deref(),
            self.item_guid.as_deref(),
            self.enclosure_url.as_deref(),
            false,
        )
        .map_err(|error| download_command_error(&error))?;
        let feed_changed = if let Some(feed_url) = self.feed_url.as_deref() {
            crate::db::reconcile_feed_subscription_by_url(&conn, feed_url)
                .map_err(|error| download_command_error(&error))?;
            true
        } else {
            false
        };
        Ok(CommandOutcome::new(
            RemoveTrackFromLibraryResult::new("Removed track"),
            track_removed_events(feed_changed),
        ))
    }
}

/// Removes one or more cached local files and clears their local DB paths.
#[derive(Clone, Debug)]
pub struct RemoveCachedFiles {
    conn: SharedConnection,
    paths: Vec<String>,
}

impl RemoveCachedFiles {
    /// Creates a cached-file removal command.
    #[must_use]
    pub fn new(conn: SharedConnection, paths: Vec<String>) -> Self {
        Self { conn, paths }
    }
}

impl ApplicationCommand for RemoveCachedFiles {
    type Output = RemoveCachedFilesResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| download_lock_error())?;
        let mut removed_count = 0;
        for path in self.paths {
            if context.cancellation().is_cancelled() {
                return Err(CommandError::Cancelled);
            }
            library_service::delete_cached_file(&conn, &path)
                .map_err(|error| download_command_error(&error))?;
            removed_count += 1;
        }
        Ok(CommandOutcome::new(
            RemoveCachedFilesResult::new(removed_count),
            track_removed_events(false),
        ))
    }
}

fn track_removed_events(feed_changed: bool) -> Vec<ApplicationEvent> {
    let mut events = vec![
        ApplicationEvent::Download(DownloadEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ];
    if feed_changed {
        events.push(ApplicationEvent::Feed(FeedEvent::Changed));
    }
    events
}

fn download_changed_events() -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Download(DownloadEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
        ApplicationEvent::Feed(FeedEvent::Changed),
    ]
}

fn playlist_download_changed_events(playlist_id: i64) -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Download(DownloadEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
        ApplicationEvent::Feed(FeedEvent::Changed),
        ApplicationEvent::Playlist(PlaylistEvent::TracksChanged { playlist_id }),
    ]
}

fn download_lock_error() -> CommandError {
    CommandError::Download("database lock poisoned".to_string())
}

fn download_command_error(error: &anyhow::Error) -> CommandError {
    CommandError::Download(format!("{error:#}"))
}

fn download_error(error: DownloadError) -> CommandError {
    match error {
        DownloadError::Cancelled => CommandError::Cancelled,
        DownloadError::Failed(message) => CommandError::Download(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::api::Track;
    use crate::application::command_bus::CommandBus;
    use crate::application::command_context::CommandContext;
    use crate::application::ports::download_manager::{DownloadOutcome, DownloadRequest};
    use crate::db;
    use crate::metadata::TrackContext;

    fn setup_test_db() -> anyhow::Result<SharedConnection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    fn create_feed(conn: &Connection, feed_url: &str) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title, is_subscribed)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![feed_url, "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_library_track(
        conn: &Connection,
        feed_id: i64,
        item_guid: &str,
        enclosure_url: &str,
    ) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, enclosure_url, track_title, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, 1)",
            rusqlite::params![feed_id, item_guid, enclosure_url, "Track Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[derive(Debug)]
    struct FakeDownloadManager;

    impl DownloadManager for FakeDownloadManager {
        fn download(
            &self,
            _request: DownloadRequest,
            _context: &CommandContext,
        ) -> Result<DownloadOutcome, DownloadError> {
            Ok(DownloadOutcome::new(PathBuf::from("/tmp/fake.mp3")))
        }

        fn subscribe_track(
            &self,
            _conn: SharedConnection,
            _request: SubscribeTrackRequest,
            _context: &CommandContext,
        ) -> Result<SubscribeTrackOutcome, DownloadError> {
            Ok(SubscribeTrackOutcome {
                path: PathBuf::from("/tmp/fake.mp3"),
                format_warning: Some("format warning".to_string()),
                applied_edits: 2,
                marked_downloaded: true,
                compare: None,
            })
        }

        fn subscribe_feed(
            &self,
            _conn: SharedConnection,
            _request: crate::subscribe_service::SubscribeFeedRequest,
            _context: &CommandContext,
        ) -> Result<crate::subscribe_service::SubscribeFeedOutcome, DownloadError> {
            Err(DownloadError::Failed("not used".to_string()))
        }

        fn subscribe_then_append_to_playlist(
            &self,
            _conn: SharedConnection,
            _playlist_id: i64,
            _track_ids: Vec<i64>,
            _context: &CommandContext,
        ) -> Result<AppendToPlaylistOutcome, DownloadError> {
            Ok(AppendToPlaylistOutcome {
                appended: 2,
                downloaded: 1,
                already_in_library: 1,
                failed: vec!["skip".to_string()],
            })
        }
    }

    #[test]
    fn remove_track_by_id_updates_library_state() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let track_id = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            create_library_track(&db, feed_id, "item-guid", "https://cdn.test/audio.mp3")?
        };

        let outcome = CommandBus::new().execute(
            RemoveTrackFromLibrary::new(Arc::clone(&conn), track_id),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().message(), "Removed track");
        assert_eq!(outcome.events(), track_removed_events(true));
        let db = conn.lock().expect("lock test db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(!track.is_in_library);
        assert!(!db::feed_is_subscribed_by_url(
            &db,
            "https://example.test/feed.xml"
        )?);

        Ok(())
    }

    #[test]
    fn remove_cached_files_deletes_files_and_clears_cached_rows() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("artist").join("album").join("track.mp3");
        std::fs::create_dir_all(path.parent().expect("path has parent"))?;
        std::fs::write(&path, b"audio")?;
        let path_string = path.display().to_string();
        let track_id = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            let track_id =
                create_library_track(&db, feed_id, "item-guid", "https://cdn.test/audio.mp3")?;
            library_service::mark_track_downloaded(&db, track_id, &path, None)?;
            library_service::set_track_in_library(&db, track_id, false)?;
            track_id
        };

        let outcome = CommandBus::new().execute(
            RemoveCachedFiles::new(Arc::clone(&conn), vec![path_string]),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().removed_count(), 1);
        assert_eq!(outcome.value().message(), "Deleted 1 cached file");
        assert_eq!(outcome.events(), track_removed_events(false));
        assert!(!path.exists());
        let db = conn.lock().expect("lock test db");
        assert!(library_service::cached_tracks(&db)?.is_empty());
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(track.local_path.is_none());

        Ok(())
    }

    #[test]
    fn remove_cached_files_honors_cancelled_context() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let context = CommandContext::next();
        context.cancellation().cancel();

        let error = CommandBus::new()
            .execute(
                RemoveCachedFiles::new(Arc::clone(&conn), vec!["/tmp/track.mp3".to_string()]),
                &context,
            )
            .expect_err("cancelled command should fail");

        assert_eq!(error, CommandError::Cancelled);

        Ok(())
    }

    #[test]
    fn subscribe_track_uses_download_manager_port_and_emits_events() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let request = SubscribeTrackRequest::SearchTrack {
            track_context: Box::new(TrackContext {
                track: Track::default(),
                feed: None,
            }),
            edits: Vec::new(),
            musicindex_endpoint: "https://api.example.test".to_string(),
            mark_feed_subscribed: false,
            return_tag_compare: false,
        };

        let outcome = CommandBus::new().execute(
            SubscribeTrack::new(
                Arc::clone(&conn),
                Arc::new(FakeDownloadManager),
                request,
                "Downloaded track",
            ),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().path(), "/tmp/fake.mp3");
        assert_eq!(outcome.value().format_warning(), Some("format warning"));
        assert_eq!(outcome.value().applied_edits(), 2);
        assert!(outcome.value().marked_downloaded());
        assert_eq!(outcome.value().message(), "Downloaded track");
        assert_eq!(outcome.events(), download_changed_events());

        Ok(())
    }

    #[test]
    fn subscribe_then_append_uses_download_manager_port_and_emits_events() -> anyhow::Result<()> {
        let conn = setup_test_db()?;

        let outcome = CommandBus::new().execute(
            SubscribeThenAppendToPlaylist::new(
                Arc::clone(&conn),
                Arc::new(FakeDownloadManager),
                42,
                vec![1, 2],
            ),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().appended(), 2);
        assert_eq!(outcome.value().downloaded(), 1);
        assert_eq!(outcome.value().already_in_library(), 1);
        assert_eq!(outcome.value().failed(), &["skip".to_string()]);
        assert_eq!(outcome.events(), playlist_download_changed_events(42));

        Ok(())
    }

    #[test]
    fn set_track_membership_can_subscribe_local_track() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let track_id = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            let track_id =
                create_library_track(&db, feed_id, "item-guid", "https://cdn.test/audio.mp3")?;
            library_service::set_track_in_library(&db, track_id, false)?;
            track_id
        };

        let outcome = CommandBus::new().execute(
            SetTrackLibraryMembership::new(Arc::clone(&conn), track_id, true),
            &CommandContext::next(),
        )?;

        assert!(outcome.value().in_library());
        assert_eq!(outcome.value().message(), "Subscribed track");
        assert_eq!(
            outcome.events(),
            &[ApplicationEvent::Library(LibraryEvent::Changed)]
        );
        let db = conn.lock().expect("lock test db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(track.is_in_library);

        Ok(())
    }

    #[test]
    fn remove_track_by_match_reconciles_feed_subscription() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let track_id = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            create_library_track(&db, feed_id, "item-guid", "https://cdn.test/audio.mp3")?
        };

        let outcome = CommandBus::new().execute(
            RemoveTrackFromLibraryByMatch::new(
                Arc::clone(&conn),
                Some("https://example.test/feed.xml".to_string()),
                Some("item-guid".to_string()),
                Some("https://cdn.test/audio.mp3".to_string()),
            ),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().message(), "Removed track");
        assert_eq!(outcome.events(), track_removed_events(true));
        let db = conn.lock().expect("lock test db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(!track.is_in_library);
        assert!(!db::feed_is_subscribed_by_url(
            &db,
            "https://example.test/feed.xml"
        )?);

        Ok(())
    }
}
