//! Download application events.

/// State-change event for track download state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DownloadEvent {
    /// Download state changed.
    Changed,
}
