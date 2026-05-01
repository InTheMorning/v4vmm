//! Metadata application events.

/// State-change event for metadata staging and provenance state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataEvent {
    /// Metadata state changed.
    Changed,
}
