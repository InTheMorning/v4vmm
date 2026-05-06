//! Playback application events.

/// State-change event for playback state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlaybackEvent {
    /// Playback state changed.
    Changed,
}
