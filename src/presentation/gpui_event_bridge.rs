//! GPUI presentation bridge for application events.

use std::fmt;
use std::sync::Mutex;

use gpui::Context;

use crate::application::application_event_bus::ApplicationEventSubscriber;
use crate::application::events::ApplicationEvent;
use crate::presentation::event_bridge::PresentationEventBridge;

/// Queues application events for later GPUI-thread draining.
#[derive(Default)]
pub struct GpuiEventBridge {
    pending_events: Mutex<Vec<ApplicationEvent>>,
}

impl GpuiEventBridge {
    /// Creates an empty GPUI event bridge.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Drains queued events on the GPUI thread and notifies the entity.
    ///
    /// # Panics
    ///
    /// Panics if the pending-event lock is poisoned.
    pub fn drain_for<T: 'static>(
        &self,
        target: &mut T,
        cx: &mut Context<T>,
        mut apply_event: impl FnMut(&mut T, &ApplicationEvent, &mut Context<T>),
    ) {
        let events = self.take_pending_events();
        if events.is_empty() {
            return;
        }
        for event in &events {
            apply_event(target, event, cx);
        }
        cx.notify();
    }

    fn take_pending_events(&self) -> Vec<ApplicationEvent> {
        let mut pending = self
            .pending_events
            .lock()
            .expect("GPUI event bridge lock poisoned");
        std::mem::take(&mut *pending)
    }
}

impl ApplicationEventSubscriber for GpuiEventBridge {
    fn on_application_events(&self, events: &[ApplicationEvent]) {
        let mut pending = self
            .pending_events
            .lock()
            .expect("GPUI event bridge lock poisoned");
        pending.extend_from_slice(events);
    }
}

impl PresentationEventBridge for GpuiEventBridge {
    fn pending_event_count(&self) -> usize {
        self.pending_events
            .lock()
            .expect("GPUI event bridge lock poisoned")
            .len()
    }
}

impl fmt::Debug for GpuiEventBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpuiEventBridge")
            .field("pending_event_count", &self.pending_event_count())
            .finish()
    }
}
