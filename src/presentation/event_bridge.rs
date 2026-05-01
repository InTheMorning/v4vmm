//! UI-neutral presentation event bridge contract.

use crate::application::application_event_bus::ApplicationEventSubscriber;

/// Presentation-side bridge from application events to a UI runtime.
pub trait PresentationEventBridge: ApplicationEventSubscriber {
    /// Returns queued events waiting for the presentation runtime.
    fn pending_event_count(&self) -> usize;
}
