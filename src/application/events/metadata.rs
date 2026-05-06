//! Metadata application events.

/// State-change event for metadata staging and provenance state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataEvent {
    /// Coarse-grained metadata change (target track unknown or
    /// fan-out spans many tracks, e.g. a feed refresh).
    Changed,
    /// Metadata for one known library track changed (e.g. a
    /// `MusicBrainz` candidate was staged for that track). Carries
    /// the track id so paged VMs can drop a single row instead of
    /// invalidating the whole list.
    TrackTagged {
        /// Library track id whose metadata changed.
        track_id: i64,
    },
}
