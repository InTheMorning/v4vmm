//! Application-wide view-model event bus (ADR 0040).
//!
//! `VmBus` is the fan-out channel actors use to broadcast invalidation
//! and identity-level updates. It is intentionally cheap (cloneable
//! handle around a `tokio::sync::broadcast`) so any actor can subscribe
//! without coordinating with every producer.
//!
//! `VmEvent` is the canonical envelope carried by the bus. Variants are
//! intentionally coarse — actors translate them into table-specific
//! invalidations on receive (see ADR 0041 paged-VM contract).

#![warn(clippy::pedantic)]

use tokio::sync::broadcast;

const DEFAULT_CAPACITY: usize = 256;

/// Coarse-grained invalidation envelope broadcast across the runtime.
#[derive(Debug, Clone)]
pub enum VmEvent {
    /// A specific track row changed (downloaded, retagged, removed).
    TrackChanged { track_id: i64 },
    /// A feed row changed (subscribed, removed, refreshed).
    FeedChanged { feed_id: i64 },
    /// A playlist row changed (created, renamed, items updated).
    PlaylistChanged { playlist_id: i64 },
    /// A bulk operation completed; consumers should drop caches.
    InvalidateAll,
}

/// Cloneable handle to the runtime-wide broadcast channel.
#[derive(Debug, Clone)]
#[must_use]
pub struct VmBus {
    sender: broadcast::Sender<VmEvent>,
}

impl VmBus {
    /// Create a new bus with the default capacity.
    pub fn new() -> Self {
        let (sender, _receiver) = broadcast::channel(DEFAULT_CAPACITY);
        Self { sender }
    }

    /// Create a new bus with a specific channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event. Returns `true` when at least one receiver is live.
    /// Producers commonly fire-and-forget; the boolean is informational.
    #[allow(clippy::must_use_candidate)]
    pub fn publish(&self, event: VmEvent) -> bool {
        self.sender.send(event).is_ok()
    }

    /// Acquire a fresh receiver. Receivers see only events published
    /// after they subscribe.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<VmEvent> {
        self.sender.subscribe()
    }
}

impl Default for VmBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_with_no_subscribers_is_noop() {
        let bus = VmBus::new();
        assert!(!bus.publish(VmEvent::InvalidateAll));
    }

    #[tokio::test]
    async fn subscriber_receives_published_events() {
        let bus = VmBus::new();
        let mut rx = bus.subscribe();
        assert!(bus.publish(VmEvent::TrackChanged { track_id: 7 }));
        let event = rx.recv().await.expect("receive");
        match event {
            VmEvent::TrackChanged { track_id } => assert_eq!(track_id, 7),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn late_subscriber_misses_earlier_events() {
        let bus = VmBus::new();
        bus.publish(VmEvent::InvalidateAll);
        let mut rx = bus.subscribe();
        assert!(bus.publish(VmEvent::FeedChanged { feed_id: 3 }));
        let event = rx.recv().await.expect("receive");
        assert!(matches!(event, VmEvent::FeedChanged { feed_id: 3 }));
    }
}
