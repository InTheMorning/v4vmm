//! Typed application events.

pub mod download;
pub mod feed;
pub mod library;
pub mod metadata;
pub mod playback;
pub mod playlist;

/// State-change event emitted by an application command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationEvent {
    /// Library state changed.
    Library(library::LibraryEvent),
    /// Playlist state changed.
    Playlist(playlist::PlaylistEvent),
    /// Feed state changed.
    Feed(feed::FeedEvent),
    /// Track download state changed.
    Download(download::DownloadEvent),
    /// Metadata staging or provenance state changed.
    Metadata(metadata::MetadataEvent),
    /// Playback state changed.
    Playback(playback::PlaybackEvent),
}
