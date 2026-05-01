//! Replaceable download boundary.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::command_context::CommandContext;
use crate::library_service::{self, AppendToPlaylistOutcome};
use crate::subscribe_service::{
    self, SubscribeFeedOutcome, SubscribeFeedRequest, SubscribeTrackOutcome, SubscribeTrackRequest,
};

/// Request passed to a download manager implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadRequest {
    source_url: String,
    destination: PathBuf,
}

impl DownloadRequest {
    /// Creates a download request.
    #[must_use]
    pub fn new(source_url: impl Into<String>, destination: PathBuf) -> Self {
        Self {
            source_url: source_url.into(),
            destination,
        }
    }

    /// Returns the source URL.
    #[must_use]
    pub fn source_url(&self) -> &str {
        &self.source_url
    }

    /// Returns the destination path.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }
}

/// Result returned by a download manager implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadOutcome {
    path: PathBuf,
}

impl DownloadOutcome {
    /// Creates a download outcome for a local path.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Returns the downloaded file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Error returned by a download manager implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DownloadError {
    /// Download was cancelled.
    Cancelled,
    /// Download failed with implementation-specific detail.
    Failed(String),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("download cancelled"),
            Self::Failed(message) => write!(f, "download failed: {message}"),
        }
    }
}

impl std::error::Error for DownloadError {}

/// Replaceable port for subscription and download work.
pub trait DownloadManager: fmt::Debug + Send + Sync + 'static {
    /// Downloads media according to the request and command context.
    ///
    /// # Errors
    ///
    /// Returns an error when the download fails or is cancelled.
    fn download(
        &self,
        request: DownloadRequest,
        context: &CommandContext,
    ) -> Result<DownloadOutcome, DownloadError>;

    /// Subscribes/downloads a track using the active download implementation.
    ///
    /// # Errors
    ///
    /// Returns an error when subscription, download, tagging, or local state
    /// update fails.
    fn subscribe_track(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: SubscribeTrackRequest,
        context: &CommandContext,
    ) -> Result<SubscribeTrackOutcome, DownloadError>;

    /// Subscribes/downloads a feed using the active download implementation.
    ///
    /// # Errors
    ///
    /// Returns an error when feed subscription, download, tagging, or local
    /// state update fails.
    fn subscribe_feed(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: SubscribeFeedRequest,
        context: &CommandContext,
    ) -> Result<SubscribeFeedOutcome, DownloadError>;

    /// Subscribes/downloads missing tracks and appends them to a playlist.
    ///
    /// # Errors
    ///
    /// Returns an error when subscription/download or playlist append fails.
    fn subscribe_then_append_to_playlist(
        &self,
        conn: Arc<Mutex<Connection>>,
        playlist_id: i64,
        track_ids: Vec<i64>,
        context: &CommandContext,
    ) -> Result<AppendToPlaylistOutcome, DownloadError>;
}

/// Download adapter backed by today's service modules.
#[derive(Clone, Copy, Debug, Default)]
pub struct ServiceDownloadManager;

impl ServiceDownloadManager {
    /// Creates a service-backed download manager.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl DownloadManager for ServiceDownloadManager {
    fn download(
        &self,
        _request: DownloadRequest,
        _context: &CommandContext,
    ) -> Result<DownloadOutcome, DownloadError> {
        Err(DownloadError::Failed(
            "raw download requests are not implemented".to_string(),
        ))
    }

    fn subscribe_track(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: SubscribeTrackRequest,
        context: &CommandContext,
    ) -> Result<SubscribeTrackOutcome, DownloadError> {
        if context.cancellation().is_cancelled() {
            return Err(DownloadError::Cancelled);
        }
        subscribe_service::subscribe_track(conn, request).map_err(|error| download_failed(&error))
    }

    fn subscribe_feed(
        &self,
        conn: Arc<Mutex<Connection>>,
        request: SubscribeFeedRequest,
        context: &CommandContext,
    ) -> Result<SubscribeFeedOutcome, DownloadError> {
        if context.cancellation().is_cancelled() {
            return Err(DownloadError::Cancelled);
        }
        subscribe_service::subscribe_feed(conn, request).map_err(|error| download_failed(&error))
    }

    fn subscribe_then_append_to_playlist(
        &self,
        conn: Arc<Mutex<Connection>>,
        playlist_id: i64,
        track_ids: Vec<i64>,
        context: &CommandContext,
    ) -> Result<AppendToPlaylistOutcome, DownloadError> {
        if context.cancellation().is_cancelled() {
            return Err(DownloadError::Cancelled);
        }
        library_service::subscribe_then_append_to_playlist(conn, playlist_id, track_ids)
            .map_err(|error| download_failed(&error))
    }
}

/// Placeholder download port used before ADR 0024 Task 004 wires downloads.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableDownloadManager;

impl UnavailableDownloadManager {
    /// Creates an unavailable download manager.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl DownloadManager for UnavailableDownloadManager {
    fn download(
        &self,
        _request: DownloadRequest,
        _context: &CommandContext,
    ) -> Result<DownloadOutcome, DownloadError> {
        Err(DownloadError::Failed(
            "download manager is not configured".to_string(),
        ))
    }

    fn subscribe_track(
        &self,
        _conn: Arc<Mutex<Connection>>,
        _request: SubscribeTrackRequest,
        _context: &CommandContext,
    ) -> Result<SubscribeTrackOutcome, DownloadError> {
        Err(DownloadError::Failed(
            "download manager is not configured".to_string(),
        ))
    }

    fn subscribe_feed(
        &self,
        _conn: Arc<Mutex<Connection>>,
        _request: SubscribeFeedRequest,
        _context: &CommandContext,
    ) -> Result<SubscribeFeedOutcome, DownloadError> {
        Err(DownloadError::Failed(
            "download manager is not configured".to_string(),
        ))
    }

    fn subscribe_then_append_to_playlist(
        &self,
        _conn: Arc<Mutex<Connection>>,
        _playlist_id: i64,
        _track_ids: Vec<i64>,
        _context: &CommandContext,
    ) -> Result<AppendToPlaylistOutcome, DownloadError> {
        Err(DownloadError::Failed(
            "download manager is not configured".to_string(),
        ))
    }
}

fn download_failed(error: &anyhow::Error) -> DownloadError {
    DownloadError::Failed(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::api::{Feed, Track};
    use crate::application::command_context::{CancellationToken, OperationId, TraceId};
    use crate::metadata::TrackContext;

    fn cancelled_context() -> CommandContext {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        CommandContext::new(OperationId::new(1), cancellation, TraceId::new(1))
    }

    fn test_connection() -> Arc<Mutex<Connection>> {
        Arc::new(Mutex::new(
            Connection::open_in_memory().expect("open test database"),
        ))
    }

    #[test]
    fn service_download_manager_honors_cancelled_context() {
        let manager = ServiceDownloadManager::new();
        let context = cancelled_context();
        let track_request = SubscribeTrackRequest::SearchTrack {
            track_context: Box::new(TrackContext {
                track: Track::default(),
                feed: None,
            }),
            edits: Vec::new(),
            musicindex_endpoint: "https://api.example.test".to_string(),
            mark_feed_subscribed: false,
            return_tag_compare: false,
        };
        let feed_request = SubscribeFeedRequest {
            feed: Feed::default(),
            musicindex_endpoint: "https://api.example.test".to_string(),
        };

        assert!(matches!(
            manager.subscribe_track(test_connection(), track_request, &context),
            Err(DownloadError::Cancelled)
        ));
        assert!(matches!(
            manager.subscribe_feed(test_connection(), feed_request, &context),
            Err(DownloadError::Cancelled)
        ));
        assert!(matches!(
            manager.subscribe_then_append_to_playlist(test_connection(), 1, vec![1], &context),
            Err(DownloadError::Cancelled)
        ));
    }
}
