//! Feed command family.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::download::DownloadEvent;
use crate::application::events::feed::FeedEvent;
use crate::application::events::library::LibraryEvent;
use crate::application::events::ApplicationEvent;
use crate::application::ports::download_manager::{DownloadError, DownloadManager};
use crate::db;
use crate::subscribe_service::{SubscribeFeedOutcome, SubscribeFeedRequest};

type SharedConnection = Arc<Mutex<Connection>>;

/// Command result for subscribing/downloading a feed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscribeFeedResult {
    downloaded: usize,
    applied_edits: usize,
    skipped: usize,
    message: String,
}

impl SubscribeFeedResult {
    /// Creates a feed subscription result from the service outcome.
    #[must_use]
    pub fn from_outcome(outcome: &SubscribeFeedOutcome) -> Self {
        let message = if outcome.skipped == 0 {
            format!(
                "Downloaded feed; downloaded {} track{}, applied {} ID3 edit{}",
                outcome.downloaded,
                plural(outcome.downloaded),
                outcome.applied_edits,
                plural(outcome.applied_edits)
            )
        } else {
            format!(
                "Downloaded feed; downloaded {} track{}, applied {} ID3 edit{}, skipped {}",
                outcome.downloaded,
                plural(outcome.downloaded),
                outcome.applied_edits,
                plural(outcome.applied_edits),
                outcome.skipped
            )
        };
        Self {
            downloaded: outcome.downloaded,
            applied_edits: outcome.applied_edits,
            skipped: outcome.skipped,
            message,
        }
    }

    /// Returns how many tracks were downloaded.
    #[must_use]
    pub const fn downloaded(&self) -> usize {
        self.downloaded
    }

    /// Returns how many ID3 edits were applied.
    #[must_use]
    pub const fn applied_edits(&self) -> usize {
        self.applied_edits
    }

    /// Returns how many tracks were skipped.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Subscribes/downloads one feed.
pub struct SubscribeFeed {
    conn: SharedConnection,
    download_manager: Arc<dyn DownloadManager>,
    request: SubscribeFeedRequest,
}

impl SubscribeFeed {
    /// Creates a feed subscription command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        download_manager: Arc<dyn DownloadManager>,
        request: SubscribeFeedRequest,
    ) -> Self {
        Self {
            conn,
            download_manager,
            request,
        }
    }
}

impl ApplicationCommand for SubscribeFeed {
    type Output = SubscribeFeedResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let outcome = self
            .download_manager
            .subscribe_feed(self.conn, self.request, context)
            .map_err(feed_download_error)?;
        Ok(CommandOutcome::new(
            SubscribeFeedResult::from_outcome(&outcome),
            feed_download_changed_events(),
        ))
    }
}

/// Command result for removing a feed subscription.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsubscribeFeedResult {
    message: String,
}

impl UnsubscribeFeedResult {
    /// Creates an unsubscribe result.
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

/// Removes a local feed subscription by local feed id.
#[derive(Clone, Debug)]
pub struct UnsubscribeFeedById {
    conn: SharedConnection,
    feed_id: i64,
}

impl UnsubscribeFeedById {
    /// Creates a feed unsubscribe command.
    #[must_use]
    pub const fn new(conn: SharedConnection, feed_id: i64) -> Self {
        Self { conn, feed_id }
    }
}

impl ApplicationCommand for UnsubscribeFeedById {
    type Output = UnsubscribeFeedResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| feed_lock_error())?;
        db::set_feed_subscribed(&conn, self.feed_id, false)
            .map_err(|error| feed_command_error(&error))?;
        db::unsubscribe_feed_tracks(&conn, self.feed_id)
            .map_err(|error| feed_command_error(&error))?;
        Ok(CommandOutcome::new(
            UnsubscribeFeedResult::new("Removed feed"),
            feed_changed_events(),
        ))
    }
}

/// Removes a local feed subscription by feed URL.
#[derive(Clone, Debug)]
pub struct UnsubscribeFeedByUrl {
    conn: SharedConnection,
    feed_url: Option<String>,
}

impl UnsubscribeFeedByUrl {
    /// Creates a feed unsubscribe command.
    #[must_use]
    pub fn new(conn: SharedConnection, feed_url: Option<String>) -> Self {
        Self { conn, feed_url }
    }
}

impl ApplicationCommand for UnsubscribeFeedByUrl {
    type Output = UnsubscribeFeedResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let feed_url = self
            .feed_url
            .ok_or_else(|| CommandError::Feed("feed has no RSS URL".to_string()))?;
        let conn = self.conn.lock().map_err(|_| feed_lock_error())?;
        db::set_feed_subscribed_by_url(&conn, &feed_url, false)
            .map_err(|error| feed_command_error(&error))?;
        Ok(CommandOutcome::new(
            UnsubscribeFeedResult::new("Removed feed"),
            feed_changed_events(),
        ))
    }
}

fn feed_changed_events() -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Feed(FeedEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ]
}

fn feed_download_changed_events() -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Feed(FeedEvent::Changed),
        ApplicationEvent::Download(DownloadEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ]
}

fn feed_lock_error() -> CommandError {
    CommandError::Feed("database lock poisoned".to_string())
}

fn feed_command_error(error: &anyhow::Error) -> CommandError {
    CommandError::Feed(format!("{error:#}"))
}

fn feed_download_error(error: DownloadError) -> CommandError {
    match error {
        DownloadError::Cancelled => CommandError::Cancelled,
        DownloadError::Failed(message) => CommandError::Feed(message),
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::application::command_bus::CommandBus;
    use crate::application::command_context::CommandContext;
    use crate::application::ports::download_manager::{DownloadOutcome, DownloadRequest};
    use crate::library_service::AppendToPlaylistOutcome;
    use crate::subscribe_service::SubscribeTrackOutcome;

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

    fn create_library_track(conn: &Connection, feed_id: i64) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, is_in_library)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![feed_id, "item-guid", "Track Title"],
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
            _request: crate::subscribe_service::SubscribeTrackRequest,
            _context: &CommandContext,
        ) -> Result<SubscribeTrackOutcome, DownloadError> {
            Err(DownloadError::Failed("not used".to_string()))
        }

        fn subscribe_feed(
            &self,
            _conn: SharedConnection,
            _request: SubscribeFeedRequest,
            _context: &CommandContext,
        ) -> Result<SubscribeFeedOutcome, DownloadError> {
            Ok(SubscribeFeedOutcome {
                downloaded: 2,
                applied_edits: 3,
                skipped: 1,
            })
        }

        fn subscribe_then_append_to_playlist(
            &self,
            _conn: SharedConnection,
            _playlist_id: i64,
            _track_ids: Vec<i64>,
            _context: &CommandContext,
        ) -> Result<AppendToPlaylistOutcome, DownloadError> {
            Err(DownloadError::Failed("not used".to_string()))
        }
    }

    #[test]
    fn subscribe_feed_uses_download_manager_port_and_emits_events() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let request = SubscribeFeedRequest {
            feed: crate::api::Feed::default(),
            musicindex_endpoint: "https://api.example.test".to_string(),
        };

        let outcome = CommandBus::new().execute(
            SubscribeFeed::new(Arc::clone(&conn), Arc::new(FakeDownloadManager), request),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().downloaded(), 2);
        assert_eq!(outcome.value().applied_edits(), 3);
        assert_eq!(outcome.value().skipped(), 1);
        assert_eq!(
            outcome.value().message(),
            "Downloaded feed; downloaded 2 tracks, applied 3 ID3 edits, skipped 1"
        );
        assert_eq!(outcome.events(), feed_download_changed_events());

        Ok(())
    }

    #[test]
    fn unsubscribe_feed_by_id_removes_feed_and_track_library_state() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let (feed_id, track_id) = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            let track_id = create_library_track(&db, feed_id)?;
            (feed_id, track_id)
        };

        let outcome = CommandBus::new().execute(
            UnsubscribeFeedById::new(Arc::clone(&conn), feed_id),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().message(), "Removed feed");
        assert_eq!(outcome.events(), feed_changed_events());
        let db = conn.lock().expect("lock test db");
        assert!(!db::feed_is_subscribed_by_url(
            &db,
            "https://example.test/feed.xml"
        )?);
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(!track.is_in_library);

        Ok(())
    }

    #[test]
    fn unsubscribe_feed_by_url_requires_url() {
        let conn = setup_test_db().expect("test db");

        let error = CommandBus::new()
            .execute(
                UnsubscribeFeedByUrl::new(Arc::clone(&conn), None),
                &CommandContext::next(),
            )
            .expect_err("missing feed URL should fail");

        assert!(
            error.to_string().contains("feed has no RSS URL"),
            "unexpected error: {error}"
        );
    }
}
