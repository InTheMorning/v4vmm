//! Playlist application events.

/// State-change event for playlist read models.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlaylistEvent {
    /// One or more playlists changed.
    Changed,
}
