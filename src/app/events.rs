//! Top-level application event bridge for presentation refreshes.

use gpui::{Context, Window};

use crate::application::ApplicationEvent;
use crate::library::LibraryApp;
use crate::presentation::PresentationEventBridge;
use crate::search::SearchApp;

use super::TopApp;

impl TopApp {
    pub(super) fn defer_application_event_drain(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.application_event_bridge.pending_event_count() == 0 {
            return;
        }
        cx.defer_in(window, |this, _window, cx| {
            this.drain_application_events(cx);
        });
    }

    fn drain_application_events(&mut self, cx: &mut Context<Self>) {
        let events = self.application_event_bridge.drain_events();
        if events.is_empty() {
            return;
        }
        if events.iter().any(affects_library_surfaces) {
            self.reload_cached();
            self.library.update(cx, LibraryApp::refresh);
            self.search.update(cx, SearchApp::refresh_application_state);
        }
        cx.notify();
    }
}

fn affects_library_surfaces(event: &ApplicationEvent) -> bool {
    matches!(
        event,
        ApplicationEvent::Library(_)
            | ApplicationEvent::Playlist(_)
            | ApplicationEvent::Feed(_)
            | ApplicationEvent::Download(_)
            | ApplicationEvent::Metadata(_)
    )
}
