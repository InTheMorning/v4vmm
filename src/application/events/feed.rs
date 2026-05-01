//! Feed application events.

/// State-change event for feed subscription or update state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeedEvent {
    /// Feed state changed.
    Changed,
}
