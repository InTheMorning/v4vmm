//! Shared command error channel.

use std::fmt;

/// Error returned by command execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandError {
    /// Playlist command failed.
    Playlist(String),
    /// Feed command failed.
    Feed(String),
    /// Download command failed.
    Download(String),
    /// Metadata command failed.
    Metadata(String),
    /// Playback command failed.
    Playback(String),
    /// Query refresh required by a command failed.
    Query(String),
    /// Command was cancelled before completion.
    Cancelled,
    /// Command failed outside a narrower family.
    Other(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Playlist(message) => write!(f, "playlist command failed: {message}"),
            Self::Feed(message) => write!(f, "feed command failed: {message}"),
            Self::Download(message) => write!(f, "download command failed: {message}"),
            Self::Metadata(message) => write!(f, "metadata command failed: {message}"),
            Self::Playback(message) => write!(f, "playback command failed: {message}"),
            Self::Query(message) => write!(f, "query refresh failed: {message}"),
            Self::Cancelled => f.write_str("command cancelled"),
            Self::Other(message) => write!(f, "command failed: {message}"),
        }
    }
}

impl std::error::Error for CommandError {}
