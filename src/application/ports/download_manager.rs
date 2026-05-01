//! Replaceable download boundary.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::application::command_context::CommandContext;

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

/// Replaceable port for track download work.
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
}
