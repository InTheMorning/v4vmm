//! App-scoped application event broadcast.

use std::fmt;
use std::sync::{Arc, RwLock};

use crate::application::events::ApplicationEvent;

/// Subscriber for app-scoped application events.
pub trait ApplicationEventSubscriber: Send + Sync + 'static {
    /// Handles an application event batch.
    fn on_application_events(&self, events: &[ApplicationEvent]);
}

/// In-process broadcaster for application event batches.
#[derive(Default)]
pub struct ApplicationEventBus {
    subscribers: RwLock<Vec<Arc<dyn ApplicationEventSubscriber>>>,
}

impl ApplicationEventBus {
    /// Creates an empty event bus.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an app-scoped event subscriber.
    ///
    /// # Panics
    ///
    /// Panics if the subscriber lock is poisoned.
    pub fn subscribe(&self, subscriber: Arc<dyn ApplicationEventSubscriber>) {
        let mut subscribers = self
            .subscribers
            .write()
            .expect("application event subscribers lock poisoned");
        subscribers.push(subscriber);
    }

    /// Broadcasts an event batch to all current subscribers.
    ///
    /// # Panics
    ///
    /// Panics if the subscriber lock is poisoned.
    pub fn broadcast(&self, events: &[ApplicationEvent]) {
        let subscribers = self
            .subscribers
            .read()
            .expect("application event subscribers lock poisoned")
            .clone();
        for subscriber in subscribers {
            subscriber.on_application_events(events);
        }
    }

    /// Returns the number of registered subscribers.
    ///
    /// # Panics
    ///
    /// Panics if the subscriber lock is poisoned.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .read()
            .expect("application event subscribers lock poisoned")
            .len()
    }
}

impl fmt::Debug for ApplicationEventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApplicationEventBus")
            .field("subscriber_count", &self.subscriber_count())
            .finish()
    }
}
