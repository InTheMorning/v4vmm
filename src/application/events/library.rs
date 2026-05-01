//! Library application events.

/// State-change event for library read models.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LibraryEvent {
    /// The library snapshot should be refreshed.
    Changed,
}
